//! Layout mods: a `layout.json` in a mod folder, checked by the rules `parseLayout` applies in
//! Bridge, handed on as the text that was checked, and listed even when it is broken. A theme's
//! behaviour is `mods.rs`' and does not change. `docs/concepts/layout-mods.md`.

use std::path::Path;

use adapters::GitVcs;
use api::{Mods, Settings};
use ipc::{ModKind, PromoteMod, SavePreference, ScaffoldMod};
use testkit::{FakeHarness, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::mods::{examine, scan};
use crate::tests::daemon::{a_fleet, fitted_over};
use crate::tests::reclaim::{a_repository, commit, git};
use crate::tests::tmp::TempDir;

const TOML: &str = "name = \"tidy\"\nkind = \"layout\"\nversion = \"0.1.0\"\ndescription = \"Merge line first\"\n";
const GOOD: &str = "{\n  \"version\": 1,\n  \"dashboard.panels\": { \"order\": [\"merge-line\", \"fleet\"] },\n  \"rail\": { \"hidden\": [\"lessons\"] }\n}\n";

fn a_layout_mod(dir: &Path, name: &str, toml: &str, layout: &str) {
    let at = dir.join(name);
    std::fs::create_dir_all(&at).expect("a folder");
    std::fs::write(at.join("mod.toml"), toml).expect("mod.toml");
    std::fs::write(at.join("layout.json"), layout).expect("layout.json");
}

fn layout_scaffold(name: &str) -> ScaffoldMod {
    ScaffoldMod { name: name.into(), description: Some("Merge line first".into()), kind: Some(ModKind::Layout) }
}

#[test]
fn a_layout_mod_is_valid_and_its_checked_text_is_what_it_holds() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_layout_mod(&mods, "tidy", TOML, GOOD);
    let found = examine(&mods, "tidy");
    assert!(found.valid(), "{:?}", found.problems);
    assert_eq!(found.manifest.kind, Some(ModKind::Layout));
    let files = found.files.expect("the checked files");
    assert_eq!(files.payload.file(), "layout.json");
    assert_eq!(files.payload.text(), GOOD);
}

#[test]
fn a_layout_mod_is_checked_by_the_layout_rules_and_not_the_stylesheet_ones() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    for (name, layout, why) in [
        ("not-json", "{ not json", "not JSON"),
        ("wrong-version", "{\"version\": 2}", "version is not 1"),
        ("extra-field", "{\"version\": 1, \"rail\": {\"colour\": \"red\"}}", "rail.colour is not a field"),
        ("bad-id", "{\"version\": 1, \"rail\": {\"hidden\": [\"Lessons\"]}}", "rail.hidden is not a list"),
        ("css-in-it", ":root { --accent: red; }", "not JSON"),
    ] {
        a_layout_mod(&mods, name, &TOML.replace("tidy", name), layout);
        let found = examine(&mods, name);
        assert!(!found.valid(), "{name}");
        assert!(found.files.is_none(), "{name}: nothing is handed on");
        assert!(found.problems.iter().any(|said| said.contains(why)), "{name}: {:?}", found.problems);
    }
    // An id and a region this build does not have are ignored, and the file passes.
    a_layout_mod(&mods, "future", &TOML.replace("tidy", "future"), "{\"version\": 1, \"sidebar\": {\"order\": [\"x\"]}, \"rail\": {\"hidden\": [\"gone\"]}}");
    assert!(examine(&mods, "future").valid());
    // 4 KiB and over is refused before it is read.
    a_layout_mod(&mods, "huge", &TOML.replace("tidy", "huge"), &format!("{{\"version\": 1}}{}", " ".repeat(5000)));
    let huge = examine(&mods, "huge");
    assert!(huge.problems.iter().any(|said| said.contains("larger than")), "{:?}", huge.problems);
}

#[test]
fn a_layout_mod_with_no_layout_file_says_so_and_does_not_ask_for_a_stylesheet() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    std::fs::create_dir_all(mods.join("tidy")).expect("folder");
    std::fs::write(mods.join("tidy/mod.toml"), TOML).expect("toml");
    let found = examine(&mods, "tidy");
    assert_eq!(found.problems, ["layout.json is missing"]);
}

/// The list of a folder with a broken layout mod in it among good ones, themes included, still answers.
#[test]
fn a_list_with_a_broken_layout_mod_is_still_a_list_and_the_themes_in_it_are_as_they_were() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_layout_mod(&mods, "tidy", TOML, GOOD);
    a_layout_mod(&mods, "broken", &TOML.replace("tidy", "broken"), "{");
    let calm = mods.join("calm");
    std::fs::create_dir_all(&calm).expect("folder");
    std::fs::write(calm.join("mod.toml"), "name = \"calm\"\nkind = \"theme\"\nversion = \"0.1.0\"\n").expect("toml");
    std::fs::write(calm.join("theme.css"), ":root { --accent: #4A9EDB; }\n").expect("css");

    let list = scan(&mods, &Default::default());
    let row = |name: &str| list.mods.iter().find(|one| one.name == name).unwrap_or_else(|| panic!("{name} is listed")).clone();
    assert!(row("tidy").valid && row("tidy").kind == Some(ModKind::Layout));
    assert!(!row("broken").valid && row("broken").reason.as_deref() == Some("not JSON"));
    assert!(row("calm").valid && row("calm").kind == Some(ModKind::Theme));

    // A theme whose stylesheet is wrong says what it always said: a layout rule never reads it.
    std::fs::write(calm.join("theme.css"), "{ \"version\": 1 }").expect("not css");
    assert!(!examine(&mods, "calm").valid());
}

#[tokio::test]
async fn a_scaffolded_layout_validates_at_once_and_changes_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let made = fleet.scaffold_mod(layout_scaffold("tidy")).await.expect("scaffolded");
    assert!(made.path.ends_with("/mods/tidy"), "{}", made.path);

    let toml = std::fs::read_to_string(home.path().join("mods/tidy/mod.toml")).expect("mod.toml");
    assert!(toml.contains("kind = \"layout\""), "{toml}");
    assert!(!home.path().join("mods/tidy/theme.css").exists(), "a layout mod holds no stylesheet");
    let checked = fleet.validate_mod("tidy".into()).await.expect("checked");
    assert!(checked.valid && checked.problems.is_empty(), "{:?}", checked.problems);
    assert!(checked.css.is_none(), "a layout mod hands on no css");
    let text = checked.layout.expect("the checked layout");
    assert!(ipc::layout::problems(&text).is_empty());
    assert_eq!(text.split_whitespace().collect::<String>(), "{\"version\":1}", "no region, so no change");

    let log = git(&home.path().join("mods/tidy"), &["log", "--format=%s"]);
    assert_eq!(log.trim(), "Start the tidy layout");
    let listed = fleet.list_mods().await.expect("listed");
    assert_eq!(listed.mods[0].kind, Some(ModKind::Layout));

    // A theme scaffold with no kind is what it was.
    fleet
        .scaffold_mod(ScaffoldMod { name: "calm".into(), description: None, kind: None })
        .await
        .expect("a theme");
    let calm = fleet.validate_mod("calm".into()).await.expect("checked");
    assert!(calm.valid && calm.css.is_some() && calm.layout.is_none());
}

#[tokio::test]
async fn a_layout_mod_edited_to_a_wrong_shape_is_invalid_and_hands_nothing_on() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    fleet.scaffold_mod(layout_scaffold("tidy")).await.expect("scaffolded");
    std::fs::write(home.path().join("mods/tidy/layout.json"), GOOD).expect("edit");
    assert_eq!(fleet.validate_mod("tidy".into()).await.expect("checked").layout.as_deref(), Some(GOOD));

    std::fs::write(home.path().join("mods/tidy/layout.json"), "{\"version\": 1, \"rail\": {\"order\": \"lessons\"}}").expect("edit");
    let broken = fleet.validate_mod("tidy".into()).await.expect("answers for a broken mod");
    assert!(!broken.valid && broken.layout.is_none() && broken.css.is_none());
    assert!(broken.problems[0].contains("rail.order"), "{:?}", broken.problems);
}

#[tokio::test]
async fn the_owners_layout_choices_are_a_preference_checked_by_the_same_rules() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    assert_eq!(fleet.get_preferences().await.expect("reads").layout_choices, "");

    let save = |text: Option<&str>| SavePreference { name: "layout_choices".into(), value: false, text: text.map(str::to_string) };
    let saved = fleet.save_preferences(save(Some(GOOD))).await.expect("saved");
    // settings.json holds the choices as JSON, so the text comes back as the
    // same value, compact and with its keys in order.
    let good = store::settings_file::SettingValue::from_json_text(GOOD).expect("JSON").to_json_text();
    assert_eq!(saved.layout_choices, good);
    assert_eq!(fleet.get_preferences().await.expect("reads").layout_choices, good);
    assert_eq!(fleet.get_preferences().await.expect("reads").theme, "dark", "the theme is not touched");

    for bad in ["{", "{\"version\": 2}", "{\"version\": 1, \"rail\": {\"hidden\": [\"Lessons\"]}}"] {
        let refused = fleet.save_preferences(save(Some(bad))).await.expect_err("not a layout");
        assert_eq!((refused.status(), refused.error().code.clone()), (422, "fleet.unacceptable_layout".to_string()), "{bad}");
    }
    assert_eq!(fleet.get_preferences().await.expect("reads").layout_choices, good, "nothing moved");

    let cleared = fleet.save_preferences(save(Some(""))).await.expect("empty takes the choices back");
    assert_eq!(cleared.layout_choices, "");
    let nothing = fleet.save_preferences(save(None)).await.expect("so does no text");
    assert_eq!(nothing.layout_choices, "");
}

#[tokio::test]
async fn a_layout_mod_is_promoted_as_its_two_files() {
    let home = TempDir::new();
    a_repository(&home);
    std::fs::create_dir_all(home.path().join("packages")).expect("packages/");
    std::fs::write(home.path().join("packages/README"), "packages\n").expect("a file");
    git(home.path(), &["add", "packages/README"]);
    commit(home.path(), "packages/");
    let fleet: Fleet<FakeHarness, GitVcs, FakeWorkProduct> =
        Fleet::assembled(fitted_over(&home, FakeWorkProduct::changed(&[]), FakeHarness::that_listens(), GitVcs::new()));
    fleet.scaffold_mod(layout_scaffold("tidy")).await.expect("scaffolded");
    std::fs::write(home.path().join("mods/tidy/layout.json"), GOOD).expect("edit");

    let promoted = fleet.promote_mod(PromoteMod { name: "tidy".into(), manifest_id: None }).await.expect("promoted");
    let tree = git(home.path(), &["ls-tree", "-r", "--name-only", &promoted.branch]);
    let files: Vec<&str> = tree.lines().filter(|path| path.starts_with("packages/mods/")).collect();
    assert_eq!(files, ["packages/mods/tidy/layout.json", "packages/mods/tidy/mod.toml"]);
    assert_eq!(git(home.path(), &["show", &format!("{}:packages/mods/tidy/layout.json", promoted.branch)]), GOOD);
    assert_eq!(git(home.path(), &["log", "-1", "--format=%s", &promoted.branch]).trim(), "Add the tidy layout mod");
}

/// The two examples in the armada-mods skill, as written there; the TypeScript side reads the skill
/// itself, and a change to them is a change to this test too.
#[test]
fn the_two_layout_examples_a_session_is_taught_from_pass() {
    let tidy = "{\n  \"version\": 1,\n  \"dashboard.panels\": { \"order\": [\"merge-line\", \"fleet\"] },\n  \"rail\": { \"hidden\": [\"lessons\"] }\n}\n";
    let job = "{\n  \"version\": 1,\n  \"job.tabs\": { \"order\": [\"overview\", \"workflow\", \"record\", \"plan\"], \"hidden\": [\"pulse\"], \"first\": \"plan\" }\n}\n";
    for text in [tidy, job] {
        assert_eq!(ipc::layout::problems(text), Vec::<String>::new(), "{text}");
    }
}
