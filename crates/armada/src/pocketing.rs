//! The Phone Gateway, run inside Fleet's process on a listener of its own.
//!
//! **The opt-in is the `pocket` marker in the machine directory**, the folder
//! `scripts/restart` reads as `$support`. Fleet starts the Gateway at each
//! start while it is there, and `set_phone` writes or removes it and starts or
//! stops the Gateway at once. A Gateway that cannot start says why on
//! `get_phone` and Fleet carries on without it.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use fleet::phone::PhoneSwitch;
use ipc::PhoneGateway;
use pocket::Gateway;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// The loopback port the owner's tunnel, `tailscale serve` by default, is pointed at.
pub const PORT: u16 = 8443;
/// The marker that opts this Mac in. `scripts/restart` reads the same name.
const MARKER: &str = "pocket";
/// Where `scripts/restart` copies the built app, outside any source tree.
const INSTALLED: &str = "pocket-app";
/// The app's build, from the folder Fleet runs in, where nothing was installed.
const BUILT: &str = "apps/pocket/dist";

struct Running {
    stop: oneshot::Sender<()>,
    serving: JoinHandle<()>,
    following: JoinHandle<()>,
}

pub struct Phone {
    machine: PathBuf,
    fleet_port: u16,
    port: u16,
    running: Mutex<Option<Running>>,
    /// Why the last start failed, while it stands.
    why: Mutex<Option<String>>,
}

/// The installed app, else the build in the folder Fleet runs in. Neither
/// there serves nothing, and the phone gets 404 until `scripts/restart` copies it.
fn assets(machine: &Path) -> PathBuf {
    let installed = machine.join(INSTALLED);
    match installed.is_dir() || !Path::new(BUILT).is_dir() {
        true => installed,
        false => PathBuf::from(BUILT),
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

impl Phone {
    pub fn new(machine: &Path, fleet_port: u16) -> Phone {
        Phone::on_port(machine, fleet_port, PORT)
    }

    fn on_port(machine: &Path, fleet_port: u16, port: u16) -> Phone {
        Phone {
            machine: machine.to_path_buf(),
            fleet_port,
            port,
            running: Mutex::new(None),
            why: Mutex::new(None),
        }
    }

    fn marked(&self) -> bool {
        self.machine.join(MARKER).is_file()
    }

    fn listening(&self) -> bool {
        self.running.lock().unwrap().as_ref().is_some_and(|running| !running.serving.is_finished())
    }

    /// Start it if this Mac is opted in. A failure is one line, and Fleet carries on.
    pub fn start_if_marked(&self) {
        if !self.marked() {
            return;
        }
        match self.start() {
            Ok(()) => println!("Phone Gateway on 127.0.0.1:{}", self.port),
            Err(why) => eprintln!("Phone Gateway not started: {why}"),
        }
    }

    fn start(&self) -> Result<(), String> {
        if self.listening() {
            return Ok(());
        }
        let started = self.started();
        *self.why.lock().unwrap() = started.as_ref().err().cloned();
        let running = started?;
        *self.running.lock().unwrap() = Some(running);
        Ok(())
    }

    fn started(&self) -> Result<Running, String> {
        let port = self.port;
        let listener = pocket::bind_now(port)
            .map_err(|why| format!("Armada could not listen on 127.0.0.1:{port}: {why}"))?;
        let store = store::Store::open(&self.machine.join(crate::serve::STORE_FILE))
            .map_err(|why| format!("Armada could not open its store for the phone: {why}"))?;
        let push = pocket::Push::open(&self.machine, pocket::PLACEHOLDER_SUBJECT)?;
        let fleet_port = self.fleet_port;
        let gateway = Gateway {
            pairing: pocket::Pairing::new(store, Arc::new(now), pocket::machine_address(self.machine.clone())),
            fleet: Arc::new(move || Ok(fleet_port)),
            assets: Some(assets(&self.machine)),
            push,
        };
        let following = tokio::spawn(pocket::follow(gateway.clone()));
        let (stop, stopped) = oneshot::channel::<()>();
        let serving = tokio::spawn(async move {
            let served = axum::serve(listener, pocket::router(gateway))
                .with_graceful_shutdown(async move {
                    let _ = stopped.await;
                })
                .await;
            if let Err(why) = served {
                eprintln!("Phone Gateway stopped: {why}");
            }
        });
        Ok(Running { stop, serving, following })
    }

    /// Stop listening and stop pushing. A phone's open `/api/live` is cut with the listener.
    fn stop(&self) {
        if let Some(running) = self.running.lock().unwrap().take() {
            let _ = running.stop.send(());
            running.serving.abort();
            running.following.abort();
        }
    }

    fn opt(&self, on: bool) -> Result<(), String> {
        let marker = self.machine.join(MARKER);
        match on {
            true => std::fs::write(&marker, b"")
                .map_err(|why| format!("Armada could not write {}: {why}", marker.display())),
            false => match std::fs::remove_file(&marker) {
                Err(why) if why.kind() != std::io::ErrorKind::NotFound => {
                    Err(format!("Armada could not remove {}: {why}", marker.display()))
                }
                _ => Ok(()),
            },
        }
    }
}

impl PhoneSwitch for Phone {
    fn state(&self) -> PhoneGateway {
        let (on, listening) = (self.marked(), self.listening());
        let why = match on && !listening {
            true => Some(self.why.lock().unwrap().clone().unwrap_or_else(|| String::from("The Phone Gateway stopped."))),
            false => None,
        };
        PhoneGateway { on, port: self.port, listening, why }
    }

    fn set(&self, on: bool) -> PhoneGateway {
        let opted = self.opt(on);
        let moved = match on {
            true => self.start(),
            false => Ok(self.stop()),
        };
        let mut state = self.state();
        if let Some(why) = opted.err().or(moved.err()) {
            state.why = Some(why);
        }
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("armada-phone-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
    }

    async fn answers(port: u16) -> bool {
        tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok()
    }

    #[tokio::test]
    async fn the_switch_writes_the_marker_and_starts_and_stops_the_gateway_live() {
        let dir = machine("live");
        let port = free_port();
        let phone = Phone::on_port(&dir, 1, port);
        assert_eq!(phone.state(), PhoneGateway { on: false, port, listening: false, why: None });

        let on = phone.set(true);
        assert_eq!(on, PhoneGateway { on: true, port, listening: true, why: None });
        assert!(dir.join(MARKER).is_file(), "the marker scripts/restart reads");
        assert!(answers(port).await);

        let off = phone.set(false);
        assert_eq!(off, PhoneGateway { on: false, port, listening: false, why: None });
        assert!(!dir.join(MARKER).exists());
        tokio::task::yield_now().await;
        assert!(!answers(port).await, "the listener is gone");

        assert!(phone.set(true).listening, "and is taken again at once");
        phone.set(false);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_gateway_that_cannot_bind_says_why_and_keeps_the_opt_in() {
        let dir = machine("taken");
        let held = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = held.local_addr().unwrap().port();
        let phone = Phone::on_port(&dir, 1, port);
        let state = phone.set(true);
        assert!(state.on && !state.listening, "{state:?}");
        assert!(state.why.as_deref().is_some_and(|why| why.contains(&format!("127.0.0.1:{port}"))), "{state:?}");
        assert_eq!(phone.state().why, state.why, "it stands until a start succeeds");
        phone.start_if_marked();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn fleet_starts_it_only_where_the_mac_is_opted_in() {
        let dir = machine("marked");
        let port = free_port();
        let phone = Phone::on_port(&dir, 1, port);
        phone.start_if_marked();
        assert!(!phone.state().listening);
        std::fs::write(dir.join(MARKER), b"").unwrap();
        phone.start_if_marked();
        assert!(phone.state().listening && answers(port).await);
        phone.set(false);
        std::fs::remove_dir_all(&dir).ok();
    }
}
