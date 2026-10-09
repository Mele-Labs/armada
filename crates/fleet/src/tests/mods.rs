//! Mods: what a folder anybody can write into may say, and that however wrong
//! it is the list still answers. `docs/concepts/mods.md`.
//!
//! The reading and checking are exercised against real folders; the Fleet half
//! (switches, theme, the event) is `Fleet` over a temporary home.

use std::path::Path;
use std::time::Duration;

use api::{Mods, Refusal};
use ipc::{ModKind, ModList, ModSummary, SavePreference};
use testkit::FakeWorkProduct;

use crate::mods::manifest;
use crate::mods::scaffold;
use crate::mods::stylesheet;
use crate::mods::{examine, scan, slug_problem};
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

const GOOD_TOML: &str = "name = \"calm\"\nkind = \"theme\"\nversion = \"0.1.0\"\ndescription = \"Softer greys\"\n";
const GOOD_CSS: &str = ":root {\n  --bg-base: #101418;\n  --accent: #4A9EDB;\n}\n";

fn a_mod(dir: &Path, name: &str, toml: &str, css: &str) {
    let at = dir.join(name);
    std::fs::create_dir_all(&at).expect("a folder");
    std::fs::write(at.join("mod.toml"), toml).expect("mod.toml");
    std::fs::write(at.join("theme.css"), css).expect("theme.css");
}

fn only(list: &ModList, name: &str) -> ModSummary {
    list.mods
        .iter()
        .find(|one| one.name == name)
        .unwrap_or_else(|| panic!("{name} is listed: {:?}", list.mods.iter().map(|m| &m.name).collect::<Vec<_>>()))
        .clone()
}

#[test]
fn the_token_list_is_read_from_the_file_the_gate_keeps_current() {
    let found = stylesheet::problems(":root { --bg-base: #000; --not-a-token: red; }");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`--not-a-token` is not a design token"), "{found:?}");
}

#[test]
fn a_stylesheet_of_tokens_in_root_passes() {
    assert_eq!(stylesheet::problems(GOOD_CSS), Vec::<String>::new());
    let with_comments = "/* a comment\nover two lines */\n:root { --accent: rgb(10 20 30 / 0.5); /* c */ --font-sans: \"IBM Plex Sans\", sans-serif; }\n[data-theme=\"calm\"] { --bg-raised: color-mix(in oklab, #111 80%, #fff); }\n";
    assert_eq!(stylesheet::problems(with_comments), Vec::<String>::new());
}

#[test]
fn what_a_theme_may_not_say_is_each_a_problem_with_its_line() {
    let cases: [(&str, &str); 14] = [
        (":root { --accent: url(https://x.example/a.png); }", "`url(`"),
        ("@import \"https://x.example/a.css\";\n:root { --accent: red; }", "@import is not accepted"),
        ("@media (min-width: 1px) { :root { --accent: red; } }", "@media is not accepted"),
        ("body { --accent: red; }", "is not a selector a theme may use"),
        (":root .child { --accent: red; }", "is not a selector a theme may use"),
        (":root { color: red; }", "`color` is not a design token"),
        (":root { --accent: red !important; }", "`!`"),
        (":root { --accent: \\75rl(x); }", "backslash"),
        (":root { --accent: expression(alert(1)); }", "`expression(`"),
        (":root { --accent: javascript:alert(1); }", "`javascript:`"),
        (":root { --accent: <b>; }", "`<`"),
        (":root { --accent: red;", "never closed"),
        (":root { /* never closed ", "comment is never closed"),
        (":root { :root { --accent: red; } }", "inside a block"),
    ];
    for (css, wanted) in cases {
        let found = stylesheet::problems(css);
        assert!(
            found.iter().any(|one| one.contains(wanted)),
            "{css:?} should be refused for {wanted:?}, got {found:?}"
        );
    }
    let on_line_three = stylesheet::problems(":root {\n  --accent: red;\n  --bg-base: url(x);\n}\n");
    assert!(on_line_three[0].starts_with("theme.css line 3:"), "{on_line_three:?}");
}

#[test]
fn a_stylesheet_with_many_problems_says_how_many_it_stopped_listing() {
    let css = ":root { ".to_string() + &"--nope: 1; ".repeat(40) + "}";
    let found = stylesheet::problems(&css);
    assert!(found.len() <= 21, "{}", found.len());
    assert!(found.last().expect("something").starts_with("and "), "{found:?}");
}

#[test]
fn a_manifest_of_four_keys_reads_and_anything_else_is_a_problem() {
    let (read, problems) = manifest::read(GOOD_TOML, "calm");
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(read.version.as_deref(), Some("0.1.0"));
    assert_eq!(read.description.as_deref(), Some("Softer greys"));

    let cases = [
        ("name = \"other\"\nkind = \"theme\"\nversion = \"1\"\n", "names `other` but the folder is `calm`"),
        ("name = \"calm\"\nkind = \"widget\"\nversion = \"1\"\n", "not a kind of mod"),
        ("name = \"calm\"\nkind = \"theme\"\n", "no `version`"),
        ("name = \"calm\"\nkind = \"theme\"\nversion = \"1\"\nauthor = \"me\"\n", "`author` is not a key"),
        ("name = \"calm\"\nname = \"calm\"\nkind = \"theme\"\nversion = \"1\"\n", "given twice"),
        ("[mod]\nname = \"calm\"\n", "expected `key = \"value\"`"),
        ("name = \"calm\nkind = \"theme\"\nversion = \"1\"\n", "closed"),
        ("name = \"calm\"\nkind = \"theme\"\nversion = 1\n", "double-quoted string"),
        ("name = \"calm\"\nkind = \"theme\"\nversion = \"1 2\"\n", "`version` must be"),
    ];
    for (text, wanted) in cases {
        let (_, found) = manifest::read(text, "calm");
        assert!(
            found.iter().any(|one| one.contains(wanted)),
            "{text:?} should say {wanted:?}, got {found:?}"
        );
    }
}

#[test]
fn a_name_that_could_climb_or_collide_is_not_a_slug() {
    for bad in [
        "", ".", "..", "../x", "a/b", "a b", "Calm", "-calm", "calm.css", "dark", "light", "system",
        "default", &"x".repeat(41), "calm\u{0}",
    ] {
        assert!(slug_problem(bad).is_some(), "{bad:?} should be refused");
    }
    for good in ["calm", "high-contrast", "a1", "0day", &"x".repeat(40)] {
        assert_eq!(slug_problem(good), None, "{good:?}");
    }
}

/// **The list of a folder somebody has written all kinds of things into is
/// still a list**, every bad one a row of its own.
#[test]
fn a_list_with_every_kind_of_broken_mod_in_it_still_answers() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_mod(&mods, "calm", GOOD_TOML, GOOD_CSS);
    a_mod(&mods, "bad-toml", "this is not toml\n", GOOD_CSS);
    a_mod(&mods, "bad-css", &GOOD_TOML.replace("calm", "bad-css"), ":root { --accent: url(x); }");
    a_mod(&mods, "wrong-name", GOOD_TOML, GOOD_CSS);
    a_mod(&mods, "huge", &GOOD_TOML.replace("calm", "huge"), &format!(":root {{ --accent: red; }}{}", " ".repeat(40_000)));
    a_mod(&mods, "binary", &GOOD_TOML.replace("calm", "binary"), "");
    std::fs::write(mods.join("binary/theme.css"), [0xff, 0xfe, 0x00, 0x80]).expect("bytes");
    a_mod(&mods, "Not A Slug", GOOD_TOML, GOOD_CSS);
    std::fs::create_dir_all(mods.join("empty")).expect("an empty folder");
    std::fs::create_dir_all(mods.join(".hidden")).expect("a hidden folder");
    std::fs::write(mods.join("stray-file"), "x").expect("a file");
    std::fs::create_dir_all(mods.join("linked-css")).expect("folder");
    std::fs::write(mods.join("linked-css/mod.toml"), GOOD_TOML.replace("calm", "linked-css")).expect("toml");
    std::os::unix::fs::symlink("/etc/hosts", mods.join("linked-css/theme.css")).expect("a link");
    std::os::unix::fs::symlink(&mods.join("calm"), mods.join("linked-folder")).expect("a link");
    std::fs::create_dir_all(mods.join("pipe-css")).expect("folder");
    std::fs::write(mods.join("pipe-css/mod.toml"), GOOD_TOML.replace("calm", "pipe-css")).expect("toml");
    let made = std::process::Command::new("mkfifo").arg(mods.join("pipe-css/theme.css")).status().expect("mkfifo");
    assert!(made.success());

    let list = scan(&mods, &Default::default());
    let names: Vec<&str> = list.mods.iter().map(|one| one.name.as_str()).collect();
    assert!(!names.contains(&".hidden") && !names.contains(&"stray-file"), "{names:?}");

    assert!(only(&list, "calm").valid);
    for (name, reason) in [
        ("bad-toml", "expected `key = \"value\"`"),
        ("bad-css", "`url(`"),
        ("wrong-name", "names `calm` but the folder is `wrong-name`"),
        ("huge", "larger than"),
        ("binary", "not UTF-8"),
        ("Not A Slug", "lowercase letters"),
        ("empty", "mod.toml is missing"),
        ("linked-css", "theme.css is a link"),
        ("pipe-css", "theme.css is not a regular file"),
        ("linked-folder", "a link, not a folder"),
    ] {
        let row = only(&list, name);
        assert!(!row.valid, "{name} is invalid");
        assert!(
            row.reason.as_deref().is_some_and(|said| said.contains(reason)),
            "{name}: {:?} should say {reason:?}",
            row.reason
        );
    }
    let fine = only(&list, "calm");
    assert_eq!(fine.description.as_deref(), Some("Softer greys"));
    assert!(fine.changed_at.is_some() && fine.reason.is_none());
}

#[test]
fn a_folder_that_is_not_there_is_an_empty_list() {
    let home = TempDir::new();
    assert_eq!(scan(&home.path().join("mods"), &Default::default()), ModList::default());
}

#[test]
fn a_mod_nobody_switched_is_on_and_a_switch_holds() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_mod(&mods, "calm", GOOD_TOML, GOOD_CSS);
    assert!(only(&scan(&mods, &Default::default()), "calm").enabled);
    let off = [("calm".to_string(), false)].into();
    assert!(!only(&scan(&mods, &off), "calm").enabled);
}

#[test]
fn a_scaffold_is_valid_from_the_first_moment_and_has_one_commit() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    let made = scaffold::make(&mods, "calm", Some("Softer \"greys\""), ModKind::Theme).expect("made");
    assert_eq!(made, mods.join("calm"));

    let found = examine(&mods, "calm");
    assert!(found.valid(), "{:?}", found.problems);
    assert_eq!(found.manifest.description.as_deref(), Some("Softer \"greys\""));

    let log = std::process::Command::new("git")
        .arg("-C")
        .arg(&made)
        .args(["log", "--format=%s"])
        .output()
        .expect("git");
    assert_eq!(String::from_utf8_lossy(&log.stdout).trim(), "Start the calm theme");
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(&made)
        .args(["status", "--porcelain"])
        .output()
        .expect("git");
    assert!(status.stdout.is_empty(), "everything is committed");
}

#[test]
fn a_scaffold_refuses_a_name_that_climbs_or_exists_and_leaves_nothing() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    for bad in ["../escape", "a/b", "..", ".", "", "Calm", "dark", "x y"] {
        let refused = scaffold::make(&mods, bad, None, ModKind::Theme).expect_err(bad);
        assert!(matches!(refused, scaffold::Refused::Unacceptable(_)), "{bad:?}: {refused:?}");
    }
    assert!(!home.path().join("escape").exists(), "nothing was made outside the folder");

    scaffold::make(&mods, "calm", None, ModKind::Theme).expect("first");
    std::fs::write(mods.join("calm/theme.css"), "kept").expect("an edit");
    let again = scaffold::make(&mods, "calm", None, ModKind::Theme).expect_err("exists");
    assert!(matches!(&again, scaffold::Refused::Unacceptable(why) if why.contains("already exists")), "{again:?}");
    assert_eq!(std::fs::read_to_string(mods.join("calm/theme.css")).expect("read"), "kept", "the existing mod is untouched");

    let long = "x".repeat(300);
    let refused = scaffold::make(&mods, "fine", Some(&long), ModKind::Theme).expect_err("too long");
    assert!(matches!(refused, scaffold::Refused::Unacceptable(_)));
    assert!(!mods.join("fine").exists());
}

fn api_refusal(refusal: Refusal) -> (u16, String) {
    (refusal.status(), refusal.error().code.clone())
}

#[tokio::test]
async fn scaffold_switch_and_validate_go_through_fleet_and_say_so_on_the_stream() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let mut events = fleet.events().subscribe();

    let made = fleet
        .scaffold_mod(ipc::ScaffoldMod { name: "calm".into(), description: None, kind: None })
        .await
        .expect("scaffolded");
    assert!(made.path.ends_with("/mods/calm"), "{}", made.path);
    let heard = tokio::time::timeout(Duration::from_secs(5), events.next())
        .await
        .expect("an event")
        .expect("open");
    let api::Next::Send(delivered) = heard else { panic!("{heard:?}") };
    let ipc::Event::ModsChanged(list) = delivered.event else { panic!("{:?}", delivered.event) };
    assert_eq!(list.mods.len(), 1);
    assert!(list.mods[0].valid && list.mods[0].enabled);

    let checked = fleet.validate_mod("calm".into()).await.expect("checked");
    assert!(checked.valid && checked.problems.is_empty());
    assert!(checked.css.expect("the checked text").contains("--bg-base"));

    std::fs::write(home.path().join("mods/calm/theme.css"), ":root { --accent: url(x); }").expect("edit");
    let broken = fleet.validate_mod("calm".into()).await.expect("answers for a broken mod");
    assert!(!broken.valid && broken.css.is_none());
    assert!(broken.problems[0].contains("`url(`"), "{:?}", broken.problems);

    let off = fleet
        .set_mod_enabled(ipc::SetModEnabled { name: "calm".into(), enabled: false })
        .await
        .expect("switched");
    assert!(!off.enabled && !off.valid, "off, and still invalid");
    assert!(!only(&fleet.list_mods().await.expect("listed"), "calm").enabled);

    let missing = fleet.validate_mod("nothing".into()).await.expect_err("no such mod");
    assert_eq!(api_refusal(missing), (404, "fleet.no_such_mod".to_string()));
    let climbing = fleet.validate_mod("../armada.db".into()).await.expect_err("not a slug");
    assert_eq!(api_refusal(climbing), (422, "fleet.unacceptable_mod".to_string()));
    let again = fleet
        .scaffold_mod(ipc::ScaffoldMod { name: "calm".into(), description: None, kind: None })
        .await
        .expect_err("exists");
    assert_eq!(api_refusal(again), (422, "fleet.unacceptable_mod".to_string()));
}

/// The rescan says a folder changed once, and not at all when it did not.
#[tokio::test]
async fn the_rescan_publishes_a_mod_a_session_wrote_and_is_quiet_otherwise() {
    let home = TempDir::new();
    let fleet = std::sync::Arc::new(a_fleet(&home, FakeWorkProduct::changed(&[])));
    let mut events = fleet.events().subscribe();
    let watching = crate::mods::keep_reading(std::sync::Arc::clone(&fleet), Duration::from_millis(20));

    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(100), events.next()).await.is_err(),
        "a quiet folder publishes nothing, and the first scan is only a baseline"
    );

    // Made beside and moved in, so a scan never meets it half written.
    a_mod(&home.path().join("staging"), "calm", GOOD_TOML, GOOD_CSS);
    std::fs::create_dir_all(home.path().join("mods")).expect("the folder");
    std::fs::rename(home.path().join("staging/calm"), home.path().join("mods/calm")).expect("moved in");
    let heard = tokio::time::timeout(Duration::from_secs(5), events.next())
        .await
        .expect("an event")
        .expect("open");
    let api::Next::Send(delivered) = heard else { panic!("{heard:?}") };
    let ipc::Event::ModsChanged(list) = delivered.event else { panic!("{:?}", delivered.event) };
    assert_eq!(list.mods.len(), 1);

    assert!(
        tokio::time::timeout(Duration::from_millis(200), events.next()).await.is_err(),
        "said once"
    );
    watching.abort();
}

#[tokio::test]
async fn the_theme_is_a_preference_that_defaults_to_dark_and_refuses_what_a_mod_could_not_be_called() {
    use api::{Commands, Queries};
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    assert_eq!(fleet.get_preferences().await.expect("reads").theme, "dark");

    let saved = fleet
        .save_preferences(SavePreference { name: "theme".into(), value: false, text: Some("calm".into()) })
        .await
        .expect("saved");
    assert_eq!(saved.theme, "calm");
    assert_eq!(fleet.get_preferences().await.expect("reads").theme, "calm");

    for text in [None, Some(String::new()), Some("Calm".into()), Some("../x".into())] {
        let refused = fleet
            .save_preferences(SavePreference { name: "theme".into(), value: true, text })
            .await
            .expect_err("not a theme");
        assert_eq!(api_refusal(refused), (422, "fleet.unacceptable_theme".to_string()));
    }
    assert_eq!(fleet.get_preferences().await.expect("reads").theme, "calm", "nothing moved");

    let shipped = fleet
        .save_preferences(SavePreference { name: "theme".into(), value: false, text: Some("light".into()) })
        .await
        .expect("a shipped id is a theme id");
    assert_eq!(shipped.theme, "light");

    let catalogue = fleet
        .save_preferences(SavePreference { name: "theme".into(), value: false, text: Some("catalogue:nord".into()) })
        .await
        .expect("a catalogue theme's id is a theme id");
    assert_eq!(catalogue.theme, "catalogue:nord");
    for text in ["catalogue:", "catalogue:Nord", "catalogue:../x", "other:nord"] {
        fleet
            .save_preferences(SavePreference { name: "theme".into(), value: false, text: Some(text.into()) })
            .await
            .expect_err("not a theme");
    }
}

/// The two examples in the armada-mods skill, as written there. A skill is not a
/// file this crate may read, so a change to them is a change to this test too.
#[test]
fn the_two_examples_a_session_is_taught_from_pass() {
    let warmer = ":root {\n  --bg-base: #17130F;\n  --bg-sunken: #110E0B;\n  --bg-raised: #1F1A15;\n  --bg-overlay: #29231C;\n  --bg-hover: #322B23;\n  --border-subtle: #2E2820;\n  --border-default: #40372C;\n  --fg-default: #EFE6DA;\n  --fg-muted: #B1A292;\n  --accent: #E0954A;\n  --accent-hover: #EBA763;\n}\n";
    let contrast = ":root {\n  --bg-base: #000000;\n  --bg-raised: #0B0B0B;\n  --border-default: #6B7684;\n  --border-strong: #9AA6B5;\n  --fg-default: #FFFFFF;\n  --fg-muted: #D0D7E0;\n  --accent: #6CB8F0;\n}\n";
    for css in [warmer, contrast] {
        assert_eq!(stylesheet::problems(css), Vec::<String>::new(), "{css}");
    }
}
