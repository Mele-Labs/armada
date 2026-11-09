//! The process a message to the Operator runs as, against a stand-in agent: the real renderer, the real
//! reader and the real process group. It is started in the folder written for the message, holding the
//! three read tools and nothing else, with strict MCP config and no server, and the repositories beside
//! it; it answers with what it said last; and one that outlasts its time is interrupted and failed.

use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use adapters::HeadlessAgent;

use crate::drone::HostPaths;
use crate::scout::ScoutHost;
use crate::tests::tmp::TempDir;
use crate::CallFailed;

/// The agent CLI as the Operator's message meets it. An ask naming `slowly` waits to be interrupted;
/// any other reads a file and answers with the block.
const STAND_IN: &str = r##"#!/bin/sh
state='@STATE@'
printf '%s\n' "$*" >> "$state/argv.log"
cat "$state/../stand-in-mcp.json" > "$state/servers.log"
IFS= read -r turn
printf '{"type":"system","subtype":"init","session_id":"talk-1","model":"stand-in","mcp_servers":[]}\n'
case "$turn" in
  *slowly*)
    trap 'kill $! 2>/dev/null; exit 0' INT
    sleep 30 >/dev/null 2>&1 &
    wait
    ;;
  *)
    printf '{"type":"assistant","message":{"content":[{"type":"text","text":"Reading the Job."}]}}\n'
    printf '{"type":"assistant","message":{"content":[{"type":"text","text":"%s"}]}}\n' 'cites: none\njob: none\nstart: none\norder: none\nfirst: none\nsaid: It waits on you.'
    printf '{"type":"result","subtype":"success","is_error":false,"num_turns":2,"total_cost_usd":0.002,"permission_denials":[]}\n'
    ;;
esac
"##;

fn host(home: &TempDir) -> (ScoutHost, std::path::PathBuf) {
    let root = home.path();
    let state = root.join("stand-in");
    std::fs::create_dir_all(&state).expect("a state directory");
    let script = root.join("stand-in.sh");
    std::fs::write(&script, STAND_IN.replace("@STATE@", &state.to_string_lossy())).expect("written");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("runs");
    let home_dir = root.to_string_lossy().to_string();
    let host = ScoutHost::new(
        HeadlessAgent::at(script.to_string_lossy()),
        "stand-in",
        root.join("stand-in-mcp.json"),
        HostPaths { path: "/usr/bin:/bin", home: &home_dir, user: "someone", width: checks_runner::CheckWidth::of(2) },
    );
    (host, state)
}

#[tokio::test]
async fn a_message_runs_in_its_folder_with_the_read_tools_alone_and_no_server_and_answers_what_it_said_last() {
    let home = TempDir::new();
    let (host, state) = host(&home);
    let desk = home.path().join("desk");
    std::fs::create_dir_all(&desk).expect("a folder");
    let repository = home.path().join("repository");
    std::fs::create_dir_all(&repository).expect("a checkout");

    let said = host
        .answer_reading("a-model", &desk.to_string_lossy(), &[repository.to_string_lossy().to_string()], 16, "what is it waiting on?", Duration::from_secs(10))
        .await
        .expect("answered");

    assert!(said.ends_with("said: It waits on you."), "the last thing it said, not the first: {said}");
    let argv = std::fs::read_to_string(state.join("argv.log")).expect("started once");
    let argv: Vec<&str> = argv.split_whitespace().collect();
    let after = |flag: &str| argv.iter().position(|arg| *arg == flag).and_then(|at| argv.get(at + 1)).copied();
    assert_eq!(after("--tools"), Some("Read,Grep,Glob"));
    assert_eq!(after("--allowedTools"), Some("Read,Grep,Glob"));
    assert!(argv.contains(&"--strict-mcp-config") && argv.contains(&"--restricted"), "{argv:?}");
    assert_eq!(after("--add-dir"), Some(repository.to_string_lossy().as_ref()));
    assert_eq!(after("--max-turns"), Some("16"));
    assert_eq!(after("--model"), Some("a-model"));
    assert_eq!(after("--permission-mode"), Some("dontAsk"));
    for write in ["Bash", "Edit", "Write", "WebFetch"] {
        assert!(after("--disallowedTools").is_some_and(|denied| denied.split(',').any(|one| one == write)), "{write} is denied");
    }
    let servers = std::fs::read_to_string(state.join("servers.log")).expect("a server file");
    assert!(servers.contains("mcpServers") && !servers.contains("command"), "no server: {servers}");
}

#[tokio::test]
async fn a_message_that_outlasts_its_time_is_interrupted_and_failed() {
    let home = TempDir::new();
    let (host, _state) = host(&home);
    let desk = home.path().join("desk");
    std::fs::create_dir_all(&desk).expect("a folder");

    let began = std::time::Instant::now();
    let failed = host.answer_reading("a-model", &desk.to_string_lossy(), &[], 16, "answer slowly", Duration::from_millis(300)).await;

    assert!(matches!(failed, Err(CallFailed::TimedOut)), "{failed:?}");
    assert!(began.elapsed() < Duration::from_secs(8), "interrupted, not waited out: {:?}", began.elapsed());
}
