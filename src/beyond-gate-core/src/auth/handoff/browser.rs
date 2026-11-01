//! One-use native browser relay to the fixed Windows pending-login endpoint.
//!
//! Only the manual package's exact origin is accepted. Origin is not peer
//! authorization: a user-entered key must authenticate the CLI before pending
//! cookie authorization. This relay never queries cookies or opens storage.
//! The trusted CLI alone supplies validation and confirmed-save evidence.

use std::{ffi::OsString, future::Future, time::Duration};

use tokio::{
    io::{AsyncRead, AsyncWrite, split},
    time::{Instant, timeout_at},
};
use zeroize::Zeroizing;

use super::{
    Browser, ErrorCode, FrameReader, FrameWriter, HandoffError, Message, Profile, Protocol,
    Transaction,
    completion::send,
    encode_message,
    endpoint::WindowsStream,
    message::parse_payload,
    transport::{Kind, PairedConnection, read_envelope},
};

/// Names the product host; no fixture or alternate host can be selected by input.
pub const HOST_NAME: &str = "io.github.plagueho.beyond_gate";
/// Identifies the public-key-pinned manual extension, not a Store publication.
pub const EXTENSION_ID: &str = "gnhdconfgjfdbbgkemadegjcjnnodefh";
/// Matches only the manual package origin, with the required trailing slash.
pub const EXTENSION_ORIGIN: &str = "chrome-extension://gnhdconfgjfdbbgkemadegjcjnnodefh/";

/// Rejects caller arguments before starting the runtime or reading standard input.
///
/// The optional parent-window value is syntax only, never launch proof.
///
/// # Errors
///
/// Returns `OriginRejected` for an absent, foreign, or malformed caller.
pub fn check_launch(arguments: &[OsString]) -> Result<(), HandoffError> {
    let valid_window = |value: &str| {
        value
            .strip_prefix("--parent-window=")
            .is_some_and(|number| {
                !number.is_empty()
                    && number.bytes().all(|byte| byte.is_ascii_digit())
                    && number.parse::<usize>().is_ok()
            })
    };
    if arguments.first().and_then(|value| value.to_str()) != Some(EXTENSION_ORIGIN)
        || arguments.len() > 2
        || arguments
            .get(1)
            .is_some_and(|value| value.to_str().is_none_or(|value| !valid_window(value)))
    {
        return Err(HandoffError::new(ErrorCode::OriginRejected));
    }
    Ok(())
}

/// Requires the connection's public transaction without reflecting its bytes.
fn bound(message: &Message, expected: &Transaction) -> Result<(), HandoffError> {
    let transaction = match message {
        Message::Submit { transaction, .. }
        | Message::Result { transaction }
        | Message::Acknowledgement { transaction } => Some(transaction),
        Message::Cancel { transaction } | Message::Error { transaction, .. } => {
            transaction.as_ref()
        }
        _ => None,
    };
    if transaction == Some(expected) {
        Ok(())
    } else {
        Err(HandoffError::new(ErrorCode::TransactionMismatch))
    }
}

/// Restricts browser failures to local cookie/control conditions, never service claims.
fn browser_failure(code: ErrorCode) -> bool {
    matches!(
        code,
        ErrorCode::CookieUnavailable
            | ErrorCode::InvalidMaterial
            | ErrorCode::ProtocolError
            | ErrorCode::TimedOut
            | ErrorCode::Cancelled
    )
}

/// Parses one encrypted CLI response and requires the established public binding.
fn cli_message(
    channel: &mut super::pairing::PairedChannel,
    bytes: Zeroizing<Vec<u8>>,
    expected: &Transaction,
) -> Result<Message, HandoffError> {
    let message = channel
        .open(&bytes)
        .and_then(|bytes| parse_payload(Protocol::HostCli, &bytes))?;
    bound(&message, expected)?;
    Ok(message)
}

/// Relays one paired operation and requires both final acknowledgements.
async fn relay<R, W, S>(
    reader: &mut FrameReader<R>,
    writer: &mut FrameWriter<W>,
    mut cli: PairedConnection<S>,
    browser: Browser,
    transaction: &mut Option<Transaction>,
    deadline: Instant,
) -> Result<(), HandoffError>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    S: AsyncRead + AsyncWrite + Unpin,
{
    let open = Message::Initiate {
        browser,
        profile: Profile::CobaltSessionV1,
    };
    cli.write_message(encode_message(Protocol::HostCli, &open)?, deadline)
        .await?;
    let authorized = cli
        .read_message(deadline.min(Instant::now() + Duration::from_secs(10)))
        .await?;
    let remaining = match &authorized {
        Message::Authorization {
            transaction: issued,
            profile: Profile::CobaltSessionV1,
            expires_in_ms,
        } => {
            *transaction = Some(issued.clone());
            Duration::from_millis(u64::from(*expires_in_ms))
        }
        Message::Error {
            transaction: None,
            code,
        } => return Err(HandoffError::new(*code)),
        _ => return Err(HandoffError::new(ErrorCode::ProtocolError)),
    };
    let deadline = deadline.min(Instant::now() + remaining);
    let expected = transaction
        .as_ref()
        .ok_or_else(|| HandoffError::new(ErrorCode::ProtocolError))?;
    writer
        .write_message(encode_message(Protocol::Browser, &authorized)?, deadline)
        .await?;
    let submit = reader
        .read_message(deadline.min(Instant::now() + Duration::from_secs(15)))
        .await?;
    bound(&submit, expected)?;
    match &submit {
        Message::Submit {
            profile: Profile::CobaltSessionV1,
            ..
        } => {}
        Message::Cancel { .. } => {
            cli.write_message(encode_message(Protocol::HostCli, &submit)?, deadline)
                .await
                .map_err(|error| error.reclassify(ErrorCode::SaveIndeterminate))?;
            return Err(HandoffError::new(ErrorCode::Cancelled));
        }
        Message::Error { code, .. } if browser_failure(*code) => {
            cli.write_message(encode_message(Protocol::HostCli, &submit)?, deadline)
                .await?;
            return Err(HandoffError::new(*code));
        }
        _ => return Err(HandoffError::new(ErrorCode::ProtocolError)),
    }
    cli.write_message(encode_message(Protocol::HostCli, &submit)?, deadline)
        .await?;
    drop(submit);

    let (stream, mut channel, original_deadline) = cli.into_completion()?;
    let deadline = deadline.min(original_deadline);
    let (mut cli_reader, mut cli_writer) = split(stream);
    let control = reader.read_completion(deadline);
    tokio::pin!(control);
    let result = {
        let incoming = timeout_at(
            deadline.min(Instant::now() + Duration::from_secs(20)),
            read_envelope(&mut cli_reader, Kind::Encrypted, deadline),
        );
        tokio::pin!(incoming);
        let decode = |result: Result<
            Result<Zeroizing<Vec<u8>>, HandoffError>,
            tokio::time::error::Elapsed,
        >,
                      channel: &mut super::pairing::PairedChannel| {
            result
                .map_err(|_| HandoffError::new(ErrorCode::TimedOut))
                .and_then(|result| result)
                .and_then(|bytes| cli_message(channel, bytes, expected))
        };
        tokio::select! {
            biased;
            message = &mut control => {
                let peer = match message.and_then(|message| {
                    bound(&message, expected)?;
                    if matches!(message, Message::Cancel { .. }) {
                        Ok(HandoffError::new(ErrorCode::Cancelled))
                    } else {
                        Err(HandoffError::new(ErrorCode::ProtocolError))
                    }
                }) {
                    Ok(peer) | Err(peer) => peer.completion_read(),
                };
                let cancellation = Message::Cancel { transaction: Some(expected.clone()) };
                if let Err(reply) = send(
                    &mut cli_writer, &mut channel, &cancellation,
                    deadline.min(Instant::now() + Duration::from_secs(5)),
                ).await {
                    return Err(HandoffError::failed_reply(
                        peer.reclassify(ErrorCode::SaveIndeterminate), reply,
                    ));
                }
                return Err(match decode((&mut incoming).await, &mut channel) {
                    Ok(Message::Error { code, .. }) if code != ErrorCode::SaveIndeterminate => peer,
                    Ok(_) => peer.reclassify(ErrorCode::SaveIndeterminate),
                    Err(reply) => HandoffError::failed_reply(
                        peer.reclassify(ErrorCode::SaveIndeterminate), reply,
                    ),
                });
            }
            result = &mut incoming => decode(result, &mut channel)
                .map_err(|error| error.reclassify(ErrorCode::SaveIndeterminate))?,
        }
    };
    match &result {
        Message::Error { code, .. } => return Err(HandoffError::new(*code)),
        Message::Result { .. } => {}
        _ => return Err(HandoffError::new(ErrorCode::SaveIndeterminate)),
    }
    let ack_deadline = deadline.min(Instant::now() + Duration::from_secs(5));
    writer
        .write_message(encode_message(Protocol::Browser, &result)?, ack_deadline)
        .await
        .map_err(|error| error.reclassify(ErrorCode::SaveIndeterminate))?;
    let ack = timeout_at(ack_deadline, &mut control)
        .await
        .map_err(|_| HandoffError::new(ErrorCode::SaveIndeterminate))?
        .map_err(|error| error.reclassify(ErrorCode::SaveIndeterminate))?;
    if bound(&ack, expected).is_err() || !matches!(ack, Message::Acknowledgement { .. }) {
        return Err(HandoffError::new(ErrorCode::SaveIndeterminate));
    }
    send(&mut cli_writer, &mut channel, &ack, ack_deadline)
        .await
        .map_err(|error| error.reclassify(ErrorCode::SaveIndeterminate))?;
    let confirmed = timeout_at(
        ack_deadline,
        read_envelope(&mut cli_reader, Kind::Encrypted, ack_deadline),
    )
    .await
    .map_err(|_| HandoffError::new(ErrorCode::SaveIndeterminate))?
    .and_then(|bytes| cli_message(&mut channel, bytes, expected))
    .map_err(|error| error.reclassify(ErrorCode::SaveIndeterminate))?;
    if bound(&confirmed, expected).is_err() || !matches!(confirmed, Message::Acknowledgement { .. })
    {
        return Err(HandoffError::new(ErrorCode::SaveIndeterminate));
    }
    writer
        .write_message(encode_message(Protocol::Browser, &confirmed)?, ack_deadline)
        .await
        .map_err(|error| error.reclassify(ErrorCode::SaveIndeterminate))
}

/// Reads one input-only pairing frame before invoking the fixed connection factory.
pub(in crate::auth) async fn run_with<R, W, S, F, C>(
    reader: R,
    writer: W,
    connect: F,
) -> Result<(), HandoffError>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    S: AsyncRead + AsyncWrite + Unpin,
    F: FnOnce(Zeroizing<Vec<u8>>, Instant) -> C,
    C: Future<Output = Result<PairedConnection<S>, HandoffError>>,
{
    let deadline = Instant::now() + Duration::from_secs(300);
    let mut reader = FrameReader::new(reader, Protocol::Browser);
    let mut writer = FrameWriter::new(writer, Protocol::Browser);
    let mut transaction = None;
    let result = async {
        let pair = reader
            .read_message(deadline.min(Instant::now() + Duration::from_secs(10)))
            .await?;
        let (browser, key) = match pair {
            Message::Pair {
                browser,
                profile: Profile::CobaltSessionV1,
                key,
            } => (browser, key),
            _ => return Err(HandoffError::new(ErrorCode::ProtocolError)),
        };
        let cli = connect(key.into_bytes(), deadline).await?;
        relay(
            &mut reader,
            &mut writer,
            cli,
            browser,
            &mut transaction,
            deadline,
        )
        .await
    }
    .await;
    if let Err(error) = result {
        let failure = Message::Error {
            transaction,
            code: error.code(),
        };
        let reply = writer
            .write_message(
                encode_message(Protocol::Browser, &failure)?,
                Instant::now() + Duration::from_secs(5),
            )
            .await;
        if let Err(reply) = reply {
            return Err(HandoffError::failed_reply(error, reply));
        }
        return Err(error);
    }
    Ok(())
}

/// Serves one authorized-origin browser port through the current Windows logon.
///
/// Origin checks must already pass through `check_launch`. No cookie read,
/// service request, or native-store operation is performed by this relay.
///
/// # Errors
///
/// Returns fixed pairing, consent, sequencing, transport, or uncertain-result errors.
pub async fn serve<R, W>(arguments: &[OsString], reader: R, writer: W) -> Result<(), HandoffError>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    check_launch(arguments)?;
    run_with(reader, writer, |key, deadline| async move {
        let stream = WindowsStream::connect(deadline)?;
        PairedConnection::connect(stream, key, deadline).await
    })
    .await
}

#[cfg(test)]
#[path = "browser_tests.rs"]
mod tests;
