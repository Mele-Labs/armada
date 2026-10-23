//! A [`Tracker`] scripted by a test: the issues it lists, reads and finds, and the queries it
//! was asked. It has no write to record, because the trait has none.

use std::sync::Mutex;

use adapter_traits::{FromOutside, NotTracked, OpenIssue, QueryGuide, ReadIssue, Tracker};

#[derive(Debug, Default)]
pub struct FakeTracker {
    issues: Mutex<Vec<OpenIssue>>,
    closed: Mutex<Vec<ReadIssue>>,
    listings: Mutex<usize>,
    unreadable: Mutex<Option<String>>,
    found: Mutex<Vec<OpenIssue>>,
    searches: Mutex<Vec<String>>,
    opens: Mutex<Option<String>>,
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

    fn search_issues(&self, _in_repo: &str, query: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        self.searches.lock().expect("searches").push(query.to_string());
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
