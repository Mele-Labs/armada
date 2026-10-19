//! A [`Tracker`] scripted by a test: the issues it lists and reads. It has no
//! write to record, because the trait has none.

use std::sync::Mutex;

use adapter_traits::{FromOutside, NotTracked, OpenIssue, ReadIssue, Tracker};

#[derive(Debug, Default)]
pub struct FakeTracker {
    issues: Mutex<Vec<OpenIssue>>,
    closed: Mutex<Vec<ReadIssue>>,
    listings: Mutex<usize>,
    unreadable: Mutex<Option<String>>,
}

impl FakeTracker {
    /// Lists nothing.
    pub fn empty() -> FakeTracker {
        FakeTracker::default()
    }

    /// Lists `issues`.
    pub fn listing(issues: Vec<OpenIssue>) -> FakeTracker {
        let tracker = FakeTracker::default();
        *tracker.issues.lock().expect("issues") = issues;
        tracker
    }

    /// An open issue with only what a test usually names.
    pub fn issue(number: u64, title: &str) -> OpenIssue {
        OpenIssue {
            reference: format!("#{number}"),
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
        self.knows(ReadIssue::bare(
            format!("#{number}"),
            FromOutside::verbatim(title),
            FromOutside::verbatim(format!("https://forge.invalid/o/r/issues/{number}")),
            true,
        ));
    }

    /// An issue read by number, never listed, with whatever else it carries.
    pub fn knows(&self, issue: ReadIssue) {
        self.closed.lock().expect("closed").push(issue);
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
    }

    /// How many times the issues were listed.
    pub fn listings(&self) -> usize {
        *self.listings.lock().expect("listings")
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
    /// and any other reference is none: a pull request's number is not told either. `214` and
    /// `#214` name the same issue.
    fn read_issue(&self, _in_repo: &str, reference: &str) -> Result<Option<ReadIssue>, NotTracked> {
        if let Some(said) = self.unreadable.lock().expect("unreadable").clone() {
            return Err(NotTracked::of("reading the issue", said));
        }
        let reference = match reference.parse::<u64>() {
            Ok(number) => format!("#{number}"),
            Err(_) => reference.to_string(),
        };
        let listed = self
            .issues
            .lock()
            .expect("issues")
            .iter()
            .find(|issue| issue.reference == reference)
            .map(|issue| {
                ReadIssue::bare(
                    issue.reference.clone(),
                    issue.title.clone(),
                    issue.url.clone(),
                    false,
                )
            });
        let closed = self
            .closed
            .lock()
            .expect("closed")
            .iter()
            .find(|issue| issue.reference == reference)
            .cloned();
        Ok(listed.or(closed))
    }
}
