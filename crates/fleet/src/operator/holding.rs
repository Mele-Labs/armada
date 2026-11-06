//! Of two Jobs kept in touch, the one that must build first, and the other held until it pushes.
//! **The hold is a `StartAfter`** on the first's pull request being pushed, kept as the Operator's,
//! so it is listed, approved and started the way any wait is.
//!
//! **Only a Job that has not begun is held**: one on its start card, waiting on nothing else. A
//! working Job is not paused, and a wait the person or an agent set is not the Operator's to
//! replace; either way nothing is held and the Log says why. **Fail open**: where the first ends
//! without pushing, the hold lifts and the Log says so.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Actor;
use ipc::{SessionId, StartAfter, StartBase, StartCondition};
use store::KeptWait;

use super::{Logged, Outcome};
use crate::daemon::Fleet;
use crate::session_host::after::BY_OPERATOR;
use crate::session_host::start_card::start_read;

impl Fleet
{
    /// `held` waits until `first` pushes its pull request, where it has not begun.
    pub(super) async fn operator_held(&self, held: &str, first: &str, shared: &[String]) {
        let (begun, waits, manifest, subject, brief, first_title) = {
            let store = self.store().lock().await;
            let (Ok(Some(session)), Ok(Some(hosting))) = (store.session(held), store.hosting(held)) else { return };
            if session.state == store::SessionState::Ended {
                return;
            }
            let waits = store.wait_of(held).ok().flatten().is_some_and(|wait| wait.started_at.is_none());
            let brief = start_read(&store, &hosting).ok().and_then(|reads| reads.first_said()).unwrap_or_default();
            let first_title = store.session(first).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| first.to_string());
            (!hosting.start_held, waits, hosting.manifest_id, session.title.unwrap_or_else(|| held.to_string()), brief, first_title)
        };
        let paths = shared.join(", ");
        let not_held = match (begun, waits) {
            (true, _) => Some("it is already working, and the Operator does not pause a working Job"),
            (false, true) => Some("it already waits on another Job, and that wait is not the Operator's to replace"),
            (false, false) => None,
        };
        if let Some(why) = not_held {
            let mut row = Logged::about(held, &manifest, &subject, Outcome::NotHeld, Actor::Fleet, self.now());
            row.because = format!("Not held until \"{first_title}\" pushes: {why}.");
            (row.other, row.first, row.shared) = (Some(first.to_string()), Some(first.to_string()), shared.to_vec());
            self.operator_logged(row).await;
            return;
        }
        let after = StartAfter {
            session: SessionId::carried(first),
            condition: StartCondition::PrPushed,
            base: StartBase::Main,
            needs: format!("its push of {paths}, which this Job builds on"),
        };
        match self.held_by_operator(held, after, brief).await {
            Ok(_) => {
                let text = format!("Held by the Operator until \"{first_title}\" pushes: it builds first on {paths}, which this Job uses.");
                self.fleet_note(held, text).await;
            }
            Err(why) => self.operator_warned("the Operator's hold was not kept", format!("{why:?}")),
        }
    }

    /// A wait whose condition can no longer be met: where it is the Operator's hold, it lifts, and
    /// the Log says so.
    pub(crate) async fn operator_hold_lifted(&self, wait: &KeptWait, said: &str) {
        if wait.by != BY_OPERATOR {
            return;
        }
        let (manifest, subject) = {
            let store = self.store().lock().await;
            let manifest = store.hosting(&wait.session_id).ok().flatten().map(|one| one.manifest_id).unwrap_or_default();
            let subject = store.session(&wait.session_id).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| wait.session_id.clone());
            (manifest, subject)
        };
        let mut row = Logged::about(&wait.session_id, &manifest, &subject, Outcome::HoldLifted, Actor::Fleet, self.now());
        row.because = format!("The hold lifted: {said}, so it waits on nothing now.");
        (row.other, row.first) = (Some(wait.on_session.clone()), Some(wait.on_session.clone()));
        self.operator_logged(row).await;
    }
}
