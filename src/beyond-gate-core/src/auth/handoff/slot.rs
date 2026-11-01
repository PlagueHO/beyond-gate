//! OS-wide pending-login reservation, separate from the authenticated endpoint.
//!
//! Windows uses a first-instance, non-inheritable named pipe with one instance
//! and a protected DACL that grants no client rights, including owner rights.
//! The reservation never accepts, reads, or writes. Dropping its server handle
//! releases the name; process termination also releases it through the OS.
//! The current process's logon SID determines the namespace, not peer identity.
//! A pre-existing object stops acquisition rather than being opened or replaced.
//! Other platforms fail explicitly. This does not enable product login.

use std::{error::Error, fmt};

#[cfg(windows)]
use {
    interprocess::os::windows::{
        named_pipe::{PipeListener, PipeListenerOptions, pipe_mode},
        security_descriptor::SecurityDescriptor,
    },
    std::{io, num::NonZeroU8},
    widestring::U16CString,
    winsafe::{HPROCESS, TokenInfo, co},
};

#[cfg(any(all(test, windows), feature = "browser-fixture"))]
use super::Transaction;

/// Identifies the failed reservation step without exposing OS identity or paths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotErrorKind {
    /// No reviewed reservation implementation exists for this platform.
    UnsupportedPlatform,
    /// The process token could not supply exactly one valid Windows logon SID.
    LogonUnavailable,
    /// The fixed protected security descriptor could not be constructed.
    SecurityDescriptor,
    /// The reservation could not be created; an existing object is never replaced.
    Unavailable,
    /// A development-only namespace was not a canonical public transaction.
    InvalidFixtureNamespace,
}

/// Retains typed OS failures behind a redacted diagnostic boundary.
pub struct SlotError {
    /// Supports adapter classification without parsing diagnostic text.
    kind: SlotErrorKind,
    /// Never includes a namespace or security descriptor in diagnostic output.
    #[cfg(windows)]
    cause: Option<SlotCause>,
}

#[cfg(windows)]
enum SlotCause {
    /// Retains the exact token-query or SID conversion failure.
    Windows(co::ERROR),
    /// Retains the exact descriptor or named-pipe operation failure.
    Io(io::Error),
}

impl SlotError {
    /// Constructs a failure for which no OS operation supplied a cause.
    fn new(kind: SlotErrorKind) -> Self {
        Self {
            kind,
            #[cfg(windows)]
            cause: None,
        }
    }

    /// Retains a Windows API failure without formatting its text.
    #[cfg(windows)]
    fn windows(error: co::ERROR) -> Self {
        Self {
            kind: SlotErrorKind::LogonUnavailable,
            cause: Some(SlotCause::Windows(error)),
        }
    }

    /// Retains the failure from one known descriptor or reservation operation.
    #[cfg(windows)]
    fn io(kind: SlotErrorKind, error: io::Error) -> Self {
        Self {
            kind,
            cause: Some(SlotCause::Io(error)),
        }
    }

    /// Returns the operation classification, not application-peer authorization.
    pub fn kind(&self) -> SlotErrorKind {
        self.kind
    }
}

impl fmt::Display for SlotError {
    /// Emits a fixed diagnostic without a SID, namespace, or dependency message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            SlotErrorKind::UnsupportedPlatform => {
                "pending-login reservation is unsupported on this platform"
            }
            SlotErrorKind::LogonUnavailable => "Windows logon information is unavailable",
            SlotErrorKind::SecurityDescriptor => {
                "pending-login reservation permissions could not be constructed"
            }
            SlotErrorKind::Unavailable => "pending-login reservation is unavailable",
            SlotErrorKind::InvalidFixtureNamespace => "invalid public fixture namespace",
        })
    }
}

impl fmt::Debug for SlotError {
    /// Uses only the fixed user-facing classification.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for SlotError {
    /// Exposes a redacted typed cause and stops before raw dependency diagnostics.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        #[cfg(windows)]
        {
            self.cause.as_ref().map(|cause| cause as &dyn Error)
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
}

#[cfg(windows)]
impl fmt::Display for SlotCause {
    /// Reports only a numeric Windows code or an I/O error category.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows(error) => write!(formatter, "Windows operation failed ({})", error.raw()),
            Self::Io(error) => write!(
                formatter,
                "local reservation operation failed ({:?})",
                error.kind()
            ),
        }
    }
}

#[cfg(windows)]
impl fmt::Debug for SlotCause {
    /// Never delegates to a dependency's diagnostic formatter.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

#[cfg(windows)]
impl Error for SlotCause {}

/// Owns one OS reservation until dropped; never exposes a listener or handle.
///
/// Keep this guard alive through the complete pending-login operation. It does
/// not own a deadline, authenticate an application, accept a submission, or save.
pub struct PendingLoginSlot {
    /// Separates synthetic endpoints from the fixed product namespace.
    #[cfg(windows)]
    namespace: Option<String>,
    /// Holds the only server instance without ever calling `accept`.
    #[cfg(windows)]
    _listener: PipeListener<pipe_mode::Bytes, pipe_mode::Bytes>,
}

impl PendingLoginSlot {
    /// Claims the fixed product reservation for this process's Windows logon.
    ///
    /// # Errors
    ///
    /// Fails on unsupported platforms, missing logon information, permission
    /// construction errors, or any pre-existing or inaccessible reservation.
    /// An unavailable slot is not proof that its owner is a trusted CLI.
    pub fn claim() -> Result<Self, SlotError> {
        Self::claim_namespace(None)
    }

    /// Claims only a separate, development-only namespace for process checks.
    ///
    /// # Errors
    ///
    /// Returns the same acquisition errors as `claim`. The public suffix never
    /// selects the product reservation and is not a pairing key or access grant.
    #[cfg(feature = "browser-fixture")]
    pub fn claim_fixture(namespace: &Transaction) -> Result<Self, SlotError> {
        Self::claim_namespace(Some(namespace.as_str()))
    }

    /// Constructs the reservation atomically with no client access or inheritance.
    #[cfg(windows)]
    pub(crate) fn claim_namespace(namespace: Option<&str>) -> Result<Self, SlotError> {
        let path = slot_path(namespace)?;
        let descriptor = reservation_descriptor()?;
        let listener = PipeListenerOptions::new()
            .path(path)
            .instance_limit(Some(NonZeroU8::MIN))
            .accept_remote(false)
            .inheritable(false)
            .security_descriptor(Some(descriptor))
            .create_duplex::<pipe_mode::Bytes>()
            .map_err(|error| SlotError::io(SlotErrorKind::Unavailable, error))?;
        Ok(Self {
            _listener: listener,
            namespace: namespace.map(str::to_owned),
        })
    }

    /// Lets the Windows endpoint reuse only this reservation's validated scope.
    #[cfg(windows)]
    pub(super) fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Refuses unsupported platforms rather than substituting an in-process lock.
    #[cfg(not(windows))]
    fn claim_namespace(_namespace: Option<&str>) -> Result<Self, SlotError> {
        Err(SlotError::new(SlotErrorKind::UnsupportedPlatform))
    }
}

impl fmt::Debug for PendingLoginSlot {
    /// Omits the OS handle, SID, and public reservation namespace.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PendingLoginSlot { reservation: [REDACTED] }")
    }
}

/// Constructs a protected DACL that explicitly denies all Owner Rights access.
///
/// An empty DACL alone leaves the owner's implicit WRITE_DAC permission. The
/// Owner Rights deny ACE removes that bypass without granting data or instance rights.
#[cfg(windows)]
fn reservation_descriptor() -> Result<SecurityDescriptor, SlotError> {
    let text = U16CString::from_str("D:P(D;;GA;;;OW)")
        .map_err(|_| SlotError::new(SlotErrorKind::SecurityDescriptor))?;
    SecurityDescriptor::deserialize(&text)
        .map_err(|error| SlotError::io(SlotErrorKind::SecurityDescriptor, error))
}

/// Selects the product namespace or a disjoint, bounded synthetic namespace.
#[cfg(windows)]
fn slot_path(namespace: Option<&str>) -> Result<String, SlotError> {
    let sid = logon_sid()?;
    match namespace {
        None => Ok(format!(r"\\.\pipe\beyond-gate-login-slot-v1-{sid}")),
        Some(value)
            if value.len() == 32
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) =>
        {
            Ok(format!(
                r"\\.\pipe\beyond-gate-fixture-slot-v1-{sid}-{value}"
            ))
        }
        Some(_) => Err(SlotError::new(SlotErrorKind::InvalidFixtureNamespace)),
    }
}

/// Copies exactly one process logon SID while its token information remains alive.
#[cfg(windows)]
pub(super) fn logon_sid() -> Result<String, SlotError> {
    let token = HPROCESS::GetCurrentProcess()
        .OpenProcessToken(co::TOKEN::QUERY)
        .map_err(SlotError::windows)?;
    let information = token
        .GetTokenInformation(co::TOKEN_INFORMATION_CLASS::LogonSid)
        .map_err(SlotError::windows)?;
    let TokenInfo::LogonSid(groups) = information else {
        return Err(SlotError::new(SlotErrorKind::LogonUnavailable));
    };
    let [group] = groups.Groups() else {
        return Err(SlotError::new(SlotErrorKind::LogonUnavailable));
    };
    let sid = group
        .Sid()
        .ok_or_else(|| SlotError::new(SlotErrorKind::LogonUnavailable))?;
    let text = winsafe::ConvertSidToStringSid(sid).map_err(SlotError::windows)?;
    if !valid_logon_sid(&text) {
        return Err(SlotError::new(SlotErrorKind::LogonUnavailable));
    }
    Ok(text)
}

/// Rejects a user SID or malformed logon SID instead of broadening the namespace.
#[cfg(windows)]
fn valid_logon_sid(text: &str) -> bool {
    let Some(value) = text.strip_prefix("S-1-5-5-") else {
        return false;
    };
    let mut parts = value.split('-');
    let (Some(high), Some(low), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    [high, low].iter().all(|part| {
        !part.is_empty()
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && part.parse::<u32>().is_ok()
    })
}

#[cfg(all(test, windows))]
mod tests {
    //! Real kernel reservation checks in disjoint synthetic namespaces.

    use super::*;
    use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

    /// Generates only a public suffix outside the fixed product namespace.
    fn namespace() -> Transaction {
        Transaction::generate().expect("public fixture randomness")
    }

    /// Requires a real reservation failure, not success with an unused guard.
    fn unavailable(result: Result<PendingLoginSlot, SlotError>) {
        assert!(matches!(result, Err(error) if error.kind() == SlotErrorKind::Unavailable));
    }

    /// Checks logon-specific syntax without recording actual OS identity.
    #[test]
    fn logon_sid_validation_rejects_user_and_malformed_sids() {
        assert!(valid_logon_sid("S-1-5-5-1-2"));
        for text in [
            "S-1-5-21-1-2",
            "S-1-5-5-1",
            "S-1-5-5-1-2-3",
            "S-1-5-5--2",
            "S-1-5-5-1-4294967296",
            "S-1-5-5-+1-2",
        ] {
            assert!(!valid_logon_sid(text));
        }
    }

    /// Checks the real kernel reservation and release without touching the product slot.
    #[test]
    fn same_logon_reservation_is_exclusive_and_drop_releases_it() {
        let namespace = namespace();
        let first =
            PendingLoginSlot::claim_namespace(Some(namespace.as_str())).expect("first slot");
        unavailable(PendingLoginSlot::claim_namespace(Some(namespace.as_str())));
        drop(first);
        let next =
            PendingLoginSlot::claim_namespace(Some(namespace.as_str())).expect("released slot");
        drop(next);
    }

    /// Checks that independent synthetic suffixes do not share a reservation.
    #[test]
    fn separate_fixture_namespaces_can_coexist() {
        let first =
            PendingLoginSlot::claim_namespace(Some(namespace().as_str())).expect("first slot");
        let second =
            PendingLoginSlot::claim_namespace(Some(namespace().as_str())).expect("second slot");
        drop((first, second));
    }

    /// Checks denied data, instance, metadata, and owner-control access using actual opens.
    #[test]
    fn reservation_denies_client_and_owner_control_rights() {
        let namespace = namespace();
        let slot = PendingLoginSlot::claim_namespace(Some(namespace.as_str())).expect("owned slot");
        let path = slot_path(Some(namespace.as_str())).expect("local fixture path");
        for access in [1, 2, 4, 0x20000, 0x40000, 0x80000, 0x80000000, 0x40000000] {
            let result = OpenOptions::new().access_mode(access).open(&path);
            assert!(
                matches!(result, Err(error) if error.raw_os_error() == Some(5)),
                "requested right must be denied"
            );
        }
        drop(slot);
    }

    /// Checks first-instance acquisition rejects a foreign object without replacing it.
    #[test]
    fn pre_existing_foreign_reservation_is_not_adopted() {
        let namespace = namespace();
        let path = slot_path(Some(namespace.as_str())).expect("local fixture path");
        let foreign = PipeListenerOptions::new()
            .path(path)
            .instance_limit(Some(NonZeroU8::MIN))
            .create_duplex::<pipe_mode::Bytes>()
            .expect("synthetic foreign reservation");
        unavailable(PendingLoginSlot::claim_namespace(Some(namespace.as_str())));
        drop(foreign);
        drop(PendingLoginSlot::claim_namespace(Some(namespace.as_str())).expect("released name"));
    }

    /// Checks fixture input cannot inject a path or select the product namespace.
    #[test]
    fn invalid_fixture_namespace_is_rejected() {
        for text in ["", "..", "A".repeat(32).as_str(), "0".repeat(33).as_str()] {
            assert!(
                matches!(slot_path(Some(text)), Err(error) if error.kind() == SlotErrorKind::InvalidFixtureNamespace)
            );
        }
    }

    /// Checks every diagnostic boundary stops before raw dependency or namespace text.
    #[test]
    fn reservation_errors_and_guard_debug_are_redacted() {
        let error = SlotError::io(
            SlotErrorKind::Unavailable,
            io::Error::other("sensitive synthetic dependency text"),
        );
        assert_eq!(
            format!("{error:?}"),
            "pending-login reservation is unavailable"
        );
        let source = error.source().expect("typed safe cause");
        assert!(!source.to_string().contains("sensitive"));
        assert!(!format!("{source:?}").contains("sensitive"));
        assert!(source.source().is_none());
        let slot =
            PendingLoginSlot::claim_namespace(Some(namespace().as_str())).expect("owned slot");
        assert_eq!(
            format!("{slot:?}"),
            "PendingLoginSlot { reservation: [REDACTED] }"
        );
    }

    /// Proves safe ownership transfer can use one kernel instance without listener accept.
    #[tokio::test]
    async fn one_instance_safe_transfer_connects_without_replacement() {
        use interprocess::os::windows::named_pipe::{
            PipeStream, tokio::PipeStream as AsyncPipeStream,
        };
        use std::os::windows::io::OwnedHandle;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let namespace = namespace();
        let sid = logon_sid().expect("process logon metadata");
        let path = slot_path(Some(namespace.as_str())).expect("synthetic namespace");
        let sddl = U16CString::from_str(format!("D:P(D;;0xc0004;;;OW)(A;;0x12019b;;;{sid})"))
            .expect("synthetic protected descriptor");
        let descriptor = SecurityDescriptor::deserialize(&sddl).expect("protected descriptor");
        let listener = PipeListenerOptions::new()
            .path(path.clone())
            .instance_limit(Some(NonZeroU8::MIN))
            .accept_remote(false)
            .inheritable(false)
            .security_descriptor(Some(descriptor))
            .create_duplex::<pipe_mode::Bytes>()
            .expect("one server instance");
        let handle: OwnedHandle = listener.into();
        let server = PipeStream::<pipe_mode::Bytes, pipe_mode::Bytes>::try_from(handle)
            .expect("safe server ownership transfer");
        server.assume_flushed();
        let pending = server.client_process_id();
        assert!(
            matches!(pending, Err(error) if error.raw_os_error() == Some(1168)),
            "unconnected pipe reports no client"
        );
        let client = OpenOptions::new()
            .access_mode(0x12019b)
            .custom_flags(0x40000000)
            .security_qos_flags(0)
            .open(&path)
            .expect("limited client rights");
        assert!(matches!(server.client_process_id(), Ok(value) if value != 0));
        let handle = OwnedHandle::try_from(server).expect("exclusive server ownership");
        let mut server = AsyncPipeStream::<pipe_mode::Bytes, pipe_mode::Bytes>::try_from(handle)
            .expect("safe asynchronous server ownership");
        let mut client = AsyncPipeStream::<pipe_mode::Bytes, pipe_mode::Bytes>::try_from(
            OwnedHandle::from(client),
        )
        .expect("safe asynchronous client ownership");
        server.assume_flushed();
        client.assume_flushed();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            client
                .write_all(b"synthetic")
                .await
                .expect("synthetic request");
            let mut bytes = [0; 9];
            server
                .read_exact(&mut bytes)
                .await
                .expect("synthetic receipt");
            assert_eq!(&bytes, b"synthetic");
            server.write_all(b"reply").await.expect("synthetic reply");
            let mut bytes = [0; 5];
            client
                .read_exact(&mut bytes)
                .await
                .expect("synthetic reply receipt");
            assert_eq!(&bytes, b"reply");
        })
        .await
        .expect("bounded one-instance exchange");
        server.evade_limbo();
        client.evade_limbo();
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                match PendingLoginSlot::claim_namespace(Some(namespace.as_str())) {
                    Ok(slot) => break drop(slot),
                    Err(error) if error.kind() == SlotErrorKind::Unavailable => {
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    }
                    Err(_) => panic!("unexpected cleanup failure"),
                }
            }
        })
        .await
        .expect("kernel completion releases the namespace");
    }
}

#[cfg(all(test, not(windows)))]
mod tests {
    //! Explicit unsupported-platform contract without a fallback reservation.

    use super::*;

    /// Requires an explicit platform failure instead of a success-shaped local lock.
    #[test]
    fn unsupported_platform_does_not_claim_a_reservation() {
        assert!(
            matches!(PendingLoginSlot::claim(), Err(error) if error.kind() == SlotErrorKind::UnsupportedPlatform)
        );
    }
}
