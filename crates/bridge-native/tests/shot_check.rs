//! `--shot-check` run as the app runs it, headless. gpui's Mac platform must be made on the process's main
//! thread, which a test thread is not, so each check is its own process, as `walks.rs` plays a walk.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// A folder of this test's own, with the sample scene's text changed by `edit` and its fixture beside it.
fn scene_in(name: &str, edit: impl Fn(&str) -> String) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shot-check-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder for the scene");
    let text = std::fs::read_to_string(fixtures().join("prototype-scene.html")).expect("the sample scene");
    std::fs::write(dir.join("scene.html"), edit(&text)).expect("the scene written");
    std::fs::copy(fixtures().join("prototype-jobs.json"), dir.join("prototype-jobs.json")).expect("its fixture");
    dir
}

fn check(dir: &Path, extra: &[&str]) -> (bool, serde_json::Value) {
    let out = dir.join("out");
    let run = Command::new(env!("CARGO_BIN_EXE_bridge-native"))
        .args(["--shot-check".as_ref(), dir.join("scene.html").as_os_str(), "--out".as_ref(), out.as_os_str()])
        .args(extra)
        .output()
        .expect("the app runs");
    let report = std::fs::read_to_string(out.join("report.json")).unwrap_or_else(|_| panic!("no report.json: {}", String::from_utf8_lossy(&run.stderr)));
    (run.status.success(), ipc::decode("report.json", report.as_bytes()).expect("the report is JSON"))
}

#[test]
fn a_scene_set_against_itself_matches_in_every_state() {
    let dir = scene_in("same", str::to_string);
    let again = dir.join("again.html");
    std::fs::copy(dir.join("scene.html"), &again).unwrap();
    let (passed, report) = check(&dir, &["--against", again.to_str().unwrap()]);
    assert!(passed, "{report}");
    assert_eq!(report["ok"], true);
    let states: Vec<_> = report["pairs"].as_array().unwrap().iter().map(|pair| (pair["state"].as_str().unwrap().to_string(), pair["status"].as_str().unwrap().to_string())).collect();
    assert_eq!(states, vec![("list".to_string(), "matched".to_string()), ("detail".to_string(), "matched".to_string())]);
    assert!(dir.join("out/list-side-by-side.png").exists() && dir.join("out/list-difference.png").exists());
}

#[test]
fn a_scene_whose_part_moved_drifts_and_the_check_fails() {
    let dir = scene_in("moved", str::to_string);
    let moved = dir.join("moved.html");
    let text = std::fs::read_to_string(dir.join("scene.html")).unwrap().replace("<state id=\"list\">", "<state id=\"list\"><box h=\"4\"></box>");
    std::fs::write(&moved, text).unwrap();
    let (passed, report) = check(&dir, &["--against", moved.to_str().unwrap()]);
    assert!(!passed, "a moved list must fail the check: {report}");
    assert_eq!(report["pairs"][0]["status"], "drifted");
    assert_eq!(report["pairs"][1]["status"], "matched");
}

#[test]
fn a_state_the_walk_has_no_step_for_is_missing_and_the_check_fails() {
    let dir = scene_in("missing", |text| text.replacen("<scene ", "<scene walk=\"jobs-list\" ", 1));
    let (passed, report) = check(&dir, &[]);
    assert!(!passed, "{report}");
    assert_eq!(report["walk"], "jobs-list");
    let statuses: Vec<_> = report["pairs"].as_array().unwrap().iter().map(|pair| pair["status"].as_str().unwrap()).collect();
    assert_eq!(statuses, vec!["missing", "missing"]);
}
