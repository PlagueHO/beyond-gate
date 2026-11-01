//! Confirmed Windows session establishment with concurrent peer control.
//!
//! The inbound read stays alive across validation and result delivery. Dropping
//! a partial encrypted read never permits resynchronization. Cancellation or
//! disconnect stops write admission; started or unacknowledged saves are uncertain.

use std::{error::Error, fmt, future::Future, time::Duration};

use tokio::{
    io::{AsyncRead, AsyncWrite, split},
    time::{Instant, timeout_at},
};

use super::{
    ErrorCode, HandoffError, Message, Protocol, Transaction, encode_message,
    flow::ReceivedSubmission,
    message::parse_payload,
    pairing::PairedChannel,
    transport::{Kind, PairedConnection, read_envelope, write_envelope},
};
use crate::auth::establishment::{
    EstablishmentError, LoginControl, SavedSession, SessionEstablishmentInput, establish_session,
};

/// Retains safe typed failures while exposing only one fixed completion category.
pub struct CompletionError {
    code: ErrorCode,
    handoff: Option<Box<HandoffError>>,
    establishment: Option<Box<EstablishmentError>>,
    reply: Option<Box<HandoffError>>,
}

impl CompletionError {
    /// Returns the shared wire category without credential or dependency text.
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// Keeps a failure to deliver the fixed error without losing the original cause.
    fn failed_reply(mut self, reply: HandoffError) -> Self {
        self.reply = Some(Box::new(reply));
        self
    }

    /// Prefers save uncertainty over cancellation once the protected write started.
    fn interrupted(peer: HandoffError, result: Result<SavedSession, EstablishmentError>) -> Self {
        let code = match &result {
            Ok(_) => ErrorCode::SaveIndeterminate,
            Err(error) if error.code() == ErrorCode::SaveIndeterminate => {
                ErrorCode::SaveIndeterminate
            }
            Err(_) => peer.code(),
        };
        Self {
            code,
            handoff: Some(Box::new(peer)),
            establishment: result.err().map(Box::new),
            reply: None,
        }
    }
}

impl From<HandoffError> for CompletionError {
    /// Retains the safe handoff cause and its fixed category.
    fn from(error: HandoffError) -> Self {
        Self {
            code: error.code(),
            handoff: Some(Box::new(error)),
            establishment: None,
            reply: None,
        }
    }
}

impl From<EstablishmentError> for CompletionError {
    /// Retains validation and storage causes without exposing their raw errors.
    fn from(error: EstablishmentError) -> Self {
        Self {
            code: error.code(),
            handoff: None,
            establishment: Some(Box::new(error)),
            reply: None,
        }
    }
}

impl fmt::Display for CompletionError {
    /// Prints only the shared fixed code.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code.as_str())
    }
}

impl fmt::Debug for CompletionError {
    /// Never exposes owned causes or operation state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for CompletionError {
    /// Exposes only already-redacted shared errors.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Some(error) = &self.establishment {
            Some(error.as_ref())
        } else {
            self.handoff
                .as_ref()
                .map(|error| error.as_ref() as &dyn Error)
        }
    }
}

/// Classifies one early encrypted control without allowing an early result acknowledgement.
fn interruption(
    channel: &mut PairedChannel,
    input: Result<zeroize::Zeroizing<Vec<u8>>, HandoffError>,
    transaction: &Transaction,
) -> HandoffError {
    let parsed = input
        .and_then(|bytes| channel.open(&bytes))
        .and_then(|bytes| parse_payload(Protocol::HostCli, &bytes));
    match parsed {
        Ok(Message::Cancel {
            transaction: Some(received),
        }) if &received == transaction => HandoffError::new(ErrorCode::Cancelled),
        Ok(_) => HandoffError::new(ErrorCode::ProtocolError),
        Err(error) => error.completion_read(),
    }
}

/// Encrypts one final message under a shared five-second delivery deadline.
pub(super) async fn send<W: AsyncWrite + Unpin>(
    writer: &mut W,
    channel: &mut PairedChannel,
    message: &Message,
    deadline: Instant,
) -> Result<(), HandoffError> {
    let plaintext = encode_message(Protocol::HostCli, message)?;
    let ciphertext = channel.seal(plaintext)?;
    timeout_at(
        deadline,
        write_envelope(writer, Kind::Encrypted, &ciphertext, deadline),
    )
    .await
    .map_err(|_| HandoffError::new(ErrorCode::TimedOut))?
}

/// Runs real completion with a private operation seam for offline service/store tests.
pub(in crate::auth) async fn complete_with<S, F, O>(
    connection: PairedConnection<S>,
    receipt: ReceivedSubmission,
    control: LoginControl,
    operation: F,
) -> Result<SavedSession, CompletionError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    F: FnOnce(SessionEstablishmentInput, LoginControl) -> O,
    O: Future<Output = Result<SavedSession, EstablishmentError>>,
{
    let transaction = receipt.transaction().clone();
    let input = SessionEstablishmentInput::from_handoff(receipt)?;
    let cancellation = control.cancellation();
    let (stream, mut channel, deadline) = connection.into_completion()?;
    let (mut reader, mut writer) = split(stream);
    let incoming = timeout_at(
        deadline.min(Instant::now() + Duration::from_secs(25)),
        read_envelope(&mut reader, Kind::Encrypted, deadline),
    );
    tokio::pin!(incoming);
    let work = operation(input, control);
    tokio::pin!(work);
    let outcome = tokio::select! {
        biased;
        input = &mut incoming => {
            let input = input.map_err(|_| HandoffError::new(ErrorCode::TimedOut)).and_then(|result| result);
            let peer = interruption(&mut channel, input, &transaction);
            cancellation.cancel();
            Err(CompletionError::interrupted(peer, (&mut work).await))
        }
        result = &mut work => result.map_err(CompletionError::from),
    };
    let saved = match outcome {
        Ok(saved) => saved,
        Err(error) => {
            let message = Message::Error {
                transaction: Some(transaction),
                code: error.code(),
            };
            let reply = send(
                &mut writer,
                &mut channel,
                &message,
                deadline.min(Instant::now() + Duration::from_secs(5)),
            )
            .await;
            return Err(match reply {
                Ok(()) => error,
                Err(reply) => error.failed_reply(reply),
            });
        }
    };
    let ack_deadline = deadline.min(Instant::now() + Duration::from_secs(5));
    send(
        &mut writer,
        &mut channel,
        &Message::Result {
            transaction: transaction.clone(),
        },
        ack_deadline,
    )
    .await
    .map_err(|error| error.reclassify(ErrorCode::SaveIndeterminate))?;
    let bytes = timeout_at(ack_deadline, &mut incoming)
        .await
        .map_err(|_| HandoffError::new(ErrorCode::SaveIndeterminate))?
        .map_err(|_| HandoffError::new(ErrorCode::SaveIndeterminate))?
        .map_err(|error| error.reclassify(ErrorCode::SaveIndeterminate))?;
    let message = channel
        .open(&bytes)
        .and_then(|bytes| parse_payload(Protocol::HostCli, &bytes))
        .map_err(|error| error.reclassify(ErrorCode::SaveIndeterminate))?;
    if !matches!(message, Message::Acknowledgement { transaction: ref received }
        if received == &transaction)
    {
        return Err(HandoffError::new(ErrorCode::SaveIndeterminate).into());
    }
    send(&mut writer, &mut channel, &message, ack_deadline)
        .await
        .map_err(|error| error.reclassify(ErrorCode::SaveIndeterminate))?;
    Ok(saved)
}

/// Consumes a bound receipt, validates once, saves once, and confirms browser receipt.
///
/// # Errors
///
/// Returns typed validation/storage failures, safe control errors, or
/// `SaveIndeterminate` after a started save or lost final acknowledgement.
pub async fn complete_login<S: AsyncRead + AsyncWrite + Unpin>(
    connection: PairedConnection<S>,
    receipt: ReceivedSubmission,
    control: LoginControl,
) -> Result<SavedSession, CompletionError> {
    complete_with(connection, receipt, control, establish_session).await
}
