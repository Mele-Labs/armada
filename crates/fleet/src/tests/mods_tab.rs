//! The tab kind: a `tab.js` Bridge loads as part of itself. A good one is accepted and handed to Bridge as it was
//! checked; one that could not load, or that `mod.toml` mislabels, is that mod's problem and holds nothing.

use std::path::Path;

use api::Mods;
use ipc::{ModKind, ScaffoldMod};
use testkit::FakeWorkProduct;

use crate::mods::scaffold;
use crate::mods::{examine, scan, tab};
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

const GOOD: &str = "export default function Tab({ host }) {\n  return host.React.createElement(\"p\", null, \"hi\");\n}\n";

fn a_tab(dir: &Path, name: &str, manifest_extra: &str, file: &str, text: &str) {
    let at = dir.join(name);
    std::fs::create_dir_all(&at).expect("a folder");
    std::fs::write(at.join("mod.toml"), format!("name = \"{name}\"\nkind = \"tab\"\nversion = \"0.1.0\"\n{manifest_extra}")).expect("mod.toml");
    std::fs::write(at.join(file), text).expect("the file");
}

fn said(problems: &[String], wanted: &str) -> bool {
    problems.iter().any(|one| one.contains(wanted))
}

#[test]
fn a_module_with_a_default_export_and_no_import_is_accepted() {
    assert_eq!(tab::problems(GOOD), Vec::<String>::new());
    // A dynamic import and `import.meta` are expressions, not statements a module loaded from text cannot resolve.
    let dynamic = format!("const later = () => import(\"x\");\nconst here = import.meta.url;\n{GOOD}");
    assert_eq!(tab::problems(&dynamic), Vec::<String>::new());
}

#[test]
fn a_module_that_could_not_load_is_refused_with_its_line() {
    let cases: Vec<(String, &str)> = vec![
        (String::new(), "tab.js is empty"),
        ("   \n\n".to_string(), "tab.js is empty"),
        ("export const Tab = () => null;\n".to_string(), "has no `export default`"),
        (format!("import React from \"react\";\n{GOOD}"), "tab.js line 1: an `import`"),
        (format!("{GOOD}\n  import {{ x }} from \"./x.js\";\n"), "tab.js line 5: an `import`"),
        (format!("import\"side-effect\";\n{GOOD}"), "tab.js line 1: an `import`"),
        (format!("export * from \"./x.js\";\n{GOOD}"), "tab.js line 1: an `export ... from`"),
    ];
    for (text, wanted) in cases {
        let found = tab::problems(&text);
        assert!(said(&found, wanted), "{text:?} should be refused for {wanted:?}: {found:?}");
    }
}

#[test]
fn a_tab_mod_is_listed_with_its_glyph_and_reads_from_tab_js_alone() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_tab(&mods, "queue", "glyph = \"git-pull-request\"\n", "tab.js", GOOD);
    a_tab(&mods, "no-code", "", "words.toml", GOOD);
    a_tab(&mods, "bad-code", "", "tab.js", "const nothing = 1;\n");
    let list = scan(&mods, &Default::default(), &Default::default());
    let row = |name: &str| list.mods.iter().find(|one| one.name == name).expect(name).clone();
    let good = row("queue");
    assert_eq!((good.kind, good.valid, good.glyph.as_deref()), (Some(ModKind::Tab), true, Some("git-pull-request")));
    assert_eq!(row("no-code").reason.as_deref(), Some("tab.js is missing"));
    assert!(row("bad-code").reason.expect("why").starts_with("tab.js has no `export default`"));
    assert!(examine(&mods, "queue").files.is_some() && examine(&mods, "bad-code").files.is_none());
}

#[test]
fn a_glyph_must_be_a_registry_shaped_name_and_belongs_to_a_tab() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_tab(&mods, "shouted", "glyph = \"Git Pull\"\n", "tab.js", GOOD);
    let found = examine(&mods, "shouted");
    assert!(said(&found.problems, "`glyph` must be 1 to 40 lowercase letters"), "{:?}", found.problems);

    std::fs::create_dir_all(mods.join("dusk")).expect("a folder");
    std::fs::write(mods.join("dusk/mod.toml"), "name = \"dusk\"\nkind = \"theme\"\nversion = \"0.1.0\"\nglyph = \"moon\"\n").expect("mod.toml");
    std::fs::write(mods.join("dusk/theme.css"), ":root {\n  --accent: #7C6FD8;\n}\n").expect("theme.css");
    let found = examine(&mods, "dusk");
    assert!(said(&found.problems, "`glyph` is for a tab mod"), "{:?}", found.problems);
}

#[test]
fn a_tab_that_is_too_large_is_that_mods_problem() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_tab(&mods, "huge", "", "tab.js", &format!("{GOOD}// {}\n", "x".repeat(tab::MOST_BYTES as usize)));
    let found = examine(&mods, "huge");
    assert!(said(&found.problems, "tab.js is larger than"), "{:?}", found.problems);
}

#[test]
fn a_scaffolded_tab_is_valid_at_once_and_wears_the_custom_glyph() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    scaffold::make(&mods, "queue", Some("Pull requests"), ModKind::Tab, None).expect("made");
    let found = examine(&mods, "queue");
    assert!(found.valid(), "{:?}", found.problems);
    assert_eq!(found.manifest.glyph.as_deref(), Some("panel-top"));
    assert!(mods.join("queue/tab.js").is_file());
}

#[tokio::test]
async fn fleet_scaffolds_validates_and_hands_bridge_the_module_as_it_was_checked() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let made = fleet
        .scaffold_mod(ScaffoldMod { name: "queue".into(), description: None, kind: Some(ModKind::Tab), session_id: None })
        .await
        .expect("scaffolded");
    let starter = fleet.validate_mod("queue".into()).await.expect("checked");
    assert!(starter.valid && starter.tab.is_some(), "a starter is a module that passes: {starter:?}");

    std::fs::write(Path::new(&made.path).join("tab.js"), GOOD).expect("edited");
    let checked = fleet.validate_mod("queue".into()).await.expect("checked");
    let module = checked.tab.expect("the module");
    assert_eq!((module.glyph.as_deref(), module.code.as_str()), (Some("panel-top"), GOOD));
    assert!(checked.css.is_none() && checked.layout.is_none() && checked.commands.is_none());

    std::fs::write(Path::new(&made.path).join("tab.js"), "const nothing = 1;\n").expect("broken");
    let broken = fleet.validate_mod("queue".into()).await.expect("answers for a broken mod");
    assert!(!broken.valid && broken.tab.is_none(), "{broken:?}");
}
