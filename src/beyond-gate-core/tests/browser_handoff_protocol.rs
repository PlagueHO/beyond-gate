//! Executes real framing and parser code against shared synthetic recipes.
//!
//! These tests do not mock peer authentication into existence or claim a
//! browser, native-host process, service validation, or protected save.

use std::{error::Error, time::Duration};

use beyond_gate_core::auth::handoff::{
    ErrorCode, FrameReader, FrameWriter, Message, Protocol, parse_message,
};
use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, duplex},
    time::{Instant, timeout},
};
use zeroize::Zeroizing;

mod support;
use support::fixture;

/// Selects the fixed protocol of a canonical synthetic example.
fn protocol(message: &Value) -> Protocol {
    match message["protocol"].as_str().expect("message protocol") {
        "bg-browser-handoff" => Protocol::Browser,
        "bg-host-cli" => Protocol::HostCli,
        _ => panic!("fixture protocol must be supported"),
    }
}

/// Encodes synthetic test data without reading any real session material.
fn bytes(message: &Value) -> Vec<u8> {
    serde_json::to_vec(message).expect("encode synthetic message")
}

/// Applies the fixture's message mutations, value-size, and padding recipes.
fn recipe(fixture: &Value, case: &Value) -> (Protocol, Vec<u8>) {
    let input = &case["input"];
    let name = input["message"].as_str().expect("named message recipe");
    let mut message = fixture["messages"][name].clone();
    let protocol = protocol(&message);
    let object = message.as_object_mut().expect("message object");
    for operation in ["set", "add"] {
        if let Some(fields) = input[operation].as_object() {
            for (name, value) in fields {
                object.insert(name.clone(), value.clone());
            }
        }
    }
    if let Some(fields) = input["remove"].as_array() {
        for field in fields {
            object.remove(field.as_str().expect("removed field name"));
        }
    }
    if let Some(length) = input["value_bytes"].as_u64() {
        let length = usize::try_from(length).expect("bounded fixture value length");
        let value = if length >= 10 {
            format!("synthetic-{}", "s".repeat(length - 10))
        } else {
            "s".repeat(length)
        };
        object.insert("value".to_owned(), Value::String(value));
    }
    let mut payload = bytes(&message);
    if let Some(length) = input["frame"]["pad_json_to_bytes"].as_u64() {
        let length = usize::try_from(length).expect("bounded fixture frame length");
        assert!(length >= payload.len());
        payload.resize(length, b' ');
    }
    (protocol, payload)
}

/// Parses every canonical example without treating a parsed result as login proof.
#[test]
fn all_canonical_messages_parse_and_redact_fields() {
    let fixture = fixture();
    for message in fixture["messages"].as_object().expect("messages").values() {
        let parsed = parse_message(protocol(message), bytes(message)).expect("canonical message");
        assert_eq!(format!("{parsed:?}"), "Message { fields: [REDACTED] }");
    }
}

/// Executes only message-parser recipes, not their later state or storage effects.
#[test]
fn shared_message_recipes_execute_the_real_parser() {
    let fixture = fixture();
    let ids = [
        "short-synthetic-value",
        "hello-refuses-false-consent",
        "unsupported-version",
        "production-profile-not-selected",
        "unknown-field-not-peer-proof",
        "missing-field",
        "wrong-field-type",
        "exact-value-bound",
        "oversized-value",
        "empty-value",
        "header-injection-value",
        "exact-frame-bound",
    ];
    let mut exercised = 0;
    for case in fixture["cases"].as_array().expect("cases") {
        if ids.contains(&case["id"].as_str().expect("case ID")) {
            let (protocol, payload) = recipe(&fixture, case);
            let result = parse_message(protocol, payload);
            match case["expected"]["code"].as_str().expect("expected code") {
                "ok" => assert!(result.is_ok(), "{}", case["id"]),
                code => assert_eq!(result.expect_err("rejected recipe").code().as_str(), code),
            }
            exercised += 1;
        }
    }
    assert_eq!(exercised, ids.len(), "every selected recipe must run");
}

/// Rejects duplicates, including escaped keys, non-object roots, BOMs, and trailing data.
#[test]
fn strict_json_rejects_duplicate_keys_and_other_encodings() {
    let fixture = fixture();
    let hello = String::from_utf8(bytes(&fixture["messages"]["browser_hello"])).expect("UTF-8");
    for invalid in [
        hello.replace("\"consent\":true", "\"consent\":true,\"consent\":true"),
        hello.replace(
            "\"consent\":true",
            "\"consent\":true,\"\\u0063onsent\":true",
        ),
        hello.replace("\"browser\":\"edge\"", "\"browser\":null"),
        format!("[{hello}]"),
        format!("{hello}{{}}"),
        format!("\u{feff}{hello}"),
    ] {
        assert_eq!(
            parse_message(Protocol::Browser, invalid.into_bytes())
                .expect_err("invalid JSON")
                .code(),
            ErrorCode::ProtocolError
        );
    }
}

/// Rejects a foreign message kind even when its fields otherwise match.
#[test]
fn protocol_identity_and_message_kind_cannot_be_interchanged() {
    let fixture = fixture();
    let mut open = fixture["messages"]["cli_open"].clone();
    assert_eq!(
        parse_message(Protocol::Browser, bytes(&open))
            .expect_err("foreign protocol")
            .code(),
        ErrorCode::ProtocolError
    );
    open["protocol"] = Value::String(Protocol::Browser.as_str().to_owned());
    assert_eq!(
        parse_message(Protocol::Browser, bytes(&open))
            .expect_err("foreign kind")
            .code(),
        ErrorCode::ProtocolError
    );
}

/// Permits a null cancel transaction but rejects omission and duplicate null keys.
#[test]
fn nullable_transactions_preserve_key_presence() {
    let fixture = fixture();
    let mut cancel = fixture["messages"]["browser_cancel"].clone();
    cancel["transaction"] = Value::Null;
    assert!(matches!(
        parse_message(Protocol::Browser, bytes(&cancel)).expect("unbound cancel"),
        Message::Cancel { transaction: None }
    ));
    let text = String::from_utf8(bytes(&cancel)).expect("UTF-8");
    let duplicate = text.replace(
        "\"transaction\":null",
        "\"transaction\":null,\"transaction\":null",
    );
    assert_eq!(
        parse_message(Protocol::Browser, duplicate.into_bytes())
            .expect_err("duplicate null")
            .code(),
        ErrorCode::ProtocolError
    );
    cancel
        .as_object_mut()
        .expect("object")
        .remove("transaction");
    assert_eq!(
        parse_message(Protocol::Browser, bytes(&cancel))
            .expect_err("missing key")
            .code(),
        ErrorCode::ProtocolError
    );
}

/// Checks depth before serde allocation and ignores delimiters inside quoted values.
#[test]
fn json_depth_is_bounded_without_counting_quoted_brackets() {
    let deep = format!("{{\"extra\":{}{{}}{}}}", "[".repeat(8), "]".repeat(8));
    let error = parse_message(Protocol::Browser, deep.into_bytes()).expect_err("depth nine");
    assert_eq!(error.code(), ErrorCode::ProtocolError);
    assert!(error.source().is_none(), "depth must fail before serde");
    let fixture = fixture();
    let mut submit = fixture["messages"]["cli_submit"].clone();
    submit["value"] = Value::String("synthetic-[[[[[[[[[{}]]]]]]]]]".to_owned());
    assert!(parse_message(Protocol::HostCli, bytes(&submit)).is_ok());
}

/// Ensures every standard formatter and cause chain hides malformed peer input.
#[test]
fn malformed_fields_and_source_chains_remain_redacted() {
    let fixture = fixture();
    let mut hello = fixture["messages"]["browser_hello"].clone();
    hello["synthetic-private-field"] = Value::String("synthetic-private-value".to_owned());
    let error = parse_message(Protocol::Browser, bytes(&hello)).expect_err("unknown field");
    let mut source: Option<&dyn Error> = Some(&error);
    while let Some(error) = source {
        for text in [
            format!("{error}"),
            format!("{error:?}"),
            format!("{error:#?}"),
        ] {
            assert!(!text.contains("synthetic-private"));
        }
        source = error.source();
    }
    assert!(ErrorCode::parse("ok").is_err());
    assert!(ErrorCode::parse("synthetic-private-value").is_err());
}

/// Checks both prefix contracts and exercises the real writer and reader separately.
#[tokio::test]
async fn both_protocols_use_their_declared_prefix_and_round_trip() {
    let fixture = fixture();
    for name in ["browser_hello", "cli_open"] {
        let message = &fixture["messages"][name];
        let protocol = protocol(message);
        let payload = bytes(message);
        let (sender, mut receiver) = duplex(32768);
        let mut writer = FrameWriter::new(sender, protocol);
        writer
            .write_message(
                Zeroizing::new(payload.clone()),
                Instant::now() + Duration::from_secs(1),
            )
            .await
            .expect("bounded write");
        let mut prefix = [0; 4];
        receiver.read_exact(&mut prefix).await.expect("prefix");
        let length = u32::try_from(payload.len()).expect("bounded payload");
        assert_eq!(
            prefix,
            match protocol {
                Protocol::Browser => length.to_ne_bytes(),
                Protocol::HostCli => length.to_le_bytes(),
            }
        );
        let mut received = vec![0; payload.len()];
        receiver.read_exact(&mut received).await.expect("payload");
        assert_eq!(received, payload);

        let mut wire = prefix.to_vec();
        wire.extend_from_slice(&payload);
        let mut reader = FrameReader::new(wire.as_slice(), protocol);
        assert!(matches!(
            reader
                .read_message(Instant::now() + Duration::from_secs(1))
                .await
                .expect("read"),
            Message::Initiate { .. }
        ));
    }
}

/// Rejects oversized length prefixes before touching any payload bytes.
#[tokio::test]
async fn oversized_prefix_never_reads_the_payload() {
    let mut wire = 16385_u32.to_le_bytes().to_vec();
    wire.extend_from_slice(b"synthetic-unread");
    let mut input = wire.as_slice();
    {
        let mut reader = FrameReader::new(&mut input, Protocol::HostCli);
        let error = reader
            .read_message(Instant::now() + Duration::from_secs(1))
            .await
            .expect_err("oversized prefix");
        assert_eq!(error.code(), ErrorCode::ProtocolError);
        assert!(
            error.source().is_none(),
            "length must fail before payload read"
        );
    }
    assert_eq!(input, b"synthetic-unread");
}

/// Stops before a ninth frame, without consuming or retrying its payload.
#[tokio::test]
async fn inbound_frame_budget_is_exactly_eight() {
    let fixture = fixture();
    let payload = bytes(&fixture["messages"]["cli_open"]);
    let mut frame = u32::try_from(payload.len())
        .expect("bounded payload")
        .to_le_bytes()
        .to_vec();
    frame.extend_from_slice(&payload);
    let wire = frame.repeat(9);
    let mut input = wire.as_slice();
    {
        let mut reader = FrameReader::new(&mut input, Protocol::HostCli);
        for _ in 0..8 {
            reader
                .read_message(Instant::now() + Duration::from_secs(1))
                .await
                .expect("frame");
        }
        assert_eq!(
            reader
                .read_message(Instant::now() + Duration::from_secs(1))
                .await
                .expect_err("ninth frame")
                .code(),
            ErrorCode::ProtocolError
        );
    }
    assert_eq!(input, frame);
}

/// Decodes the fixture's small hexadecimal recipes without accepting malformed test data.
fn hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).expect("fixture hex"))
        .collect()
}

/// Executes the shared malformed-frame recipes against the actual frame reader.
#[tokio::test]
async fn shared_frame_recipes_execute_the_real_reader() {
    let fixture = fixture();
    let ids = [
        "oversized-frame-before-allocation",
        "empty-frame",
        "truncated-prefix",
        "truncated-payload",
        "invalid-utf8",
        "duplicate-json-key",
    ];
    let mut exercised = 0;
    for case in fixture["cases"].as_array().expect("cases") {
        if !ids.contains(&case["id"].as_str().expect("case ID")) {
            continue;
        }
        let protocol = match case["boundary"].as_str().expect("boundary") {
            "browser" => Protocol::Browser,
            "host-cli" => Protocol::HostCli,
            _ => panic!("frame recipe boundary"),
        };
        let frame = &case["input"]["frame"];
        let mut wire = if let Some(prefix) = frame["prefix_hex"].as_str() {
            hex(prefix)
        } else {
            let length = u32::try_from(frame["declared_length"].as_u64().expect("length"))
                .expect("u32 length");
            match protocol {
                Protocol::Browser => length.to_ne_bytes().to_vec(),
                Protocol::HostCli => length.to_le_bytes().to_vec(),
            }
        };
        if let Some(payload) = frame["payload_hex"].as_str() {
            wire.extend_from_slice(&hex(payload));
        }
        let mut reader = FrameReader::new(wire.as_slice(), protocol);
        let error = reader
            .read_message(Instant::now() + Duration::from_secs(1))
            .await
            .expect_err("invalid frame");
        assert_eq!(
            error.code().as_str(),
            case["expected"]["code"].as_str().expect("code")
        );
        exercised += 1;
    }
    assert_eq!(exercised, ids.len());
}

/// Writes the exact 16 KiB payload bound without accepting or truncating an extra byte.
#[tokio::test]
async fn exact_frame_bound_round_trips_without_truncation() {
    let fixture = fixture();
    let case = fixture["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| case["id"] == "exact-frame-bound")
        .expect("exact frame recipe");
    let (protocol, payload) = recipe(&fixture, case);
    assert_eq!(payload.len(), 16384);
    let (sender, mut receiver) = duplex(32768);
    let mut writer = FrameWriter::new(sender, protocol);
    let mut oversized = payload.clone();
    oversized.push(b' ');
    assert_eq!(
        writer
            .write_message(
                Zeroizing::new(oversized),
                Instant::now() + Duration::from_secs(1)
            )
            .await
            .expect_err("oversized write")
            .code(),
        ErrorCode::ProtocolError
    );
    drop(writer);
    let mut rejected_output = Vec::new();
    receiver
        .read_to_end(&mut rejected_output)
        .await
        .expect("rejected output");
    assert!(
        rejected_output.is_empty(),
        "invalid payload must not write a prefix"
    );

    let (sender, receiver) = duplex(32768);
    let mut writer = FrameWriter::new(sender, protocol);
    writer
        .write_message(
            Zeroizing::new(payload),
            Instant::now() + Duration::from_secs(1),
        )
        .await
        .expect("exact bound write");
    let mut reader = FrameReader::new(receiver, protocol);
    assert!(matches!(
        reader
            .read_message(Instant::now() + Duration::from_secs(1))
            .await
            .expect("exact read"),
        Message::Submit { .. }
    ));
}

/// Enforces a separate outbound budget and emits no ninth prefix or payload.
#[tokio::test]
async fn outbound_frame_budget_is_exactly_eight() {
    let fixture = fixture();
    let payload = bytes(&fixture["messages"]["cli_open"]);
    let frame_bytes = payload.len() + 4;
    let (sender, mut receiver) = duplex(32768);
    let mut writer = FrameWriter::new(sender, Protocol::HostCli);
    for _ in 0..8 {
        writer
            .write_message(
                Zeroizing::new(payload.clone()),
                Instant::now() + Duration::from_secs(1),
            )
            .await
            .expect("outbound frame");
    }
    assert_eq!(
        writer
            .write_message(
                Zeroizing::new(payload),
                Instant::now() + Duration::from_secs(1)
            )
            .await
            .expect_err("ninth outbound frame")
            .code(),
        ErrorCode::ProtocolError
    );
    drop(writer);
    let mut output = Vec::new();
    receiver
        .read_to_end(&mut output)
        .await
        .expect("owned output");
    assert_eq!(output.len(), 8 * frame_bytes);
}

/// Rejects expired and out-of-range authorization syntax without guessing new time bounds.
#[test]
fn relative_expiry_is_a_bounded_integer() {
    let fixture = fixture();
    let mut message = fixture["messages"]["cli_authorized"].clone();
    for value in [
        serde_json::json!(0),
        serde_json::json!(300001),
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!("1"),
    ] {
        message["expires_in_ms"] = value;
        assert_eq!(
            parse_message(Protocol::HostCli, bytes(&message))
                .expect_err("invalid expiry")
                .code(),
            ErrorCode::ProtocolError
        );
    }
    for value in [1, 300000] {
        message["expires_in_ms"] = serde_json::json!(value);
        assert!(parse_message(Protocol::HostCli, bytes(&message)).is_ok());
    }
}

/// Rejects intermediate receipt, unverified validation, and invented expiry result literals.
#[test]
fn result_literals_cannot_describe_receipt_as_validated_save() {
    let fixture = fixture();
    for (field, value) in [
        ("outcome", "received"),
        ("validation", "stored-unverified"),
        ("expiry", "expired"),
    ] {
        let mut message = fixture["messages"]["cli_result"].clone();
        message[field] = Value::String(value.to_owned());
        assert_eq!(
            parse_message(Protocol::HostCli, bytes(&message))
                .expect_err("invalid result")
                .code(),
            ErrorCode::ProtocolError
        );
    }
}

/// Caps silent partial-prefix waits and prevents reuse after timeout.
#[tokio::test(start_paused = true)]
async fn partial_prefix_times_out_and_cannot_resynchronize() {
    let (mut sender, receiver) = duplex(16);
    sender.write_all(&[1, 2]).await.expect("partial prefix");
    let mut reader = FrameReader::new(receiver, Protocol::HostCli);
    let started = Instant::now();
    let error = reader
        .read_message(started + Duration::from_secs(300))
        .await
        .expect_err("bounded partial prefix");
    assert_eq!(error.code(), ErrorCode::TimedOut);
    assert_eq!(Instant::now() - started, Duration::from_secs(15));
    assert_eq!(
        reader
            .read_message(Instant::now() + Duration::from_secs(1))
            .await
            .expect_err("poisoned reader")
            .code(),
        ErrorCode::ProtocolError
    );
}

/// Lets an already reached deadline win over immediately available input.
#[tokio::test(start_paused = true)]
async fn exact_deadline_does_not_admit_ready_input() {
    let prefix = 1_u32.to_le_bytes();
    let mut input = prefix.as_slice();
    {
        let mut reader = FrameReader::new(&mut input, Protocol::HostCli);
        assert_eq!(
            reader
                .read_message(Instant::now())
                .await
                .expect_err("exact deadline")
                .code(),
            ErrorCode::TimedOut
        );
    }
    assert_eq!(input, prefix);
}

/// Poisons a partially written stream when its caller cancels the future.
#[tokio::test(start_paused = true)]
async fn externally_cancelled_write_cannot_replay() {
    let fixture = fixture();
    let payload = bytes(&fixture["messages"]["cli_open"]);
    let (sender, mut receiver) = duplex(1);
    let mut writer = FrameWriter::new(sender, Protocol::HostCli);
    assert!(
        timeout(
            Duration::from_secs(1),
            writer.write_message(
                Zeroizing::new(payload.clone()),
                Instant::now() + Duration::from_secs(10),
            )
        )
        .await
        .is_err()
    );
    assert_eq!(
        writer
            .write_message(
                Zeroizing::new(payload),
                Instant::now() + Duration::from_secs(1)
            )
            .await
            .expect_err("cancelled writer")
            .code(),
        ErrorCode::ProtocolError
    );
    drop(writer);
    let mut written = Vec::new();
    receiver
        .read_to_end(&mut written)
        .await
        .expect("owned test stream");
    assert_eq!(written.len(), 1, "no retry may add another prefix");
}
