//! Auto land's guard on the merge queue: a pull request is not queued behind one it conflicts with.
//! `docs/concepts/session.md`, *Auto land*.
//!
//! The forge's own `conflicting` covers the base. This covers the entries ahead in the queue, which
//! the forge learns of only when it tests the queued commit, drops it as unmergeable, and the loop
//! of queueing it again begins. **Fail open**: a queue the forge would not list, a head not known
//! and a git that could not test are all read as "nothing clashes", which is how Auto land behaved
//! before this guard.

use std::path::Path;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, QueueEntry, Vcs, WorkProduct};
use adapters::clash::{between_commits, Verdict};

use crate::daemon::Fleet;
use crate::repositories::Served;

/// How long fetching a pull request's head may take before it is read as not fetched.
const FETCH_WITHIN: Duration = Duration::from_secs(20);

/// The one trigger a clean test is kept under, whatever sits ahead.
const TESTED: &str = "auto_land_tested";

/// The pull request ahead in the queue that a head clashes with, and the files that do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Blocker {
    pub number: u64,
    pub files: Vec<String>,
}

/// What testing a head against the queue ahead of it decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Guard {
    /// Everything ahead was tested and merges clean, or nothing is ahead.
    Go,
    /// Something ahead could not be tested, and nothing tested clashed. Acts as `Go`, and is not
    /// kept as tested.
    Unchecked,
    /// Clashes with one ahead, and is not queued: not to be.
    Hold(Blocker),
    /// Clashes with one ahead, and is queued: to be taken out.
    Dequeue(Blocker),
}

/// How one entry ahead tested against the head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Tested {
    Clean,
    Clash(Vec<String>),
    /// Git could not say, or the queue named no head for it.
    Unknown,
}

/// The entries that merge before pull request `number`, soonest first: all of them where it is not
/// in the queue (it would go last), those at a lower position where it is.
pub(crate) fn ahead_of(number: u64, queue: &[QueueEntry]) -> Vec<QueueEntry> {
    let own = queue.iter().find(|entry| entry.number == number).map(|entry| entry.position);
    let mut ahead: Vec<QueueEntry> = queue
        .iter()
        .filter(|entry| entry.number != number && own.map_or(true, |at| entry.position < at))
        .copied()
        .collect();
    ahead.sort_by_key(|entry| entry.position);
    ahead
}

/// The decision, with the test of one entry against the head given. The first entry that clashes
/// is the blocker, so a pull request waits on the next one to merge rather than the last.
pub(crate) fn decide(number: u64, queue: &[QueueEntry], mut test: impl FnMut(&QueueEntry) -> Tested) -> Guard {
    let queued = queue.iter().any(|entry| entry.number == number);
    let mut unknown = false;
    for entry in ahead_of(number, queue) {
        match test(&entry) {
            Tested::Clean => {}
            Tested::Unknown => unknown = true,
            Tested::Clash(files) => {
                let blocker = Blocker { number: entry.number, files };
                return if queued { Guard::Dequeue(blocker) } else { Guard::Hold(blocker) };
            }
        }
    }
    if unknown {
        Guard::Unchecked
    } else {
        Guard::Go
    }
}

/// The key a head's test is kept under: the entries ahead, each at the head the queue holds. A
/// queue that moves or an entry that is pushed to changes it.
fn sketch_of(ahead: &[QueueEntry]) -> String {
    let each: Vec<String> =
        ahead.iter().map(|entry| format!("{}@{}", entry.number, entry.head.as_ref().map_or("", |head| head.as_str()))).collect();
    format!("{TESTED}:{}", each.join(","))
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
    /// Pull request `number` at `head`, tested against the queue ahead of it. **A clean test is kept
    /// for the queue it was made against**, so a queued pull request costs git nothing on the
    /// readings where nothing moved; a clash is tested again each time.
    pub(super) async fn queue_guarded(&self, served: &Served, number: u64, head: &str) -> Guard {
        let Some(base) = served.manifest().base().map(str::to_string) else {
            return Guard::Go;
        };
        let root = served.root();
        let listed = self.forge_asked(root, move |vcs: &V, root: &str| vcs.merge_queue(root, &base)).await;
        let Some(queue) = listed else {
            return Guard::Go;
        };
        let ahead = ahead_of(number, &queue);
        if ahead.is_empty() {
            return Guard::Go;
        }
        let sketch = sketch_of(&ahead);
        let tested = self.store().lock().await.pull_notice_kept(root, number, head, &sketch, "fleet");
        if tested.unwrap_or(false) {
            return Guard::Go;
        }
        let (repo, mine) = (root.to_string(), head.to_string());
        let guard = tokio::task::spawn_blocking(move || tested_against(&repo, number, &mine, &queue))
            .await
            .unwrap_or(Guard::Go);
        if guard == Guard::Go {
            let now = self.now();
            let _ = self.store().lock().await.keep_pull_notice(root, number, head, &sketch, "fleet", &now);
        }
        guard
    }
}

/// The words to the Job for a pull request Auto land holds, or took out of the queue.
fn held_words(number: u64, blocker: &Blocker, taken_out: bool) -> String {
    let at = blocker.number;
    let shown: Vec<&str> = blocker.files.iter().take(5).map(String::as_str).collect();
    let more = blocker.files.len().saturating_sub(shown.len());
    let files = match more {
        0 => shown.join(", "),
        _ => format!("{}, and {more} more", shown.join(", ")),
    };
    let (what, then) = match taken_out {
        true => (format!("#{number} was taken out of the merge queue: it now conflicts with"), "queues it again"),
        false => (format!("#{number} waits: it conflicts with"), "queues it"),
    };
    format!(
        "Auto land is on, and {what} #{at}, which is ahead of it in the merge queue (files: {files}). \
         Merge main in and resolve it once #{at} has merged; Fleet {then} then."
    )
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
    /// Tell the Job what its pull request waits on, **once a commit and a blocker**: a blocker that
    /// merges and is followed by another is told again, the same one on the next reading is not.
    /// Kept only where the Job was told, so a Job not listening is told on a later reading.
    pub(super) async fn hold_told(
        self: &std::sync::Arc<Self>,
        root: &str,
        session: &str,
        recipient: &str,
        (number, head): (u64, &str),
        blocker: &Blocker,
        taken_out: bool,
    ) {
        let trigger = format!("auto_land_held:{}", blocker.number);
        let told = self.store().lock().await.pull_notice_kept(root, number, head, &trigger, recipient);
        if told.unwrap_or(true) {
            return;
        }
        let text = held_words(number, blocker, taken_out);
        if self.told_by_fleet(session, &text, ipc::Notice::AutoLandRefused).await.unwrap_or(false) {
            let now = self.now();
            let _ = self.store().lock().await.keep_pull_notice(root, number, head, &trigger, recipient, &now);
        }
    }
}

/// Blocking: fetches every head it tests, then merge-tests in a checkout of the repository.
fn tested_against(root: &str, number: u64, head: &str, queue: &[QueueEntry]) -> Guard {
    if adapters::fetch_pull_head(root, number, FETCH_WITHIN).is_err() {
        return Guard::Unchecked;
    }
    decide(number, queue, |entry| {
        let Some(theirs) = entry.head else {
            return Tested::Unknown;
        };
        if adapters::fetch_pull_head(root, entry.number, FETCH_WITHIN).is_err() {
            return Tested::Unknown;
        }
        match between_commits(Path::new(root), head, theirs.as_str()) {
            Ok(Verdict::Clean) => Tested::Clean,
            Ok(Verdict::Conflict(files)) => Tested::Clash(files),
            Err(_) => Tested::Unknown,
        }
    })
}
