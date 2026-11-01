//! Windows login orchestration with protected key display and one consuming flow.
//!
//! No credential is accepted through arguments, stdin, environment input, or
//! logs. Only an authenticated browser host can supply the selected cookie.
//! Validation and storage remain in the shared core operation.

use std::{
    error::Error,
    fmt,
    io::{self, IsTerminal},
    sync::Arc,
    time::Duration,
};

use beyond_gate_core::auth::{
    establishment::{LoginControl, SavedSession},
    handoff::{
        ErrorCode, HandoffError, Profile, Transaction,
        completion::{CompletionError, complete_login},
        endpoint::WindowsEndpoint,
        flow::PendingHandoff,
        pairing::{PairingKey, PendingPairing, RandomnessError},
        slot::{PendingLoginSlot, SlotError},
        transport::PairedConnection,
    },
};
use tokio::time::Instant;
use zeroize::Zeroizing;

use crate::pairing::{PairingDisplay, PairingDisplayError};

/// Holds typed setup/control causes behind fixed local diagnostics.
enum Cause {
    Display(PairingDisplayError),
    Slot(SlotError),
    Randomness(RandomnessError),
    Handoff(HandoffError),
    Completion(CompletionError),
    Io(io::Error),
}

impl fmt::Display for Cause {
    /// Prints safe shared diagnostics and never raw OS error strings.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Display(error) => fmt::Display::fmt(error, formatter),
            Self::Slot(error) => fmt::Display::fmt(error, formatter),
            Self::Randomness(error) => fmt::Display::fmt(error, formatter),
            Self::Handoff(error) => fmt::Display::fmt(error, formatter),
            Self::Completion(error) => fmt::Display::fmt(error, formatter),
            Self::Io(error) => write!(
                formatter,
                "login runtime or control failed ({:?})",
                error.kind()
            ),
        }
    }
}

impl fmt::Debug for Cause {
    /// Uses only safe cause categories.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for Cause {
    /// Preserves shared typed causes without exposing raw operating-system text.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Display(error) => Some(error),
            Self::Slot(error) => Some(error),
            Self::Randomness(error) => Some(error),
            Self::Handoff(error) => Some(error),
            Self::Completion(error) => Some(error),
            Self::Io(_) => None,
        }
    }
}

/// Reports one typed login failure without secrets or process details.
pub(crate) struct LoginError {
    code: ErrorCode,
    cause: Option<Box<Cause>>,
}

impl LoginError {
    /// Wraps a known safe cause without losing its operation category.
    fn caused(code: ErrorCode, cause: Cause) -> Self {
        Self {
            code,
            cause: Some(Box::new(cause)),
        }
    }
}

impl fmt::Display for LoginError {
    /// Includes useful terminal/setup guidance while keeping shared categories fixed.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code.as_str())?;
        if let Some(cause) = &self.cause {
            write!(formatter, ": {cause}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for LoginError {
    /// Never prints cause fields or operation state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for LoginError {
    /// Exposes only the redacted cause wrapper.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_ref()
            .map(|cause| cause.as_ref() as &dyn Error)
    }
}

/// Owns the display and channel until success, cancellation, or bounded failure.
async fn run_login() -> Result<SavedSession, LoginError> {
    let deadline = Instant::now() + Duration::from_secs(300);
    let reservation = Arc::new(
        PendingLoginSlot::claim()
            .map_err(|error| LoginError::caused(ErrorCode::Busy, Cause::Slot(error)))?,
    );
    let endpoint = WindowsEndpoint::bind(&reservation, deadline)
        .map_err(|error| LoginError::caused(error.code(), Cause::Handoff(error)))?;
    let key = PairingKey::generate()
        .map_err(|error| {
            LoginError::caused(ErrorCode::UnsupportedContract, Cause::Randomness(error))
        })?
        .into_bytes();
    let ticket = PendingPairing::new(Zeroizing::new(key.to_vec()), deadline)
        .map_err(|error| LoginError::caused(error.code(), Cause::Handoff(error)))?;
    let display = PairingDisplay::open(key, deadline.into_std()).map_err(|error| {
        LoginError::caused(ErrorCode::UnsupportedContract, Cause::Display(error))
    })?;
    let transaction = Transaction::generate().map_err(|error| {
        LoginError::caused(ErrorCode::UnsupportedContract, Cause::Randomness(error))
    })?;
    let pending = PendingHandoff::for_profile(Profile::CobaltSessionV1, transaction, deadline)
        .map_err(|error| LoginError::caused(error.code(), Cause::Handoff(error)))?;
    let control = LoginControl::new(Arc::clone(&reservation), deadline)
        .map_err(|error| LoginError::caused(error.code(), Cause::Completion(error.into())))?;
    let cancellation = control.cancellation();
    let operation = async {
        let stream = endpoint
            .accept_for_pairing()
            .await
            .map_err(|error| LoginError::caused(error.code(), Cause::Handoff(error)))?;
        let connection = PairedConnection::accept(stream, ticket)
            .await
            .map_err(|error| LoginError::caused(error.code(), Cause::Handoff(error)))?;
        display.close().map_err(|error| {
            LoginError::caused(ErrorCode::UnsupportedContract, Cause::Display(error))
        })?;
        let (connection, receipt) = pending
            .receive(connection)
            .await
            .map_err(|error| LoginError::caused(error.code(), Cause::Handoff(error)))?;
        complete_login(connection, receipt, control)
            .await
            .map_err(|error| LoginError::caused(error.code(), Cause::Completion(error)))
    };
    tokio::pin!(operation);
    tokio::select! {
        biased;
        signal = tokio::signal::ctrl_c() => {
            let code = cancellation.cancel_outcome();
            Err(match signal {
                Ok(()) => LoginError { code, cause: None },
                Err(error) => LoginError::caused(code, Cause::Io(error)),
            })
        }
        result = &mut operation => result,
    }
}

/// Refuses redirected streams before runtime, reservation, key generation, or storage.
///
/// # Errors
///
/// Returns safe terminal/setup failures or the shared completion category.
pub(crate) fn login() -> Result<SavedSession, LoginError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() || !io::stderr().is_terminal() {
        return Err(LoginError::caused(
            ErrorCode::UnsupportedContract,
            Cause::Display(PairingDisplayError::TerminalRequired),
        ));
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| LoginError::caused(ErrorCode::UnsupportedContract, Cause::Io(error)))?;
    let result = runtime.block_on(run_login());
    runtime.shutdown_timeout(Duration::from_secs(1));
    result
}
