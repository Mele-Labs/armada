//! A [`Tracker`] scripted by a test: the issues it lists, reads and finds, and the queries it
//! was asked. **Its writes are [`TrackerWrites`]'**, recorded as [`IssueWritten`] and never sent
//! anywhere, since a test never calls a real tracker.

use std::sync::Mutex;

use adapter_traits::{FromOutside, IssueWrite, NotTracked, OpenIssue, QueryGuide, ReadIssue, Tracker, TrackerWrites};

/// One write the fake was asked to make to an issue, in the order it was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IssueWritten {
    Moved { reference: String, status: String },
    Assigned { reference: String, to: String },
    Closed { reference: String, status: Option<String> },
}

#[derive(Debug, Default)]
pub struct FakeTracker {
    issues: Mutex<Vec<OpenIssue>>,
    closed: Mutex<Vec<ReadIssue>>,
    listings: Mutex<usize>,
    unreadable: Mutex<Option<String>>,
    lacking: Mutex<Option<String>>,
    unset: Mutex<Option<String>>,
    found: Mutex<Vec<OpenIssue>>,
    searches: Mutex<Vec<String>>,
    opens: Mutex<Option<String>>,
    written: Mutex<Vec<IssueWritten>>,
    /// The writes this tracker has no call for, by verb, and why.
    unable: Mutex<Vec<(&'static str, String)>>,
    write_refused: Mutex<Option<String>>,
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
            author: FromOutside::verbatim(""),
            closed: false,
            fields: Vec::new(),
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

    /// [`knows`](Self::knows), replacing what was known of the same reference: an issue whose
    /// status moves between two reads.
    pub fn now_knows(&self, issue: ReadIssue) {
        let mut known = self.closed.lock().expect("closed");
        known.retain(|held| held.reference != issue.reference);
        known.push(issue);
    }

    /// What the next listing answers, from now on.
    pub fn now_lists(&self, issues: Vec<OpenIssue>) {
        *self.issues.lock().expect("issues") = issues;
    }

    /// Every listing is refused with `said` until [`answers_again`](Self::answers_again).
    pub fn will_not_list(&self, said: &str) {
        *self.unreadable.lock().expect("unreadable") = Some(said.to_string());
    }

    /// Every call is refused as a machine without `tool` refuses it, until
    /// [`answers_again`](Self::answers_again).
    pub fn lacks(&self, tool: &str) {
        *self.lacking.lock().expect("lacking") = Some(tool.to_string());
    }

    /// Every search is refused as `tool` refuses it with no project here, until
    /// [`answers_again`](Self::answers_again).
    pub fn needs_setup(&self, tool: &str) {
        *self.unset.lock().expect("unset") = Some(tool.to_string());
    }

    pub fn answers_again(&self) {
        *self.unset.lock().expect("unset") = None;
        *self.unreadable.lock().expect("unreadable") = None;
        *self.lacking.lock().expect("lacking") = None;
        *self.write_refused.lock().expect("write_refused") = None;
    }

    /// What every search answers, whatever its query, from now on. A refusal from
    /// [`will_not_list`](Self::will_not_list) refuses a search too.
    pub fn finds(&self, issues: Vec<OpenIssue>) {
        *self.found.lock().expect("found") = issues;
    }

    /// Every query searched for, oldest first, refused ones included.
    pub fn searches(&self) -> Vec<String> {
        self.searches.lock().expect("searches").clone()
    }

    /// The guide every fake answers with.
    pub fn guide() -> QueryGuide {
        QueryGuide {
            language: "Fake search".to_string(),
            guide: "Words match a title. `state = open` or `state = closed`; `assignee = me`.".to_string(),
            open: "state = open".to_string(),
            assigned_to_me: "state = open assignee = me".to_string(),
            fields: vec!["Labels".to_string()],
        }
    }

    /// From now on an example opens at `template`, with `{match}` where its reference goes.
    pub fn opens_at(&self, template: &str) {
        *self.opens.lock().expect("opens") = Some(template.to_string());
    }

    /// From now on this tracker has no call to `verb` (`move`, `assign` or `close`), and says `why`.
    pub fn cannot(&self, verb: &'static str, why: &str) {
        self.unable.lock().expect("unable").push((verb, why.to_string()));
    }

    /// Every write is refused with `said` as the tracker's own sentence, until
    /// [`answers_again`](Self::answers_again).
    pub fn will_not_write(&self, said: &str) {
        *self.write_refused.lock().expect("write_refused") = Some(said.to_string());
    }

    /// Every write made, oldest first. A refused one is not here.
    pub fn writes(&self) -> Vec<IssueWritten> {
        self.written.lock().expect("written").clone()
    }

    /// How many times the issues were listed.
    pub fn listings(&self) -> usize {
        *self.listings.lock().expect("listings")
    }
}

impl Tracker for FakeTracker {
    fn open_issues(&self, _in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        *self.listings.lock().expect("listings") += 1;
        if let Some(tool) = self.lacking.lock().expect("lacking").clone() {
            return Err(NotTracked::lacking("listing the open issues", &tool));
        }
        if let Some(said) = self.unreadable.lock().expect("unreadable").clone() {
            return Err(NotTracked::of("listing the open issues", said));
        }
        Ok(self.issues.lock().expect("issues").clone())
    }

    /// A listed issue is open, a closed one is what [`knows_closed`](Self::knows_closed) was told,
    /// and any other reference is none: a pull request's number is not told either. `214` and
    /// `#214` name the same issue.
    fn read_issue(&self, _in_repo: &str, reference: &str) -> Result<Option<ReadIssue>, NotTracked> {
        if let Some(tool) = self.lacking.lock().expect("lacking").clone() {
            return Err(NotTracked::lacking("reading the issue", &tool));
        }
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

    fn search_issues(&self, _in_repo: &str, query: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        self.searches.lock().expect("searches").push(query.to_string());
        if let Some(tool) = self.lacking.lock().expect("lacking").clone() {
            return Err(NotTracked::lacking("searching the issues", &tool));
        }
        if let Some(tool) = self.unset.lock().expect("unset").clone() {
            return Err(NotTracked::unset("searching the issues", &tool, "no beads project found"));
        }
        if let Some(said) = self.unreadable.lock().expect("unreadable").clone() {
            return Err(NotTracked::of("searching the issues", said));
        }
        Ok(self.found.lock().expect("found").clone())
    }

    fn query_guide(&self) -> QueryGuide {
        FakeTracker::guide()
    }

    fn address_template(&self, example: &str) -> Option<String> {
        self.opens.lock().expect("opens").clone().filter(|_| !example.trim().is_empty())
    }

    /// The forge's own reading, since a fixture repository is one: the numbers of its issues the
    /// text links. Delegated rather than written again, so a fake cannot pass for a format the
    /// shipped reader does not take.
    fn issues_linked_in(&self, in_repo: &str, text: &str) -> Vec<String> {
        match adapters::remote_of(std::path::Path::new(in_repo)) {
            Some(repository) => adapters::issues_linked_in(text, &repository).iter().map(u64::to_string).collect(),
            None => Vec::new(),
        }
    }
}

impl TrackerWrites for FakeTracker {
    fn can(&self, write: &IssueWrite<'_>) -> Result<(), String> {
        match self.unable.lock().expect("unable").iter().find(|(verb, _)| *verb == write.verb()) {
            Some((_, why)) => Err(why.clone()),
            None => Ok(()),
        }
    }

    fn write(&self, _in_repo: &str, reference: &str, write: &IssueWrite<'_>) -> Result<(), NotTracked> {
        self.can(write).map_err(|why| NotTracked::of(write.doing(), why))?;
        if let Some(said) = self.write_refused.lock().expect("write_refused").clone() {
            return Err(NotTracked::of(write.doing(), said));
        }
        let reference = reference.to_string();
        self.written.lock().expect("written").push(match write {
            IssueWrite::Move { status } => IssueWritten::Moved { reference, status: status.to_string() },
            IssueWrite::Assign { to } => IssueWritten::Assigned { reference, to: to.to_string() },
            IssueWrite::Close { status } => IssueWritten::Closed { reference, status: status.map(str::to_string) },
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_search_is_recorded_and_answers_what_the_fake_was_given_or_its_refusal() {
        let tracker = FakeTracker::empty();
        tracker.finds(vec![FakeTracker::issue(7, "Found")]);
        let found = tracker.search_issues("/repo", "state = open").expect("found");
        assert_eq!(found, vec![FakeTracker::issue(7, "Found")]);
        tracker.will_not_list("offline");
        let refused = tracker.search_issues("/repo", "assignee = me").expect_err("refused");
        assert_eq!((refused.doing, refused.said.as_str()), ("searching the issues", "offline"));
        assert_eq!(tracker.searches(), ["state = open", "assignee = me"]);
        assert_eq!(tracker.listings(), 0, "a search is not a listing");
    }

    #[test]
    fn a_write_is_recorded_unless_the_fake_cannot_make_it_or_was_told_to_refuse() {
        let tracker = FakeTracker::empty();
        let moves = IssueWrite::Move { status: "In Progress" };
        assert_eq!(tracker.can(&moves), Ok(()));
        tracker.write("/repo", "ARM-41", &moves).expect("made");
        assert_eq!(tracker.writes(), [IssueWritten::Moved { reference: "ARM-41".into(), status: "In Progress".into() }]);

        tracker.cannot("assign", "this tracker cannot assign an issue");
        let assigns = IssueWrite::Assign { to: "ana" };
        assert_eq!(tracker.can(&assigns), Err("this tracker cannot assign an issue".to_string()));
        assert!(tracker.write("/repo", "ARM-41", &assigns).is_err());

        tracker.will_not_write("offline");
        let refused = tracker.write("/repo", "ARM-42", &moves).expect_err("refused");
        assert_eq!((refused.doing, refused.said.as_str()), ("moving the issue", "offline"));
        assert_eq!(tracker.writes().len(), 1, "a refused write is not recorded");
    }

    #[test]
    fn a_fake_opens_an_example_where_it_was_told_and_nowhere_before() {
        let tracker = FakeTracker::empty();
        assert_eq!(tracker.address_template("ARM-1"), None);
        tracker.opens_at("https://t.example/{match}");
        assert_eq!(tracker.address_template("ARM-1"), Some("https://t.example/{match}".to_string()));
        assert_eq!(tracker.address_template(" "), None);
    }

    #[test]
    fn the_fakes_guide_carries_its_open_and_assigned_to_me() {
        let guide = FakeTracker::empty().query_guide();
        assert_eq!(guide.open, "state = open");
        assert_eq!(guide.assigned_to_me, "state = open assignee = me");
    }
}
