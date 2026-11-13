//! A disk under the hold floor, said once. Fleet reclaims build output on its own and the person
//! hears of it only when that cannot keep up: a look that ends with a served repository's volume
//! under `limits.diskFloorGib` puts one item in Your turn, and the look that finds it back over the
//! floor takes it away. `.armada/documents/disk-reclaim-policy.html`.
//!
//! **One item for each repository while it is short.** It stands, answered or not, until free is
//! back over the floor, so a shortage is said once and a second one is said again.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::leasing::disk::{drop_first, inventory, mark_landed};
use ipc::{Instant, WaitingItem, WaitingOption, WaitingSource};

use super::raising::ToRaise;
use crate::daemon::Fleet;
use crate::sweeping::Seen;

/// An item of a short disk starts with this: `disk-hold:<manifest>`.
pub(super) const DISK_HOLD: &str = "disk-hold:";
/// The two picks. Dropping takes what a pass would take next; leaving takes nothing.
pub(super) const DROP_THEM: &str = "Drop them";
pub(super) const LEAVE_THEM: &str = "Leave them";

/// How many folders the text names, and a press drops.
const NAMED: usize = 3;
const GIB: u64 = 1024 * 1024 * 1024;

/// `folder` as the text says it: from the repository's root, and from `slot-N` for a slot's.
fn shortened(root: &Path, folder: &Path) -> String {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let within = folder.strip_prefix(root).or_else(|_| folder.strip_prefix(&canonical)).unwrap_or(folder);
    within.strip_prefix(".armada/slots").unwrap_or(within).display().to_string()
}

/// The words of the item: free and the hold in whole gigabytes, and where a pass would go next.
fn said(free: u64, hold: u64, root: &Path, next: &[PathBuf]) -> String {
    let mut text = format!("Disk has {} GB free, under the {} GB hold. No new Jobs start.", free / GIB, hold / GIB);
    let named: Vec<String> = next.iter().take(NAMED).map(|folder| shortened(root, folder)).collect();
    if !named.is_empty() {
        text.push_str(&format!(" Next to drop: {}.", named.join(", ")));
    }
    text
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
    /// What a look left of one root's disk, against the hold floor in force: under it, the item is
    /// offered, and over it, the item has left. **Nothing while the Operator is off**, as a red main.
    pub(crate) async fn disk_card_for(self: &Arc<Self>, seen: &Seen) {
        if !self.operator_on() {
            return;
        }
        let Some(free) = seen.free else { return };
        let Some(served) = self.repositories().served().into_iter().find(|served| Path::new(served.root()) == seen.root) else { return };
        let manifest = served.manifest().id().as_str().to_string();
        let hold = self.headroom().disk_floor().count();
        if free >= hold {
            self.disk_turns_swept(&format!("{DISK_HOLD}{manifest}")).await;
        } else {
            self.disk_hold_offered(manifest, &seen.root, free, hold, &seen.next).await;
        }
    }

    /// Queued once while the disk is short, and done by a rule where one covers it, which none does.
    async fn disk_hold_offered(self: &Arc<Self>, manifest: String, root: &Path, free: u64, hold: u64, next: &[PathBuf]) {
        let id = format!("{DISK_HOLD}{manifest}");
        if self.store().lock().await.operator_turn_present().unwrap_or_default().iter().any(|row| row.item == id) {
            return;
        }
        let options = if next.is_empty() {
            Vec::new()
        } else {
            [DROP_THEM, LEAVE_THEM].map(|label| WaitingOption { label: label.to_string(), description: None, picture: None }).into()
        };
        let item = WaitingItem {
            approves: false,
            id,
            text: said(free, hold, root, next),
            since: Instant::carried(self.now().as_str()),
            source: WaitingSource::OperatorSuggestion,
            act: None,
            options,
            steps: Vec::new(),
            place: None,
            confirmed: false,
            holders: Vec::new(),
            creating_since: None,
            create_refused: None,
            operator_read: None,
        };
        let one = ToRaise { session: String::new(), job: None, manifest_id: manifest, go_ahead: true, item, thens: Vec::new(), vet: None };
        let raising = &self.operator_seat().raising;
        let _turn = raising.turn.lock().await;
        let standing = self.store().lock().await.operator_turn_present().unwrap_or_default().iter().any(|row| row.item == one.item.id);
        if standing || !self.raise_marked(&one) {
            return;
        }
        let (ruled, rest) = self.rules_applied(vec![one]).await;
        if ruled {
            self.operator_page_changed();
        }
        self.review_queued(rest);
    }

    /// Every item `id` still in Your turn, answered or not, has left it.
    async fn disk_turns_swept(&self, id: &str) {
        let present = self.store().lock().await.operator_turn_present().unwrap_or_default();
        let mut left = false;
        for row in present.iter().filter(|row| row.item == id) {
            left |= self.turn_left(row).await;
        }
        if left {
            self.operator_page_changed();
        }
    }

    /// The press of Drop them: the first folders a pass would drop now, each as the pass would and
    /// behind the lock it takes, a folder a build holds left. Why nothing was dropped, where none was.
    pub(super) async fn disk_dropped(self: &Arc<Self>, manifest: &str) -> Result<(), String> {
        let served = self.repositories().served().into_iter().find(|served| served.manifest().id().as_str() == manifest).ok_or("that repository is not served now")?;
        let root = PathBuf::from(served.root());
        let base = served.repository().manifest().and_then(|manifest| manifest.base()).unwrap_or("main").to_string();
        let dropped = tokio::task::spawn_blocking(move || {
            let mut folders = inventory(&root, &base, &[]);
            mark_landed(&mut folders, &base);
            drop_first(&folders, SystemTime::now(), NAMED)
        })
        .await
        .map_err(|_| "the drop did not finish")?;
        if dropped.is_empty() {
            return Err("No build folder was free to drop".to_string());
        }
        Ok(())
    }
}
