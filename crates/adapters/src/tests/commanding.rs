//! A Job's Commander's process, asserted on the rendering and held apart from
//! Helm's, a Drone's and a scout's. **Nothing here starts anything.** `#2166`.

use adapter_traits::{Environment, McpConfig, Model, Toolbelt};

use super::harness::value_after;
use crate::commanding::{denied_to_a_commander, Commanding};
use crate::conversing::{door_tools, wrote_the_checkout, ConversationRefused, Conversing};
use crate::harness::HeadlessAgent;

fn environment() -> Environment {
    Environment::nothing()
        .and("PATH", "/usr/bin:/bin")
        .expect("a legal name")
}

fn door() -> McpConfig {
    McpConfig::only_these("/var/armada/commander-mcp.json").expect("an absolute path")
}

fn commanding() -> Commanding {
    Commanding::in_worktree(
        "/repos/armada/.armada/worktrees/job-14",
        Model::named("a-model").expect("a named model"),
        door(),
        environment(),
    )
    .expect("an absolute worktree")
}

fn rendered(commanding: &Commanding) -> adapter_traits::Launch {
    HeadlessAgent::at("/usr/local/bin/agent")
        .render_commander(commanding)
        .expect("renders")
}

#[test]
fn a_commander_opens_in_the_jobs_worktree_with_the_door_and_nothing_else() {
    let launch = rendered(&commanding());
    let args = launch.args();

    assert_eq!(launch.directory(), "/repos/armada/.armada/worktrees/job-14");
    assert!(args.iter().any(|arg| arg == "--strict-mcp-config"));
    assert_eq!(
        value_after(args, "--mcp-config").as_deref(),
        Some("/var/armada/commander-mcp.json")
    );
    assert!(!args.iter().any(|arg| arg == "--resume"));
    assert!(
        launch
            .environment()
            .vars()
            .iter()
            .any(|(name, _)| name == "ARMADA_DRONE"),
        "unattended, so the mod keeps it out of the Sessions list"
    );
}

/// **No owner to ask, so a write tool is not there to be asked about.**
#[test]
fn a_commander_is_given_the_three_reads_and_the_door_and_every_other_built_in_is_denied() {
    let launch = rendered(&commanding());
    let args = launch.args();

    assert_eq!(
        value_after(args, "--tools").as_deref(),
        Some("Read,Grep,Glob")
    );
    let allowed = value_after(args, "--allowedTools").expect("an allowlist");
    assert_eq!(allowed, format!("Read,Grep,Glob,{}", door_tools()));

    let denied = value_after(args, "--disallowedTools").expect("a deny list");
    let denied: Vec<&str> = denied.split(',').collect();
    assert_eq!(denied, denied_to_a_commander());
    for write in ["Edit", "Write", "NotebookEdit", "Bash", "Task", "WebFetch"] {
        assert!(denied.contains(&write), "`{write}` is not denied: {denied:?}");
    }
    for tool in ["Edit", "Write", "NotebookEdit"] {
        assert!(wrote_the_checkout(tool));
        assert!(!allowed.contains(tool), "`{tool}` is allowed: {allowed}");
    }
}

#[test]
fn a_commander_is_never_asked_and_never_put_a_call_to_a_person() {
    let args = rendered(&commanding()).args().to_vec();
    assert_eq!(
        value_after(&args, "--permission-mode").as_deref(),
        Some("dontAsk")
    );
    assert!(!args.iter().any(|arg| arg == "--permission-prompt-tool"));
}

/// **The three launches differ in the one place each is confined**, rendered
/// side by side because a flag taken off one is easily taken off all.
#[test]
fn a_commanders_launch_is_held_apart_from_helms_and_a_drones() {
    let commander = rendered(&commanding()).args().to_vec();

    let helm = HeadlessAgent::at("/usr/local/bin/agent")
        .render_conversation(
            &Conversing::in_repository(
                "/repos/armada",
                Model::named("a-model").expect("a named model"),
                door(),
                environment(),
            )
            .expect("an absolute root"),
        )
        .expect("renders");
    for confined in ["--strict-mcp-config", "--tools", "--disallowedTools"] {
        assert!(
            commander.iter().any(|arg| arg == confined),
            "a Commander is confined by {confined}"
        );
        assert!(
            !helm.args().iter().any(|arg| arg == confined),
            "Helm is the operator at work and passes none: {confined}"
        );
    }
    assert!(
        helm.args().iter().any(|arg| arg == "--permission-prompt-tool"),
        "Helm puts what its settings do not cover to a person"
    );

    let drone = super::harness::rendered(Toolbelt::evidence_only());
    assert_eq!(
        value_after(&drone, "--permission-mode").as_deref(),
        Some("default"),
        "a Drone is asked for by Fleet and a Commander is not asked at all"
    );
    assert!(drone.iter().any(|arg| arg == "--allowedTools"));
    assert_ne!(
        value_after(&drone, "--allowedTools"),
        value_after(&commander, "--allowedTools"),
        "a Drone's allowlist is its step's and a Commander's is the reads"
    );
}

#[test]
fn a_resumed_commander_names_its_session_and_an_unportable_id_is_refused() {
    let launch = rendered(&commanding().resuming("0b7e-4f").expect("portable"));
    assert_eq!(
        value_after(launch.args(), "--resume").as_deref(),
        Some("0b7e-4f")
    );
    assert!(matches!(
        commanding().resuming("--dangerously-skip-permissions"),
        Err(ConversationRefused::SessionNotPortable { .. })
    ));
}

#[test]
fn a_relative_worktree_is_refused() {
    let refused = Commanding::in_worktree(
        "worktrees/job-14",
        Model::named("a-model").expect("a named model"),
        door(),
        environment(),
    );
    assert!(matches!(
        refused,
        Err(ConversationRefused::DirectoryNotAbsolute { .. })
    ));
}
