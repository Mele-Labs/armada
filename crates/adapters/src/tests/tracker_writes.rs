//! The writes a saved rule makes to a tracker and a forge: the commands each adapter builds, and
//! that what an adapter has no call for is refused before anything is asked of the owner.
//!
//! **Nothing here reaches a real tracker or forge.** Arguments are built apart from the run and
//! read; `bd` and `acli` answer from a script, and a taught tracker's command is a script in a
//! scratch folder.

use std::collections::BTreeMap;

use adapter_traits::{IssueWrite, TrackerChoice, TrackerWrites, Trackers};

use super::jira::{settings, Scripted};
use super::scratch::Scratch;
use super::taught_tracker::{script, GUIDE, READ_MAP, SEARCH_MAP};
use crate::beads_tracker::{bd_arguments, BeadsTracker};
use crate::issue_writes::arguments as gh_arguments;
use crate::jira_tracker::{acli_arguments, JiraTracker};
use crate::pull_request_facts::reviewer_arguments;
use crate::taught_tracker::{self, TaughtTracker};
use crate::TrackersByProvider;

const MOVE: IssueWrite<'static> = IssueWrite::Move { status: "In Progress" };
const ASSIGN: IssueWrite<'static> = IssueWrite::Assign { to: "ana" };
const CLOSE: IssueWrite<'static> = IssueWrite::Close { status: None };

fn strings(all: &[&str]) -> Vec<String> {
    all.iter().map(|one| one.to_string()).collect()
}

fn choosing(provider: Option<&str>, keys: &[(&str, &str)]) -> TrackerChoice {
    let keys: BTreeMap<String, String> = keys.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
    TrackerChoice::named(provider, &keys)
}

// ------------------------------------------------------------------ the forge's own issues

#[test]
fn a_forges_issue_closes_reopens_and_assigns_and_moves_nowhere_else() {
    assert_eq!(gh_arguments("7", &IssueWrite::Move { status: "Closed" }), Ok(strings(&["issue", "close", "7"])));
    assert_eq!(gh_arguments("7", &IssueWrite::Move { status: "Open" }), Ok(strings(&["issue", "reopen", "7"])));
    assert_eq!(gh_arguments("7", &CLOSE), Ok(strings(&["issue", "close", "7"])));
    assert_eq!(gh_arguments("7", &ASSIGN), Ok(strings(&["issue", "edit", "7", "--add-assignee=ana"])));

    // A board's column is not something `gh issue` writes, and a different case is a different word.
    for status in ["In Progress", "open"] {
        let why = gh_arguments("7", &IssueWrite::Move { status }).expect_err("refused");
        assert!(why.contains("only between Open and Closed"), "{why}");
    }
    // Two people, or none, are not a login; one that begins with a dash is, and stays one argument.
    for who in ["", "ana, bo", "a b"] {
        assert!(gh_arguments("7", &IssueWrite::Assign { to: who }).is_err(), "{who:?}");
    }
    assert_eq!(gh_arguments("7", &IssueWrite::Assign { to: "--admin" }).expect("one argument")[3], "--add-assignee=--admin");
}

// ------------------------------------------------------------------ beads

#[test]
fn beads_updates_a_status_closes_with_a_reason_past_a_blocker_and_assigns() {
    assert_eq!(
        bd_arguments("armada-3fz", &IssueWrite::Move { status: "in_progress" }),
        Ok(strings(&["update", "armada-3fz", "--status", "in_progress"]))
    );
    let closes = Ok(strings(&["close", "armada-3fz", "--reason", "Closed by a rule", "--force"]));
    assert_eq!(bd_arguments("armada-3fz", &IssueWrite::Move { status: "closed" }), closes);
    assert_eq!(bd_arguments("armada-3fz", &CLOSE), closes);
    assert_eq!(bd_arguments("armada-3fz", &ASSIGN), Ok(strings(&["update", "armada-3fz", "--assignee", "ana"])));

    let why = bd_arguments("armada-3fz", &MOVE).expect_err("a word bd has no status for");
    assert!(why.contains("open, in_progress, blocked and closed"), "{why}");
    assert!(bd_arguments("armada-3fz", &IssueWrite::Assign { to: "-x" }).is_err());
}

#[test]
fn beads_runs_the_write_in_the_main_checkout_and_says_bds_own_words_when_it_refuses() {
    let bd = Scripted::saying(&[(0, "", ""), (1, "", "Error: no issue found matching armada-9")]);
    let tracker = BeadsTracker::with_runner(&bd);
    tracker.write("/repo", "armada-3fz", &IssueWrite::Move { status: "open" }).expect("moved");
    let calls = bd.calls();
    assert_eq!(calls[0].0, "bd");
    assert_eq!(&calls[0].1[2..], ["update", "armada-3fz", "--status", "open"], "after `-C <checkout>`");

    let refused = tracker.write("/repo", "armada-9", &CLOSE).expect_err("refused");
    assert_eq!((refused.doing, refused.said.as_str()), ("closing the issue", "Error: no issue found matching armada-9"));

    let before = bd.calls().len();
    assert!(tracker.write("/repo", "--force", &CLOSE).is_err(), "an id that is a flag never reaches bd");
    assert!(tracker.write("/repo", "armada-3fz", &MOVE).is_err(), "nor a status bd has no word for");
    assert_eq!(bd.calls().len(), before);
}

// ------------------------------------------------------------------ Jira

#[test]
fn jira_transitions_to_a_status_and_assigns_and_will_not_guess_what_closed_is_called() {
    let on = |verb: &str, flag: &str, value: &str| strings(&["jira", "workitem", verb, "--key", "ABC-7", flag, value, "--yes"]);
    assert_eq!(acli_arguments("ABC-7", &MOVE), Ok(on("transition", "--status", "In Progress")));
    assert_eq!(acli_arguments("ABC-7", &IssueWrite::Close { status: Some("Done") }), Ok(on("transition", "--status", "Done")));
    assert_eq!(acli_arguments("ABC-7", &ASSIGN), Ok(on("assign", "--assignee", "ana")));

    let why = acli_arguments("ABC-7", &CLOSE).expect_err("no word for closed");
    assert!(why.contains("names none for Closed"), "{why}");
    assert!(acli_arguments("ABC-7", &IssueWrite::Assign { to: "ana, bo" }).is_err());
    assert!(acli_arguments("ABC-7", &IssueWrite::Move { status: "--yes" }).is_err());
}

#[test]
fn jira_writes_only_to_a_key_and_says_aclis_own_words_when_it_refuses() {
    let acli = Scripted::saying(&[(0, "", ""), (1, "", "Transition not allowed")]);
    let tracker = JiraTracker::with_runner(settings(), &acli);
    tracker.write("/repo", "ABC-7", &MOVE).expect("moved");
    assert_eq!(acli.calls()[0].0, "acli");

    let refused = tracker.write("/repo", "ABC-7", &MOVE).expect_err("refused");
    assert_eq!(refused.said, "Transition not allowed");

    let before = acli.calls().len();
    assert!(tracker.write("/repo", "7", &MOVE).is_err(), "a number alone names no issue across projects");
    assert_eq!(acli.calls().len(), before);
}

// ------------------------------------------------------------------ the forge's reviewers

#[test]
fn reviewers_are_asked_in_one_argument_and_nobody_is_not_a_request() {
    assert_eq!(
        reviewer_arguments("812", &["ana", "bo", "acme/web"]),
        Ok(strings(&["pr", "edit", "812", "--add-reviewer=ana,bo,acme/web"]))
    );
    assert_eq!(reviewer_arguments("#812", &["ana"]).expect("a number")[2], "812");
    assert!(reviewer_arguments("812", &[]).is_err());
    assert!(reviewer_arguments("812", &["ana bo"]).is_err());
    assert!(reviewer_arguments("--web", &["ana"]).is_err());
    assert_eq!(reviewer_arguments("812", &["--x"]).expect("one argument")[3], "--add-reviewer=--x");
}

// ------------------------------------------------------------------ which writer a choice gets

#[test]
fn the_writer_follows_the_choice_and_a_choice_that_could_not_be_honoured_says_why() {
    let trackers = TrackersByProvider;
    let forge = trackers.writer(&TrackerChoice::of_the_forge());
    assert_eq!(forge.can(&CLOSE), Ok(()));
    assert!(forge.can(&MOVE).is_err(), "a board's column is not written");

    let jira = trackers.writer(&choosing(Some("jira"), &[("site", "acme"), ("project", "ABC")]));
    assert_eq!(jira.can(&MOVE), Ok(()));
    assert!(jira.can(&CLOSE).is_err());

    assert_eq!(trackers.writer(&choosing(Some("beads"), &[])).can(&ASSIGN), Ok(()));

    let refused = trackers.writer(&choosing(Some("jira"), &[("site", "acme"), ("fields", "x")]));
    let why = refused.can(&MOVE).expect_err("a block that could not be read");
    assert!(why.contains("takes no `fields`"), "the reason is the block's own: {why}");
    assert_eq!(refused.write("/repo", "ABC-1", &MOVE).expect_err("refused").said, why);

    let unknown = trackers.writer(&choosing(Some("nowhere"), &[]));
    assert!(unknown.can(&CLOSE).expect_err("no such tracker").contains("no tracker is called `nowhere`"));
}

#[test]
fn a_tracker_that_is_only_read_cannot_write_and_says_so_per_write() {
    struct Reads;
    impl Trackers for Reads {
        fn tracker(&self, _: &TrackerChoice) -> std::sync::Arc<dyn adapter_traits::Tracker + Send + Sync> {
            unreachable!("not read here")
        }
        fn links(&self, _: &TrackerChoice) -> std::sync::Arc<dyn adapter_traits::LinkLookup + Send + Sync> {
            unreachable!("not read here")
        }
    }
    let writer = Reads.writer(&TrackerChoice::of_the_forge());
    for (write, said) in [
        (MOVE, "this tracker cannot move an issue"),
        (ASSIGN, "this tracker cannot assign an issue"),
        (CLOSE, "this tracker cannot close an issue"),
    ] {
        assert_eq!(writer.can(&write), Err(said.to_string()));
        assert_eq!(writer.write("/repo", "1", &write).expect_err("refused").said, said);
    }
}

// ------------------------------------------------------------------ a tracker a mod taught

fn taught_with(extra: &str, dir: &Scratch) -> String {
    let at = dir.path().display();
    let cli = script(dir, "fake-tracker", &format!("printf '%s\\n' \"$@\" >> {at}/asked\nexit 0"));
    format!(
        "{GUIDE}\n[search]\ntransport = \"cli\"\nargv = [\"{cli}\", \"search\", \"{{query}}\"]\nmap = '''{SEARCH_MAP}'''\n\n\
         [read]\ntransport = \"cli\"\nargv = [\"{cli}\", \"show\", \"{{reference}}\"]\nmap = '''{READ_MAP}'''\n\n{extra}"
    )
}

fn moves_and_assigns(cli: &str) -> String {
    format!(
        "[move]\ntransport = \"cli\"\nargv = [\"{cli}\", \"move\", \"{{reference}}\", \"{{status}}\"]\n\n\
         [assign]\ntransport = \"cli\"\nargv = [\"{cli}\", \"assign\", \"{{reference}}\", \"{{assignee}}\"]\n"
    )
}

#[test]
fn a_mod_teaches_the_writes_it_has_and_a_write_it_lacks_is_not_one_it_can_make() {
    let dir = Scratch::new();
    let cli = script(&dir, "writes", "exit 0");
    let tracker = TaughtTracker::over(taught_tracker::read(&taught_with(&moves_and_assigns(&cli), &dir)).expect("reads"));
    assert_eq!(tracker.can(&MOVE), Ok(()));
    assert_eq!(tracker.can(&ASSIGN), Ok(()));
    assert_eq!(
        tracker.can(&CLOSE),
        Err("this tracker mod closes by a move, and the ticket lifecycle names no word for Closed".to_string())
    );
    assert_eq!(tracker.can(&IssueWrite::Close { status: Some("Done") }), Ok(()), "a close is a move to the word for Closed");

    let bare = TaughtTracker::over(taught_tracker::read(&taught_with("", &dir)).expect("reads"));
    for (write, verb) in [(MOVE, "move"), (ASSIGN, "assign"), (CLOSE, "close")] {
        let why = bare.can(&write).expect_err("taught nothing");
        assert!(why.contains(&format!("cannot {verb} an issue")), "{why}");
    }
}

#[test]
fn a_taught_write_is_one_call_with_its_placeholders_filled_and_no_shell() {
    let dir = Scratch::new();
    let cli = script(&dir, "writes", &format!("printf '%s\\n' \"$@\" > {}/asked\nexit 0", dir.path().display()));
    let tracker = TaughtTracker::over(taught_tracker::read(&taught_with(&moves_and_assigns(&cli), &dir)).expect("reads"));

    tracker.write("/repo", "LIN-1", &IssueWrite::Move { status: "In Review; echo $(whoami)" }).expect("moved");
    let asked = std::fs::read_to_string(dir.path().join("asked")).expect("the command ran");
    assert_eq!(asked, "move\nLIN-1\nIn Review; echo $(whoami)\n", "the status is one argument");

    tracker.write("/repo", "LIN-1", &ASSIGN).expect("assigned");
    assert_eq!(std::fs::read_to_string(dir.path().join("asked")).expect("ran"), "assign\nLIN-1\nana\n");

    tracker.write("/repo", "LIN-1", &IssueWrite::Close { status: Some("Done") }).expect("closed by a move");
    assert_eq!(std::fs::read_to_string(dir.path().join("asked")).expect("ran"), "move\nLIN-1\nDone\n");
}

#[test]
fn a_taught_write_that_fails_says_what_the_command_said() {
    let dir = Scratch::new();
    let cli = script(&dir, "writes", "echo 'no such transition' >&2\nexit 3");
    let tracker = TaughtTracker::over(taught_tracker::read(&taught_with(&moves_and_assigns(&cli), &dir)).expect("reads"));
    let refused = tracker.write("/repo", "LIN-1", &MOVE).expect_err("failed");
    assert_eq!((refused.doing, refused.said.as_str()), ("moving the issue", "no such transition"));
}

#[test]
fn a_write_section_is_checked_as_strictly_as_a_read_and_asks_nothing_a_read_does() {
    let dir = Scratch::new();
    let cli = script(&dir, "writes", "exit 0");
    let file = |extra: &str| taught_with(extra, &dir);
    let section = |name: &str, line: &str| format!("[{name}]\ntransport = \"cli\"\n{line}\n");
    let cases = [
        (section("move", &format!("argv = [\"{cli}\", \"{{reference}}\"]")), "`[move]` never uses `{status}`"),
        (section("assign", &format!("argv = [\"{cli}\", \"{{assignee}}\"]")), "`[assign]` never uses `{reference}`"),
        (section("close", &format!("argv = [\"{cli}\", \"{{reference}}\", \"{{status}}\"]")), "`[close]` uses `{status}`, which only `[move]` can fill"),
        (section("move", &format!("argv = [\"{cli}\", \"{{reference}}\", \"{{status}}\", \"{{query}}\"]")), "`[move]` uses `{query}`, which only `[search]` can fill"),
        (section("move", &format!("argv = [\"{cli}\", \"{{reference}}\", \"{{status}}\"]\nmap = \".\"")), "`[move]` has `map`, which a cli call does not take"),
    ];
    for (extra, reason) in cases {
        let problems = taught_tracker::problems(&file(&extra));
        assert!(problems.iter().any(|one| one.contains(reason)), "wanted {reason:?} in {problems:?}");
    }
    let problems = taught_tracker::problems(&format!("move = 3\n{}", file("")));
    assert!(problems.iter().any(|one| one.contains("`move` must be a section")), "{problems:?}");

    // A write over HTTP is a POST, PUT or PATCH, and a read's GET is not one.
    let http = |method: &str| {
        file(&format!(
            "[move]\ntransport = \"http\"\nmethod = \"{method}\"\nurl = \"https://t.example/{{reference}}\"\nbody = '{{\"s\":\"{{status}}\"}}'\n"
        ))
    };
    assert!(taught_tracker::problems(&http("PATCH")).is_empty(), "{:?}", taught_tracker::problems(&http("PATCH")));
    let problems = taught_tracker::problems(&http("GET"));
    assert!(problems.iter().any(|one| one.contains("`POST`, `PUT` or `PATCH`")), "{problems:?}");
}
