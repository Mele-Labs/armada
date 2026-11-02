//! The instrument kind: an `instrument.js` Bridge keeps mounted for a bay of the console. Checked as a tab's `tab.js` is,
//! handed to Bridge as it was checked, and the mod's own problem where it could not load.

use std::path::Path;

use api::Mods;
use ipc::{ModKind, ScaffoldMod};
use testkit::FakeWorkProduct;

use crate::mods::scaffold;
use crate::mods::{examine, scan, tab, BuiltIn, BuiltIns};
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

const GOOD: &str = "const paint = (draw, { w, h, ink }) => { draw.fillStyle = ink.run; draw.fillRect(0, 0, w, h); };\nexport default function Reviews({ host }) {\n  host.React.useEffect(() => { host.face(paint); }, []);\n  return null;\n}\n";

fn an_instrument(dir: &Path, name: &str, manifest_extra: &str, file: &str, text: &str) {
    let at = dir.join(name);
    std::fs::create_dir_all(&at).expect("a folder");
    std::fs::write(at.join("mod.toml"), format!("name = \"{name}\"\nkind = \"instrument\"\nversion = \"0.1.0\"\n{manifest_extra}")).expect("mod.toml");
    std::fs::write(at.join(file), text).expect("the file");
}

fn said(problems: &[String], wanted: &str) -> bool {
    problems.iter().any(|one| one.contains(wanted))
}

#[test]
fn an_instrument_that_could_not_load_is_refused_naming_instrument_js() {
    let cases: Vec<(String, &str)> = vec![
        (String::new(), "instrument.js is empty"),
        ("export const Reviews = () => null;\n".to_string(), "instrument.js has no `export default`, which must be the instrument's component"),
        (format!("import React from \"react\";\n{GOOD}"), "instrument.js line 1: an `import` cannot be resolved, the instrument is handed `host` instead"),
        (format!("export * from \"./x.js\";\n{GOOD}"), "instrument.js line 1: an `export ... from`"),
    ];
    for (text, wanted) in cases {
        let found = tab::problems_in(&text, "instrument.js", "instrument");
        assert!(said(&found, wanted), "{text:?} should be refused for {wanted:?}: {found:?}");
    }
    assert_eq!(tab::problems_in(GOOD, "instrument.js", "instrument"), Vec::<String>::new());
}

#[test]
fn a_tabs_check_still_names_tab_js() {
    assert_eq!(tab::problems(""), vec!["tab.js is empty".to_string()]);
    assert!(said(&tab::problems("import x from \"y\";\n"), "tab.js line 1: an `import` cannot be resolved, the tab is handed `host` instead"));
}

#[test]
fn an_instrument_mod_is_listed_with_its_glyph_and_reads_from_instrument_js_alone() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    an_instrument(&mods, "reviews", "glyph = \"git-pull-request\"\n", "instrument.js", GOOD);
    an_instrument(&mods, "holds-a-tab", "", "tab.js", GOOD);
    an_instrument(&mods, "bad-code", "", "instrument.js", "const nothing = 1;\n");
    an_instrument(&mods, "shouted", "glyph = \"Git Pull\"\n", "instrument.js", GOOD);
    an_instrument(&mods, "huge", "", "instrument.js", &format!("{GOOD}// {}\n", "x".repeat(tab::MOST_BYTES as usize)));

    let list = scan(&mods, &Default::default(), &Default::default());

    let row = |name: &str| list.mods.iter().find(|one| one.name == name).expect(name).clone();
    let good = row("reviews");
    assert_eq!((good.kind, good.valid, good.glyph.as_deref()), (Some(ModKind::Instrument), true, Some("git-pull-request")));
    assert_eq!(row("holds-a-tab").reason.as_deref(), Some("instrument.js is missing"));
    assert!(row("bad-code").reason.expect("why").starts_with("instrument.js has no `export default`"));
    assert!(row("shouted").reason.expect("why").contains("`glyph` must be 1 to 40 lowercase letters"));
    assert!(row("huge").reason.expect("why").starts_with("instrument.js is larger than"));
    assert!(examine(&mods, "reviews").files.is_some() && examine(&mods, "bad-code").files.is_none());
}

#[test]
fn a_scaffolded_instrument_is_valid_at_once_wears_a_registry_glyph_and_paints_with_ink() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    scaffold::make(&mods, "reviews", Some("Waiting pull requests"), ModKind::Instrument, None).expect("made");

    let found = examine(&mods, "reviews");

    assert!(found.valid(), "{:?}", found.problems);
    assert_eq!(found.manifest.kind, Some(ModKind::Instrument));
    assert_eq!(found.manifest.glyph.as_deref(), Some("gauge"));
    assert!(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../packages/icons/icons/gauge.toml")).is_file(), "the glyph is in the registry");
    let starter = std::fs::read_to_string(mods.join("reviews/instrument.js")).expect("the starter");
    for wanted in ["host.face(paint)", "ink.run", "export default function Instrument({ host })", "return el("] {
        assert!(starter.contains(wanted), "{wanted}: {starter}");
    }
}

#[test]
fn a_built_in_instrument_is_checked_from_the_text_compiled_in() {
    let home = TempDir::new();
    let toml = "name = \"reviews\"\nkind = \"instrument\"\nversion = \"0.1.0\"\n";
    let shipped = BuiltIns::of(vec![BuiltIn::new("reviews", &[("mod.toml", toml), ("instrument.js", GOOD)])]);

    let list = scan(&home.path().join("mods"), &Default::default(), &shipped);

    assert_eq!((list.mods[0].name.as_str(), list.mods[0].kind, list.mods[0].valid), ("built-in:reviews", Some(ModKind::Instrument), true), "{list:?}");
}

#[tokio::test]
async fn fleet_scaffolds_validates_and_hands_bridge_the_instrument_as_it_was_checked() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let made = fleet
        .scaffold_mod(ScaffoldMod { name: "reviews".into(), description: None, kind: Some(ModKind::Instrument), session_id: None })
        .await
        .expect("scaffolded");
    let starter = fleet.validate_mod("reviews".into()).await.expect("checked");
    assert!(starter.valid && starter.instrument.is_some(), "a starter is a module that passes: {starter:?}");

    std::fs::write(Path::new(&made.path).join("instrument.js"), GOOD).expect("edited");
    let checked = fleet.validate_mod("reviews".into()).await.expect("checked");
    let module = checked.instrument.expect("the module");
    assert_eq!((module.glyph.as_deref(), module.code.as_str()), (Some("gauge"), GOOD));
    assert!(checked.tab.is_none() && checked.css.is_none() && checked.layout.is_none(), "only what its kind has");

    std::fs::write(Path::new(&made.path).join("instrument.js"), "const nothing = 1;\n").expect("broken");
    let broken = fleet.validate_mod("reviews".into()).await.expect("answers for a broken mod");
    assert!(!broken.valid && broken.instrument.is_none(), "{broken:?}");
}
