//! What a hosted Job is started with to cost fewer tokens, asserted on the rendering: the compaction
//! window, the skills hidden from the model, and the connectors in both directions.

use adapter_traits::{Environment, Launch, McpConfig};
use ipc::SessionMode;

use super::harness::value_after;
use crate::harness::HeadlessAgent;
use crate::hosted_session::HostedLaunch;

fn launching() -> HostedLaunch {
    HostedLaunch::at(
        "/repos/armada",
        "7b1f3c52-9a40-4d27-8c1e-3f0d5a6b9e11",
        false,
        "s-7b1f3c52",
        Some("a-model"),
        Some("high"),
        SessionMode::Auto,
        McpConfig::only_these("/var/armada/session-mcp.json").expect("an absolute path"),
        Environment::nothing().and("PATH", "/usr/bin:/bin").expect("a legal name"),
        "http://127.0.0.1:4100/sessions/gate",
        Vec::new(),
    )
    .expect("renders")
}

fn rendered(launch: &HostedLaunch) -> Launch {
    HeadlessAgent::at("/usr/local/bin/agent").render_hosted_session(launch).expect("renders")
}

fn env_of(launch: &HostedLaunch, name: &str) -> Option<String> {
    rendered(launch).environment().vars().iter().find(|(n, _)| n == name).map(|(_, v)| v.clone())
}

/// The `--settings` text, whose keys are written in order.
fn settings_of(launch: &HostedLaunch) -> String {
    value_after(rendered(launch).args(), "--settings").expect("settings")
}

#[test]
fn a_launch_tells_the_cli_when_to_compact_and_holds_the_value_to_the_range_it_reads() {
    let name = "CLAUDE_CODE_AUTO_COMPACT_WINDOW";
    assert_eq!(env_of(&launching(), name), None, "nothing set leaves the CLI's own");
    assert_eq!(env_of(&launching().compacting_at(250_000), name).as_deref(), Some("250000"));
    assert_eq!(env_of(&launching().compacting_at(5), name).as_deref(), Some("100000"));
    assert_eq!(env_of(&launching().compacting_at(9_000_000), name).as_deref(), Some("1000000"));
}

#[test]
fn skills_switched_off_are_hidden_from_the_model_and_keep_the_persons_own_slash_command() {
    let off = ["pptx".to_string(), "anthropic-skills:docx".to_string()];
    let settings = settings_of(&launching().without_skills(&off));
    assert!(
        settings.contains(r#""skillOverrides":{"anthropic-skills:docx":"user-invocable-only","pptx":"user-invocable-only"}"#),
        "{settings}"
    );
    assert!(!settings_of(&launching()).contains("skillOverrides"), "none off writes none");
}

#[test]
fn armadas_own_skills_are_never_hidden_whatever_is_named() {
    let off = ["armada:reading-a-brief".to_string(), "pptx".to_string()];
    let settings = settings_of(&launching().without_skills(&off));
    assert!(settings.contains(r#""skillOverrides":{"pptx":"user-invocable-only"}"#), "{settings}");
    assert!(!settings.contains("reading-a-brief"), "{settings}");
    let only = settings_of(&launching().without_skills(&["armada:root-cause".to_string()]));
    assert!(!only.contains("skillOverrides"), "{only}");
}

#[test]
fn connectors_are_kept_out_when_off_and_fetched_when_on() {
    let name = "ENABLE_CLAUDEAI_MCP_SERVERS";
    assert_eq!(env_of(&launching().without_connectors(), name).as_deref(), Some("false"));
    assert_eq!(env_of(&launching(), name), None);
}

#[test]
fn a_repositorys_own_skill_is_found_by_its_folder_and_file() {
    let root = std::env::temp_dir().join(format!("armada-own-skill-{}", std::process::id()));
    let folder = root.join(".claude/skills/release-notes");
    std::fs::create_dir_all(&folder).expect("a folder");
    assert!(!crate::has_own_skill(&root, "release-notes"), "a folder with no SKILL.md is no skill");
    std::fs::write(folder.join("SKILL.md"), "---\nname: release-notes\n---\n").expect("a skill");
    assert!(crate::has_own_skill(&root, "release-notes"));
    assert!(!crate::has_own_skill(&root, "pptx"));
    std::fs::remove_dir_all(&root).expect("cleaned up");
}

#[test]
fn a_launch_asks_for_thinking_summaries_only_when_told_to() {
    assert_eq!(value_after(rendered(&launching()).args(), "--thinking-display"), None, "untold leaves the CLI's own");
    let told = rendered(&launching().summarising_thinking());
    assert_eq!(value_after(told.args(), "--thinking-display").as_deref(), Some("summarized"));
}
