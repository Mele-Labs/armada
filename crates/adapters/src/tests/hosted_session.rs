//! A hosted session's process, asserted on the rendering. **Nothing here starts
//! anything**, for `harness`'s reason.

use adapter_traits::{Environment, McpConfig};
use ipc::SessionMode;

use super::harness::value_after;
use crate::harness::HeadlessAgent;
use crate::hosted_session::{reads_only, HostedLaunch, HostedRefused};
use crate::{init_commands, sent_message};

const ID: &str = "7b1f3c52-9a40-4d27-8c1e-3f0d5a6b9e11";

fn environment() -> Environment {
    Environment::nothing()
        .and("PATH", "/usr/bin:/bin")
        .expect("a legal name")
        .and("HOME", "/Users/user")
        .expect("a legal name")
}

fn launching(resuming: bool, mode: SessionMode) -> HostedLaunch {
    HostedLaunch::at(
        "/repos/armada",
        ID,
        resuming,
        "s-7b1f3c52",
        Some("a-model"),
        Some("high"),
        mode,
        McpConfig::only_these("/var/armada/session-mcp.json").expect("an absolute path"),
        environment(),
        "http://127.0.0.1:4100/sessions/gate",
        vec![String::from("/var/armada/attachments/sessions/one")],
    )
    .expect("renders")
}

fn rendered(launch: &HostedLaunch) -> Vec<String> {
    HeadlessAgent::at("/usr/local/bin/agent")
        .render_hosted_session(launch)
        .expect("renders")
        .args()
        .to_vec()
}

#[test]
fn a_session_holds_what_the_person_holds_and_is_told_its_id_and_its_name() {
    let args = rendered(&launching(false, SessionMode::Auto));
    for withheld in [
        "--strict-mcp-config",
        "--setting-sources",
        "--tools",
        "--allowedTools",
        "--disallowedTools",
        "--restricted",
    ] {
        assert!(!args.iter().any(|arg| arg == withheld), "{withheld}");
    }
    assert_eq!(value_after(&args, "--session-id").as_deref(), Some(ID));
    assert!(!args.iter().any(|arg| arg == "--resume"));
    assert_eq!(value_after(&args, "--name").as_deref(), Some("s-7b1f3c52"));
    assert_eq!(value_after(&args, "--effort").as_deref(), Some("high"));
    assert_eq!(
        value_after(&args, "--add-dir").as_deref(),
        Some("/var/armada/attachments/sessions/one")
    );
    assert_eq!(
        value_after(&args, "--permission-prompt-tool").as_deref(),
        Some("mcp__armada-fleet__ask_the_person")
    );
}

#[test]
fn the_next_process_resumes_by_the_same_id() {
    let args = rendered(&launching(true, SessionMode::Auto));
    assert_eq!(value_after(&args, "--resume").as_deref(), Some(ID));
    assert!(!args.iter().any(|arg| arg == "--session-id"));
}

#[test]
fn another_sessions_message_is_accepted_and_the_first_write_is_held_by_a_hook() {
    let args = rendered(&launching(false, SessionMode::Auto));
    let settings = value_after(&args, "--settings").expect("settings");
    assert!(
        settings.contains(r#""crossSessionInbound":"accept""#),
        "{settings}"
    );
    assert!(
        settings.contains(r#""matcher":"Write|Edit|NotebookEdit|Bash""#),
        "{settings}"
    );
    assert!(
        settings.contains("http://127.0.0.1:4100/sessions/gate"),
        "{settings}"
    );
}

#[test]
fn auto_and_ask_are_the_default_mode_because_auto_is_not_reachable_for_a_spawned_session() {
    for (mode, spelled) in [
        (SessionMode::Auto, "default"),
        (SessionMode::Ask, "default"),
        (SessionMode::AcceptEdits, "acceptEdits"),
        (SessionMode::Plan, "plan"),
    ] {
        let args = rendered(&launching(false, mode));
        assert_eq!(
            value_after(&args, "--permission-mode").as_deref(),
            Some(spelled)
        );
    }
}

#[test]
fn what_would_be_read_as_a_flag_or_a_relative_path_is_refused() {
    let with = |directory: &str, session: &str, effort: &str| {
        HostedLaunch::at(
            directory,
            session,
            false,
            "s-1",
            None,
            Some(effort),
            SessionMode::Auto,
            McpConfig::only_these("/var/armada/session-mcp.json").expect("an absolute path"),
            environment(),
            "http://127.0.0.1:4100/sessions/gate",
            Vec::new(),
        )
    };
    assert!(matches!(
        with("repos", ID, "high"),
        Err(HostedRefused::DirectoryNotAbsolute { .. })
    ));
    assert!(matches!(
        with("/repos", "--resume", "high"),
        Err(HostedRefused::SessionNotPortable { .. })
    ));
    assert!(matches!(
        with("/repos", ID, "-x"),
        Err(HostedRefused::EffortNotPortable { .. })
    ));
}

#[test]
fn a_shell_line_is_a_read_only_where_every_part_of_it_is_and_nothing_redirects() {
    for read in [
        "git status",
        "git log --oneline | head -5",
        "ls -la && pwd",
        "grep -rn foo crates",
        "git branch --show-current",
        "gh pr view 12",
        "armada need --status",
    ] {
        assert!(reads_only(read), "{read}");
    }
    for write in [
        "cargo build",
        "echo hi > file",
        "git checkout main",
        "git branch topic",
        "rm -rf target",
        "ls; touch x",
        "cat $(touch x)",
        "find . -delete",
        "npm install",
    ] {
        assert!(!reads_only(write), "{write}");
    }
}

#[test]
fn who_a_message_went_to_and_what_it_said_is_read_off_the_detail_beside_the_line_that_wrote_it() {
    assert_eq!(
        sent_message("SendMessage", "to s-7b1f3c52: please review"),
        Some(("s-7b1f3c52", "please review"))
    );
    assert_eq!(sent_message("Bash", "to s-1: hi"), None);
    assert_eq!(sent_message("SendMessage", "no recipient"), None);
}

#[test]
fn the_init_line_names_the_commands_the_agent_has_and_no_other_line_does() {
    let init = r#"{"type":"system","subtype":"init","slash_commands":["compact","review"],"skills":["review","commit"]}"#;
    assert_eq!(
        init_commands(init),
        Some(vec!["compact".into(), "review".into(), "commit".into()])
    );
    let other = r#"{"type":"system","subtype":"hook_started","slash_commands":["x"]}"#;
    assert_eq!(init_commands(other), None);
    assert_eq!(init_commands(r#"{"type":"result","num_turns":1}"#), None);
    assert_eq!(init_commands("not json, slash_commands"), None);
}
