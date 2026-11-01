//! One-instance Windows handoff channel with logon-only client data permissions.
//!
//! Safe owned-handle conversions bypass the listener's replacement-instance
//! accept path. Client process metadata is used only to observe a connection;
//! it never authenticates a process. Noise pairing remains mandatory before
//! application authorization. Names and OS identities never enter diagnostics.
//! Writes reach the kernel without a user-space buffer. Flush is not a peer
//! receipt; application acknowledgements prove delivery. Drop discards unread
//! output instead of starting the library's blocking flush or linger workers.
//! Windows can retain the object briefly until cancelled I/O completions drain.

use std::{
    fs::OpenOptions,
    io,
    num::NonZeroU8,
    os::windows::{fs::OpenOptionsExt, io::OwnedHandle},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use interprocess::os::windows::{
    named_pipe::{
        PipeListenerOptions, PipeStream, pipe_mode, tokio::PipeStream as AsyncPipeStream,
    },
    security_descriptor::SecurityDescriptor,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    time::{Instant, sleep_until},
};
use widestring::U16CString;

use super::{
    ErrorCode, HandoffError,
    framing::before_deadline,
    pairing::check_pending_deadline,
    slot::{PendingLoginSlot, logon_sid},
};

/// Excludes FILE_CREATE_PIPE_INSTANCE, WRITE_DAC, and WRITE_OWNER from client access.
const CLIENT_ACCESS: u32 = 0x0012_019b;
/// Required by safe asynchronous pipe-handle conversion.
const OVERLAPPED: u32 = 0x4000_0000;
/// Reports no client from the initial GetNamedPipeClientProcessId query.
const CLIENT_NOT_FOUND: i32 = 1168;
/// Bounds metadata polling without extending the caller's pending expiry.
const CONNECT_WAIT: Duration = Duration::from_secs(10);
/// Yields between local metadata queries; no account or network request is retried.
const POLL_WAIT: Duration = Duration::from_millis(10);

type SyncPipe = PipeStream<pipe_mode::Bytes, pipe_mode::Bytes>;
type AsyncPipe = AsyncPipeStream<pipe_mode::Bytes, pipe_mode::Bytes>;

/// Retains conversion details and failed handle ownership behind a safe error boundary.
enum HandleFailure {
    /// Owns a failed synchronous conversion and any returned OS handle.
    Sync(interprocess::os::windows::named_pipe::FromHandleError),
    /// Owns a failed Tokio conversion and any returned OS handle.
    Async(interprocess::os::windows::named_pipe::tokio::FromHandleError),
}

impl std::fmt::Display for HandleFailure {
    /// Formats only the library's fixed conversion category, never the handle or raw cause.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sync(error) => write!(
                formatter,
                "synchronous pipe conversion failed ({:?})",
                error.details
            ),
            Self::Async(error) => write!(
                formatter,
                "asynchronous pipe conversion failed ({:?})",
                error.details
            ),
        }
    }
}

impl std::fmt::Debug for HandleFailure {
    /// Stops Debug before the dependency's failed-handle and error fields.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

impl std::error::Error for HandleFailure {}

/// Selects only the current logon's product channel or disjoint fixture channel.
fn path(namespace: Option<&str>) -> Result<String, HandoffError> {
    let sid = logon_sid().map_err(|error| {
        HandoffError::with_io(ErrorCode::UnsupportedContract, io::Error::other(error))
    })?;
    match namespace {
        None => Ok(format!(r"\\.\pipe\beyond-gate-handoff-v1-{sid}")),
        Some(value) => Ok(format!(
            r"\\.\pipe\beyond-gate-fixture-handoff-v1-{sid}-{value}"
        )),
    }
}

/// Constructs a protected logon-only DACL without granting implicit owner control.
fn descriptor() -> Result<SecurityDescriptor, HandoffError> {
    let sid = logon_sid().map_err(|error| {
        HandoffError::with_io(ErrorCode::UnsupportedContract, io::Error::other(error))
    })?;
    let sddl = U16CString::from_str(format!(
        "D:P(D;;0xc0004;;;OW)(A;;0x{CLIENT_ACCESS:x};;;{sid})"
    ))
    .map_err(|error| HandoffError::io(io::Error::other(error)))?;
    SecurityDescriptor::deserialize(&sddl).map_err(HandoffError::io)
}

/// Owns the one initial server instance and borrows its OS-wide reservation.
///
/// A connected server cannot outlive its reservation:
///
/// ```compile_fail
/// use beyond_gate_core::auth::handoff::{
///     endpoint::WindowsEndpoint, slot::PendingLoginSlot,
/// };
/// use std::time::Duration;
/// use tokio::time::Instant;
///
/// async fn release_too_early() -> Result<(), Box<dyn std::error::Error>> {
///     let slot = PendingLoginSlot::claim()?;
///     let endpoint = WindowsEndpoint::bind(&slot, Instant::now() + Duration::from_secs(30))?;
///     let stream = endpoint.accept().await?;
///     drop(slot);
///     drop(stream);
///     Ok(())
/// }
/// ```
pub struct WindowsEndpoint<'a> {
    /// Allows consuming ownership transfer while Drop can close failed or cancelled waits.
    pipe: Option<SyncPipe>,
    /// Prevents release of the reservation before the server stream is dropped.
    reservation: &'a PendingLoginSlot,
    /// Caps connection readiness by the original pending lifetime.
    deadline: Instant,
}

impl std::fmt::Debug for WindowsEndpoint<'_> {
    /// Omits handles, paths, OS identity, and connection metadata.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WindowsEndpoint { channel: [REDACTED] }")
    }
}

impl<'a> WindowsEndpoint<'a> {
    /// Creates exactly one instance in the held reservation's namespace.
    ///
    /// # Errors
    ///
    /// Rejects invalid deadlines, missing logon information, DACL failures, and
    /// every existing or inaccessible endpoint. No foreign instance is adopted.
    pub fn bind(
        reservation: &'a PendingLoginSlot,
        deadline: Instant,
    ) -> Result<Self, HandoffError> {
        check_pending_deadline(deadline)?;
        let listener = PipeListenerOptions::new()
            .path(path(reservation.namespace())?)
            .instance_limit(Some(NonZeroU8::MIN))
            .accept_remote(false)
            .inheritable(false)
            .security_descriptor(Some(descriptor()?))
            .create_duplex::<pipe_mode::Bytes>()
            .map_err(|error| {
                let code = if error.raw_os_error() == Some(5) {
                    ErrorCode::Busy
                } else {
                    ErrorCode::Disconnected
                };
                HandoffError::with_io(code, error)
            })?;
        let handle: OwnedHandle = listener.into();
        let pipe = SyncPipe::try_from(handle)
            .map_err(|error| HandoffError::io(io::Error::other(HandleFailure::Sync(error))))?;
        pipe.assume_flushed();
        before_deadline(deadline)?;
        Ok(Self {
            pipe: Some(pipe),
            reservation,
            deadline,
        })
    }

    /// Consumes the waiting endpoint after one connection, without trusting its PID.
    ///
    /// # Errors
    ///
    /// Returns timeout, metadata, or safe conversion failures. Failure and future
    /// cancellation close the owned instance. The returned stream is not paired.
    pub async fn accept(self) -> Result<WindowsStream<'a>, HandoffError> {
        let deadline = self.deadline.min(Instant::now() + CONNECT_WAIT);
        self.wait_for_client(deadline).await
    }

    /// Waits for human key entry within the original five-minute pending lifetime.
    ///
    /// The subsequent cryptographic and hello phases retain their ten-second caps.
    ///
    /// # Errors
    ///
    /// Returns the same ownership-safe failures as `accept`, without a ten-second
    /// limit on the user's time to open the extension and enter the key.
    pub async fn accept_for_pairing(self) -> Result<WindowsStream<'a>, HandoffError> {
        let deadline = self.deadline;
        self.wait_for_client(deadline).await
    }

    /// Polls only local readiness while retaining the one original server instance.
    async fn wait_for_client(
        mut self,
        deadline: Instant,
    ) -> Result<WindowsStream<'a>, HandoffError> {
        loop {
            before_deadline(deadline)?;
            let pipe = self
                .pipe
                .as_ref()
                .ok_or_else(|| HandoffError::new(ErrorCode::Disconnected))?;
            match pipe.client_process_id() {
                Ok(value) if value != 0 => break,
                Err(error) if error.raw_os_error() == Some(CLIENT_NOT_FOUND) => {}
                Ok(_) => return Err(HandoffError::new(ErrorCode::ProtocolError)),
                Err(error) => return Err(HandoffError::io(error)),
            }
            sleep_until(deadline.min(Instant::now() + POLL_WAIT)).await;
        }
        before_deadline(deadline)?;
        let pipe = self
            .pipe
            .take()
            .ok_or_else(|| HandoffError::new(ErrorCode::Disconnected))?;
        let handle = OwnedHandle::try_from(pipe).map_err(|pipe| {
            pipe.evade_limbo();
            HandoffError::new(ErrorCode::ProtocolError)
        })?;
        let stream = WindowsStream::from_handle(handle, Some(self.reservation))?;
        before_deadline(deadline)?;
        Ok(stream)
    }
}

impl Drop for WindowsEndpoint<'_> {
    /// Closes a cancelled or failed wait without invoking the listener accept path.
    fn drop(&mut self) {
        if let Some(pipe) = self.pipe.take() {
            pipe.evade_limbo();
        }
    }
}

/// Owns a bufferless asynchronous Windows pipe with drop-closing semantics.
pub struct WindowsStream<'a> {
    /// Never exposes the library's blocking flush or linger operations.
    pipe: Option<AsyncPipe>,
    /// Keeps the server's reservation alive; a client does not own that reservation.
    _reservation: Option<&'a PendingLoginSlot>,
}

impl std::fmt::Debug for WindowsStream<'_> {
    /// Omits the library stream, reservation, handles, and all peer data.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WindowsStream { channel: [REDACTED] }")
    }
}

impl<'a> WindowsStream<'a> {
    /// Registers an owned overlapped pipe with Tokio without raw-pointer operations.
    fn from_handle(
        handle: OwnedHandle,
        reservation: Option<&'a PendingLoginSlot>,
    ) -> Result<Self, HandoffError> {
        let pipe = AsyncPipe::try_from(handle)
            .map_err(|error| HandoffError::io(io::Error::other(HandleFailure::Async(error))))?;
        pipe.assume_flushed();
        Ok(Self {
            pipe: Some(pipe),
            _reservation: reservation,
        })
    }
}

impl WindowsStream<'static> {
    /// Opens only the current logon's product endpoint, without waiting or impersonation.
    ///
    /// # Errors
    ///
    /// Returns no-pending, busy, denied, deadline, or typed I/O failures. This
    /// connection proves no application identity and must complete pairing.
    pub fn connect(deadline: Instant) -> Result<Self, HandoffError> {
        Self::connect_namespace(None, deadline)
    }

    /// Opens only a separate feature-only synthetic namespace for process checks.
    ///
    /// # Errors
    ///
    /// Returns the same connection failures as the product connection method.
    #[cfg(feature = "browser-fixture")]
    pub fn connect_fixture(
        namespace: &super::Transaction,
        deadline: Instant,
    ) -> Result<Self, HandoffError> {
        Self::connect_namespace(Some(namespace.as_str()), deadline)
    }

    /// Requests exactly the DACL's client data rights, not GENERIC_WRITE or instance rights.
    fn connect_namespace(namespace: Option<&str>, deadline: Instant) -> Result<Self, HandoffError> {
        check_pending_deadline(deadline)?;
        let file = OpenOptions::new()
            .access_mode(CLIENT_ACCESS)
            .custom_flags(OVERLAPPED)
            .security_qos_flags(0)
            .open(path(namespace)?)
            .map_err(|error| {
                let code = match error.raw_os_error() {
                    Some(2) => ErrorCode::NoPendingLogin,
                    Some(5) => ErrorCode::UnauthorizedPeer,
                    Some(231) => ErrorCode::Busy,
                    _ => ErrorCode::Disconnected,
                };
                HandoffError::with_io(code, error)
            })?;
        let stream = Self::from_handle(file.into(), None)?;
        before_deadline(deadline)?;
        Ok(stream)
    }
}

impl AsyncRead for WindowsStream<'_> {
    /// Reads only through the registered asynchronous handle.
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut().pipe.as_mut() {
            Some(pipe) => Pin::new(pipe).poll_read(cx, buffer),
            None => Poll::Ready(Err(io::ErrorKind::BrokenPipe.into())),
        }
    }
}

impl AsyncWrite for WindowsStream<'_> {
    /// Completes a write to the kernel; peer receipt requires an application acknowledgement.
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut().pipe.as_mut() {
            Some(pipe) => Pin::new(pipe).poll_write(cx, bytes),
            None => Poll::Ready(Err(io::ErrorKind::BrokenPipe.into())),
        }
    }

    /// Has no user-space buffer to drain and does not block on a peer's reads.
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.pipe.is_some() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()))
        }
    }

    /// Closes the complete pipe without starting a blocking flush worker.
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if let Some(pipe) = self.get_mut().pipe.take() {
            pipe.evade_limbo();
        }
        Poll::Ready(Ok(()))
    }
}

impl Drop for WindowsStream<'_> {
    /// Discards unacknowledged output and closes the owned handle on cancellation.
    fn drop(&mut self) {
        if let Some(pipe) = self.pipe.take() {
            pipe.evade_limbo();
        }
    }
}

#[cfg(test)]
mod tests {
    //! Synthetic kernel permission, connection, deadline, and cancellation checks.

    use super::super::Transaction;
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Claims only an isolated synthetic reservation without product or account access.
    fn reservation() -> (Transaction, PendingLoginSlot) {
        let namespace = Transaction::generate().expect("public randomness");
        let slot =
            PendingLoginSlot::claim_namespace(Some(namespace.as_str())).expect("synthetic slot");
        (namespace, slot)
    }

    /// Checks fixed Debug output before and after a real synthetic client connects.
    #[tokio::test]
    async fn endpoint_and_stream_debug_omit_os_and_peer_data() {
        let (namespace, slot) = reservation();
        let deadline = Instant::now() + Duration::from_secs(2);
        let endpoint = WindowsEndpoint::bind(&slot, deadline).expect("endpoint");
        assert_eq!(
            format!("{endpoint:?}"),
            "WindowsEndpoint { channel: [REDACTED] }"
        );
        let client = WindowsStream::connect_namespace(Some(namespace.as_str()), deadline)
            .expect("synthetic client");
        let server = endpoint.accept().await.expect("synthetic server");
        for stream in [&client, &server] {
            assert_eq!(
                format!("{stream:?}"),
                "WindowsStream { channel: [REDACTED] }"
            );
        }
        drop(client);
        drop(server);
        released(&slot).await;
    }

    /// Waits only for cancelled kernel I/O completions to release an owned fixture name.
    async fn released(slot: &PendingLoginSlot) {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            match WindowsEndpoint::bind(slot, deadline) {
                Ok(endpoint) => break drop(endpoint),
                Err(error) if error.code() == ErrorCode::Busy => {
                    before_deadline(deadline).expect("owned endpoint cleanup deadline");
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                Err(_) => panic!("unexpected endpoint cleanup failure"),
            }
        }
    }

    /// Requires denied owner control and actual instance creation with a spare server slot.
    #[tokio::test]
    async fn clients_cannot_create_instances_or_change_permissions() {
        let (_, slot) = reservation();
        let endpoint = WindowsEndpoint::bind(&slot, Instant::now() + Duration::from_secs(2))
            .expect("endpoint");
        let path = path(slot.namespace()).expect("synthetic path");
        assert_eq!(CLIENT_ACCESS & (4 | 0x40000 | 0x80000), 0);
        for access in [0x40000, 0x80000] {
            let result = OpenOptions::new().access_mode(access).open(&path);
            let error_code = result.as_ref().err().and_then(io::Error::raw_os_error);
            assert!(
                error_code == Some(5),
                "forbidden client mask {access:x}: OS error {error_code:?}"
            );
        }
        assert!(
            matches!(WindowsEndpoint::bind(&slot, Instant::now() + Duration::from_secs(2)), Err(error) if error.code() == ErrorCode::Busy)
        );
        drop(endpoint);
        let first = PipeListenerOptions::new()
            .path(path.clone())
            .instance_limit(NonZeroU8::new(2))
            .accept_remote(false)
            .inheritable(false)
            .security_descriptor(Some(descriptor().expect("same endpoint DACL")))
            .create_duplex::<pipe_mode::Bytes>()
            .expect("two-instance permission fixture");
        let attempted = tokio::net::windows::named_pipe::ServerOptions::new()
            .max_instances(2)
            .first_pipe_instance(false)
            .create(&path);
        assert!(
            matches!(attempted, Err(error) if error.raw_os_error() == Some(5)),
            "DACL denies actual instance creation despite a spare instance slot"
        );
        drop(first);
    }

    /// Checks narrow client access, real bidirectional bytes, and no extra pipe instance.
    #[tokio::test]
    async fn narrow_client_and_single_server_exchange_bytes() {
        let (namespace, slot) = reservation();
        let deadline = Instant::now() + Duration::from_secs(2);
        let endpoint = WindowsEndpoint::bind(&slot, deadline).expect("endpoint");
        let mut client = WindowsStream::connect_namespace(Some(namespace.as_str()), deadline)
            .expect("narrow client");
        let mut server = endpoint.accept().await.expect("connected transport");
        client.write_all(b"request").await.expect("request");
        client.flush().await.expect("bufferless flush");
        let mut bytes = [0; 7];
        server
            .read_exact(&mut bytes)
            .await
            .expect("request receipt");
        assert_eq!(&bytes, b"request");
        server.write_all(b"reply").await.expect("reply");
        let mut bytes = [0; 5];
        client.read_exact(&mut bytes).await.expect("reply receipt");
        assert_eq!(&bytes, b"reply");
        drop((client, server));
        released(&slot).await;
    }

    /// Rejects an expired deadline before creating an endpoint or opening a client.
    #[test]
    fn expired_deadlines_do_not_create_or_connect() {
        let (namespace, slot) = reservation();
        let deadline = Instant::now();
        assert!(
            matches!(WindowsEndpoint::bind(&slot, deadline), Err(error) if error.code() == ErrorCode::TimedOut)
        );
        assert!(
            matches!(WindowsStream::connect_namespace(Some(namespace.as_str()), deadline), Err(error) if error.code() == ErrorCode::TimedOut)
        );
    }

    /// Requires an explicit no-pending error when a synthetic endpoint is absent.
    #[tokio::test]
    async fn missing_endpoint_never_becomes_a_connection() {
        let (namespace, _slot) = reservation();
        assert!(
            matches!(WindowsStream::connect_namespace(Some(namespace.as_str()), Instant::now() + Duration::from_secs(2)), Err(error) if error.code() == ErrorCode::NoPendingLogin)
        );
    }

    /// Keeps connection waiting within the original deadline and releases the unclaimed handle.
    #[tokio::test(start_paused = true)]
    async fn unconnected_endpoint_expires_and_releases_ownership() {
        let (_, slot) = reservation();
        let deadline = Instant::now() + Duration::from_millis(100);
        let endpoint = WindowsEndpoint::bind(&slot, deadline).expect("endpoint");
        assert!(
            matches!(endpoint.accept().await, Err(error) if error.code() == ErrorCode::TimedOut)
        );
        drop(
            WindowsEndpoint::bind(&slot, Instant::now() + Duration::from_secs(2))
                .expect("released endpoint"),
        );
    }

    /// Cancels a started wait and confirms its server name can be acquired again.
    #[tokio::test]
    async fn cancelled_accept_closes_the_initial_instance() {
        let (_, slot) = reservation();
        let endpoint = WindowsEndpoint::bind(&slot, Instant::now() + Duration::from_secs(2))
            .expect("endpoint");
        let mut wait = Box::pin(endpoint.accept());
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(20)) => {}
            _ = &mut wait => panic!("an absent client cannot be accepted"),
        }
        drop(wait);
        drop(
            WindowsEndpoint::bind(&slot, Instant::now() + Duration::from_secs(2))
                .expect("released endpoint"),
        );
    }

    /// Drop does not wait for unread peer buffers or start a blocking linger worker.
    #[tokio::test]
    async fn unread_output_is_discarded_on_drop() {
        let (namespace, slot) = reservation();
        let deadline = Instant::now() + Duration::from_secs(2);
        let endpoint = WindowsEndpoint::bind(&slot, deadline).expect("endpoint");
        let mut client =
            WindowsStream::connect_namespace(Some(namespace.as_str()), deadline).expect("client");
        let server = endpoint.accept().await.expect("transport");
        client
            .write_all(&[42; 512])
            .await
            .expect("bounded unread synthetic bytes");
        client.flush().await.expect("no peer-read wait");
        drop((client, server));
        released(&slot).await;
    }
}
