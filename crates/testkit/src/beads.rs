//! A beads backlog held in memory: one remote, and a copy of it per machine that meets the others
//! only through [`Backlog::sync`]. For the tests that put two machines on one backlog.
//!
//! # What it is faithful about
//!
//! What spike 38 measured (`docs/spikes/038-how-far-behind-is-another-machine-on-beads.md`). A
//! write lands in its own copy and nowhere else until that copy syncs, and the other copy sees it
//! after its own sync. A sync merges field by field against what the copy last synced: a field
//! one side changed takes that side, and **a field both changed takes the later write, on both
//! copies**. "Later" is one counter every copy on a remote shares, standing in for the clock.
//!
//! It is **not** faithful about time: a sync is instant, and how long one takes is the spike's.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use adapter_traits::{Backlog, CopyMark, FromOutside, NotTracked, OpenIssue, QueryGuide, ReadIssue, Tracker};

/// One field's value and when it was written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Stamped {
    value: String,
    at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Issue {
    title: Stamped,
    status: Stamped,
    assignee: Stamped,
}

type Issues = BTreeMap<String, Issue>;

#[derive(Debug, Default)]
struct Shared {
    issues: Mutex<Issues>,
    /// Every write's stamp, and every new issue's number.
    clock: AtomicU64,
}

impl Shared {
    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::SeqCst) + 1
    }
}

/// The remote every machine's copy pushes to and pulls from.
#[derive(Clone, Debug, Default)]
pub struct FakeBeadsRemote(Arc<Shared>);

impl FakeBeadsRemote {
    pub fn new() -> FakeBeadsRemote {
        FakeBeadsRemote::default()
    }

    /// Who the remote says holds `reference`: empty where nobody does, `None` where it has no such issue.
    pub fn holder(&self, reference: &str) -> Option<String> {
        lock(&self.0.issues).get(reference).map(|issue| issue.assignee.value.clone())
    }
}

#[derive(Debug, Default)]
struct Copy {
    issues: Issues,
    /// What this copy and the remote agreed on at its last sync.
    base: Issues,
    /// Moves on every write to this copy.
    writes: u64,
}

/// One machine's copy of the backlog: the tracker its Fleet reads and the backlog it writes.
#[derive(Debug)]
pub struct FakeBeads {
    remote: FakeBeadsRemote,
    copy: Mutex<Copy>,
    syncs: AtomicUsize,
    unreachable: AtomicBool,
}

impl FakeBeads {
    /// A machine's copy of `remote`, empty until it syncs.
    pub fn on(remote: &FakeBeadsRemote) -> FakeBeads {
        FakeBeads {
            remote: remote.clone(),
            copy: Mutex::new(Copy::default()),
            syncs: AtomicUsize::new(0),
            unreachable: AtomicBool::new(false),
        }
    }

    /// File an issue in this copy, as an agent's `bd create` does, and its reference.
    pub fn create(&self, title: &str) -> String {
        let at = self.remote.0.tick();
        let reference = format!("armada-{at:03x}");
        let mut copy = lock(&self.copy);
        copy.issues.insert(
            reference.clone(),
            Issue {
                title: Stamped { value: title.to_string(), at },
                status: Stamped { value: "open".to_string(), at },
                assignee: Stamped { value: String::new(), at },
            },
        );
        copy.writes += 1;
        reference
    }

    /// How many syncs this copy ran, however they were asked for.
    pub fn syncs(&self) -> usize {
        self.syncs.load(Ordering::SeqCst)
    }

    /// While set, a sync fails as the network would and changes nothing.
    pub fn set_unreachable(&self, unreachable: bool) {
        self.unreachable.store(unreachable, Ordering::SeqCst);
    }

    fn open(&self) -> Vec<OpenIssue> {
        lock(&self.copy)
            .issues
            .iter()
            .filter(|(_, issue)| issue.status.value != "closed")
            .map(|(reference, issue)| OpenIssue {
                reference: reference.clone(),
                title: FromOutside::verbatim(issue.title.value.as_str()),
                url: FromOutside::verbatim(""),
                labels: Vec::new(),
                assignees: assignees(issue),
                updated_at: String::new(),
                excerpt: FromOutside::verbatim(""),
                linked_pull_request: false,
                author: FromOutside::verbatim(""),
                closed: false,
                fields: Vec::new(),
            })
            .collect()
    }
}

fn assignees(issue: &Issue) -> Vec<FromOutside> {
    Some(issue.assignee.value.as_str())
        .filter(|one| !one.is_empty())
        .map(FromOutside::verbatim)
        .into_iter()
        .collect()
}

fn lock<T>(held: &Mutex<T>) -> MutexGuard<'_, T> {
    held.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One field, three ways: the side that changed it since `base`, or the later write where both did.
fn merged_field(base: Option<&Stamped>, mine: &Stamped, theirs: &Stamped) -> Stamped {
    match (base == Some(mine), base == Some(theirs)) {
        (true, _) => theirs.clone(),
        (_, true) => mine.clone(),
        _ if theirs.at > mine.at => theirs.clone(),
        _ => mine.clone(),
    }
}

fn merged(base: &Issues, mine: &Issues, theirs: &Issues) -> Issues {
    let mut out = theirs.clone();
    for (reference, issue) in mine {
        let Some(other) = theirs.get(reference) else {
            out.insert(reference.clone(), issue.clone());
            continue;
        };
        let was = base.get(reference);
        out.insert(
            reference.clone(),
            Issue {
                title: merged_field(was.map(|one| &one.title), &issue.title, &other.title),
                status: merged_field(was.map(|one| &one.status), &issue.status, &other.status),
                assignee: merged_field(was.map(|one| &one.assignee), &issue.assignee, &other.assignee),
            },
        );
    }
    out
}

impl Backlog for FakeBeads {
    fn sync(&self, _in_repo: &str) -> Result<(), NotTracked> {
        self.syncs.fetch_add(1, Ordering::SeqCst);
        if self.unreachable.load(Ordering::SeqCst) {
            return Err(NotTracked::of("syncing the backlog", "the fake remote is switched off"));
        }
        let mut remote = lock(&self.remote.0.issues);
        let mut copy = lock(&self.copy);
        let merged = merged(&copy.base, &copy.issues, &remote);
        if merged != copy.issues {
            copy.writes += 1;
        }
        *remote = merged.clone();
        copy.issues = merged.clone();
        copy.base = merged;
        Ok(())
    }

    fn mark(&self, _in_repo: &str) -> Option<CopyMark> {
        Some(CopyMark::of(lock(&self.copy).writes.to_string()))
    }

    fn claim(&self, _in_repo: &str, reference: &str, actor: &str) -> Result<(), NotTracked> {
        let doing = "claiming the issue";
        let at = self.remote.0.tick();
        let mut copy = lock(&self.copy);
        let issue = copy
            .issues
            .get_mut(reference)
            .ok_or_else(|| NotTracked::of(doing, format!("no issue found matching \"{reference}\"")))?;
        let holder = issue.assignee.value.clone();
        if !holder.is_empty() && holder != actor {
            return Err(NotTracked::of(doing, format!("issue already claimed by {holder}")));
        }
        issue.assignee = Stamped { value: actor.to_string(), at };
        issue.status = Stamped { value: "in_progress".to_string(), at };
        copy.writes += 1;
        Ok(())
    }
}

impl Tracker for FakeBeads {
    fn open_issues(&self, _in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        Ok(self.open())
    }

    fn read_issue(&self, _in_repo: &str, reference: &str) -> Result<Option<ReadIssue>, NotTracked> {
        Ok(lock(&self.copy).issues.get(reference).map(|issue| {
            let mut read = ReadIssue::bare(
                reference.to_string(),
                FromOutside::verbatim(issue.title.value.as_str()),
                FromOutside::verbatim(""),
                issue.status.value == "closed",
            );
            read.assignees = assignees(issue);
            read
        }))
    }

    fn search_issues(&self, _in_repo: &str, _query: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        Ok(self.open())
    }

    fn query_guide(&self) -> QueryGuide {
        QueryGuide::default()
    }
}
