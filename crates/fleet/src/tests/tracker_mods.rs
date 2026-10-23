//! Tracker mods: checked without being run, tried against a repository Fleet serves, and handed to
//! a Manifest naming one only while the file it tried is the file that is there.
//! `docs/concepts/mods.md`, *Tracker mods*.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use adapters::TaughtTrackers;
use api::Mods;
use ipc::{ManifestId, ModKind, TryTrackerMod};
use testkit::FakeWorkProduct;

use crate::mods::{scan, TrackerMods};
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

const MOD_TOML: &str = "name = \"linear\"\nkind = \"tracker\"\nversion = \"0.1.0\"\n";

/// A fake tracker CLI in `dir` that touches `ran` whenever it runs, lists `LIN-1` and reads it.
fn fake_cli(dir: &Path) -> String {
    let path = dir.join("fake-tracker");
    let ran = dir.join("ran");
    let issue = r#"{"id":"LIN-1","title":"Fix it","url":"https://t.example/LIN-1","done":false}"#;
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\ntouch {}\ncase \"$1\" in\n  secret) echo tok ;;\n  search) echo '[{issue}]' ;;\n  show) echo '{issue}' ;;\nesac\n",
            ran.display()
        ),
    )
    .expect("the script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("executable");
    path.display().to_string()
}

fn tracker_toml(program: &str) -> String {
    format!(
        "[guide]\nlanguage = \"Plain\"\nguide = \"Words.\"\nopen = \"open\"\nassigned_to_me = \"mine\"\n\n\
         [credential]\nargv = [\"{program}\", \"secret\"]\n\n\
         [search]\ntransport = \"cli\"\nargv = [\"{program}\", \"search\", \"{{query}}\"]\n\
         map = \"map({{reference: .id, title, url, closed: .done}})\"\n\n\
         [read]\ntransport = \"cli\"\nargv = [\"{program}\", \"show\", \"{{reference}}\"]\n\
         map = \"{{reference: .id, title, url, closed: .done}}\"\n"
    )
}

fn a_tracker_mod(mods: &Path, program: &str) {
    let at = mods.join("linear");
    std::fs::create_dir_all(&at).expect("a folder");
    std::fs::write(at.join("mod.toml"), MOD_TOML).expect("mod.toml");
    std::fs::write(at.join("tracker.toml"), tracker_toml(program)).expect("tracker.toml");
}

#[test]
fn scanning_a_tracker_mod_runs_none_of_its_commands() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    let program = fake_cli(home.path());
    a_tracker_mod(&mods, &program);

    let list = scan(&mods, &BTreeMap::new());
    let row = &list.mods[0];
    assert_eq!((row.kind, row.valid, row.tried.is_some()), (Some(ModKind::Tracker), true, false), "{row:?}");
    let refused = TrackerMods::in_dir(mods.clone(), Default::default()).taught("linear").expect("a mod").expect_err("not tried");
    assert!(refused.contains("has not passed a try"), "{refused}");
    assert!(!home.path().join("ran").exists(), "a scan or a lookup ran the tracker's command");
}

#[tokio::test]
async fn a_tracker_mod_is_used_only_while_the_file_it_tried_is_the_file_there() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let mods = home.path().join("mods");
    a_tracker_mod(&mods, &fake_cli(home.path()));
    let manifest_id = ManifestId::carried(fleet.repositories().first().expect("served").manifest().id().as_str());

    let tried = fleet
        .try_tracker_mod(TryTrackerMod { name: "linear".into(), manifest_id: manifest_id.clone() })
        .await
        .expect("tried");
    assert!(tried.passed, "{tried:?}");
    assert_eq!((tried.listed, tried.read.as_deref()), (1, Some("LIN-1")));
    let listed = fleet.list_mods().await.expect("listed");
    assert_eq!(listed.mods[0].tried.as_ref().map(|it| it.passed), Some(true));
    let used = TrackerMods::in_dir(mods.clone(), Default::default()).taught("linear").expect("a mod").expect("tried");
    assert_eq!(used.guide().language, "Plain");

    let edited = format!("{}# edited\n", tracker_toml(&fake_cli(home.path())));
    std::fs::write(mods.join("linear/tracker.toml"), edited).expect("an edit");
    assert_eq!(fleet.list_mods().await.expect("listed").mods[0].tried, None, "an edit undoes the try");
    let refused = TrackerMods::in_dir(mods.clone(), Default::default()).taught("linear").expect("a mod").expect_err("untried");
    assert!(refused.contains("has not passed a try"), "{refused}");

    std::fs::write(mods.join("linear/tracker.toml"), tracker_toml("/nowhere/tracker-cli")).expect("a broken one");
    let failed = fleet
        .try_tracker_mod(TryTrackerMod { name: "linear".into(), manifest_id: manifest_id.clone() })
        .await
        .expect("a failed try is an answer");
    assert!(!failed.passed);
    assert!(failed.reason.as_deref().unwrap_or_default().contains("`/nowhere/tracker-cli` is not installed"), "{failed:?}");
    let refused = TrackerMods::in_dir(mods.clone(), Default::default()).taught("linear").expect("a mod").expect_err("failed");
    assert!(refused.contains("failed its last try"), "{refused}");
    assert_eq!(TrackerMods::in_dir(mods, Default::default()).taught("nothing-here"), None);
}

#[tokio::test]
async fn a_scaffolded_tracker_validates_and_fails_its_try_until_it_names_a_real_command() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    fleet
        .scaffold_mod(ipc::ScaffoldMod { name: "teach".into(), description: None, kind: Some(ModKind::Tracker) })
        .await
        .expect("scaffolded");
    let checked = fleet.validate_mod("teach".into()).await.expect("checked");
    assert!(checked.valid, "{:?}", checked.problems);
    let manifest_id = ManifestId::carried(fleet.repositories().first().expect("served").manifest().id().as_str());
    let tried = fleet
        .try_tracker_mod(TryTrackerMod { name: "teach".into(), manifest_id: manifest_id.clone() })
        .await
        .expect("tried");
    assert!(!tried.passed && tried.reason.as_deref().unwrap_or_default().contains("`tracker-cli` is not installed"), "{tried:?}");

    fleet
        .scaffold_mod(ipc::ScaffoldMod { name: "calm".into(), description: None, kind: None })
        .await
        .expect("a theme");
    let refused = fleet
        .try_tracker_mod(TryTrackerMod { name: "calm".into(), manifest_id })
        .await
        .expect_err("not a tracker");
    assert_eq!((refused.status(), refused.error().code.as_str()), (422, "fleet.unacceptable_mod"));
}

#[tokio::test]
async fn a_tracker_mod_switched_off_in_mods_is_not_used_by_a_manifest() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let mods = home.path().join("mods");
    a_tracker_mod(&mods, &fake_cli(home.path()));
    let manifest_id = ManifestId::carried(fleet.repositories().first().expect("served").manifest().id().as_str());
    let tried = fleet.try_tracker_mod(TryTrackerMod { name: "linear".into(), manifest_id }).await.expect("tried");
    assert!(tried.passed, "{tried:?}");
    let lookup = TrackerMods::in_dir(mods, fleet.host().mod_switches.clone());

    fleet.set_mod_enabled(ipc::SetModEnabled { name: "linear".into(), enabled: false }).await.expect("off");
    let refused = lookup.taught("linear").expect("a mod").expect_err("switched off");
    assert_eq!(refused, "the tracker mod `linear` is switched off in Mods");

    fleet.set_mod_enabled(ipc::SetModEnabled { name: "linear".into(), enabled: true }).await.expect("on");
    assert!(lookup.taught("linear").expect("a mod").is_ok(), "on again, it is used");
}
