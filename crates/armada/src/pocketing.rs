//! `armada pocket`: the Phone Gateway, found Fleet through the runtime file.
//!
//! The runtime file is read on every call rather than once, so a Fleet that
//! restarted is found at its new port, and `listening` verifies the pid before
//! the port is trusted.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fleet::runtime;
use pocket::Gateway;

use crate::mcp::listening;

/// Where the PWA's build goes, from the repository root.
const BUILT: &str = "apps/pocket/dist";

/// Where Fleet is now, or the sentence saying why it is not.
pub fn fleet_port(file: &Path) -> Result<u16, String> {
    listening(runtime::read(file), file)
}

pub async fn run(port: u16, assets: Option<PathBuf>) -> Result<(), String> {
    let file = runtime::machine_path().map_err(|why| why.to_string())?;
    let listener = pocket::bind(port)
        .await
        .map_err(|why| format!("the Phone Gateway could not listen on 127.0.0.1:{port}: {why}"))?;
    let machine = file.parent().ok_or("the machine directory has no parent")?;
    let store = store::Store::open(&machine.join(crate::serve::STORE_FILE))
        .map_err(|why| format!("the Phone Gateway could not open the store: {why}"))?;
    let clock: pocket::Clock = Arc::new(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64)
    });
    let address: pocket::Address = Arc::new(pocket::tailscale_address);
    let push = pocket::Push::open(machine, pocket::PLACEHOLDER_SUBJECT)?;
    let gateway = Gateway {
        pairing: pocket::Pairing::new(store, clock, address),
        fleet: Arc::new(move || fleet_port(&file)),
        assets: Some(assets.unwrap_or_else(|| PathBuf::from(BUILT))),
        push,
    };
    tokio::spawn(pocket::follow(gateway.clone()));
    println!("Phone Gateway on 127.0.0.1:{port}");
    axum::serve(listener, pocket::router(gateway))
        .await
        .map_err(|why| why.to_string())
}
