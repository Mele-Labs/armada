//! A tracker taught by a mod's `tracker.toml`: what the file may say, and the same three acts
//! as any tracker over a command line that is a script in a scratch folder, with `jq` real.
//! The HTTP half and the credential are `taught_tracker_http`.

use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use adapter_traits::{FromOutside, IssueField, LinkedPullRequestState, Tracker, TrackerChoice};

use super::scratch::Scratch;
use crate::taught_tracker::{self, TaughtTracker, TaughtTrackers, TrackerFile};
use crate::tracker_providers::tracker_over;

/// An executable script at `name` in `dir`.
pub(super) fn script(dir: &Scratch, name: &str, body: &str) -> String {
    let path = dir.file(name, &format!("#!/bin/sh\n{body}\n"));
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("executable");
    path.to_string_lossy().into_owned()
}

const SEARCHED: &str = r#"{"data":{"issues":[
 {"identifier":"LIN-1","title":"Fix the thing","url":"https://t.example/LIN-1","state":{"name":"Todo","type":"unstarted"},
  "priority":"High","assignee":{"name":"sam"},"labels":["web","bug"],"updatedAt":"2026-10-20T10:00:00Z","description":"First line\nsecond"},
 {"identifier":"LIN-2","title":"Done one","url":"https://t.example/LIN-2","state":{"name":"Done","type":"completed"},
  "priority":null,"assignee":null,"labels":[],"updatedAt":"2026-10-21T10:00:00Z","description":""}
]}}"#;

const ONE: &str = r#"{"identifier":"LIN-1","title":"Fix the thing","url":"https://t.example/LIN-1","state":{"type":"started"},
 "description":"The whole body","creator":"pat","createdAt":"2026-10-01T00:00:00Z",
 "comments":[{"user":"sam","createdAt":"2026-10-02T00:00:00Z","body":"On it"}],
 "pulls":[{"number":12,"title":"Fix it","url":"https://t.example/pr/12","state":"merged"}]}"#;

pub(super) const SEARCH_MAP: &str = r#".data.issues | map({reference: .identifier, title, url,
  closed: (.state.type == "completed"), assignees: [.assignee.name?], labels, updated_at: .updatedAt,
  body: .description, fields: [{name: "Status", values: [.state.name]}, {name: "Priority", values: [.priority]}]})"#;

pub(super) const READ_MAP: &str = r#"if . == null then null else {reference: .identifier, title, url,
  closed: (.state.type == "completed"), body: .description, author: .creator, opened_at: .createdAt,
  comments: [.comments[] | {author: .user, at: .createdAt, body}], linked_pull_requests: .pulls,
  fields: [{name: "Status", values: [.state.type]}]} end"#;

pub(super) const GUIDE: &str = r#"[guide]
language = "Linear filter"
guide = "Write `field:value` pairs."
open = "state:open"
assigned_to_me = "state:open assignee:me"
fields = ["Status", "Priority"]
"#;

/// A fake tracker CLI: answers `search` and `show LIN-1`, and writes the arguments it was given,
/// one a line, to `asked`.
fn fake_cli(dir: &Scratch) -> String {
    dir.file("search.json", SEARCHED);
    dir.file("one.json", ONE);
    let at = dir.path().display();
    script(
        dir,
        "fake-tracker",
        &format!(
            "printf '%s\\n' \"$@\" > {at}/asked\ncase \"$1\" in\n  search) cat {at}/search.json ;;\n  \
             show) if [ \"$2\" = LIN-1 ]; then cat {at}/one.json; else echo null; fi ;;\n  \
             *) echo \"no such act $1\" >&2; exit 2 ;;\nesac"
        ),
    )
}

fn cli_file(program: &str, search_map: &str) -> String {
    format!(
        "{GUIDE}\n[search]\ntransport = \"cli\"\nargv = [\"{program}\", \"search\", \"--filter\", \"{{query}}\"]\n\
         map = '''{search_map}'''\n\n[read]\ntransport = \"cli\"\nargv = [\"{program}\", \"show\", \"{{reference}}\"]\n\
         map = '''{READ_MAP}'''\n"
    )
}

fn field(name: &str, values: &[&str]) -> IssueField {
    IssueField { name: name.to_string(), values: values.iter().map(|one| FromOutside::verbatim(*one)).collect() }
}

fn tracker_of(text: &str) -> TaughtTracker {
    TaughtTracker::over(taught_tracker::read(text).unwrap_or_else(|problems| panic!("{problems:?}")))
}

// ------------------------------------------------------------------ the file

fn a_good_file() -> String {
    cli_file("/usr/bin/true", SEARCH_MAP)
}

#[test]
fn a_tracker_file_with_each_section_reads_and_says_its_guide() {
    let file = taught_tracker::read(&a_good_file()).expect("it reads");
    assert_eq!(file.guide().language, "Linear filter");
    assert_eq!(file.guide().open, "state:open");
    assert_eq!(file.guide().fields, ["Status", "Priority"]);
    assert!(!file.lists_apart());
}

#[test]
fn each_thing_a_tracker_file_may_not_say_is_refused_with_its_reason() {
    let good = a_good_file();
    let cases: [(String, &str); 7] = [
        (good.replacen("map = '''", "mapp = '''", 1), "`[search]` has no `map`"),
        (good.replacen("transport = \"cli\"", "transport = \"cli\"\nshell = true", 1), "`[search]` has `shell`, which a cli call does not take"),
        (good.replacen("transport = \"cli\"", "transport = \"grpc\"", 1), "`[search]` transport `grpc` is not one this build has: cli, http, mcp"),
        (good.replacen("{reference}", "{query}", 1), "`[read]` uses `{query}`, which only `[search]` can fill"),
        (good.replacen("\"--filter\"", "\"{credential}\"", 1), "`[search]` uses `{credential}`, and there is no `[credential]` to fill it"),
        (format!("{good}\n[comments]\nx = 1\n"), "`[comments]` is not a section a tracker reads"),
        (good.replacen("\"{query}\"", "\"{querry}\"", 1), "`[search]` never uses `{query}`"),
    ];
    for (text, reason) in cases {
        let problems = taught_tracker::problems(&text);
        assert!(problems.iter().any(|one| one.contains(reason)), "wanted {reason:?} in {problems:?}");
    }
    let http = format!(
        "{GUIDE}\n[search]\ntransport = \"http\"\nurl = \"http://tracker.example/q?f={{query}}\"\nmap = \".\"\n\
         [read]\ntransport = \"http\"\nurl = \"https://{{reference}}.example/\"\nmap = \".\"\n"
    );
    let problems = taught_tracker::problems(&http);
    assert!(problems.iter().any(|one| one.contains("must be `https://`")), "{problems:?}");
    assert!(problems.iter().any(|one| one.contains("must name its host as written")), "{problems:?}");
}

// ------------------------------------------------------------------ over a command line

#[test]
fn a_taught_tracker_searches_over_its_command_line_with_fields_mapped() {
    let dir = Scratch::new();
    let tracker = tracker_of(&cli_file(&fake_cli(&dir), SEARCH_MAP));
    let query = "label:web; echo $(whoami)";
    let found = tracker.search_issues("/repo", query).expect("searched");
    let asked = std::fs::read_to_string(dir.path().join("asked")).expect("the CLI was run");
    assert_eq!(asked, format!("search\n--filter\n{query}\n"), "the query is one argument, never through a shell");
    assert_eq!(found.iter().map(|one| one.reference.as_str()).collect::<Vec<_>>(), ["LIN-2", "LIN-1"], "newest edit first");
    let one = &found[1];
    assert_eq!(one.title.as_written(), "Fix the thing");
    assert_eq!(one.url.as_written(), "https://t.example/LIN-1");
    assert!(!one.closed && found[0].closed);
    assert_eq!(one.assignees, [FromOutside::verbatim("sam")]);
    assert_eq!(one.labels, [FromOutside::verbatim("web"), FromOutside::verbatim("bug")]);
    assert_eq!(one.excerpt.as_written(), "First line second");
    assert_eq!(one.fields, [field("Status", &["Todo"]), field("Priority", &["High"])]);
    assert_eq!(found[0].fields, [field("Status", &["Done"])], "a field with no value is left out");
}

#[test]
fn a_taught_tracker_lists_by_searching_its_open_query_and_keeps_the_open_ones() {
    let dir = Scratch::new();
    let tracker = tracker_of(&cli_file(&fake_cli(&dir), SEARCH_MAP));
    let open = tracker.open_issues("/repo").expect("listed");
    let asked = std::fs::read_to_string(dir.path().join("asked")).expect("the CLI was run");
    assert_eq!(asked, "search\n--filter\nstate:open\n");
    assert_eq!(open.iter().map(|one| one.reference.as_str()).collect::<Vec<_>>(), ["LIN-1"]);
}

#[test]
fn a_taught_tracker_reads_one_issue_whole_and_none_where_its_map_yields_null() {
    let dir = Scratch::new();
    let tracker = tracker_of(&cli_file(&fake_cli(&dir), SEARCH_MAP));
    let read = tracker.read_issue("/repo", "LIN-1").expect("read").expect("an issue");
    assert_eq!(read.reference, "LIN-1");
    assert_eq!(read.body.as_written(), "The whole body");
    assert_eq!(read.author.as_written(), "pat");
    assert_eq!(read.opened_at, "2026-10-01T00:00:00Z");
    assert_eq!(read.comments.len(), 1);
    assert_eq!((read.comments[0].author.as_written(), read.comments[0].body.as_written()), ("sam", "On it"));
    assert_eq!(read.linked_pull_requests.len(), 1);
    assert_eq!(read.linked_pull_requests[0].number, 12);
    assert_eq!(read.linked_pull_requests[0].state, LinkedPullRequestState::Merged);
    assert_eq!(read.fields, [field("Status", &["started"])]);
    assert_eq!(tracker.read_issue("/repo", "LIN-9").expect("asked"), None);
}

#[test]
fn a_row_missing_its_title_fails_the_call_and_names_it() {
    let dir = Scratch::new();
    let without_title = SEARCH_MAP.replacen("title, url", "url", 1);
    let tracker = tracker_of(&cli_file(&fake_cli(&dir), &without_title));
    let refused = tracker.search_issues("/repo", "x").expect_err("a row has no title");
    assert_eq!(refused.doing, "searching the issues");
    assert_eq!(refused.said, "row 1 has no `title`");
}

#[test]
fn a_program_that_is_not_there_is_said_by_name() {
    let tracker = tracker_of(&cli_file("/nowhere/tracker-cli", SEARCH_MAP));
    let refused = tracker.search_issues("/repo", "x").expect_err("no program");
    assert!(refused.said.contains("`/nowhere/tracker-cli` is not installed"), "{}", refused.said);
}

// ------------------------------------------------------------------ choosing it

/// Tracker mods by name: one tried, one not.
struct Planted {
    tried: TrackerFile,
}

impl TaughtTrackers for Planted {
    fn taught(&self, name: &str) -> Option<Result<TrackerFile, String>> {
        match name {
            "linear" => Some(Ok(self.tried.clone())),
            "draft" => Some(Err("the tracker mod `draft` has not passed a try since it was last edited".to_string())),
            _ => None,
        }
    }
}

fn choosing(provider: &str) -> TrackerChoice {
    TrackerChoice::named(Some(provider), &Default::default())
}

fn planted() -> Arc<dyn TaughtTrackers> {
    Arc::new(Planted { tried: taught_tracker::read(&a_good_file()).expect("it reads") })
}

#[test]
fn a_manifest_naming_a_tried_tracker_mod_uses_it_and_an_untried_or_unknown_one_is_refused() {
    let mods = planted();
    let tried = tracker_over(&choosing("linear"), None, Some(&mods));
    assert_eq!(tried.query_guide().language, "Linear filter");

    let untried = tracker_over(&choosing("draft"), None, Some(&mods)).open_issues("/repo").expect_err("not tried");
    assert_eq!(untried.doing, "listing the open issues");
    assert!(untried.said.contains("has not passed a try"), "{}", untried.said);

    let unknown = tracker_over(&choosing("trello"), None, Some(&mods)).open_issues("/repo").expect_err("unknown");
    assert!(unknown.said.starts_with("no tracker is called `trello`"), "{}", unknown.said);
    let without_mods = tracker_over(&choosing("trello"), None, None).open_issues("/repo").expect_err("unknown");
    assert!(without_mods.said.starts_with("no tracker is called `trello`"), "{}", without_mods.said);
}
