//! The Wi-Fi door: a second listener, on every network the Mac is on, that his other Macs on the same
//! network reach with no tunnel. TLS with this Mac's own certificate, which the other Mac pins when they
//! pair, and only the routes Macs call each other on: everything else is 404. The Gateway's own listener
//! stays on loopback. `docs/concepts/pocket.md`, *Macs nearby*.

use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use hyper_util::rt::TokioIo;
use hyper_util::service::TowerToHyperService;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;

use crate::pairing::to_hex;
use crate::Gateway;

/// The file in the machine directory holding the door's certificate and its private key.
pub const CERT_FILE: &str = "pocket-door.key";

/// How long a caller has to finish the TLS handshake before its connection is dropped.
const HANDSHAKE: Duration = Duration::from_secs(10);

/// This Mac's certificate for the door, and the pin other Macs know it by.
#[derive(Clone)]
pub struct DoorCert {
    cert: CertificateDer<'static>,
    key: Vec<u8>,
    /// The SHA-256 of the certificate, as hex.
    pub pin: String,
}

impl DoorCert {
    /// The certificate in `dir`, made on first use. The file is owner-only from the moment it exists.
    pub fn open(dir: &Path) -> Result<DoorCert, String> {
        let file = dir.join(CERT_FILE);
        match std::fs::read_to_string(&file) {
            Ok(text) => DoorCert::read(&text).ok_or_else(|| format!("The certificate at {} could not be read.", file.display())),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                let made = DoorCert::made()?;
                let text = format!("{}\n{}\n", B64.encode(made.cert.as_ref()), B64.encode(&made.key));
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&file)
                    .and_then(|mut f| f.write_all(text.as_bytes()))
                    .map_err(|why| format!("The certificate at {} could not be written: {why}", file.display()))?;
                Ok(made)
            }
            Err(why) => Err(format!("The certificate at {} could not be read: {why}", file.display())),
        }
    }

    fn read(text: &str) -> Option<DoorCert> {
        let mut lines = text.lines();
        let cert = B64.decode(lines.next()?.trim()).ok()?;
        let key = B64.decode(lines.next()?.trim()).ok()?;
        Some(DoorCert { pin: pin_of(&cert), cert: CertificateDer::from(cert), key })
    }

    /// A fresh certificate, kept nowhere.
    pub(crate) fn made() -> Result<DoorCert, String> {
        let made = rcgen::generate_simple_self_signed(vec!["armada-mac".to_string()])
            .map_err(|why| format!("Armada could not make this Mac's certificate: {why}"))?;
        let cert = made.cert.der().clone();
        Ok(DoorCert { pin: pin_of(&cert), cert, key: made.signing_key.serialize_der() })
    }

    fn server_config(&self) -> Result<rustls::ServerConfig, String> {
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.key.clone()));
        let mut config = rustls::ServerConfig::builder_with_provider(provider())
            .with_safe_default_protocol_versions()
            .and_then(|config| config.with_no_client_auth().with_single_cert(vec![self.cert.clone()], key))
            .map_err(|why| format!("Armada could not use this Mac's certificate: {why}"))?;
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(config)
    }
}

/// What a certificate is known by: the SHA-256 of its DER, as hex.
pub(crate) fn pin_of(der: &[u8]) -> String {
    to_hex(&Sha256::digest(der))
}

/// Whether `pin` reads as one [`pin_of`] made.
pub(crate) fn a_pin(pin: &str) -> bool {
    pin.len() == 64 && pin.bytes().all(|b| b.is_ascii_hexdigit())
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Bind the door on every IPv4 network the Mac is on, at a port the system picks: Bonjour says which.
pub fn bind_door() -> std::io::Result<TcpListener> {
    bind_door_on(Ipv4Addr::UNSPECIFIED)
}

pub(crate) fn bind_door_on(host: Ipv4Addr) -> std::io::Result<TcpListener> {
    let socket = tokio::net::TcpSocket::new_v4()?;
    socket.bind(SocketAddr::from((host, 0)))?;
    socket.listen(1024)
}

/// Only what Macs call each other on: the pairing steps and the signed `/api/mac/*`. The phone's
/// routes, Bridge's `/admin/*` and the app itself are 404 here, as if they were not there.
async fn door_only(request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if path.starts_with("/pair/mac/") || path.starts_with("/api/mac/") {
        next.run(request).await
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

/// The Gateway's own router, behind [`door_only`].
pub(crate) fn door_router(gateway: Gateway) -> Router {
    crate::router(gateway).layer(from_fn(door_only))
}

/// The door, serving, and Bonjour saying where it is.
pub struct Door {
    pub port: u16,
    /// Why Bonjour did not start, for the caller to log. The door still answers a Mac already paired.
    pub bonjour_off: Option<String>,
    serving: JoinHandle<()>,
    bonjour: Option<crate::nearby::Bonjour>,
}

impl Door {
    /// Open the door on every network with the certificate kept in `machine`, and say so over Bonjour.
    /// Called from inside a Tokio runtime. **Bonjour that cannot start is said in `bonjour_off`**, and the
    /// door is still open.
    pub fn open(machine: &Path, gateway: Gateway) -> Result<Door, String> {
        let cert = DoorCert::open(machine)?;
        let listener = bind_door().map_err(|why| format!("Armada could not listen for other Macs on this network: {why}"))?;
        let mut door = Door::serve(listener, &cert, gateway.clone())?;
        match crate::nearby::advertise_and_browse(&gateway, door.port) {
            Ok(bonjour) => door.bonjour = Some(bonjour),
            Err(why) => door.bonjour_off = Some(why),
        }
        Ok(door)
    }

    /// Serve the door on `listener` and note its port and pin, so this Mac's hello carries the pin.
    pub(crate) fn serve(listener: TcpListener, cert: &DoorCert, gateway: Gateway) -> Result<Door, String> {
        let port = listener.local_addr().map_err(|why| why.to_string())?.port();
        let acceptor = TlsAcceptor::from(Arc::new(cert.server_config()?));
        gateway.macs.nearby.set_own(port, cert.pin.clone());
        let app = door_router(gateway);
        let serving = tokio::spawn(async move {
            loop {
                let stream = match listener.accept().await {
                    Ok((stream, _)) => stream,
                    // Out of descriptors, most likely: wait rather than spin.
                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };
                let (acceptor, app) = (acceptor.clone(), app.clone());
                tokio::spawn(async move {
                    let Ok(Ok(tls)) = tokio::time::timeout(HANDSHAKE, acceptor.accept(stream)).await else { return };
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(TokioIo::new(tls), TowerToHyperService::new(app))
                        .with_upgrades()
                        .await;
                });
            }
        });
        Ok(Door { port, bonjour_off: None, serving, bonjour: None })
    }

    /// Stop answering and stop saying so. A Mac's open `/api/mac/live` ends with [`crate::Macs::close`].
    pub fn stop(self) {
        self.serving.abort();
        if let Some(bonjour) = self.bonjour {
            bonjour.stop();
        }
    }
}

/// The certificate a client met, as [`Pinned`] saw it.
pub(crate) type Seen = Arc<Mutex<Option<String>>>;

/// A client to another Mac's door. With `pin` it refuses any certificate but that one; without, it takes
/// any and notes the one it met in the [`Seen`], for the four digits to cover.
pub(crate) fn client(pin: Option<&str>) -> Result<(reqwest::Client, Seen), String> {
    let seen = Seen::default();
    let verifier = Pinned { pin: pin.map(str::to_string), seen: seen.clone(), provider: provider() };
    let tls = rustls::ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .map_err(|why| why.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    // No redirect is followed, as on the page leg: one followed would arrive unsigned.
    let client = reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|why| format!("Armada could not make its client for other Macs: {why}"))?;
    Ok((client, seen))
}

/// **Not a CA check, on purpose**: no CA signs for an address on a home network. The pin is what
/// the owner's Confirm proved, through the four digits.
#[derive(Debug)]
struct Pinned {
    pin: Option<String>,
    seen: Seen,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let met = pin_of(end_entity);
        *self.seen.lock().unwrap() = Some(met.clone());
        match &self.pin {
            Some(pin) if *pin != met => Err(rustls::Error::General("not the certificate this Mac paired with".to_string())),
            _ => Ok(ServerCertVerified::assertion()),
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}
