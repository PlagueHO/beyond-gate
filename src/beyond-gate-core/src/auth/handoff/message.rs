//! Strict message shapes with owned, zeroizing input and credential buffers.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use zeroize::Zeroizing;

use super::{ErrorCode, HandoffError, MAX_FRAME_BYTES, MAX_JSON_DEPTH};
use crate::auth::SessionSecret;

/// Selects both the payload protocol and its specified length-prefix encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protocol {
    /// Uses Chromium native messaging's native-endian prefix.
    Browser,
    /// Uses the separate host-to-CLI little-endian prefix.
    HostCli,
}

impl Protocol {
    /// Returns the exact version 1 protocol name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Browser => "bg-browser-handoff",
            Self::HostCli => "bg-host-cli",
        }
    }
}

/// Identifies a message's declared browser, not authenticated launch evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Browser {
    /// Declares Chrome without proving its presence or identity.
    Chrome,
    /// Declares Edge without proving its presence or identity.
    Edge,
}

/// Selects a compiled credential contract without authorizing browser access.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Profile {
    /// Never selects a D&D Beyond host or production cookie.
    SyntheticV1,
    /// Uses the observed cookie scope and bounded Windows persistent encoding.
    #[cfg(windows)]
    CobaltSessionV1,
}

impl Profile {
    /// Returns the exact wire identifier without selecting a URL or cookie store.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SyntheticV1 => "synthetic-v1",
            #[cfg(windows)]
            Self::CobaltSessionV1 => "cobalt-session-v1",
        }
    }
}

/// Holds a validated public correlation identifier, never an access grant.
#[derive(Clone, Eq, PartialEq)]
pub struct Transaction(String);

impl Transaction {
    /// Generates a public 128-bit correlation identifier from system randomness.
    ///
    /// This identifier is not a pairing secret or an access grant.
    ///
    /// # Errors
    ///
    /// Returns a redacted randomness failure without a fallback identifier.
    pub fn generate() -> Result<Self, super::pairing::RandomnessError> {
        let mut bytes = Zeroizing::new([0; 16]);
        super::pairing::fill_random(&mut *bytes)?;
        let mut value = String::with_capacity(32);
        let digits = b"0123456789abcdef";
        for byte in bytes.iter() {
            value.push(char::from(digits[usize::from(byte >> 4)]));
            value.push(char::from(digits[usize::from(byte & 15)]));
        }
        Ok(Self(value))
    }

    /// Checks the exact 32-character lowercase hexadecimal syntax.
    fn parse(mut value: ProtectedText) -> Result<Self, HandoffError> {
        if value.0.len() != 32
            || !value
                .0
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        Ok(Self(std::mem::take(&mut *value.0)))
    }

    /// Exposes the correlation value without implying authentication.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Transaction {
    /// Omits correlation data from diagnostic messages.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Transaction { value: [REDACTED] }")
    }
}

/// Represents parsed syntax only; no variant proves authorization or login.
pub enum Message {
    /// Transfers one user-entered key before any production cookie authorization.
    Pair {
        /// A declared browser, not independently verified launch evidence.
        browser: Browser,
        /// The compiled contract requested with explicit consent.
        profile: Profile,
        /// Owns the decoded, zeroizing key; never copied into diagnostics.
        key: super::pairing::PairingKey,
    },
    /// Parses `hello` or `open`, including explicit Boolean consent.
    Initiate {
        /// A declared browser, not a verified process.
        browser: Browser,
        /// The compiled credential contract, not browser metadata evidence.
        profile: Profile,
    },
    /// Parses `pending` or `authorized`, without creating a local authorization.
    Authorization {
        /// A correlation identifier, not a capability.
        transaction: Transaction,
        /// Must match the pending login's compiled credential contract.
        profile: Profile,
        /// A relative time bound that cannot extend an authoritative deadline.
        expires_in_ms: u32,
    },
    /// Owns one syntactically valid value within its profile's size limit.
    Submit {
        /// A correlation identifier that still needs connection binding.
        transaction: Transaction,
        /// Must match the profile bound to the authenticated connection.
        profile: Profile,
        /// Clears this owned credential allocation on drop.
        value: SessionSecret,
    },
    /// Carries cancellation syntax without cancelling any pending operation.
    Cancel {
        /// Is null before binding and non-null after binding.
        transaction: Option<Transaction>,
    },
    /// Parses the exact final literals without establishing service or save proof.
    Result {
        /// Still requires the correct connection, state, and operation evidence.
        transaction: Transaction,
    },
    /// Confirms receipt of the final result on its bound connection.
    Acknowledgement {
        /// Must match the result; an early acknowledgement is a protocol failure.
        transaction: Transaction,
    },
    /// Carries a fixed safe code, never a peer-supplied diagnostic string.
    Error {
        /// Is null before binding and non-null after binding.
        transaction: Option<Transaction>,
        /// A known version 1 code.
        code: ErrorCode,
    },
}

impl fmt::Debug for Message {
    /// Omits every message field, including cookie values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Message { fields: [REDACTED] }")
    }
}

/// Clears owned decoded strings, including rejected field values, on drop.
struct ProtectedText(Zeroizing<String>);

impl<'de> Deserialize<'de> for ProtectedText {
    /// Takes ownership immediately after serde decodes a string.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|value| Self(Zeroizing::new(value)))
    }
}

/// Distinguishes absent fields from explicitly supplied null or other values.
enum Field<T> {
    /// No such key occurred in the input object.
    Missing,
    /// The key occurred, including a null transaction.
    Present(T),
}

impl<T> Default for Field<T> {
    /// Marks an omitted field without inventing a wire value.
    fn default() -> Self {
        Self::Missing
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Field<T> {
    /// Keeps presence separate from the field's permitted type.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::Present)
    }
}

impl<T> Field<T> {
    /// Reports key presence even when the permitted value is null.
    fn present(&self) -> bool {
        matches!(self, Self::Present(_))
    }

    /// Requires a key without substituting a default.
    fn require(self) -> Result<T, HandoffError> {
        match self {
            Self::Present(value) => Ok(value),
            Self::Missing => Err(HandoffError::new(ErrorCode::ProtocolError)),
        }
    }
}

/// Rejects duplicate and unknown keys before conversion to exact message variants.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMessage {
    protocol: ProtectedText,
    version: u32,
    #[serde(rename = "type")]
    kind: ProtectedText,
    #[serde(default)]
    browser: Field<ProtectedText>,
    #[serde(default)]
    profile: Field<ProtectedText>,
    #[serde(default)]
    consent: Field<bool>,
    #[serde(default)]
    transaction: Field<Option<ProtectedText>>,
    #[serde(default)]
    expires_in_ms: Field<u32>,
    #[serde(default)]
    value: Field<ProtectedText>,
    #[serde(default)]
    key: Field<ProtectedText>,
    #[serde(default)]
    outcome: Field<ProtectedText>,
    #[serde(default)]
    validation: Field<ProtectedText>,
    #[serde(default)]
    expiry: Field<ProtectedText>,
    #[serde(default)]
    code: Field<ProtectedText>,
}

impl RawMessage {
    /// Rejects fields belonging to a different message, including explicit nulls.
    fn shape(&self, expected: &[&str]) -> Result<(), HandoffError> {
        for (name, present) in [
            ("browser", self.browser.present()),
            ("profile", self.profile.present()),
            ("consent", self.consent.present()),
            ("transaction", self.transaction.present()),
            ("expires_in_ms", self.expires_in_ms.present()),
            ("value", self.value.present()),
            ("key", self.key.present()),
            ("outcome", self.outcome.present()),
            ("validation", self.validation.present()),
            ("expiry", self.expiry.present()),
            ("code", self.code.present()),
        ] {
            if present != expected.contains(&name) {
                return Err(HandoffError::new(ErrorCode::ProtocolError));
            }
        }
        Ok(())
    }

    /// Checks protocol-specific kinds and exact fields before consuming owned data.
    fn convert(self, protocol: Protocol) -> Result<Message, HandoffError> {
        if self.protocol.0.as_str() != protocol.as_str() {
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        if self.version != 1 {
            return Err(HandoffError::new(ErrorCode::VersionMismatch));
        }
        let fields: &[&str] = match (protocol, self.kind.0.as_str()) {
            (Protocol::Browser, "pair") => &["browser", "profile", "consent", "key"],
            (Protocol::Browser, "hello") | (Protocol::HostCli, "open") => {
                &["browser", "profile", "consent"]
            }
            (Protocol::Browser, "pending") | (Protocol::HostCli, "authorized") => {
                &["transaction", "profile", "expires_in_ms"]
            }
            (_, "submit") => &["transaction", "profile", "value"],
            (_, "cancel") => &["transaction"],
            (_, "ack") => &["transaction"],
            (_, "result") => &["transaction", "outcome", "validation", "expiry"],
            (_, "error") => &["transaction", "code"],
            _ => return Err(HandoffError::new(ErrorCode::ProtocolError)),
        };
        self.shape(fields)?;
        match self.kind.0.as_str() {
            "pair" => {
                if !self.consent.require()? {
                    return Err(HandoffError::new(ErrorCode::ConsentRequired));
                }
                Ok(Message::Pair {
                    browser: browser(self.browser.require()?)?,
                    profile: profile(self.profile.require()?)?,
                    key: super::pairing::PairingKey::from_encoded(self.key.require()?.0)?,
                })
            }
            "hello" | "open" => {
                if !self.consent.require()? {
                    return Err(HandoffError::new(ErrorCode::ConsentRequired));
                }
                Ok(Message::Initiate {
                    browser: browser(self.browser.require()?)?,
                    profile: profile(self.profile.require()?)?,
                })
            }
            "pending" | "authorized" => {
                let expires_in_ms = self.expires_in_ms.require()?;
                if !(1..=300_000).contains(&expires_in_ms) {
                    return Err(HandoffError::new(ErrorCode::ProtocolError));
                }
                Ok(Message::Authorization {
                    transaction: required_transaction(self.transaction)?,
                    profile: profile(self.profile.require()?)?,
                    expires_in_ms,
                })
            }
            "submit" => {
                let profile = profile(self.profile.require()?)?;
                Ok(Message::Submit {
                    transaction: required_transaction(self.transaction)?,
                    profile,
                    value: material(self.value.require()?, profile)?,
                })
            }
            "cancel" => Ok(Message::Cancel {
                transaction: self
                    .transaction
                    .require()?
                    .map(Transaction::parse)
                    .transpose()?,
            }),
            "ack" => Ok(Message::Acknowledgement {
                transaction: required_transaction(self.transaction)?,
            }),
            "result" => {
                if self.outcome.require()?.0.as_str() != "saved"
                    || self.validation.require()?.0.as_str() != "service-validated"
                    || self.expiry.require()?.0.as_str() != "unknown"
                {
                    return Err(HandoffError::new(ErrorCode::ProtocolError));
                }
                Ok(Message::Result {
                    transaction: required_transaction(self.transaction)?,
                })
            }
            "error" => Ok(Message::Error {
                transaction: self
                    .transaction
                    .require()?
                    .map(Transaction::parse)
                    .transpose()?,
                code: ErrorCode::parse(&self.code.require()?.0)?,
            }),
            _ => Err(HandoffError::new(ErrorCode::ProtocolError)),
        }
    }
}

/// Checks a browser literal without interpreting it as process identity.
fn browser(value: ProtectedText) -> Result<Browser, HandoffError> {
    match value.0.as_str() {
        "chrome" => Ok(Browser::Chrome),
        "edge" => Ok(Browser::Edge),
        _ => Err(HandoffError::new(ErrorCode::ProtocolError)),
    }
}

/// Accepts exact compiled profile identifiers without a default or downgrade.
fn profile(value: ProtectedText) -> Result<Profile, HandoffError> {
    match value.0.as_str() {
        "synthetic-v1" => Ok(Profile::SyntheticV1),
        #[cfg(windows)]
        "cobalt-session-v1" => Ok(Profile::CobaltSessionV1),
        _ => Err(HandoffError::new(ErrorCode::UnsupportedProfile)),
    }
}

/// Rejects null correlation identifiers on messages that require binding.
fn required_transaction(field: Field<Option<ProtectedText>>) -> Result<Transaction, HandoffError> {
    let value = field
        .require()?
        .ok_or_else(|| HandoffError::new(ErrorCode::ProtocolError))?;
    Transaction::parse(value)
}

/// Moves profile-bounded cookie octets into the existing zeroizing secret type.
fn material(mut value: ProtectedText, profile: Profile) -> Result<SessionSecret, HandoffError> {
    let within_profile = match profile {
        Profile::SyntheticV1 => value.0.len() <= 8192 && value.0.starts_with("synthetic-"),
        #[cfg(windows)]
        Profile::CobaltSessionV1 => value.0.len() <= crate::auth::service::MAX_COOKIE_BYTES,
    };
    if !within_profile
        || value.0.is_empty()
        || !value.0.bytes().all(
            |byte| matches!(byte, 0x21 | 0x23..=0x2b | 0x2d..=0x3a | 0x3c..=0x5b | 0x5d..=0x7e),
        )
    {
        return Err(HandoffError::new(ErrorCode::InvalidMaterial));
    }
    let bytes = std::mem::take(&mut *value.0).into_bytes();
    SessionSecret::new(bytes).map_err(HandoffError::secret)
}

/// Rejects excessive nesting before serde allocates decoded field values.
fn check_depth(bytes: &[u8]) -> Result<(), HandoffError> {
    let mut depth = 0_usize;
    let mut quoted = false;
    let mut escaped = false;
    for &byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > MAX_JSON_DEPTH {
                        return Err(HandoffError::new(ErrorCode::ProtocolError));
                    }
                }
                b'}' | b']' => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| HandoffError::new(ErrorCode::ProtocolError))?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Parses a borrowed zeroizing frame buffer without relaxing the JSON object shape.
pub(super) fn parse_payload(protocol: Protocol, bytes: &[u8]) -> Result<Message, HandoffError> {
    if bytes.is_empty()
        || bytes.len() > MAX_FRAME_BYTES
        || bytes
            .iter()
            .find(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
            != Some(&b'{')
    {
        return Err(HandoffError::new(ErrorCode::ProtocolError));
    }
    check_depth(bytes)?;
    let raw: RawMessage = serde_json::from_slice(bytes).map_err(HandoffError::json)?;
    raw.convert(protocol)
}

/// Parses and clears an owned payload; dependency-internal copies are not guaranteed to clear.
///
/// This validates syntax only. No variant grants peer authentication, cookie
/// access, service validation, or protected-save success.
///
/// ```
/// use beyond_gate_core::auth::handoff::{Message, Protocol, parse_message};
///
/// let payload = br#"{"protocol":"bg-browser-handoff","version":1,"type":"hello",
/// "browser":"edge","profile":"synthetic-v1","consent":true}"#.to_vec();
/// let message = parse_message(Protocol::Browser, payload)?;
/// assert!(matches!(message, Message::Initiate { .. }));
/// # Ok::<(), beyond_gate_core::auth::handoff::HandoffError>(())
/// ```
///
/// # Errors
///
/// Returns fixed, redacted errors for invalid frames, fields, versions, profiles,
/// consent, and cookie material.
pub fn parse_message(protocol: Protocol, bytes: Vec<u8>) -> Result<Message, HandoffError> {
    let bytes = Zeroizing::new(bytes);
    parse_payload(protocol, &bytes)
}

/// Serializes only borrowed fields so cookie values have no intermediate JSON string.
#[derive(Serialize)]
#[serde(tag = "type")]
enum Outgoing<'a> {
    #[serde(rename = "hello")]
    Hello {
        browser: &'static str,
        profile: &'static str,
        consent: bool,
    },
    #[serde(rename = "open")]
    Open {
        browser: &'static str,
        profile: &'static str,
        consent: bool,
    },
    #[serde(rename = "pending")]
    Pending {
        transaction: &'a str,
        profile: &'static str,
        expires_in_ms: u32,
    },
    #[serde(rename = "authorized")]
    Authorized {
        transaction: &'a str,
        profile: &'static str,
        expires_in_ms: u32,
    },
    #[serde(rename = "submit")]
    Submit {
        transaction: &'a str,
        profile: &'static str,
        value: &'a str,
    },
    #[serde(rename = "cancel")]
    Cancel { transaction: Option<&'a str> },
    #[serde(rename = "ack")]
    Ack { transaction: &'a str },
    #[serde(rename = "result")]
    Result {
        transaction: &'a str,
        outcome: &'static str,
        validation: &'static str,
        expiry: &'static str,
    },
    #[serde(rename = "error")]
    Error {
        transaction: Option<&'a str>,
        code: &'static str,
    },
}

/// Borrows the message body and adds only fixed protocol metadata.
#[derive(Serialize)]
struct Envelope<'a> {
    protocol: &'static str,
    version: u32,
    #[serde(flatten)]
    message: Outgoing<'a>,
}

/// Encodes one known message into owned zeroizing bytes without granting its claims.
///
/// Pairing keys are input-only here. A result still requires separate confirmed
/// service validation and save proof from the trusted CLI controller.
///
/// # Errors
///
/// Rejects pairing-key output, invalid cookie bytes, and invalid message fields.
pub fn encode_message(
    protocol: Protocol,
    message: &Message,
) -> Result<Zeroizing<Vec<u8>>, HandoffError> {
    let outgoing = match message {
        Message::Pair { .. } => return Err(HandoffError::new(ErrorCode::ProtocolError)),
        Message::Initiate { browser, profile } => {
            let browser = match browser {
                Browser::Chrome => "chrome",
                Browser::Edge => "edge",
            };
            match protocol {
                Protocol::Browser => Outgoing::Hello {
                    browser,
                    profile: profile.as_str(),
                    consent: true,
                },
                Protocol::HostCli => Outgoing::Open {
                    browser,
                    profile: profile.as_str(),
                    consent: true,
                },
            }
        }
        Message::Authorization {
            transaction,
            profile,
            expires_in_ms,
        } => match protocol {
            Protocol::Browser => Outgoing::Pending {
                transaction: transaction.as_str(),
                profile: profile.as_str(),
                expires_in_ms: *expires_in_ms,
            },
            Protocol::HostCli => Outgoing::Authorized {
                transaction: transaction.as_str(),
                profile: profile.as_str(),
                expires_in_ms: *expires_in_ms,
            },
        },
        Message::Submit {
            transaction,
            profile,
            value,
        } => Outgoing::Submit {
            transaction: transaction.as_str(),
            profile: profile.as_str(),
            value: std::str::from_utf8(&value.0)
                .map_err(|_| HandoffError::new(ErrorCode::InvalidMaterial))?,
        },
        Message::Cancel { transaction } => Outgoing::Cancel {
            transaction: transaction.as_ref().map(Transaction::as_str),
        },
        Message::Acknowledgement { transaction } => Outgoing::Ack {
            transaction: transaction.as_str(),
        },
        Message::Result { transaction } => Outgoing::Result {
            transaction: transaction.as_str(),
            outcome: "saved",
            validation: "service-validated",
            expiry: "unknown",
        },
        Message::Error { transaction, code } => Outgoing::Error {
            transaction: transaction.as_ref().map(Transaction::as_str),
            code: code.as_str(),
        },
    };
    let bytes = serde_json::to_vec(&Envelope {
        protocol: protocol.as_str(),
        version: 1,
        message: outgoing,
    })
    .map(Zeroizing::new)
    .map_err(HandoffError::json)?;
    drop(parse_payload(protocol, &bytes)?);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodes invented bytes under the production profile without browser access.
    fn cobalt_payload(protocol: Protocol, kind: &str, value: &str) -> Vec<u8> {
        let mut message = serde_json::json!({
            "protocol": protocol.as_str(),
            "version": 1,
            "type": kind,
            "profile": "cobalt-session-v1"
        });
        match kind {
            "hello" | "open" => {
                message["browser"] = "edge".into();
                message["consent"] = true.into();
            }
            "pending" | "authorized" => {
                message["transaction"] = "0123456789abcdef0123456789abcdef".into();
                message["expires_in_ms"] = 300_000.into();
            }
            "submit" => {
                message["transaction"] = "0123456789abcdef0123456789abcdef".into();
                message["value"] = value.into();
            }
            _ => panic!("test message kind"),
        }
        serde_json::to_vec(&message).expect("invented message")
    }

    /// Checks both wire protocols without treating parsing as authorization.
    #[cfg(windows)]
    #[test]
    fn windows_profile_parses_both_protocols_and_redacts_material() {
        for (protocol, kinds) in [
            (Protocol::Browser, ["hello", "pending", "submit"]),
            (Protocol::HostCli, ["open", "authorized", "submit"]),
        ] {
            for kind in kinds {
                let message = parse_message(
                    protocol,
                    cobalt_payload(protocol, kind, "invented-cookie-value"),
                )
                .expect("compiled Windows profile");
                let profile = match &message {
                    Message::Initiate { profile, .. }
                    | Message::Authorization { profile, .. }
                    | Message::Submit { profile, .. } => *profile,
                    _ => panic!("profile-bearing message"),
                };
                assert_eq!(profile, Profile::CobaltSessionV1);
                assert_eq!(profile.as_str(), "cobalt-session-v1");
                assert_eq!(format!("{message:?}"), "Message { fields: [REDACTED] }");
            }
        }
    }

    /// Enforces the exact native-format limit, not the larger synthetic wire cap.
    #[cfg(windows)]
    #[test]
    fn windows_cookie_limit_and_syntax_apply_to_both_protocols() {
        for protocol in [Protocol::Browser, Protocol::HostCli] {
            assert!(
                parse_message(
                    protocol,
                    cobalt_payload(protocol, "submit", &"x".repeat(2_552))
                )
                .is_ok()
            );
            for invalid in [
                "x".repeat(2_553),
                String::new(),
                " x".to_owned(),
                "x;y".to_owned(),
                "\"x\"".to_owned(),
                "x\r\nInjected: value".to_owned(),
                "\u{e9}".to_owned(),
            ] {
                let error = parse_message(protocol, cobalt_payload(protocol, "submit", &invalid))
                    .expect_err("invalid material");
                assert_eq!(error.code(), ErrorCode::InvalidMaterial);
            }
        }
    }

    /// Keeps synthetic admission separate from production and rejects unknown profiles.
    #[cfg(windows)]
    #[test]
    fn profile_names_never_default_or_relax_synthetic_material() {
        for protocol in [Protocol::Browser, Protocol::HostCli] {
            let mut message: serde_json::Value = serde_json::from_slice(&cobalt_payload(
                protocol,
                "submit",
                "invented-cookie-value",
            ))
            .expect("invented message");
            for (profile, expected) in [
                ("synthetic-v1", ErrorCode::InvalidMaterial),
                ("cobalt-session-v2", ErrorCode::UnsupportedProfile),
                ("Cobalt-session-v1", ErrorCode::UnsupportedProfile),
                ("ddb-session-v1", ErrorCode::UnsupportedProfile),
            ] {
                message["profile"] = profile.into();
                assert_eq!(
                    parse_message(protocol, serde_json::to_vec(&message).expect("message"))
                        .expect_err("profile rejection")
                        .code(),
                    expected
                );
            }
        }
    }

    /// Leaves deferred platforms unable to admit production cookie material.
    #[cfg(not(windows))]
    #[test]
    fn production_profile_remains_unsupported_outside_windows() {
        for protocol in [Protocol::Browser, Protocol::HostCli] {
            assert_eq!(
                parse_message(
                    protocol,
                    cobalt_payload(protocol, "submit", "invented-cookie-value")
                )
                .expect_err("deferred platform")
                .code(),
                ErrorCode::UnsupportedProfile
            );
        }
    }
}
