//! A [`Tracker`] scripted by a test: the issues it lists, every write it was
//! asked for, and whether those writes are refused. #91.

use std::sync::Mutex;

use adapter_traits::{FromOutside, IssueAddress, NotTracked, OpenIssue, ReadIssue, Tracker};

/// One write the fake was asked for, in the order it was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrackerWrite {
    Commented { url: String, text: String },
    Relabelled { url: String, add: Vec<String>, remove: Vec<String> },
}

#[derive(Debug, Default)]
pub struct FakeTracker {
    issues: Mutex<Vec<OpenIssue>>,
    closed: Mutex<Vec<ReadIssue>>,
    listings: Mutex<usize>,
    unreadable: Mutex<Option<String>>,
    refusing: Mutex<Option<String>>,
    writes: Mutex<Vec<TrackerWrite>>,
    titling: Mutex<bool>,
}

impl FakeTracker {
    /// Lists nothing and takes every write.
    pub fn empty() -> FakeTracker {
        FakeTracker::default()
    }

    /// Lists `issues` and takes every write.
    pub fn listing(issues: Vec<OpenIssue>) -> FakeTracker {
        let tracker = FakeTracker::default();
        *tracker.issues.lock().expect("issues") = issues;
        tracker
    }

    /// A tracker that links by the pull request's title: it marks the title with the issue's
    /// reference and has no closing line for the body.
    pub fn linking_by_title() -> FakeTracker {
        let tracker = FakeTracker::default();
        *tracker.titling.lock().expect("titling") = true;
        tracker
    }

    /// An open issue with only what a test usually names.
    pub fn issue(number: u64, title: &str) -> OpenIssue {
        OpenIssue {
            number,
            title: FromOutside::verbatim(title),
            url: FromOutside::verbatim(format!("https://forge.invalid/o/r/issues/{number}")),
            labels: Vec::new(),
            assignees: Vec::new(),
            updated_at: "2026-10-11T12:00:00Z".to_string(),
            excerpt: FromOutside::verbatim(""),
            linked_pull_request: false,
        }
    }

    /// An issue that is closed: read by number, never listed.
    pub fn knows_closed(&self, number: u64, title: &str) {
        self.closed.lock().expect("closed").push(ReadIssue {
            number,
            title: FromOutside::verbatim(title),
            url: FromOutside::verbatim(format!("https://forge.invalid/o/r/issues/{number}")),
            closed: true,
        });
    }

    /// What the next listing answers, from now on.
    pub fn now_lists(&self, issues: Vec<OpenIssue>) {
        *self.issues.lock().expect("issues") = issues;
    }

    /// Every listing is refused with `said` until [`answers_again`](Self::answers_again).
    pub fn will_not_list(&self, said: &str) {
        *self.unreadable.lock().expect("unreadable") = Some(said.to_string());
    }

    pub fn answers_again(&self) {
        *self.unreadable.lock().expect("unreadable") = None;
        *self.refusing.lock().expect("refusing") = None;
    }

    /// Every write is refused with `said`, and none is kept.
    pub fn refuses_writes(&self, said: &str) {
        *self.refusing.lock().expect("refusing") = Some(said.to_string());
    }

    /// How many times the issues were listed.
    pub fn listings(&self) -> usize {
        *self.listings.lock().expect("listings")
    }

    /// Every write taken, oldest first.
    pub fn writes(&self) -> Vec<TrackerWrite> {
        self.writes.lock().expect("writes").clone()
    }

    fn take(&self, doing: &'static str, write: TrackerWrite) -> Result<(), NotTracked> {
        if let Some(said) = self.refusing.lock().expect("refusing").clone() {
            return Err(NotTracked::of(doing, said));
        }
        self.writes.lock().expect("writes").push(write);
        Ok(())
    }
}

impl Tracker for FakeTracker {
    fn open_issues(&self, _in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        *self.listings.lock().expect("listings") += 1;
        if let Some(said) = self.unreadable.lock().expect("unreadable").clone() {
            return Err(NotTracked::of("listing the open issues", said));
        }
        Ok(self.issues.lock().expect("issues").clone())
    }

    /// A listed issue is open, a closed one is what [`knows_closed`](Self::knows_closed) was told,
    /// and any other number is none: a pull request's number is not told either.
    fn read_issue(&self, _in_repo: &str, number: u64) -> Result<Option<ReadIssue>, NotTracked> {
        if let Some(said) = self.unreadable.lock().expect("unreadable").clone() {
            return Err(NotTracked::of("reading the issue", said));
        }
        let listed = self
            .issues
            .lock()
            .expect("issues")
            .iter()
            .find(|issue| issue.number == number)
            .map(|issue| ReadIssue {
                number,
                title: issue.title.clone(),
                url: issue.url.clone(),
                closed: false,
            });
        let closed = self
            .closed
            .lock()
            .expect("closed")
            .iter()
            .find(|issue| issue.number == number)
            .cloned();
        Ok(listed.or(closed))
    }

    fn comment(&self, issue: &IssueAddress, text: &str) -> Result<(), NotTracked> {
        self.take(
            "commenting on the issue",
            TrackerWrite::Commented {
                url: issue.url().to_string(),
                text: text.to_string(),
            },
        )
    }

    fn relabel(
        &self,
        issue: &IssueAddress,
        add: &[&str],
        remove: &[&str],
    ) -> Result<(), NotTracked> {
        self.take(
            "moving the issue's labels",
            TrackerWrite::Relabelled {
                url: issue.url().to_string(),
                add: add.iter().map(|one| one.to_string()).collect(),
                remove: remove.iter().map(|one| one.to_string()).collect(),
            },
        )
    }

    /// The reference itself, so a test can find it in a body.
    fn closing_line(&self, issue: &IssueAddress) -> Option<String> {
        match *self.titling.lock().expect("titling") {
            true => None,
            false => Some(format!("Closes {}", issue.reference())),
        }
    }

    fn title_mark(&self, issue: &IssueAddress) -> Option<String> {
        (*self.titling.lock().expect("titling")).then(|| issue.reference().to_string())
    }
}
