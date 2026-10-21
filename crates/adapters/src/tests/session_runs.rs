//! How a folder was run, derived from fixture lines of a session's record: no file but the one
//! case that proves the read is bounded, and no process.

use crate::session_runs::{derive, missing_commands, Reading, Record, RunFact, RunKind};

use super::scratch::Scratch;

const ROOT: &str = "/work/shop";

fn escaped(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

/// The agent's `Bash` call.
fn call(id: &str, cwd: &str, at: &str, command: &str) -> String {
    format!(
        "{{\"type\":\"assistant\",\"cwd\":\"{cwd}\",\"timestamp\":\"{at}\",\"message\":{{\"content\":[{{\"type\":\"tool_use\",\"id\":\"{id}\",\"name\":\"Bash\",\"input\":{{\"command\":\"{}\"}}}}]}}}}",
        escaped(command)
    )
}

fn background_call(id: &str, cwd: &str, at: &str, command: &str) -> String {
    call(id, cwd, at, command).replace("\"input\":{", "\"input\":{\"run_in_background\":true,")
}

/// What came back; `failed` is the harness's own flag.
fn result(id: &str, failed: bool, printed: &str) -> String {
    let flag = if failed { ",\"is_error\":true" } else { "" };
    format!(
        "{{\"type\":\"user\",\"message\":{{\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"{id}\"{flag},\"content\":\"{}\"}}]}}}}",
        escaped(printed)
    )
}

fn read(lines: &[String]) -> Vec<Record> {
    let mut reading = Reading::default();
    for line in lines {
        reading.feed(line.as_bytes(), 7);
    }
    reading.records
}

fn facts(lines: &[String]) -> Vec<RunFact> {
    derive(&read(lines), ROOT)
}

fn only(lines: &[String]) -> RunFact {
    let mut found = facts(lines);
    assert_eq!(found.len(), 1, "{found:?}");
    found.remove(0)
}

const T1: &str = "2026-10-01T10:00:00.000Z";
const T2: &str = "2026-10-02T10:00:00.000Z";
const T1_MS: u64 = 1_790_848_800_000;
const T2_MS: u64 = 1_790_935_200_000;

#[test]
fn a_test_run_that_passed_is_a_fact_with_the_time_it_ran() {
    let fact = only(&[call("1", ROOT, T1, "cargo test -p shop"), result("1", false, "test result: ok")]);

    assert_eq!(
        fact,
        RunFact {
            workspace: ".".into(),
            kind: RunKind::Tests,
            command: "cargo test -p shop".into(),
            port: None,
            succeeded: true,
            last_ran: T1_MS,
        }
    );
}

#[test]
fn a_command_that_only_ever_failed_is_left_out() {
    let failed_twice = [
        call("1", ROOT, T1, "pnpm test"),
        result("1", true, "Exit code 1"),
        call("2", ROOT, T2, "pnpm test"),
        result("2", false, "Exit code 2\nFAIL"),
    ];
    assert!(facts(&failed_twice).is_empty());

    let passed_once = [
        call("1", ROOT, T1, "pnpm test"),
        result("1", true, "Exit code 1"),
        call("2", ROOT, T2, "pnpm test"),
        result("2", false, "all passed"),
    ];
    assert!(only(&passed_once).succeeded);
}

#[test]
fn the_latest_run_decides_succeeded_and_the_time() {
    let fact = only(&[
        call("1", ROOT, T1, "npm run lint"),
        result("1", false, "ok"),
        call("2", ROOT, T2, "npm run lint"),
        result("2", true, "Exit code 1"),
    ]);

    assert_eq!((fact.kind, fact.succeeded, fact.last_ran), (RunKind::Lint, false, T2_MS));
}

#[test]
fn a_workspace_is_the_folder_below_the_root_from_the_line_or_a_cd() {
    let found = facts(&[
        call("1", "/work/shop/apps/web", T1, "pnpm test"),
        result("1", false, ""),
        call("2", ROOT, T1, "cd packages/ui && npm run build"),
        result("2", false, ""),
        call("3", "/work/shop/apps/web", T1, "cd ../api; npm run typecheck"),
        result("3", false, ""),
        call("4", "/elsewhere", T1, "cargo test"),
        result("4", false, ""),
        call("5", ROOT, T1, "make -C tools test"),
        result("5", false, ""),
    ]);

    let places: Vec<(String, RunKind)> = found.into_iter().map(|fact| (fact.workspace, fact.kind)).collect();
    assert_eq!(
        places,
        [
            ("apps/api".to_string(), RunKind::Typecheck),
            ("apps/web".to_string(), RunKind::Tests),
            ("packages/ui".to_string(), RunKind::Build),
            ("tools".to_string(), RunKind::Tests),
        ]
    );
}

#[test]
fn pipes_redirects_and_environment_are_not_part_of_the_command() {
    let fact = only(&[
        call("1", ROOT, T1, "CI=1 cargo test --workspace 2>&1 | tail -20"),
        result("1", false, ""),
    ]);

    assert_eq!(fact.command, "cargo test --workspace");
}

#[test]
fn one_failed_result_fails_every_command_of_its_line() {
    let lines = [call("1", ROOT, T1, "cargo fmt && cargo test"), result("1", true, "Exit code 101")];
    assert!(facts(&lines).is_empty());

    let worked = facts(&[call("1", ROOT, T1, "cargo fmt && cargo test"), result("1", false, "")]);
    assert_eq!(
        worked.iter().map(|fact| fact.kind).collect::<Vec<_>>(),
        [RunKind::Tests, RunKind::Format]
    );
}

#[test]
fn each_kind_is_named_by_plain_rules_on_the_command() {
    let cases = [
        ("cargo nextest run", RunKind::Tests),
        ("python -m pytest -q", RunKind::Tests),
        ("npx vitest run", RunKind::Tests),
        ("yarn test:unit", RunKind::Tests),
        ("go test ./...", RunKind::Tests),
        ("cargo clippy --all-targets", RunKind::Lint),
        ("pnpm lint", RunKind::Lint),
        ("ruff check .", RunKind::Lint),
        ("cargo check", RunKind::Typecheck),
        ("pnpm exec tsc --noEmit", RunKind::Typecheck),
        ("npm run typecheck", RunKind::Typecheck),
        ("uv run mypy src", RunKind::Typecheck),
        ("cargo build --release", RunKind::Build),
        ("npm run build", RunKind::Build),
        ("make", RunKind::Build),
        ("cargo fmt --check", RunKind::Format),
        ("prettier --write .", RunKind::Format),
        ("npm run dev", RunKind::DevServer),
        ("next dev", RunKind::DevServer),
    ];
    for (command, kind) in cases {
        let fact = only(&[call("1", ROOT, T1, command), result("1", false, "")]);
        assert_eq!(fact.kind, kind, "{command}");
    }
}

#[test]
fn commands_no_rule_names_are_left_out_and_so_is_a_call_never_answered() {
    let lines = [
        call("1", ROOT, T1, "ls -la"),
        result("1", false, ""),
        call("2", ROOT, T1, "git status"),
        result("2", false, ""),
        call("3", ROOT, T1, "cargo test"),
    ];

    assert!(facts(&lines).is_empty());
}

#[test]
fn a_server_is_one_if_it_stayed_up_or_said_where_it_listens() {
    let printed = only(&[
        call("1", ROOT, T1, "npm run dev"),
        result("1", false, "  VITE v5  ready\n  Local: http://localhost:5173/"),
    ]);
    assert_eq!((printed.kind, printed.port), (RunKind::DevServer, Some(5173)));

    let named = only(&[call("1", ROOT, T1, "PORT=4000 npm run dev"), result("1", false, "")]);
    assert_eq!(named.port, Some(4000));

    let flagged = only(&[call("1", ROOT, T1, "vite --port 3001"), result("1", false, "")]);
    assert_eq!(flagged.port, Some(3001));

    let stopped_by_hand = only(&[
        call("1", ROOT, T1, "npm run dev"),
        result("1", true, "Exit code 130\nLocal: http://127.0.0.1:5174/"),
    ]);
    assert!(stopped_by_hand.succeeded, "it listened before it was stopped");

    let backgrounded =
        only(&[background_call("1", ROOT, T1, "npm start"), result("1", false, "running in the background")]);
    assert_eq!((backgrounded.kind, backgrounded.port), (RunKind::DevServer, None));
}

#[test]
fn start_serve_and_cargo_run_are_not_servers_unless_they_stayed_up_or_said_where() {
    for command in ["npm start", "pnpm serve", "cargo run", "go run ."] {
        assert!(facts(&[call("1", ROOT, T1, command), result("1", false, "done")]).is_empty(), "{command}");
    }
    let said = only(&[call("1", ROOT, T1, "cargo run"), result("1", false, "listening on 0.0.0.0:8080")]);
    assert_eq!((said.kind, said.port), (RunKind::DevServer, Some(8080)));
}

#[test]
fn only_bash_calls_and_their_own_results_are_kept() {
    let other_tool = "{\"type\":\"assistant\",\"cwd\":\"/work/shop\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"9\",\"name\":\"Read\",\"input\":{\"file_path\":\"/x\"}}]}}".to_string();
    let its_result = result("9", true, "no such file");
    let records =
        read(&[other_tool, its_result, "not json at all tool_use".to_string(), call("1", ROOT, T1, "make test")]);

    assert_eq!(records.len(), 1);
    assert!(matches!(&records[0], Record::Call { id, .. } if id == "1"));
}

#[test]
fn a_result_in_parts_is_read_like_one_in_text() {
    let parts = "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"1\",\"content\":[{\"type\":\"text\",\"text\":\"Local: http://localhost:6006\"}]}]}}".to_string();

    let fact = only(&[call("1", ROOT, T1, "npm run storybook"), parts]);

    assert_eq!(fact.port, Some(6006));
}

#[test]
fn a_file_is_read_line_by_line_and_a_line_past_the_cap_is_skipped() {
    let scratch = Scratch::new();
    let huge = format!("{{\"type\":\"user\",\"pad\":\"{}\"}}", "x".repeat(600 * 1024));
    let text = [
        call("1", ROOT, T1, "cargo test"),
        result("1", false, ""),
        huge,
        call("2", ROOT, T2, "cargo clippy"),
        result("2", false, ""),
    ]
    .join("\n");
    let path = scratch.file("session.jsonl", &text);

    let mut reading = Reading::default();
    reading.feed_file(&path, 0);

    assert_eq!(derive(&reading.records, ROOT).len(), 2);
}

#[test]
fn a_line_with_no_time_takes_the_files_own() {
    let line = call("1", ROOT, T1, "make test").replace(&format!(",\"timestamp\":\"{T1}\""), "");
    let mut reading = Reading::default();
    reading.feed(line.as_bytes(), 42);
    reading.feed(result("1", false, "").as_bytes(), 42);

    assert_eq!(derive(&reading.records, ROOT)[0].last_ran, 42);
}

#[test]
fn a_failed_commands_exit_code_is_kept_with_what_it_printed() {
    let records = read(&[call("1", ROOT, T1, "cargo test"), result("1", true, "Exit code 101\nerror: boom")]);

    let Some(Record::Done { failed, exit, said, .. }) = records.last() else { panic!("a result: {records:?}") };
    assert_eq!((*failed, *exit), (true, Some(101)));
    assert!(said.contains("boom"));
}

#[test]
fn a_command_that_printed_no_exit_code_has_none_and_a_flagged_one_still_failed() {
    let ok = read(&[call("1", ROOT, T1, "cargo test"), result("1", false, "ok")]);
    assert!(matches!(ok.last(), Some(Record::Done { failed: false, exit: None, .. })));

    let refused = read(&[call("2", ROOT, T1, "cargo test"), result("2", true, "refused")]);
    assert!(matches!(refused.last(), Some(Record::Done { failed: true, exit: None, .. })));
}

/// The live stream names no folder and no time on its lines: the call has none, and the reader of
/// the stream puts the session's own folder there before deriving.
#[test]
fn a_stream_line_with_no_folder_or_time_is_a_call_at_the_fallback_time() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"1","name":"Bash","input":{"command":"cargo test"}}]}}"#;
    let mut reading = Reading::default();
    reading.feed(line.as_bytes(), 42);

    assert_eq!(
        reading.records,
        vec![Record::Call { id: "1".into(), command: "cargo test".into(), cwd: None, at_ms: 42, background: false }]
    );
}

#[test]
fn a_command_the_shell_lacks_is_named_from_each_shells_wording() {
    let said = |text: &str| {
        missing_commands(&read(&[call("1", ROOT, T1, "x"), result("1", true, text)]))
    };
    assert_eq!(said("zsh: command not found: timeout"), vec![("timeout".to_string(), T1_MS)]);
    assert_eq!(said("bash: gtimeout: command not found"), vec![("gtimeout".to_string(), T1_MS)]);
    assert_eq!(said("sh: jq: command not found"), vec![("jq".to_string(), T1_MS)]);
    assert_eq!(said("bash: line 2: jq: command not found"), vec![("jq".to_string(), T1_MS)]);
    assert!(said("zsh: command not found: ./run it").is_empty());
    assert!(said("error: command not found in the lockfile").is_empty());
}

#[test]
fn a_name_seen_twice_is_kept_once_at_the_later_time() {
    let lines = [
        call("1", ROOT, T1, "timeout 5 true"),
        result("1", true, "Exit code 127\nzsh: command not found: timeout"),
        call("2", ROOT, T2, "timeout 5 true"),
        result("2", true, "Exit code 127\nzsh: command not found: timeout"),
    ];
    assert_eq!(missing_commands(&read(&lines)), vec![("timeout".to_string(), T2_MS)]);
}

/// A pipe hides the exit of the command before it, so the line came back as a success and `timeout`
/// was kept as a way to run the tests.
#[test]
fn a_line_whose_command_was_not_found_behind_a_pipe_is_not_a_fact() {
    let lines = [
        call("1", ROOT, T1, "timeout 200 cargo test -p shop 2>&1 | tail -20"),
        result("1", false, "zsh: command not found: timeout"),
    ];
    assert!(facts(&lines).is_empty());
}
