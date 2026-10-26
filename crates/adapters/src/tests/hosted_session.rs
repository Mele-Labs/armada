//! A hosted session's process, asserted on the rendering. **Nothing here starts
//! anything**, for `harness`'s reason.

use adapter_traits::{Environment, McpConfig};
use ipc::SessionMode;

use super::harness::value_after;
use crate::harness::HeadlessAgent;
use crate::hosted_session::{reads_only, HostedLaunch, HostedRefused, SESSION_RULES};
use crate::{init_commands, inside_a_subagent, sent_message, viewed_image, written_document, written_job_page};

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
fn a_session_has_the_task_tools_so_its_build_ring_has_pieces() {
    let launch = HeadlessAgent::at("/usr/local/bin/agent")
        .render_hosted_session(&launching(false, SessionMode::Auto))
        .expect("renders");
    assert_eq!(
        launch.environment(),
        &crate::browser_shell::environment(
            environment()
                .and(crate::conversing::TASK_TOOLS, "1")
                .and_then(|env| env.and("CLAUDE_CODE_MCP_STARTUP_WAIT_MS", "0"))
                .and_then(|env| env.and("MCP_TIMEOUT", "120000"))
                .expect("a legal name"),
            crate::browser_shell::found().as_deref(),
        )
        .expect("a legal name")
    );
}

/// Spike 34: the first turn waits only for the door, and a person's slow server still connects
/// behind it. The CLI's spelling is pinned here rather than read from the constants.
#[test]
fn a_session_starts_its_first_turn_without_waiting_on_a_slow_server() {
    let launch = HeadlessAgent::at("/usr/local/bin/agent")
        .render_hosted_session(&launching(false, SessionMode::Auto))
        .expect("renders");
    let vars = launch.environment().vars();
    for (name, value) in [("CLAUDE_CODE_MCP_STARTUP_WAIT_MS", "0"), ("MCP_TIMEOUT", "120000")] {
        assert!(vars.iter().any(|(n, v)| n == name && v == value), "{name}={value} in {vars:?}");
    }
}

#[test]
fn the_next_process_resumes_by_the_same_id() {
    let args = rendered(&launching(true, SessionMode::Auto));
    assert_eq!(value_after(&args, "--resume").as_deref(), Some(ID));
    assert!(!args.iter().any(|arg| arg == "--session-id"));
}

const OLD: &str = "2c9e41d8-5b73-4f06-a1d2-8e7c0b3f9a64";
const NOTE: &str = "This session is a fork of an older one.";

#[test]
fn a_fork_starts_from_the_old_conversation_under_its_own_id() {
    let args = rendered(&launching(false, SessionMode::Auto).forking(OLD, NOTE).expect("portable"));
    assert_eq!(value_after(&args, "--resume").as_deref(), Some(OLD));
    assert!(args.iter().any(|arg| arg == "--fork-session"));
    assert_eq!(value_after(&args, "--session-id").as_deref(), Some(ID));
}

#[test]
fn a_fork_that_has_run_resumes_itself_and_not_the_one_it_came_from() {
    let args = rendered(&launching(true, SessionMode::Auto).forking(OLD, NOTE).expect("portable"));
    assert_eq!(value_after(&args, "--resume").as_deref(), Some(ID));
    assert!(!args.iter().any(|arg| arg == "--fork-session"));
    assert!(!args.iter().any(|arg| arg == "--session-id"));
}

#[test]
fn every_process_of_a_fork_is_told_it_is_one() {
    for resuming in [false, true] {
        let args = rendered(&launching(resuming, SessionMode::Auto).forking(OLD, NOTE).expect("portable"));
        assert_eq!(appended(&args), format!("{}\n\n{NOTE}", rules()), "resuming: {resuming}");
    }
}

/// The rules as they are appended: trimmed, so a note follows after one blank line.
fn rules() -> &'static str {
    SESSION_RULES.trim()
}

/// The one appended system prompt, asserting there is exactly one.
fn appended(args: &[String]) -> String {
    assert_eq!(args.iter().filter(|arg| *arg == "--append-system-prompt").count(), 1);
    value_after(args, "--append-system-prompt").expect("appended")
}

/// Armada's rules ride every launch, plain, noted, forked or resuming, and are the whole of a
/// plain one.
#[test]
fn every_launch_carries_the_session_rules_once() {
    for resuming in [false, true] {
        let plain = rendered(&launching(resuming, SessionMode::Auto));
        assert_eq!(appended(&plain), rules(), "resuming: {resuming}");
        let noted = rendered(&launching(resuming, SessionMode::Auto).noting("You are Helm."));
        assert_eq!(appended(&noted).matches(rules()).count(), 1, "resuming: {resuming}");
        let forked = rendered(&launching(resuming, SessionMode::Auto).forking(OLD, NOTE).expect("portable"));
        assert_eq!(appended(&forked).matches(rules()).count(), 1, "resuming: {resuming}");
    }
}

/// The rules name the tools Fleet's door offers, so a rename there fails here.
#[test]
fn the_rules_name_the_tools_that_surface_a_need() {
    for named in ["waiting_for", "finish_up", "all_done", "TaskCreate", "TaskUpdate", "AskUserQuestion", "record_plan"] {
        assert!(rules().contains(named), "{named}");
    }
}

/// Fleet runs judges itself, so a plan's group is never given the `judge` subagent: the rules say so, in the planning paragraph.
#[test]
fn the_rules_say_no_group_names_a_judge() {
    let planning = rules().split("\n\n").find(|block| block.starts_with("Planning.")).expect("a planning paragraph");
    assert!(planning.contains("A group's subagent is the type that does its work, and Fleet runs judges itself, so no group names one."));
}

/// A plan reaches the person plain: a summary in his words, a line per group of what it gives him, and
/// a plan he said he is not ready for is not recorded.
#[test]
fn the_rules_ask_for_a_plain_summary_and_a_gives_line_and_no_plan_until_he_asks() {
    let planning = rules().split("\n\n").find(|block| block.starts_with("Planning.")).expect("a planning paragraph");
    assert!(planning.contains("a `summary`, a few lines in the person's own words saying what he gets, with no file names or types"));
    assert!(planning.contains("a `gives` line"));
    assert!(planning.contains("If he says he is not ready for a plan, keep talking and record no plan until he asks for one."));
}

/// Fleet asks some questions itself, how the work is delivered among them, so a session that put
/// every decision on `waiting_for` would show the person the same question twice.
#[test]
fn the_rules_say_a_question_fleet_asks_is_not_asked_again() {
    let needs = rules().split("\n\n").find(|block| block.starts_with("Needs.")).expect("a needs paragraph");
    assert!(needs.contains("A question Fleet puts to the person itself, such as how the work is delivered, is already in Bridge: never ask it again."));
}

/// A follow-up the session keeps for itself is deferred through the call, not as a sentence closing
/// its reply, and settled with the call Fleet already holds `all_done` on.
#[test]
fn the_rules_say_a_followup_is_deferred_through_the_call() {
    let progress = rules().split("\n\n").find(|block| block.starts_with("Progress.")).expect("a progress paragraph");
    for named in ["`defer_later`", "`settle_later`"] {
        assert!(progress.contains(named), "{named}");
    }
}

/// A session in any repository is told where its subagents come from, in order, and which model
/// each kind of work runs on, so a lookup never runs on the session's own model.
#[test]
fn the_rules_say_which_subagent_and_which_model() {
    let subagents = rules().split("\n\n").find(|block| block.starts_with("Subagents.")).expect("a subagents paragraph");
    for named in ["`.claude/agents/`", "`armada:`", "built-in", "`model`", "`haiku`", "`sonnet`", "the job's own model"] {
        assert!(subagents.contains(named), "{named}");
    }
}

/// A watch is what a session starts to follow a deploy, and is a wait it holds without a `waiting_for`
/// item, so the rules name the call and the ending that a live one forbids.
#[test]
fn the_rules_say_to_start_a_watch_and_that_a_live_one_is_a_wait() {
    let watching = rules().split("\n\n").find(|block| block.starts_with("Watching.")).expect("a watching paragraph");
    for named in ["`start_watch`", "`weigh_watch`", "pull request's own checks", "set aside"] {
        assert!(watching.contains(named), "{named}");
    }
    let finishing = rules().split("\n\n").find(|block| block.starts_with("Finishing.")).expect("a finishing paragraph");
    assert!(finishing.contains("A live watch is a wait as an open pull request is: do not call `all_done` while one runs, add no `waiting_for` item for it"));
}

/// Fleet takes a merge wait off Needs you, so the rules must not tell an agent to state one: the
/// two contradicted, and an agent that followed the rules put the pull request on his list.
#[test]
fn the_rules_never_put_an_open_pull_requests_wait_in_waiting_for() {
    let finishing = rules().split("\n\n").find(|block| block.starts_with("Finishing.")).expect("a finishing paragraph");
    assert!(finishing.contains("never put a wait on it in `waiting_for`: Fleet takes such an item off Needs you"));
    assert!(!rules().contains("name the wait in `waiting_for`"));
}

/// A hosted session is told to dispute a Fleet notice it finds wrong through the call, not only in
/// its reply, since the call is what starts the review.
#[test]
fn the_rules_say_a_wrong_fleet_notice_is_disputed_through_the_call() {
    let fleet = rules().split("\n\n").find(|block| block.starts_with(HOSTED_ONLY)).expect("a Fleet paragraph");
    for named in ["`dispute_fleet`", "`why`"] {
        assert!(fleet.contains(named), "{named}");
    }
}

/// The paragraph only a hosted session is given: a dispute is tied to a row of the thread, and a
/// terminal job's thread is not Fleet's, so `dispute_fleet` refuses one.
const HOSTED_ONLY: &str = "Fleet's notices.";

/// The mod's `Armada` output style repeats the rules by hand, so a drift between the two fails
/// here. The rules' opening paragraph addresses a hosted session and the style has its own, and
/// the paragraph on Fleet's notices is a hosted session's alone.
#[test]
fn the_output_style_carries_the_rules_verbatim() {
    let style = include_str!("../../../../plugins/armada/output-styles/armada.md");
    let blocks: Vec<&str> = rules().split("\n\n").skip(1).filter(|block| !block.starts_with(HOSTED_ONLY)).collect();
    assert_eq!(blocks.len(), 6, "needs, progress, planning, subagents, watching, finishing");
    for block in blocks {
        assert!(style.contains(block), "the style has drifted from: {block}");
    }
}

/// Helm's note is appended to every process of its Session after the rules, and a blank one is
/// none at all.
#[test]
fn helms_note_is_appended_to_every_process_of_its_session() {
    let note = "You are Helm, docked in Armada over repository armada.";
    for resuming in [false, true] {
        let args = rendered(&launching(resuming, SessionMode::Auto).noting(note));
        assert_eq!(appended(&args), format!("{}\n\n{note}", rules()), "resuming: {resuming}");
    }
    let blank = rendered(&launching(false, SessionMode::Auto).noting("  \n "));
    assert_eq!(appended(&blank), rules());
}

/// A fork of a Session that has a note is told both, in one argument and not two, which the CLI
/// would read as the last of them.
#[test]
fn a_note_and_a_forks_note_are_one_appended_prompt() {
    let launch = launching(false, SessionMode::Auto)
        .forking(OLD, NOTE)
        .expect("portable")
        .noting("You are Helm.");
    let args = rendered(&launch);
    assert_eq!(appended(&args), format!("{}\n\nYou are Helm.\n\n{NOTE}", rules()));
}

#[test]
fn a_fork_with_no_note_is_refused_before_it_starts() {
    for note in ["", "  \n"] {
        assert_eq!(launching(false, SessionMode::Auto).forking(OLD, note), Err(HostedRefused::ForkNotTold));
    }
}

const LINE: &str = "7d0a52c4-91be-4f3a-8c26-0b5e1f47a9d3";

#[test]
fn a_fork_at_a_message_ends_the_copy_at_that_line() {
    let launch = launching(false, SessionMode::Auto).forking(OLD, NOTE).expect("portable").cut_at(LINE).expect("portable");
    let args = rendered(&launch);
    assert_eq!(value_after(&args, "--resume-session-at").as_deref(), Some(LINE));
    assert_eq!(value_after(&args, "--resume").as_deref(), Some(OLD));
    assert!(args.iter().any(|arg| arg == "--fork-session"));
}

#[test]
fn a_fork_that_has_run_is_not_cut_again() {
    let launch = launching(true, SessionMode::Auto).forking(OLD, NOTE).expect("portable").cut_at(LINE).expect("portable");
    assert!(!rendered(&launch).iter().any(|arg| arg == "--resume-session-at"));
}

#[test]
fn a_cut_that_would_be_read_as_a_flag_is_refused() {
    assert!(launching(false, SessionMode::Auto).forking(OLD, NOTE).expect("portable").cut_at("--dangerous").is_err());
}

#[test]
fn a_fork_of_something_that_would_be_read_as_a_flag_is_refused() {
    assert!(launching(false, SessionMode::Auto).forking("--dangerous", NOTE).is_err());
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
        settings.contains(r#""matcher":"*""#),
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
        "armada worktree --status",
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
        "armada worktree release /repo/.armada/slots/slot-5",
        "armada worktree --status && armada worktree release",
    ] {
        assert!(!reads_only(write), "{write}");
    }
}

#[test]
fn another_sessions_slot_is_read_like_any_directory() {
    for read in [
        "git -C /repo/.armada/slots/slot-9 status",
        "git -C /repo/.armada/slots/slot-9 log --oneline -5",
        "ls /repo/.armada/slots/slot-9/crates",
        "cd /repo/.armada/slots/slot-9 && git status",
    ] {
        assert!(reads_only(read), "{read}");
    }
    assert!(!reads_only("git -C /repo/.armada/slots/slot-9 checkout main"));
}

#[test]
fn a_redirect_to_nowhere_and_quoted_shell_marks_do_not_make_a_line_a_write() {
    for read in [
        r#"armada worktree --status 2>&1 | grep -E "slot-1 |07541e89""#,
        "git log 2>&1 | head",
        "gh pr view 2250 --json state 2>/dev/null",
        r#"grep -E "a|b" file"#,
        r#"echo "x; y""#,
        "armada worktree --status 2>&1",
        "cd /tmp && git -C /x log >/dev/null 2>&1",
        "git status &>/dev/null",
        "git log 1>&2",
        "git log >&2 | head",
        r#"echo 'a & b' \; pwd"#,
        r#"echo "2>&1""#,
        r#"grep -n "a -> b" file"#,
        "grep -rn 'Vec<String> =>' crates",
        r#"cd /repo && grep -rn "Ran: \|\`Ran" apps --include='*.ts' | grep -v mock | head"#,
        "rg '`[a-z]+`' docs",
        "grep -n '$(' scripts/preview",
    ] {
        assert!(reads_only(read), "{read}");
    }
    for write in [
        "echo hi > file",
        "echo hi >> file",
        "git log > out.txt",
        "cat a 2>&1 > out",
        "git log >/dev/null2",
        "git log 2>/dev/nullx",
        r#"echo "$(rm -rf x)""#,
        r#"grep "`rm x`" file"#,
        "echo a>b",
        "cat `touch x`",
        "grep \"a\" f | tee out",
        "grep \"a f",
        "echo 'a f",
        "armada worktree release /x",
        "git commit -m \"x\"",
        "gh pr create",
        "sed -i s/a/b/ f",
        "git log 2>&1 | rm x",
        "git log \"a|b\" ; rm x",
    ] {
        assert!(!reads_only(write), "{write}");
    }
}

#[test]
fn a_line_that_only_looks_elsewhere_or_asks_the_forge_is_still_a_read() {
    for read in [
        "cd /repo/.armada/slots/slot-3 && git log --oneline | head",
        "git -C /repo/.armada/slots/slot-3 status",
        "git --no-pager -C /repo log -5",
        "git merge-base origin/main HEAD",
        "git remote -v",
        "git worktree list",
        "git -C /repo worktree list --porcelain | head -20",
        "git stash list",
        "git tag --list 'v*'",
        "git config --get remote.origin.url",
        "git for-each-ref --format=x refs/heads",
        "git ls-tree -r HEAD",
        "git rev-parse --abbrev-ref HEAD",
        "gh issue view 2239",
        "gh issue list --state open",
        "gh pr checks 2234",
        "gh run list --limit 3",
        "gh api repos/Mele-Labs/armada/pulls/2234",
        "gh api -X GET repos/Mele-Labs/armada/pulls --jq '.[].number'",
        "gh api --paginate repos/Mele-Labs/armada/issues",
        "sed -n 250,420p crates/fleet/src/session_host/hearing.rs",
        "sed -n '10,40p' file | head",
        "sed -n '$p' file",
        "ls",
        "git log",
    ] {
        assert!(reads_only(read), "{read}");
    }
    for write in [
        "git commit -m x",
        "git push origin topic",
        "git -C /repo commit -m x",
        "git -C /repo push",
        "git -C",
        "git remote add origin x",
        "git worktree add ../x topic",
        "git worktree remove /repo/.armada/slots/slot-3",
        "git stash pop",
        "git tag v1",
        "git config user.name x",
        "git merge topic",
        "cd /repo && git commit -m x",
        "cd /repo && sed -i s/a/b/ file",
        "sed -i s/a/b/ file",
        "sed s/a/b/ file",
        "sed -n -i 1p file",
        "sed -n 'w out' file",
        "sed -n '1,3p;w out' file",
        "sed -n 1e file",
        "sed -n p file",
        "cd /repo && echo x > file",
        "gh pr create --fill",
        "gh pr merge 12",
        "gh api -X POST repos/o/r/issues",
        "gh api --method=DELETE repos/o/r/issues/1",
        "gh api repos/o/r/issues -f title=x",
        "gh api graphql --input query.json",
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

/// Measured: a haiku run that started a background subagent. The subagent's
/// lines carried the id of the `Agent` call; the agent's own carried `null`.
#[test]
fn a_subagents_own_message_is_told_from_the_agents_by_the_call_that_started_it() {
    let inside = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Clean. Run the walk."}]},"parent_tool_use_id":"toolu_011Ke1eATFaYcZ1Fyz5jBx8S","session_id":"s"}"#;
    let own = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"done"}]},"parent_tool_use_id":null,"session_id":"s"}"#;
    let inside_result = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"ok"}]},"parent_tool_use_id":"toolu_011Ke1eATFaYcZ1Fyz5jBx8S"}"#;
    assert!(inside_a_subagent(inside));
    assert!(inside_a_subagent(inside_result));
    assert!(!inside_a_subagent(own));
    // No key at all, a Bash call's heartbeat, and a line that is not JSON.
    assert!(!inside_a_subagent(r#"{"type":"assistant","message":{"content":[]}}"#));
    assert!(!inside_a_subagent(
        r#"{"type":"tool_progress","tool_name":"Bash","parent_tool_use_id":"toolu_1"}"#
    ));
    assert!(!inside_a_subagent("not json, parent_tool_use_id"));
    // `read` still hears the subagent's line: a Drone's evidence needs it.
    assert!(matches!(
        crate::transcript::read(inside).as_slice(),
        [adapter_traits::DroneEvent::Said { text, .. }] if text == "Clean. Run the walk."
    ));
}

#[test]
fn a_written_document_is_read_off_the_write_detail_and_code_and_scratch_are_not() {
    let write = |detail| written_document("Write", detail, "/Users/user");
    assert_eq!(
        write("~/repo/docs/spike.md +40"),
        Some("/Users/user/repo/docs/spike.md".into())
    );
    assert_eq!(
        write("/repo/shots/ledger.PNG +1"),
        Some("/repo/shots/ledger.PNG".into())
    );
    assert_eq!(write("/repo/src/clock.rs +9"), None);
    assert_eq!(write("/repo/package.json +3"), None);
    assert_eq!(write("/tmp/notes.md +3"), None);
    assert_eq!(write("/repo/node_modules/x/README.md +3"), None);
    assert_eq!(write("/repo/docs/spike.md +4 -2"), None);
    assert_eq!(
        written_document("Edit", "/repo/docs/spike.md +4", "/Users/user"),
        None
    );
}

#[test]
fn a_page_in_a_jobs_documents_folder_is_read_off_a_write_an_edit_and_a_multi_edit_and_nothing_else_is() {
    let page = |tool, detail| written_job_page(tool, detail, "/Users/user");
    let mine = Some("/Users/user/repo/.armada/documents/lease.html".to_string());
    assert_eq!(page("Write", "~/repo/.armada/documents/lease.html +120"), mine);
    assert_eq!(page("Edit", "~/repo/.armada/documents/lease.html +4 -2"), mine);
    assert_eq!(page("MultiEdit", "~/repo/.armada/documents/lease.html"), mine);
    assert_eq!(
        page("Write", "/repo/.armada/slots/slot-5/.armada/documents/notes/lease.HTML +9"),
        Some("/repo/.armada/slots/slot-5/.armada/documents/notes/lease.HTML".to_string())
    );
    // A page anywhere else, a document of another kind, and a call that is not a write.
    assert_eq!(page("Write", "/repo/docs/lease.html +9"), None);
    assert_eq!(page("Write", "/repo/.armada/documents/lease.md +9"), None);
    assert_eq!(page("Write", "/repo/.armada/other/lease.html +9"), None);
    assert_eq!(page("Read", "/repo/.armada/documents/lease.html"), None);
    assert_eq!(page("Bash", "/repo/.armada/documents/lease.html"), None);
    // The page is not a document the ledger lists as a file.
    assert_eq!(written_document("Write", "/repo/.armada/documents/lease.html +9", "/Users/user"), None);
}

#[test]
fn a_picture_read_is_looked_at_and_code_and_a_written_picture_are_not() {
    let read = |detail| viewed_image("Read", detail, "/Users/user");
    assert_eq!(read("/tmp/shot.png"), Some("/tmp/shot.png".into()));
    assert_eq!(
        read("~/repo/shots/ledger.JPEG"),
        Some("/Users/user/repo/shots/ledger.JPEG".into())
    );
    assert_eq!(read("/repo/a.webp"), Some("/repo/a.webp".into()));
    assert_eq!(read("/repo/src/clock.rs"), None);
    assert_eq!(read("/repo/docs/spike.md"), None);
    assert_eq!(viewed_image("Write", "/repo/shot.png +1", "/Users/user"), None);
    assert_eq!(
        written_document("Write", "/repo/shot.png +1", "/Users/user"),
        Some("/repo/shot.png".into())
    );
}

/// Both refused a read on 16 Oct 2026: a line that sets a name first, and a subshell.
#[test]
fn an_assignment_and_a_subshell_read_as_what_they_run() {
    for read in [
        "L=/x/audit.jsonl; grep -n foo \"$L\" | tail -30",
        "F=~/a.jsonl; wc -l $F",
        "(cd /repo && git status)",
        "{ ls; pwd; }",
    ] {
        assert!(reads_only(read), "{read}");
    }
    for write in ["(cd /repo && rm x)", "X=1; touch y", "{ ls; npm install; }"] {
        assert!(!reads_only(write), "{write}");
    }
}
