//! What this Mac has, told to every Job: a command one Job's shell lacked is in the next Job's
//! note, a stand-in is named where there is one, and what now resolves on `PATH` drops out.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::run_memory::{ran, running, the_next_job, told, turn_over};
use super::session_host::rig_placing;
use crate::machine_note::{browser_line, missing_line, on_path, starts_with_missing};
use crate::session_host::Heard;

#[test]
fn a_missing_command_is_named_with_its_stand_in_where_there_is_one() {
    assert_eq!(missing_line("timeout"), "`timeout` is not installed here. Use the Bash tool's `timeout` parameter.");
    assert_eq!(missing_line("jq"), "`jq` is not installed here.");
}

#[test]
fn the_browser_is_the_shell_found_or_the_command_that_installs_one() {
    let found = browser_line(Some(Path::new("/cache/chrome-headless-shell")));
    assert!(found.contains("headless shell at `/cache/chrome-headless-shell`"), "{found}");
    assert!(!found.contains("playwright install"), "{found}");

    let none = browser_line(None);
    assert!(none.contains("`npx playwright install chromium --only-shell`"), "{none}");
}

#[test]
fn a_command_resolves_when_an_executable_of_its_name_is_in_a_folder_of_the_path() {
    let dir = std::env::temp_dir().join(format!("machine-note-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let tool = dir.join("gtimeout");
    let path = format!("/nowhere:{}", dir.display());
    assert!(!on_path("gtimeout", &path));

    std::fs::write(&tool, "#!/bin/sh\n").expect("a file");
    assert!(!on_path("gtimeout", &path), "a file that does not run is not a command");
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).expect("made runnable");
    assert!(on_path("gtimeout", &path));
}

#[tokio::test]
async fn a_command_found_missing_in_one_jobs_stream_is_in_the_next_jobs_note() {
    let rig = rig_placing(None);
    let first = running(&rig).await;
    rig.stand_in.hears(
        0,
        Heard::Ran(ran("m1", "armada-no-such-tool --version", "Exit code 127\nzsh: command not found: armada-no-such-tool")),
    );
    turn_over(&rig, &first).await;
    super::session_host::eventually(|| async { rig.fleet.missing_now().await == vec!["armada-no-such-tool"] }).await;

    let note = told(&rig, &the_next_job(&rig).await).await;

    assert!(note.contains("What this Mac has:"), "{note}");
    assert!(note.contains("- `armada-no-such-tool` is not installed here."), "{note}");
}

#[tokio::test]
async fn a_kept_name_drops_out_once_it_resolves_on_the_path_a_job_is_started_with() {
    let rig = rig_placing(None);
    let first = running(&rig).await;
    // `sh` is on the rig's `PATH`, so it is kept and then no longer missing.
    rig.stand_in.hears(0, Heard::Ran(ran("m1", "sh -c x", "Exit code 127\nbash: sh: command not found")));
    turn_over(&rig, &first).await;
    super::session_host::eventually(|| async {
        rig.fleet.store().lock().await.missing_commands().expect("reads") == vec!["sh"]
    })
    .await;

    let note = told(&rig, &the_next_job(&rig).await).await;

    assert!(!note.contains("`sh`"), "{note}");
    assert!(rig.fleet.store().lock().await.missing_commands().expect("reads").is_empty());
}

#[test]
fn a_remembered_line_that_starts_with_a_missing_command_is_left_out_of_the_logbook_note() {
    let missing = vec!["timeout".to_string()];

    assert!(starts_with_missing("timeout 200 pnpm exec vitest run", &missing));
    assert!(!starts_with_missing("pnpm exec vitest run", &missing));
    assert!(!starts_with_missing("pnpm exec timeout", &missing));
    assert!(!starts_with_missing("", &missing));
}
