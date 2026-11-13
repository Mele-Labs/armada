//! The one setup Settings › Macs runs from a single press, as far as the running Gateway carries it: the
//! Cloudflare sign-in and this Mac's tunnel. Fleet answers the step before it, turning the Gateway on.
//!
//! **Each state carries the press that moves it on.** Working is nothing to press; `login` is the page to
//! sign in on; `why` is the setup to press again; done is nothing more.

use std::sync::mpsc;
use std::time::Duration;

use ipc::{CloudflareState, MacSetup, MacSetupStep};

use crate::tunnel::{ForMac, Tunnel};
use crate::Gateway;

/// How long the press waits for the setup to name a sign-in page before it answers where it stands.
pub const SETUP_WAIT: Duration = Duration::from_secs(3);
/// How long a tunnel just made or started may stay unconnected before it is said to be stuck.
pub const CONNECTING: i64 = 30;

/// Where the setup stands on this running Gateway. Blocking: it reads the tunnel's files.
pub fn mac_setup(gateway: &Gateway) -> MacSetup {
    setup_of(&gateway.way_in.tunnel, gateway.pairing.now())
}

pub(crate) fn setup_of(tunnel: &Tunnel, now: i64) -> MacSetup {
    let busy = tunnel.busy();
    let at = |step: MacSetupStep, why: Option<String>| MacSetup {
        step,
        working: busy,
        login: None,
        why: if busy { None } else { why },
        address: None,
    };
    match tunnel.state().0 {
        CloudflareState::Missing | CloudflareState::SignedOut => at(MacSetupStep::SignIn, tunnel.failed()),
        CloudflareState::SigningIn { login } => MacSetup { login: Some(login), working: false, ..at(MacSetupStep::SignIn, None) },
        CloudflareState::NoTunnel { why } => at(MacSetupStep::Tunnel, why),
        CloudflareState::Running { address } => {
            MacSetup { step: MacSetupStep::Done, working: false, login: None, why: None, address: Some(address) }
        }
        CloudflareState::Down { since, why, .. } => {
            let connecting = now - since < CONNECTING;
            MacSetup { working: busy || connecting, why: (!busy && !connecting).then_some(why), ..at(MacSetupStep::Tunnel, None) }
        }
    }
}

/// The press: on from where it stands, the tunnel's own setup in the background, and the answer once it
/// names a sign-in page or [`SETUP_WAIT`] passes. A tunnel already made is left to its runner. Blocking.
pub fn set_up_mac(gateway: &Gateway) -> MacSetup {
    let pressable = matches!(
        gateway.way_in.tunnel.state().0,
        CloudflareState::Missing | CloudflareState::SignedOut | CloudflareState::SigningIn { .. } | CloudflareState::NoTunnel { .. }
    );
    if pressable && !gateway.way_in.tunnel.busy() {
        let (done, pressed) = mpsc::channel::<()>();
        let gateway = gateway.clone();
        std::thread::spawn(move || {
            if let Ok(mac) = crate::machine::this_mac(&gateway) {
                // Busy or failed is said by the state read after.
                let _ = gateway.way_in.tunnel.press(ForMac { id: &mac.id, name: &mac.name });
            }
            drop(done);
        });
        let _ = pressed.recv_timeout(SETUP_WAIT);
    }
    mac_setup(gateway)
}
