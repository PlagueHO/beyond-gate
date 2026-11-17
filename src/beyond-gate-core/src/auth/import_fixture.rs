//! Development-only import lifecycle with a gated synthetic credential store.

use super::*;
use keyring_core::{
    Entry,
    api::{Credential, CredentialApi, CredentialStoreApi},
};
use std::{
    any::Any,
    io::{Read, Write},
    net::TcpListener,
    sync::{Mutex, mpsc},
    time::{SystemTime, UNIX_EPOCH},
};

/// Fixed invented input; no real cookie can enter this fixture.
pub const COOKIE: &str = "invented-cli-import-cookie";
/// Fixed prior entry used only by the in-memory credential backend.
const PRIOR: &[u8] = b"invented-prior-entry";

/// Selects one deterministic point around the real write-admission boundary.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// Confirms one complete validated save.
    Success,
    /// Blocks store opening before write admission.
    BeforeWrite,
    /// Blocks the credential call after write admission.
    DuringWrite,
}

/// Shares a bounded gate and the underlying mock entry with the test process.
struct Gate {
    ready: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl Gate {
    /// Announces the selected phase and waits for the adapter result.
    fn wait(&self) {
        self.ready.send(()).expect("fixture phase observer");
        self.release
            .lock()
            .expect("fixture gate")
            .recv_timeout(Duration::from_secs(15))
            .expect("fixture release");
    }
}

/// Holds only invented material in an explicitly constructed mock credential.
struct GatedCredential {
    inner: Entry,
    gate: Arc<Gate>,
    scenario: Scenario,
}

impl CredentialApi for GatedCredential {
    /// Pauses only after the actual lifecycle admits the write.
    fn set_secret(&self, secret: &[u8]) -> keyring_core::Result<()> {
        if self.scenario == Scenario::DuringWrite {
            self.gate.wait();
        }
        self.inner.set_secret(secret)
    }

    /// Reads only the in-memory entry.
    fn get_secret(&self) -> keyring_core::Result<Vec<u8>> {
        self.inner.get_secret()
    }

    /// Deletes only the in-memory entry.
    fn delete_credential(&self) -> keyring_core::Result<()> {
        self.inner.delete_credential()
    }

    /// Keeps the explicitly supplied credential wrapper.
    fn get_credential(&self) -> keyring_core::Result<Option<Arc<Credential>>> {
        Ok(None)
    }

    /// Retains the fixed synthetic identity.
    fn get_specifiers(&self) -> Option<(String, String)> {
        self.inner.get_specifiers()
    }

    /// Supports required concrete type inspection.
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Owns the gate and store; it cannot select a native credential backend.
pub struct ImportFixture {
    scenario: Scenario,
    store: Arc<SessionStore>,
    gate: Arc<Gate>,
    ready: Mutex<mpsc::Receiver<()>>,
    release: mpsc::Sender<()>,
    control: Mutex<Option<Weak<ControlState>>>,
}

impl ImportFixture {
    /// Seeds one isolated mock entry without installing a global backend.
    pub fn new(scenario: Scenario) -> Self {
        let backend = keyring_core::mock::Store::new().expect("mock backend");
        let inner = backend
            .build("beyond-gate-import-fixture", "invented", None)
            .expect("mock entry");
        inner.set_secret(PRIOR).expect("seed prior");
        let (ready_tx, ready) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        let gate = Arc::new(Gate {
            ready: ready_tx,
            release: Mutex::new(release_rx),
        });
        let store = Arc::new(SessionStore {
            entry: Entry::new_with_credential(Arc::new(GatedCredential {
                inner,
                gate: gate.clone(),
                scenario,
            })),
        });
        Self {
            scenario,
            store,
            gate,
            ready: Mutex::new(ready),
            release,
            control: Mutex::new(None),
        }
    }

    /// Waits until cancellation can deterministically race the selected phase.
    pub fn wait_ready(&self) {
        self.ready
            .lock()
            .expect("phase receiver")
            .recv_timeout(Duration::from_secs(15))
            .expect("phase reached");
    }

    /// Releases the owned blocking worker after the adapter reports its result.
    pub fn release(&self) {
        self.release.send(()).expect("release gate");
    }

    /// Waits until no admitted or queued worker can still change this mock entry.
    pub fn wait_finished(&self) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while self
            .control
            .lock()
            .expect("fixture control")
            .as_ref()
            .is_some_and(|control| control.upgrade().is_some())
        {
            assert!(
                std::time::Instant::now() < deadline,
                "fixture worker deadline"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Checks saved bytes without returning credential material.
    pub fn prior_is_unchanged(&self) -> bool {
        self.store.entry.get_secret().expect("mock read") == PRIOR
    }

    /// Checks that only the exact invented cookie envelope was saved.
    pub fn imported_cookie_is_saved(&self) -> bool {
        let expected = SessionEstablishmentInput::from_prompt(COOKIE.as_bytes().to_vec())
            .expect("invented input")
            .cookie
            .encode();
        self.store.entry.get_secret().expect("mock read") == expected.0.as_slice()
    }

    /// Runs real service validation and write admission against private local seams.
    ///
    /// # Errors
    ///
    /// Rejects all non-fixture material and preserves actual lifecycle failures.
    pub async fn establish(
        &self,
        input: SessionEstablishmentInput,
        control: LoginControl,
    ) -> Result<SavedSession, EstablishmentError> {
        if input.cookie.0.as_slice() != COOKIE.as_bytes() {
            return Err(EstablishmentError::new(ErrorCode::InvalidMaterial));
        }
        *self.control.lock().expect("fixture control") = Some(Arc::downgrade(&control.0));
        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback");
        let url = format!(
            "http://{}/v1/cobalt-token",
            listener.local_addr().expect("address")
        );
        let expiry = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_secs()
            + 3600;
        let body = crate::auth::session::fixture::response(expiry);
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("loopback request");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("read bound");
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).expect("header");
                headers.push(byte[0]);
                assert!(headers.len() < 16_384);
            }
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).expect("response");
            stream.write_all(&body).expect("body");
        });
        let store = self.store.clone();
        let gate = self.gate.clone();
        let scenario = self.scenario;
        let result = establish_with(
            input,
            control,
            direct_client_builder().build().expect("loopback client"),
            &url,
            move || {
                if scenario == Scenario::BeforeWrite {
                    gate.wait();
                }
                Ok(store)
            },
        )
        .await;
        worker.join().expect("loopback completed");
        result
    }
}
