//! A taught tracker over an MCP server on stdio: a fake server that is a script in a scratch folder
//! speaking JSON-RPC, as the command-line tests use a fake program, with `jq` real.

use std::time::{Duration, Instant};

use adapter_traits::{FromOutside, IssueField, Tracker};

use super::scratch::Scratch;
use super::taught_tracker::{script, GUIDE, READ_MAP, SEARCH_MAP};
use crate::taught_tracker::{self, TaughtTracker};

const SECRET: &str = "tok-s3cret/+";

const INITIALIZED: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"fake","version":"1"}}}"#;

fn result_line(result: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":2,"result":{result}}}"#)
}

/// A result that carries its answer as `structuredContent`, on the one line a message is.
fn structured(answer: &str) -> String {
    let answer = answer.replace('\n', " ");
    result_line(&format!(r#"{{"content":[{{"type":"text","text":"ok"}}],"structuredContent":{answer}}}"#))
}

/// A result that carries its answer as JSON in a text part, as most servers do.
fn text(answer: &str) -> String {
    let quoted = ipc::encode(&answer).expect("a string");
    result_line(&format!(r#"{{"content":[{{"type":"text","text":{quoted}}}]}}"#))
}

const FOUND: &str = r#"{"data":{"issues":[
 {"identifier":"M-1","title":"Fix the thing","url":"https://t.example/M-1","state":{"name":"Todo","type":"unstarted"},
  "priority":"High","assignee":{"name":"sam"},"labels":["web","bug"],"updatedAt":"2026-10-20T10:00:00Z","description":"First line\nsecond"},
 {"identifier":"M-2","title":"Done one","url":"https://t.example/M-2","state":{"name":"Done","type":"completed"},
  "priority":null,"assignee":null,"labels":[],"updatedAt":"2026-10-21T10:00:00Z","description":""}
]}}"#;

const ONE: &str = r#"{"identifier":"M-1","title":"Fix the thing","url":"https://t.example/M-1","state":{"type":"started"},
 "description":"The whole body","creator":"pat","createdAt":"2026-10-01T00:00:00Z",
 "comments":[{"user":"sam","createdAt":"2026-10-02T00:00:00Z","body":"On it"}],"pulls":[]}"#;

/// A fake server. It answers `initialize`, takes the notification and the call, keeps what it was
/// sent and the `TRACKER_TOKEN` it was started with in the scratch folder, and then runs `answer`.
fn fake_server(dir: &Scratch, answer: &str) -> String {
    let at = dir.path().display();
    script(
        dir,
        "fake-mcp",
        &format!(
            "read -r hello\nprintf '%s\\n' '{INITIALIZED}'\nread -r inited\nread -r call\n\
             printf '%s\\n' \"$hello\" > {at}/hello\nprintf '%s\\n' \"$inited\" > {at}/inited\n\
             printf '%s\\n' \"$call\" > {at}/call\nprintf '%s' \"$TRACKER_TOKEN\" > {at}/token\n{answer}"
        ),
    )
}

/// The answers the three tools give, by the id the call asks for.
fn answering(dir: &Scratch) -> String {
    let at = dir.path().display();
    dir.file("find.json", &structured(FOUND));
    dir.file("open.json", &text(FOUND));
    dir.file("get.json", &structured(ONE));
    dir.file("none.json", &text("null"));
    fake_server(
        dir,
        &format!(
            "echo 'starting up, not JSON'\necho '{{\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\",\"params\":{{}}}}'\n\
             case \"$call\" in\n  *'\"name\":\"find\"'*) cat {at}/find.json ;;\n  *'\"name\":\"open\"'*) cat {at}/open.json ;;\n  \
             *'\"id\":\"M-1\"'*) cat {at}/get.json ;;\n  *) cat {at}/none.json ;;\nesac"
        ),
    )
}

fn mcp_file(server: &str) -> String {
    format!(
        "{GUIDE}\n[search]\ntransport = \"mcp\"\nserver = [\"{server}\"]\ntool = \"find\"\n\
         arguments = {{ filter = \"{{query}}\", limit = 10 }}\nmap = '''{SEARCH_MAP}'''\n\n\
         [list]\ntransport = \"mcp\"\nserver = [\"{server}\"]\ntool = \"open\"\nmap = '''{SEARCH_MAP}'''\n\n\
         [read]\ntransport = \"mcp\"\nserver = [\"{server}\"]\ntool = \"get\"\n\
         arguments = {{ id = \"{{reference}}\" }}\nmap = '''{READ_MAP}'''\n"
    )
}

fn tracker_of(text: &str) -> TaughtTracker {
    TaughtTracker::over(taught_tracker::read(text).unwrap_or_else(|problems| panic!("{problems:?}")))
}

fn field(name: &str, values: &[&str]) -> IssueField {
    IssueField { name: name.to_string(), values: values.iter().map(|one| FromOutside::verbatim(*one)).collect() }
}

fn kept(dir: &Scratch, name: &str) -> String {
    std::fs::read_to_string(dir.path().join(name)).unwrap_or_else(|why| panic!("the server did not keep {name}: {why}"))
}

// ------------------------------------------------------------------ the three acts

#[test]
fn a_taught_tracker_searches_over_an_mcp_server_with_fields_mapped_from_structured_content() {
    let dir = Scratch::new();
    let tracker = tracker_of(&mcp_file(&answering(&dir)));
    let query = "label:web; echo $(whoami) \"q\"";
    let found = tracker.search_issues("/repo", query).expect("searched");

    assert!(kept(&dir, "hello").contains(r#""method":"initialize""#), "{}", kept(&dir, "hello"));
    assert!(kept(&dir, "inited").contains(r#""method":"notifications/initialized""#), "{}", kept(&dir, "inited"));
    assert_eq!(
        kept(&dir, "call"),
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"find\",\"arguments\":\
         {\"filter\":\"label:web; echo $(whoami) \\\"q\\\"\",\"limit\":10}}}\n",
        "the query is one JSON string, escaped"
    );
    assert_eq!(found.iter().map(|one| one.reference.as_str()).collect::<Vec<_>>(), ["M-2", "M-1"], "newest edit first");
    let one = &found[1];
    assert_eq!(one.title.as_written(), "Fix the thing");
    assert_eq!(one.assignees, [FromOutside::verbatim("sam")]);
    assert_eq!(one.labels, [FromOutside::verbatim("web"), FromOutside::verbatim("bug")]);
    assert_eq!(one.excerpt.as_written(), "First line second");
    assert_eq!(one.fields, [field("Status", &["Todo"]), field("Priority", &["High"])]);
    assert!(found[0].closed && !one.closed);
}

#[test]
fn a_taught_tracker_lists_over_an_mcp_server_from_json_in_a_text_part_and_keeps_the_open_ones() {
    let dir = Scratch::new();
    let tracker = tracker_of(&mcp_file(&answering(&dir)));
    let open = tracker.open_issues("/repo").expect("listed");
    assert!(kept(&dir, "call").contains(r#""name":"open","arguments":{}"#), "{}", kept(&dir, "call"));
    assert_eq!(open.iter().map(|one| one.reference.as_str()).collect::<Vec<_>>(), ["M-1"]);
    assert_eq!(open[0].fields, [field("Status", &["Todo"]), field("Priority", &["High"])]);
}

#[test]
fn a_taught_tracker_reads_one_issue_over_an_mcp_server_and_none_where_it_answers_null() {
    let dir = Scratch::new();
    let tracker = tracker_of(&mcp_file(&answering(&dir)));
    let read = tracker.read_issue("/repo", "M-1").expect("read").expect("an issue");
    assert!(kept(&dir, "call").contains(r#""arguments":{"id":"M-1"}"#), "{}", kept(&dir, "call"));
    assert_eq!(read.body.as_written(), "The whole body");
    assert_eq!(read.author.as_written(), "pat");
    assert_eq!((read.comments[0].author.as_written(), read.comments[0].body.as_written()), ("sam", "On it"));
    assert_eq!(read.fields, [field("Status", &["started"])]);
    assert_eq!(tracker.read_issue("/repo", "M-9").expect("asked"), None);
}

// ------------------------------------------------------------------ the credential

fn with_credential(file: &str, dir: &Scratch) -> String {
    let secret = script(dir, "secret", &format!("printf '%s\\n' '{SECRET}'"));
    let file = file.replacen("tool = \"find\"", "tool = \"find\"\nenv = { TRACKER_TOKEN = \"{credential}\" }", 1);
    format!("[credential]\nargv = [\"{secret}\"]\n\n{file}")
}

#[test]
fn the_credential_reaches_an_mcp_server_through_its_environment() {
    let dir = Scratch::new();
    let file = with_credential(&mcp_file(&answering(&dir)), &dir);
    let tracker = tracker_of(&file);
    tracker.search_issues("/repo", "x").expect("searched");
    assert_eq!(kept(&dir, "token"), SECRET, "the server was started with it");
    assert!(!kept(&dir, "call").contains(SECRET), "and it was never in the call");
}

#[test]
fn the_credential_never_reaches_what_a_refusal_says_over_mcp() {
    let dir = Scratch::new();
    let leaking = fake_server(
        &dir,
        "printf '%s\\n' \"{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":2,\\\"result\\\":{\\\"isError\\\":true,\\\"content\\\":\
         [{\\\"type\\\":\\\"text\\\",\\\"text\\\":\\\"token $TRACKER_TOKEN is revoked\\\"}]}}\"",
    );
    let refused = tracker_of(&with_credential(&mcp_file(&leaking), &dir)).search_issues("/repo", "x").expect_err("refused");
    assert!(!refused.said.contains(SECRET), "{}", refused.said);
    assert!(refused.said.contains("token [credential] is revoked"), "{}", refused.said);

    let dying = fake_server(&dir, "echo \"bad token: $TRACKER_TOKEN\" >&2\nexit 1");
    let refused = tracker_of(&with_credential(&mcp_file(&dying), &dir)).search_issues("/repo", "x").expect_err("refused");
    assert!(!refused.said.contains(SECRET), "{}", refused.said);
    assert!(refused.said.contains("bad token: [credential]"), "{}", refused.said);
}

// ------------------------------------------------------------------ what a server can say

#[test]
fn an_mcp_result_marked_as_an_error_fails_the_call_with_its_text() {
    let dir = Scratch::new();
    let failing = fake_server(
        &dir,
        &format!(
            "printf '%s\\n' '{}'",
            result_line(r#"{"isError":true,"content":[{"type":"text","text":"rate limited"},{"type":"text","text":"try later"}]}"#)
        ),
    );
    let refused = tracker_of(&mcp_file(&failing)).search_issues("/repo", "x").expect_err("the tool failed");
    assert_eq!(refused.doing, "searching the issues");
    assert_eq!(refused.said, "the tool reported an error: rate limited\ntry later");
}

#[test]
fn an_mcp_answer_that_cannot_be_read_fails_the_call_with_the_reason() {
    let dir = Scratch::new();
    let cases = [
        (result_line(r#"{"content":[{"type":"image","data":"AAAA"}]}"#), "has neither structured content nor a text part"),
        (result_line(r#"{"content":[{"type":"text","text":"3 issues found"}]}"#), "its text is not JSON"),
        (
            r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32602,"message":"unknown filter"}}"#.to_string(),
            "the server refused: unknown filter",
        ),
    ];
    for (line, reason) in cases {
        let server = fake_server(&dir, &format!("printf '%s\\n' '{line}'"));
        let refused = tracker_of(&mcp_file(&server)).search_issues("/repo", "x").expect_err("unreadable");
        assert!(refused.said.contains(reason), "wanted {reason:?} in {:?}", refused.said);
    }
}

#[test]
fn an_mcp_server_that_is_not_there_is_said_by_name() {
    let refused = tracker_of(&mcp_file("/nowhere/tracker-mcp")).search_issues("/repo", "x").expect_err("no program");
    assert!(refused.said.contains("`/nowhere/tracker-mcp` is not installed"), "{}", refused.said);
}

#[test]
fn an_mcp_server_that_never_answers_is_stopped_within_the_time_it_is_given() {
    let dir = Scratch::new();
    let silent = script(&dir, "silent", "read -r hello\nexec sleep 30");
    let tracker = tracker_of(&mcp_file(&silent)).within(Duration::from_millis(300));
    let started = Instant::now();
    let refused = tracker.search_issues("/repo", "x").expect_err("no answer");
    assert!(started.elapsed() < Duration::from_secs(10), "{:?}", started.elapsed());
    assert!(refused.said.contains("did not answer within 300ms"), "{}", refused.said);
}

#[test]
fn an_mcp_server_that_says_more_than_the_most_an_answer_may_weigh_is_stopped() {
    let dir = Scratch::new();
    let loud = script(&dir, "loud", "read -r hello\nhead -c 5000000 /dev/zero | tr '\\0' a");
    let refused = tracker_of(&mcp_file(&loud)).search_issues("/repo", "x").expect_err("too much");
    assert!(refused.said.contains("said more than 4194304 bytes"), "{}", refused.said);
}

// ------------------------------------------------------------------ the file

#[test]
fn each_thing_an_mcp_call_may_not_say_is_refused_with_its_reason() {
    let good = mcp_file("/usr/bin/true");
    taught_tracker::read(&good).expect("the good file reads");
    let server = "server = [\"/usr/bin/true\"]\n";
    let cases: Vec<(String, &str)> = vec![
        (good.replacen(server, "", 1), "`[search]` has no `server`"),
        (good.replacen("tool = \"find\"\n", "", 1), "`[search]` has no `tool`"),
        (good.replacen("tool = \"find\"", "tool = \"find\"\nargv = []", 1), "`[search]` has `argv`, which an mcp call does not take"),
        (good.replacen(server, "server = []\n", 1), "`[search]` `server` must start with the program to run"),
        (good.replacen(server, "server = \"npx\"\n", 1), "`[search]` `server` must be a list of strings"),
        (good.replacen("\"/usr/bin/true\"]", "\"/usr/bin/true\", \"{query}\"]", 1), "`[search]` holds a placeholder in its `server`"),
        (good.replacen("tool = \"find\"", "tool = \"{query}\"", 1), "`[search]` holds a placeholder in its `tool`"),
        (good.replacen("tool = \"find\"", "tool = \"find\"\nenv = { A = 1 }", 1), "`[search]` env `A` must be a string"),
        (good.replacen("tool = \"find\"", "tool = \"find\"\nenv = { \"A=B\" = \"x\" }", 1), "`[search]` env `A=B` is not a variable name"),
        (good.replacen("tool = \"find\"", "tool = \"find\"\nenv = [\"A\"]", 1), "`[search]` `env` must be a table of strings"),
        (good.replacen("arguments = { filter", "arguments = 3\nx = { filter", 1), "`[search]` `arguments` must be a table"),
        (good.replacen("limit = 10", "limit = 2026-10-01", 1), "`[search]` `arguments` holds a date"),
        (good.replacen("{ filter = \"{query}\"", "{ \"{query}\" = \"x\", filter = \"{query}\"", 1), "holds a placeholder in a name"),
        (good.replacen("\"{query}\"", "\"{reference}\"", 1), "`[search]` uses `{reference}`, which only `[read]` can fill"),
        (good.replacen("\"{query}\"", "\"{credential}\"", 1), "`[search]` uses `{credential}`, and there is no `[credential]` to fill it"),
        (good.replacen("\"{query}\"", "\"all\"", 1), "`[search]` never uses `{query}`"),
        (good.replacen("\"{reference}\"", "\"M-1\"", 1), "`[read]` never uses `{reference}`"),
    ];
    for (text, reason) in cases {
        let problems = taught_tracker::problems(&text);
        assert!(problems.iter().any(|one| one.contains(reason)), "wanted {reason:?} in {problems:?}");
    }
}
