//! What the Operator reads of a running Fleet: its switch, the live Jobs, a repository's name and what went
//! wrong, in Fleet's log.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings as keys;
use core_model::{Component, Envelope, FieldValue, Level};

use super::LiveJob;
use crate::daemon::Fleet;

/// The most of a first line a title made of words keeps.
const TITLE_CHARS: usize = 60;

pub(super) fn first_line(words: &str) -> String {
    words.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default().chars().take(TITLE_CHARS).collect()
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
    /// The Operator's own switch, as settings.json reads now. Off is Fleet as it was before it.
    pub(crate) fn operator_on(&self) -> bool {
        self.machine_settings().now().get(keys::OPERATOR)
    }

    pub(super) async fn title_of(&self, id: &str) -> String {
        let session = self.store().lock().await.session(id).ok().flatten();
        session.and_then(|one| one.title).unwrap_or_else(|| id.to_string())
    }

    /// Every hosted Job not ended.
    pub(super) async fn live_jobs(&self) -> Vec<LiveJob> {
        let store = self.store().lock().await;
        let Ok(hostings) = store.hostings() else { return Vec::new() };
        hostings
            .into_iter()
            .filter_map(|hosting| {
                let session = store.session(&hosting.session_id).ok().flatten().filter(|session| session.state != store::SessionState::Ended)?;
                Some(LiveJob { title: session.title.unwrap_or_else(|| session.id.clone()), session_id: session.id, manifest_id: hosting.manifest_id })
            })
            .collect()
    }

    /// The name a repository goes by on the page, from the root's last folder.
    pub(super) fn board_repository(&self, manifest_id: &str) -> Option<String> {
        self.repositories().hosted().into_iter().find(|one| one.manifest().id().as_str() == manifest_id).map(|one| {
            let root = one.root();
            std::path::Path::new(root).file_name().map_or_else(|| root.to_string(), |name| name.to_string_lossy().to_string())
        })
    }

    /// The master switch was possibly moved by a save or a hand edit: keep the moment where it differs
    /// from `was_on`, and tell Bridge to read the page again. A save that left it alone keeps and tells nothing.
    pub(crate) async fn operator_switched(&self, was_on: bool) {
        let on = self.operator_on();
        if on == was_on {
            return;
        }
        self.operator_page_changed();
        let kept = self.store().lock().await.add_operator_switch(&store::KeptSwitch { on, at: self.now().as_str().to_string() });
        if let Err(why) = kept {
            self.operator_warned("a move of the Operator's switch was not kept", why);
            return;
        }
        if on {
            self.operator_started_fresh().await;
        }
        self.operator_board_changed();
    }

    /// **Coming on starts the Operator fresh.** Every card left open from before is settled, and the features that piled up
    /// while it was off are kept as handed, so its first tick tells nothing. A red main and a short disk are what is broken
    /// now, and stay: their own sweeps settle them.
    async fn operator_started_fresh(&self) {
        let at = self.now();
        let answer = format!("{}Cleared when the Operator was turned on.", super::cards::SETTLED);
        let open = self.store().lock().await.operator_turn_open().unwrap_or_default();
        for row in open.iter().filter(|row| !row.item.starts_with(super::turning::RED) && !row.item.starts_with(super::disking::DISK_HOLD)) {
            if let Err(why) = self.store().lock().await.settle_operator_turn(row.n, at.as_str(), Some(&answer)) {
                self.operator_warned("a card from before the Operator came on was not cleared", why);
            }
        }
        self.catalog_kept_as_handed().await;
        self.operator_page_changed();
    }

    /// The switch moved: the page's `on` is read again.
    pub(super) fn operator_board_changed(&self) {
        self.events().publish(ipc::Event::OperatorBoardChanged(ipc::operator_board::OperatorBoardChanged {}));
    }

    /// What went wrong for the Operator, in Fleet's log.
    pub(crate) fn operator_warned(&self, msg: &str, why: impl std::fmt::Display) {
        let envelope = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), msg).with_field("why", FieldValue::Str(why.to_string()));
        crate::transcript::note_aloud(&envelope);
    }
}
