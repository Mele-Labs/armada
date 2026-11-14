//! Whether this Mac serves the phone app. Fleet owns the routes; the Phone
//! Gateway itself is `crates/pocket`, which Fleet does not depend on, so the
//! composition root hands in what turns it on and off. `docs/concepts/pocket.md`.

use std::sync::{Arc, OnceLock};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{MacSetup, MacSetupStep, PhoneGateway, WireError};

use crate::daemon::Fleet;

const PHONE_NOT_SERVED: &str = "fleet.phone_not_served";

/// Turns the Phone Gateway on and off, and says where it stands. Its own calls
/// are quick: a marker file and a loopback bind.
pub trait PhoneSwitch: Send + Sync + 'static {
    fn state(&self) -> PhoneGateway;
    fn set(&self, on: bool) -> PhoneGateway;

    /// How far the one-press setup for his other Macs has come. A switch with no tunnel to set up stops at
    /// the Gateway.
    fn setup(&self) -> MacSetup {
        let state = self.state();
        let step = if state.listening { MacSetupStep::SignIn } else { MacSetupStep::Gateway };
        MacSetup { step, working: false, login: None, why: state.why, address: None }
    }

    /// The press: the Gateway on, then the rest of the setup from where it stands. Blocking for a moment,
    /// so a sign-in page that is ready comes back with it.
    fn set_up(&self) -> MacSetup {
        self.set(true);
        self.setup()
    }

    /// Read `path`, one of the other Mac's `/other_mac/*` Fleet paths with its query, from paired Mac `mac`
    /// through the link: the status its Fleet answered and the bytes. A Gateway with no link to it says why.
    fn read_other_mac(&self, mac: &str, path: &str) -> OtherMacRead {
        let _ = path;
        let why = format!("this Mac has no link to Mac {mac}");
        Box::pin(async move { Err(why) })
    }

    /// Send `body` to the other Mac's Fleet as a `POST` of `path`, which is `/other_mac/project_act` or
    /// `/other_mac/act` and nothing else, through the link: the status its Fleet answered and the bytes. A Gateway
    /// with no link to it says why.
    fn send_other_mac(&self, mac: &str, path: &str, body: Vec<u8>) -> OtherMacRead {
        let _ = (path, body);
        let why = format!("this Mac has no link to Mac {mac}");
        Box::pin(async move { Err(why) })
    }

    /// Open the page Job `job` of paired Mac `mac` serves on its loopback port `port`, at `rest` (its path and
    /// query, `/` first), as an address on this Mac. A Gateway with no link to it says why.
    fn open_other_mac_page(&self, mac: &str, job: &str, port: u16, rest: &str) -> OtherMacPageOpening {
        let _ = (job, port, rest);
        let why = format!("this Mac has no link to Mac {mac}");
        Box::pin(async move { Err(why) })
    }

    /// Follow `path`, `/other_mac/terminal/observe` with its query and nothing else, on paired Mac `mac` through the
    /// link: its messages a line at a time until the stream drops, or what that Mac refused. A Gateway with no link
    /// to it says why.
    fn follow_other_mac(&self, mac: &str, path: &str) -> OtherMacFollowing {
        let _ = path;
        let why = format!("this Mac has no link to Mac {mac}");
        Box::pin(async move { Err(why) })
    }
}

/// What opening another Mac's page came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OtherMacPage {
    /// The page's address on this Mac.
    Open(String),
    /// The Mac is asleep, by its name and since when (Unix seconds).
    Asleep { name: String, since: i64 },
    /// The other Mac refused, with its status and bytes as it said them.
    Refused { status: u16, body: Vec<u8> },
}

/// What [`PhoneSwitch::open_other_mac_page`] answers.
pub type OtherMacPageOpening = std::pin::Pin<Box<dyn std::future::Future<Output = Result<OtherMacPage, String>> + Send>>;

/// What following a stream on another Mac came to.
pub enum OtherMacFollowed {
    /// Its messages, one a line. Closed when the stream drops or goes quiet.
    Open(tokio::sync::mpsc::Receiver<String>),
    /// That Mac refused, with its status and bytes as it said them.
    Refused { status: u16, body: Vec<u8> },
}

/// What [`PhoneSwitch::follow_other_mac`] answers.
pub type OtherMacFollowing = std::pin::Pin<Box<dyn std::future::Future<Output = Result<OtherMacFollowed, String>> + Send>>;

/// What [`PhoneSwitch::read_other_mac`] answers, once the other Mac has.
pub type OtherMacRead = std::pin::Pin<Box<dyn std::future::Future<Output = Result<(u16, Vec<u8>), String>> + Send>>;

/// The switch, once the composition root has handed it in.
#[derive(Default)]
pub(crate) struct PhoneHeld {
    switch: OnceLock<Arc<dyn PhoneSwitch>>,
    /// The Macs whose queued edits are being sent now, so one task sends them and not two.
    pub(crate) flushing: std::sync::Mutex<std::collections::BTreeSet<String>>,
    /// One act from another Mac runs at a time, so an id is run once. `crate::mac_acts`.
    pub(crate) acts: Arc<tokio::sync::Mutex<()>>,
    /// The terminals of Jobs held from another Mac that this Mac has opened. `crate::mac_terminals`.
    pub(crate) terminals: Arc<crate::mac_terminals::Followed>,
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Hand Fleet what turns the Phone Gateway on and off. Only the first is kept.
    pub fn switching_phone_with(&self, switch: Arc<dyn PhoneSwitch>) {
        let _ = self.phone_held().switch.set(switch);
    }

    pub(crate) fn phone_switch(&self) -> Result<&Arc<dyn PhoneSwitch>, Refusal> {
        self.phone_held().switch.get().ok_or_else(|| {
            Refusal::IllegalMove(WireError::raised(
                PHONE_NOT_SERVED,
                "this Fleet was started without the Phone Gateway",
                self.run_id(),
            ))
        })
    }

    pub(crate) fn phone_now(&self) -> Result<PhoneGateway, Refusal> {
        Ok(self.phone_switch()?.state())
    }

    /// A Gateway that stops, or starts afresh, holds no claim from another Mac, so none calls him any more.
    pub(crate) fn phone_set(&self, on: bool) -> Result<PhoneGateway, Refusal> {
        let switch = self.phone_switch()?;
        let was = switch.state().listening;
        let state = switch.set(on);
        if !was || !state.listening {
            self.calls_claimed(Vec::new());
        }
        Ok(state)
    }

    pub(crate) fn mac_setup_now(&self) -> Result<MacSetup, Refusal> {
        Ok(self.phone_switch()?.setup())
    }

    /// Off the async threads, since the press waits a moment for a sign-in page.
    pub(crate) async fn mac_set_up(&self) -> Result<MacSetup, Refusal> {
        let switch = self.phone_switch()?.clone();
        let run_id = self.run_id();
        tokio::task::spawn_blocking(move || switch.set_up()).await.map_err(|why| {
            Refusal::IllegalMove(WireError::raised(PHONE_NOT_SERVED, format!("the setup stopped: {why}"), run_id))
        })
    }
}
