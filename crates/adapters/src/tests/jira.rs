//! The Jira tracker and its link reader: the commands they build, and how they
//! read what comes back.
//!
//! **A runner that prints what it was told to**, so no `acli` is needed and the
//! cases read the arguments it was given. The filters are the other half: those
//! are run by the real `jq` over both output shapes, which the Jira tracker
//! needs on any machine it runs on, so a machine without it fails them.

use std::collections::{BTreeMap, VecDeque};
use std::io::ErrorKind;
use std::os::unix::process::ExitStatusExt;
use std::process::{ExitStatus, Output};
use std::sync::Mutex;

use adapter_traits::{LinkLookup, Tracker, TrackerChoice, Trackers};

use crate::jira::{self, JiraSettings};
use crate::jira_lookup::JiraLookup;
use crate::jira_tracker::{issue_of_line, read_issue_of_line, JiraTracker, Run};
use crate::TrackersByProvider;

pub(super) fn block(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

pub(super) fn settings() -> JiraSettings {
    JiraSettings::read(&block(&[("site", "acme"), ("project", "ABC")])).expect("a block")
}

// ------------------------------------------------------------------ the block

#[test]
fn a_site_name_is_a_cloud_site_and_a_host_with_a_dot_is_taken_as_written() {
    let cloud = settings();
    assert_eq!(cloud.browse("ABC-1"), "https://acme.atlassian.net/browse/ABC-1");
    let own = JiraSettings::read(&block(&[("site", "Jira.Example.com"), ("project", "ABC")]))
        .expect("a block");
    assert_eq!(own.browse("ABC-1"), "https://jira.example.com/browse/ABC-1");
}

#[test]
fn a_block_that_cannot_be_read_says_which_key_and_why() {
    let refused = |pairs: &[(&str, &str)]| JiraSettings::read(&block(pairs)).expect_err("refused");
    assert!(refused(&[("project", "ABC")]).contains("`site`"));
    let neither = refused(&[("site", "acme")]);
    assert!(neither.contains("`project` or `jql`"), "{neither}");
    assert!(refused(&[("site", "acme"), ("project", "abc")]).contains("not a Jira project key"));
    assert!(refused(&[("site", "acme"), ("project", "A\" OR 1=1")]).contains("not a Jira project key"));
    assert!(refused(&[("site", "acme"), ("project", "ABC, ops")]).contains("`project: ops`"));
    assert!(refused(&[("site", "ac me"), ("project", "ABC")]).contains("`site: ac me`"));
    let unknown = refused(&[("site", "acme"), ("project", "ABC"), ("board", "7")]);
    assert!(unknown.contains("`board`") && unknown.contains("site, project, jql"), "{unknown}");
    let ordered = refused(&[("site", "acme"), ("project", "ABC"), ("jql", "labels = a ORDER BY rank")]);
    assert!(ordered.contains("ORDER BY"), "{ordered}");
}

#[test]
fn the_search_is_always_bound_to_the_project_and_an_extra_filter_is_anded_in() {
    assert_eq!(
        settings().search(),
        "project = \"ABC\" AND statusCategory != Done ORDER BY updated DESC"
    );
    let filtered = JiraSettings::read(&block(&[
        ("site", "acme"),
        ("project", "ABC"),
        ("jql", "labels = web OR labels = api"),
    ]))
    .expect("a block");
    assert_eq!(
        filtered.search(),
        "project = \"ABC\" AND statusCategory != Done AND (labels = web OR labels = api) ORDER BY updated DESC"
    );
}

#[test]
fn several_projects_are_listed_together_and_a_filter_alone_is_enough() {
    let many = JiraSettings::read(&block(&[("site", "acme"), ("project", "ABC, OPS  WEB")])).expect("a block");
    assert_eq!(
        many.search(),
        "project in (\"ABC\", \"OPS\", \"WEB\") AND statusCategory != Done ORDER BY updated DESC"
    );
    let only_jql = JiraSettings::read(&block(&[("site", "acme"), ("jql", "labels = web OR assignee = currentUser()")]))
        .expect("a filter is enough");
    assert_eq!(
        only_jql.search(),
        "statusCategory != Done AND (labels = web OR assignee = currentUser()) ORDER BY updated DESC"
    );
}

// ------------------------------------------------------------------ the runner

pub(super) type Answer = (i32, &'static str, &'static str);

/// Answers each call in turn, and keeps every call it was given.
#[derive(Default)]
pub(super) struct Scripted {
    answers: Mutex<VecDeque<Answer>>,
    calls: Mutex<Vec<(String, Vec<String>, Option<String>)>>,
    absent: Option<&'static str>,
}

impl Scripted {
    pub(super) fn saying(answers: &[Answer]) -> Scripted {
        Scripted {
            answers: Mutex::new(answers.iter().copied().collect()),
            ..Scripted::default()
        }
    }

    fn without(program: &'static str) -> Scripted {
        Scripted {
            absent: Some(program),
            ..Scripted::default()
        }
    }

    pub(super) fn calls(&self) -> Vec<(String, Vec<String>, Option<String>)> {
        self.calls.lock().expect("not poisoned").clone()
    }
}

impl Run for &Scripted {
    fn run(&self, program: &str, args: &[String], stdin: Option<&[u8]>) -> std::io::Result<Output> {
        self.calls.lock().expect("not poisoned").push((
            program.to_string(),
            args.to_vec(),
            stdin.map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
        ));
        if self.absent == Some(program) {
            return Err(std::io::Error::from(ErrorKind::NotFound));
        }
        let (code, stdout, stderr) = self
            .answers
            .lock()
            .expect("not poisoned")
            .pop_front()
            .unwrap_or((0, "", ""));
        Ok(Output {
            status: ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        })
    }
}

// ------------------------------------------------------------------ the listing

#[test]
fn the_listing_searches_the_project_through_acli_and_reduces_it_with_jq() {
    let run = Scripted::saying(&[(0, "[{\"key\":\"ABC-1\"}]", ""), (0, "", "")]);
    JiraTracker::with_runner(settings(), &run)
        .open_issues("/repo")
        .expect("listed");
    let calls = run.calls();
    assert_eq!(calls[0].0, "acli");
    assert_eq!(
        calls[0].1,
        [
            "jira", "workitem", "search", "--jql",
            "project = \"ABC\" AND statusCategory != Done ORDER BY updated DESC",
            "--fields", "key,summary,labels,assignee,updated,description",
            "--json", "--limit", "200",
        ]
    );
    assert_eq!(calls[1].0, "jq");
    assert_eq!(calls[1].1[0], "-r");
    assert_eq!(calls[1].2.as_deref(), Some("[{\"key\":\"ABC-1\"}]"));
}

#[test]
fn issues_are_read_off_the_lines_newest_edit_first_whatever_project_the_search_let_through() {
    let lines = "ABC-3\tOlder\t\t\t2026-10-13T10:00:00+02:00\t\n\
                 ABC-12\tReader drops a row\tweb\u{1f}armada-in-progress\tNick\t2026-10-14T10:11:12.123+02:00\tFirst line\\tmore\n\
                 OTHER-9\tNot ours\t\t\t2026-10-14T11:00:00+02:00\t";
    let run = Scripted::saying(&[(0, "[]", ""), (0, lines, "")]);
    let issues = JiraTracker::with_runner(settings(), &run)
        .open_issues("/repo")
        .expect("listed");
    let references: Vec<&str> = issues.iter().map(|one| one.reference.as_str()).collect();
    assert_eq!(references, ["OTHER-9", "ABC-12", "ABC-3"]);
    let first = &issues[1];
    assert_eq!(first.title.as_written(), "Reader drops a row");
    assert_eq!(first.url.as_written(), "https://acme.atlassian.net/browse/ABC-12");
    let labels: Vec<&str> = first.labels.iter().map(|one| one.as_written()).collect();
    assert_eq!(labels, ["web", "armada-in-progress"]);
    assert_eq!(first.assignees[0].as_written(), "Nick");
    assert_eq!(first.excerpt.as_written(), "First line\tmore");
    assert!(!first.linked_pull_request);
    assert!(issues[2].assignees.is_empty());
}

#[test]
fn a_line_whose_first_field_is_not_a_key_is_not_an_issue() {
    assert_eq!(issue_of_line(&settings(), "not-a-key\tTitle\t\t\t2026-10-14T10:00:00Z\t"), None);
    assert_eq!(issue_of_line(&settings(), ""), None);
}

#[test]
fn an_acli_that_refuses_is_the_trackers_refusal_in_its_own_words() {
    let run = Scripted::saying(&[(1, "", "not signed in: run `acli jira auth login`")]);
    let refused = JiraTracker::with_runner(settings(), &run)
        .open_issues("/repo")
        .expect_err("refused");
    assert_eq!(refused.doing, "listing the open issues");
    assert!(refused.said.contains("acli jira auth login"), "{}", refused.said);
    assert_eq!(run.calls().len(), 1, "jq is not asked about an answer that never came");
}

#[test]
fn a_tool_that_is_not_installed_is_named_rather_than_left_to_a_raw_error() {
    for tool in ["acli", "jq"] {
        let run = Scripted::without(tool);
        let refused = JiraTracker::with_runner(settings(), &run)
            .open_issues("/repo")
            .expect_err("refused");
        assert!(refused.said.contains(&format!("`{tool}` is not installed")), "{}", refused.said);
    }
}

// ------------------------------------------------------------------ one issue

#[test]
fn an_issue_is_read_by_its_key_in_whatever_project_it_is_in() {
    let run = Scripted::saying(&[(0, "{}", ""), (0, "ABC-7\tThe title\tdone\tresolved\n", "")]);
    let read = JiraTracker::with_runner(settings(), &run)
        .read_issue("/repo", "ABC-7")
        .expect("read")
        .expect("an issue");
    assert_eq!(run.calls()[0].1[..4], ["jira", "workitem", "view", "ABC-7"]);
    assert_eq!(read.reference, "ABC-7");
    assert!(read.closed);
    assert_eq!(read.url.as_written(), "https://acme.atlassian.net/browse/ABC-7");
}

#[test]
fn an_issue_that_is_open_reads_as_open_and_a_missing_one_as_none() {
    let it = settings();
    assert!(!read_issue_of_line(&it, "ABC-8\tOpen one\tnew\t").expect("an issue").closed);
    assert!(!read_issue_of_line(&it, "ABC-8\tOpen one\tindeterminate\t").expect("an issue").closed);
    assert!(read_issue_of_line(&it, "ABC-8\tClosed\tdone\t").expect("an issue").closed);
    assert_eq!(read_issue_of_line(&it, "7\tNot a key\tdone\t"), None);
    assert_eq!(
        read_issue_of_line(&it, "OTHER-8\tElsewhere\tdone\t").map(|one| one.reference),
        Some("OTHER-8".to_string())
    );
    let run = Scripted::saying(&[(1, "", "Issue does not exist or you do not have permission")]);
    assert_eq!(
        JiraTracker::with_runner(settings(), &run).read_issue("/repo", "ABC-99"),
        Ok(None)
    );
    let broken = Scripted::saying(&[(1, "", "the server is down")]);
    assert!(JiraTracker::with_runner(settings(), &broken)
        .read_issue("/repo", "ABC-99")
        .is_err());
}

#[test]
fn text_that_is_not_a_key_names_no_issue_and_never_reaches_the_cli() {
    let run = Scripted::saying(&[]);
    let jira = JiraTracker::with_runner(settings(), &run);
    for text in ["214", "#214", "--key=ABC-1", "abc-1", ""] {
        assert_eq!(jira.read_issue("/repo", text), Ok(None), "{text}");
    }
    assert!(run.calls().is_empty());
}

// ------------------------------------------------------------------ links

#[test]
fn a_browse_link_on_the_site_resolves_to_one_pipeline_with_the_key_apart_from_the_script() {
    let links = JiraLookup::over(settings());
    let call = links
        .resolve("please do https://acme.atlassian.net/browse/ABC-90?focusedId=1 today")
        .expect("a call");
    assert_eq!(call.program(), "sh");
    let args = call.args();
    assert_eq!(args[0], "-c");
    assert!(args[1].contains("acli jira workitem view \"$1\""), "{}", args[1]);
    assert_eq!(args[2], "sh");
    assert_eq!(args[3], "ABC-90");
    assert!(args[4].contains("issues"), "the filter is its own argument");
    assert_eq!(args[5], "summary,description");
    assert!(!args[1].contains("ABC-90"), "the key is never in the script");
}

#[test]
fn a_link_with_no_scheme_resolves_and_the_address_is_the_canonical_one() {
    let links = JiraLookup::over(settings());
    let issue = links
        .issue("acme.atlassian.net/browse/ABC-90#comment")
        .expect("an issue");
    assert_eq!(issue.reference(), "ABC-90");
    assert_eq!(issue.url(), "https://acme.atlassian.net/browse/ABC-90");
    let edited = links.edited(&issue).expect("a call");
    assert_eq!(edited.args()[3], "ABC-90");
    assert_eq!(edited.args()[5], "updated");
}

#[test]
fn only_a_browse_link_on_this_site_is_recognised() {
    let links = JiraLookup::over(settings());
    for text in [
        "fix the flaky test in checkout",
        "https://other.atlassian.net/browse/ABC-90",
        "https://acme.atlassian.net/jira/software/projects/ABC/boards/1",
        "https://acme.atlassian.net/browse/lowercase-1",
        "https://github.com/o/r/issues/90",
        "#90",
    ] {
        assert!(links.resolve(text).is_none(), "{text}");
        assert!(links.issue(text).is_none(), "{text}");
    }
}

#[test]
fn a_link_into_another_project_on_the_same_site_names_its_own_key() {
    let links = JiraLookup::over(settings());
    let issue = links
        .issue("https://acme.atlassian.net/browse/OTHER-4")
        .expect("an issue");
    assert_eq!(issue.reference(), "OTHER-4");
}

// ------------------------------------------------------------------ the filters

/// The real `jq` over some JSON.
pub(super) fn through_jq(filter: &str, json: &str) -> String {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let spawned = Command::new("jq")
        .args(["-r", filter])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = spawned.expect("jq is needed to run these cases: brew install jq");
    child
        .stdin
        .take()
        .expect("piped")
        .write_all(json.as_bytes())
        .expect("fed");
    let out = child.wait_with_output().expect("ran");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

const ONE: &str = r#"{"key":"ABC-12","fields":{"summary":"Reader drops a row","labels":["web","armada-in-progress"],
  "assignee":{"displayName":"Pat Doe","emailAddress":"n@example.com"},
  "updated":"2026-10-14T10:11:12.123+0200",
  "description":{"type":"doc","version":1,"content":[
    {"type":"paragraph","content":[{"type":"text","text":"First"},{"type":"text","text":" line","marks":[{"type":"strong"}]}]},
    {"type":"paragraph","content":[{"type":"text","text":"Second\tone"}]}]}}}"#;

const BARE: &str = r#"{"key":"ABC-13","fields":{"summary":"Bare","labels":[],"assignee":null,"updated":"2026-10-14T09:00:00.000+0000","description":null}}"#;

const FLAT: &str = r#"{"issues":[
 {"key":"ABC-14","summary":"Flat shape","labels":[{"name":"api"}],"assignee":"Sam","updated":"2026-10-14T09:00:00Z","description":"Plain   text"}
]}"#;

#[test]
fn the_listing_filter_reads_the_nested_shape() {
    let nested = format!("[{ONE},{BARE}]");
    let printed = through_jq(&jira::listing_filter(240), &nested);
    let it = settings();
    let issues: Vec<_> = printed.lines().filter_map(|line| issue_of_line(&it, line)).collect();
    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].reference, "ABC-12");
    assert_eq!(issues[0].assignees[0].as_written(), "Pat Doe");
    assert_eq!(issues[0].labels.len(), 2);
    assert_eq!(
        issues[0].updated_at, "2026-10-14T10:11:12.123+02:00",
        "an offset without a colon is not RFC 3339"
    );
    assert_eq!(issues[0].excerpt.as_written(), "First line Second one");
    assert!(issues[1].assignees.is_empty());
    assert!(issues[1].excerpt.as_written().is_empty());
}

#[test]
fn the_listing_filter_reads_the_flat_shape_too() {
    let printed = through_jq(&jira::listing_filter(240), FLAT);
    let issue = issue_of_line(&settings(), printed.lines().next().expect("a line")).expect("an issue");
    assert_eq!(issue.reference, "ABC-14");
    assert_eq!(issue.labels[0].as_written(), "api");
    assert_eq!(issue.assignees[0].as_written(), "Sam");
    assert_eq!(issue.excerpt.as_written(), "Plain text");
}

#[test]
fn the_reading_filter_says_when_an_issue_is_done() {
    let it = settings();
    let done = r#"{"key":"ABC-7","fields":{"summary":"S","status":{"name":"Done","statusCategory":{"key":"done"}},"resolution":{"name":"Done"}}}"#;
    let open = r#"{"key":"ABC-8","fields":{"summary":"S","status":{"name":"In Progress","statusCategory":{"key":"indeterminate"}},"resolution":null}}"#;
    let flat = r#"{"key":"ABC-9","summary":"S","status":"Closed"}"#;
    for (json, closed) in [(done, true), (open, false), (flat, true)] {
        let printed = through_jq(&jira::reading_filter(), json);
        let read = read_issue_of_line(&it, printed.lines().next().expect("a line")).expect("an issue");
        assert_eq!(read.closed, closed, "{json}");
    }
}

#[test]
fn the_other_filters_print_a_briefing_and_an_instant() {
    let text = through_jq(&jira::text_filter(), ONE);
    assert_eq!(text, "Reader drops a row\n\nFirst line\nSecond\tone\n\n");
    let edited = through_jq(&jira::edited_filter(), ONE);
    assert_eq!(edited.trim(), "2026-10-14T10:11:12.123+02:00");
}

// ------------------------------------------------------------------ the choice

fn chose(provider: Option<&str>, pairs: &[(&str, &str)]) -> TrackerChoice {
    TrackerChoice::named(provider, &block(pairs))
}

fn refusal_of(choice: &TrackerChoice) -> String {
    TrackersByProvider
        .tracker(choice)
        .open_issues("/repo")
        .expect_err("refused")
        .said
}

#[test]
fn a_manifest_that_names_no_tracker_gets_the_forges_own_and_so_does_the_word_for_it() {
    for choice in [TrackerChoice::of_the_forge(), chose(Some("github"), &[])] {
        assert!(
            TrackersByProvider
                .links(&choice)
                .resolve("https://github.com/o/r/issues/1")
                .is_some(),
            "the forge's link reader"
        );
    }
}

#[test]
fn jira_gets_its_own_link_reader_and_tracker() {
    let choice = chose(Some("jira"), &[("site", "acme"), ("project", "ABC")]);
    let links = TrackersByProvider.links(&choice);
    assert!(links.resolve("https://acme.atlassian.net/browse/ABC-1").is_some());
    assert!(links.resolve("https://github.com/o/r/issues/1").is_none());
    assert!(TrackersByProvider.tracker(&choice).read_issue("/repo", "text that is no key").is_ok());
}

#[test]
fn a_choice_that_cannot_be_honoured_answers_with_a_tracker_that_says_why() {
    let unknown = refusal_of(&chose(Some("trello"), &[]));
    assert!(unknown.contains("`trello`") && unknown.contains("jira"), "{unknown}");
    let incomplete = refusal_of(&chose(Some("jira"), &[("site", "acme")]));
    assert!(incomplete.contains("`project` or `jql`"), "{incomplete}");
    let orphan = refusal_of(&chose(None, &[("project", "ABC")]));
    assert!(orphan.contains("no `provider`"), "{orphan}");
    let stray = refusal_of(&chose(Some("github"), &[("project", "ABC")]));
    assert!(stray.contains("takes no keys"), "{stray}");
}

#[test]
fn a_refused_choice_resolves_no_link_and_refuses_every_read() {
    let choice = chose(Some("trello"), &[]);
    assert!(TrackersByProvider.links(&choice).resolve("https://acme.atlassian.net/browse/ABC-1").is_none());
    let tracker = TrackersByProvider.tracker(&choice);
    assert!(tracker.open_issues("/repo").is_err());
    assert!(tracker.read_issue("/repo", "ABC-1").is_err());
}
