//! One-use application exchange on an already authenticated, owned connection.
//!
//! The caller must pair the connection with the correct pending ticket and
//! enforce the OS-wide slot. This controller does not accept unauthenticated
//! streams, select endpoints, validate an account, or access protected storage.
//! A pending login fixes its compiled profile. Receipt cannot produce login success.

use std::{fmt, time::Duration};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use zeroize::Zeroizing;

use super::{
    ErrorCode, HandoffError, Message, Profile, Protocol, Transaction, framing::before_deadline,
    pairing::check_pending_deadline, transport::PairedConnection,
};
use crate::auth::SessionSecret;

/// Owns one transaction's authorization exchange, with no clone or reset operation.
pub struct PendingHandoff {
    transaction: Transaction,
    deadline: Instant,
    profile: Profile,
}

impl PendingHandoff {
    /// Binds a synthetic exchange to its pending transaction and original expiry.
    ///
    /// The deadline must be future and within five minutes. The transaction is
    /// public correlation data; neither this constructor nor parsing authenticates it.
    ///
    /// # Errors
    ///
    /// Returns a fixed timeout or excess-lifetime error without issuing authorization.
    pub fn new(transaction: Transaction, deadline: Instant) -> Result<Self, HandoffError> {
        Self::for_profile(Profile::SyntheticV1, transaction, deadline)
    }

    /// Fixes the profile before any peer can request authorization or submit material.
    ///
    /// This constructor does not authenticate a peer or inspect browser metadata.
    /// Callers must retain the OS-wide slot and use the matching pairing ticket.
    ///
    /// # Errors
    ///
    /// Returns a fixed timeout or excess-lifetime error without issuing authorization.
    pub fn for_profile(
        profile: Profile,
        transaction: Transaction,
        deadline: Instant,
    ) -> Result<Self, HandoffError> {
        check_pending_deadline(deadline)?;
        Ok(Self {
            transaction,
            deadline,
            profile,
        })
    }

    /// Checks correlation without reflecting identifiers or changing another transaction.
    fn check_transaction(&self, transaction: Option<&Transaction>) -> Result<(), HandoffError> {
        if transaction == Some(&self.transaction) {
            Ok(())
        } else {
            Err(HandoffError::new(ErrorCode::TransactionMismatch))
        }
    }

    /// Builds authorization only after a consented open on the paired connection.
    fn authorization(&self) -> Result<Zeroizing<Vec<u8>>, HandoffError> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        let expires_in_ms = u32::try_from(remaining.as_millis())
            .map_err(|_| HandoffError::new(ErrorCode::ProtocolError))?;
        if expires_in_ms == 0 {
            return Err(HandoffError::new(ErrorCode::TimedOut));
        }
        serde_json::to_vec(&serde_json::json!({
            "protocol": Protocol::HostCli.as_str(),
            "version": 1,
            "type": "authorized",
            "transaction": self.transaction.as_str(),
            "profile": self.profile.as_str(),
            "expires_in_ms": expires_in_ms
        }))
        .map(Zeroizing::new)
        .map_err(HandoffError::json)
    }

    /// Consumes authorization before returning one profile-bound submission.
    ///
    /// Missing consent, wrong sequencing, wrong transaction, cancellation,
    /// malformed input, or expiry drops the connection and any received material.
    /// A successful return means receipt only. The returned connection may carry
    /// a later fixed failure; validation and confirmed save require separate proof.
    ///
    /// ```compile_fail
    /// use beyond_gate_core::auth::handoff::flow::PendingHandoff;
    /// use beyond_gate_core::auth::handoff::transport::PairedConnection;
    /// fn cannot_receive_twice(
    ///     pending: PendingHandoff,
    ///     first: PairedConnection<tokio::io::DuplexStream>,
    ///     second: PairedConnection<tokio::io::DuplexStream>,
    /// ) {
    ///     let _first = pending.receive(first);
    ///     let _second = pending.receive(second);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns safe protocol, consent, correlation, cancellation, or transport errors.
    pub async fn receive<S: AsyncRead + AsyncWrite + Unpin>(
        self,
        mut connection: PairedConnection<S>,
    ) -> Result<(PairedConnection<S>, ReceivedSubmission), HandoffError> {
        let hello_deadline = self.deadline.min(Instant::now() + Duration::from_secs(10));
        match connection.read_message(hello_deadline).await? {
            Message::Initiate { profile, .. } => {
                if profile != self.profile {
                    return Err(HandoffError::new(ErrorCode::UnsupportedProfile));
                }
            }
            Message::Cancel { transaction: None } => {
                return Err(HandoffError::new(ErrorCode::Cancelled));
            }
            _ => return Err(HandoffError::new(ErrorCode::ProtocolError)),
        }
        connection
            .write_message(self.authorization()?, self.deadline)
            .await?;
        let value = match connection.read_message(self.deadline).await? {
            Message::Submit {
                transaction,
                profile,
                value,
            } => {
                self.check_transaction(Some(&transaction))?;
                if profile != self.profile {
                    return Err(HandoffError::new(ErrorCode::UnsupportedProfile));
                }
                value
            }
            Message::Cancel { transaction } => {
                self.check_transaction(transaction.as_ref())?;
                return Err(HandoffError::new(ErrorCode::Cancelled));
            }
            Message::Error { transaction, code } => {
                self.check_transaction(transaction.as_ref())?;
                if !matches!(
                    code,
                    ErrorCode::CookieUnavailable
                        | ErrorCode::InvalidMaterial
                        | ErrorCode::ProtocolError
                        | ErrorCode::TimedOut
                        | ErrorCode::Cancelled
                ) {
                    return Err(HandoffError::new(ErrorCode::ProtocolError));
                }
                return Err(HandoffError::new(code));
            }
            _ => return Err(HandoffError::new(ErrorCode::ProtocolError)),
        };
        before_deadline(self.deadline)?;
        Ok((
            connection,
            ReceivedSubmission {
                transaction: self.transaction,
                profile: self.profile,
                value,
            },
        ))
    }
}

impl fmt::Debug for PendingHandoff {
    /// Omits identifiers and timing from diagnostic output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PendingHandoff { fields: [REDACTED] }")
    }
}

/// Owns received material without claiming validation, expiry, or protected save.
pub struct ReceivedSubmission {
    transaction: Transaction,
    profile: Profile,
    value: SessionSecret,
}

impl ReceivedSubmission {
    /// Returns the bound public correlation identifier, not a credential or access grant.
    pub fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    /// Returns the pending login's bound contract, not independently checked metadata.
    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// Consumes receipt material for a separately supported shared establishment operation.
    pub fn into_value(self) -> SessionSecret {
        self.value
    }
}

impl fmt::Debug for ReceivedSubmission {
    /// Omits all material and correlation fields from diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ReceivedSubmission { fields: [REDACTED] }")
    }
}
