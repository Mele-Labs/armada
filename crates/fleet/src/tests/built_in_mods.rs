//! Built-in mods: the folders under `packages/mods/` Fleet is built with, listed beside the
//! machine's own under an id of their own, and held to the rules a local mod is. `crate::mods::built_in`.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use api::{Mods, Refusal, Settings};
use ipc::{ModOrigin, PromoteMod, SavePreference, SetModEnabled};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::mods::{scan, BuiltIn, BuiltIns};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

const CALM_TOML: &str = "name = \"calm\"\nkind = \"theme\"\nversion = \"0.1.0\"\ndescription = \"Softer greys\"\n";
const CALM_CSS: &str = ":root {\n  --bg-base: #101418;\n  --accent: #4A9EDB;\n}\n";

/// A fixture, never a mod in the repository: `calm`, valid, and `loud`, whose stylesheet a theme
/// may not hold.
fn shipped() -> BuiltIns {
    BuiltIns::of(vec![
        BuiltIn::new("calm", &[("mod.toml", CALM_TOML), ("theme.css", CALM_CSS)]),
        BuiltIn::new("loud", &[("mod.toml", "name = \"loud\"\nkind = \"theme\"\nversion = \"0.1.0\"\n"), ("theme.css", ":root { --accent: url(x); }\n")]),
    ])
}

fn a_local_calm(mods: &Path) {
    let at = mods.join("calm");
    std::fs::create_dir_all(&at).expect("a folder");
    std::fs::write(at.join("mod.toml"), CALM_TOML).expect("mod.toml");
    std::fs::write(at.join("theme.css"), ":root {\n  --accent: #7C6FD8;\n}\n").expect("theme.css");
}

/// **Beside, never instead**: a local mod named as a built-in one and changed from it is listed
/// next to it, each under its own id and switched apart; a built-in that breaks a rule is listed with the problem.
#[test]
fn a_built_in_mod_is_listed_beside_a_local_one_of_the_same_name_and_checked_as_one_is() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_local_calm(&mods);
    let switches = BTreeMap::from([("built-in:calm".to_string(), false)]);

    let list = scan(&mods, &switches, &shipped());

    let rows: Vec<(&str, ModOrigin, bool, bool)> =
        list.mods.iter().map(|one| (one.name.as_str(), one.origin, one.valid, one.enabled)).collect();
    assert_eq!(
        rows,
        [
            ("calm", ModOrigin::Local, true, true),
            ("built-in:calm", ModOrigin::BuiltIn, true, false),
            ("built-in:loud", ModOrigin::BuiltIn, false, true),
        ]
    );
    assert!(list.mods[2].reason.as_deref().is_some_and(|why| why.contains("`url(`")), "{:?}", list.mods[2].reason);
    assert_eq!(list.mods[1].changed_at, None, "a built-in mod has no file on this machine");
}

/// **Shipped, so folded**: a local mod whose files are the built-in one's, as a promoted mod's are
/// once its pull request merges, is listed once, as the built-in mod.
#[test]
fn a_local_mod_identical_to_its_built_in_copy_is_listed_once() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    let at = mods.join("calm");
    std::fs::create_dir_all(&at).expect("a folder");
    std::fs::write(at.join("mod.toml"), CALM_TOML).expect("mod.toml");
    std::fs::write(at.join("theme.css"), CALM_CSS).expect("theme.css");

    let names: Vec<String> = scan(&mods, &BTreeMap::new(), &shipped()).mods.into_iter().map(|one| one.name).collect();

    assert_eq!(names, ["built-in:calm", "built-in:loud"]);
}

/// A local copy that has been changed since is the owner's own and stays listed.
#[test]
fn a_local_mod_that_differs_from_its_built_in_copy_is_still_listed_beside_it() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_local_calm(&mods);

    let names: Vec<String> = scan(&mods, &BTreeMap::new(), &shipped()).mods.into_iter().map(|one| one.name).collect();

    assert_eq!(names, ["calm", "built-in:calm", "built-in:loud"]);
}

/// An empty set lists only the machine's own: no `packages/mods/` is no built-in mod.
#[test]
fn no_built_in_mods_lists_only_the_machines_own() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_local_calm(&mods);
    let names: Vec<String> = scan(&mods, &BTreeMap::new(), &BuiltIns::default()).mods.into_iter().map(|one| one.name).collect();
    assert_eq!(names, ["calm"]);
}

fn refused(refusal: Refusal) -> (u16, String) {
    (refusal.status(), refusal.error().code.clone())
}

/// **Its own id everywhere**: validated, switched and chosen as the theme as `built-in:calm`, and
/// refused a promote, since it is in the repository already.
#[tokio::test]
async fn a_built_in_mod_is_validated_switched_and_chosen_by_its_own_id_and_is_never_promoted() {
    let home = TempDir::new();
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&[]));
    fitted.host.built_in_mods = shipped();
    let fleet = Arc::new(Fleet::assembled(fitted));

    let checked = fleet.validate_mod("built-in:calm".into()).await.expect("checked");
    assert!(checked.valid && checked.problems.is_empty(), "{:?}", checked.problems);
    assert_eq!(checked.css.as_deref(), Some(CALM_CSS), "the text compiled in, as it was checked");
    let broken = fleet.validate_mod("built-in:loud".into()).await.expect("answers for a broken one");
    assert!(!broken.valid && broken.css.is_none());
    let missing = fleet.validate_mod("built-in:nothing".into()).await.expect_err("no such mod");
    assert_eq!(refused(missing), (404, "fleet.no_such_mod".to_string()));

    let off = fleet.set_mod_enabled(SetModEnabled { name: "built-in:calm".into(), enabled: false }).await.expect("switched");
    assert!(!off.enabled && off.origin == ModOrigin::BuiltIn);

    let theme = SavePreference { name: "theme".into(), value: false, text: Some("built-in:calm".into()) };
    assert_eq!(fleet.save_preferences(theme).await.expect("a built-in mod's id is a theme id").theme, "built-in:calm");

    let promote = PromoteMod { name: "built-in:calm".into(), manifest_id: None };
    let refusal = Arc::clone(&fleet).promote_mod(promote).await.expect_err("not promoted");
    assert_eq!(refused(refusal.clone()), (409, "fleet.mod_not_promotable".to_string()));
    assert!(refusal.error().message.contains("built in"), "{}", refusal.error().message);
}
