//! A repair's process, asserted on the rendering. **The launch starts nothing**, for `harness`'s reason.

use adapter_traits::{Environment, McpConfig, Model};

use super::harness::value_after;
use crate::harness::HeadlessAgent;
use crate::repairing::{denied_to_a_repair, Repairing};

fn launch() -> adapter_traits::Launch {
    let repairing = Repairing::in_checkout(
        "/repos/armada/.armada/slots/slot-2/",
        Model::named("a-model").expect("a named model"),
        McpConfig::only_these("/var/armada/repair-mcp.json").expect("an absolute path"),
        Environment::nothing().and("PATH", "/usr/bin:/bin").expect("a legal name"),
    )
    .expect("an absolute checkout");
    HeadlessAgent::at("/usr/local/bin/agent").render_repair(&repairing).expect("renders")
}

/// A repair writes, so it is given the edit and run tools, and each tool that schedules, messages,
/// starts an agent, reaches the web or loads a skill is denied by name.
#[test]
fn a_repair_is_given_the_edit_and_run_tools_and_denied_the_rest() {
    let launch = launch();
    let args = launch.args();
    let given = Some("Bash,Read,Edit,Write,Grep,Glob".to_string());
    assert_eq!(value_after(args, "--tools"), given);
    assert_eq!(value_after(args, "--allowedTools"), given);
    let denied = value_after(args, "--disallowedTools").expect("a deny list");
    let denied: Vec<&str> = denied.split(',').collect();
    assert_eq!(denied, denied_to_a_repair());
    for tool in ["Task", "WebFetch", "WebSearch", "EnterWorktree", "SendMessage", "ScheduleWakeup", "PushNotification", "Skill", "NotebookEdit"] {
        assert!(denied.contains(&tool), "`{tool}` is not denied: {denied:?}");
    }
    for tool in ["Bash", "Read", "Edit", "Write", "Grep", "Glob"] {
        assert!(!denied.contains(&tool), "a repair is given {tool}");
    }
}

/// One turn in its checkout, holding no server, asked nothing and never resumed.
#[test]
fn a_repair_runs_in_its_checkout_with_no_server_no_prompt_tool_and_no_session_kept() {
    let launch = launch();
    let args = launch.args();
    assert_eq!(launch.directory(), "/repos/armada/.armada/slots/slot-2");
    assert!(args.iter().any(|arg| arg == "--strict-mcp-config"));
    assert_eq!(value_after(args, "--mcp-config").as_deref(), Some("/var/armada/repair-mcp.json"));
    assert_eq!(value_after(args, "--permission-mode").as_deref(), Some("dontAsk"));
    assert!(!args.iter().any(|arg| arg == "--permission-prompt-tool" || arg == "--resume" || arg == "--restricted"));
    assert!(args.iter().any(|arg| arg == "--no-session-persistence"));
}

#[test]
fn a_relative_checkout_is_refused_a_repair() {
    let refused = Repairing::in_checkout(
        "repos/armada",
        Model::named("a-model").expect("a named model"),
        McpConfig::only_these("/var/armada/repair-mcp.json").expect("an absolute path"),
        Environment::nothing(),
    );
    assert!(refused.is_err());
}
