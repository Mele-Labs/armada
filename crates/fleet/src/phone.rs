//! Whether this Mac serves the phone app. Fleet owns the routes; the Phone
//! Gateway itself is `crates/pocket`, which Fleet does not depend on, so the
//! composition root hands in what turns it on and off. `docs/concepts/pocket.md`.

use std::sync::{Arc, OnceLock};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{PhoneGateway, WireError};

use crate::daemon::Fleet;

const PHONE_NOT_SERVED: &str = "fleet.phone_not_served";

/// Turns the Phone Gateway on and off, and says where it stands. Its own calls
/// are quick: a marker file and a loopback bind.
pub trait PhoneSwitch: Send + Sync + 'static {
    fn state(&self) -> PhoneGateway;
    fn set(&self, on: bool) -> PhoneGateway;
}

/// The switch, once the composition root has handed it in.
#[derive(Default)]
pub(crate) struct PhoneHeld {
    switch: OnceLock<Arc<dyn PhoneSwitch>>,
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

    fn phone_switch(&self) -> Result<&Arc<dyn PhoneSwitch>, Refusal> {
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

    pub(crate) fn phone_set(&self, on: bool) -> Result<PhoneGateway, Refusal> {
        Ok(self.phone_switch()?.set(on))
    }
}
