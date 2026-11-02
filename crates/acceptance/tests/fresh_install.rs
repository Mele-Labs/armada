//! Fresh install's claim: **Armada works in any repository on a Mac that has the app and nothing
//! else: no sign-in, no Kit, no `armada.yml`, no checkout of Armada.** A Job there has the Armada
//! mod, and the door offers it every act the rules it is told to follow name.
//!
//! | Proved | Not proved |
//! |---|---|
//! | An app's bundle is found from the binary's own path, and a binary anywhere else has none | That the release workflow puts the mod there. `.github/workflows/checks.yml` fails its own step when it did not arrive |
//! | An empty home's mod copy is filled from that bundle, a copy of the same version is left alone, and a bundle of another version replaces it whole, nothing of the old copy kept and no half copy left beside it | That a hosted Job's launch fills it and still starts when it cannot. `fleet`'s `fresh_install` tests, against a repository with no `armada.yml` |
//! | Every act the rules name is offered at the door a Job in a root with no `armada.yml` stands at | That each answers there. `api`'s `door_without_manifest` tests call them |
//!
//! The copy is files, so this test writes a folder under the system's temporary one and removes it.

use std::path::{Path, PathBuf};

use adapters::agent_mod::{bundle_beside, filled_from_bundle, is_populated};

/// A folder for one test, gone after it.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let path = std::env::temp_dir().join(format!("armada-fresh-install-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Folder(path)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn written(at: &Path, text: &str) {
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(at, text).unwrap();
}

/// The mod as the release lays it in an app: `plugins/`'s contents, at `version`.
fn bundled(resources: &Path, version: &str, skill: &str) {
    let bundle = resources.join("mod");
    written(&bundle.join(".claude-plugin/marketplace.json"), "{}");
    written(&bundle.join("armada/.claude-plugin/plugin.json"), &format!("{{\"name\":\"armada\",\"version\":\"{version}\"}}"));
    written(&bundle.join("armada/skills").join(skill).join("SKILL.md"), skill);
}

#[test]
fn a_job_in_a_repository_nothing_was_set_up_in_has_the_mod_and_every_act_its_rules_name() {
    let tree = Folder::new("app");
    let resources = tree.0.join("Armada.app/Contents/Resources");
    let exe = resources.join("bin/armada");
    written(&exe, "");
    bundled(&resources, "1.0.0", "reading-a-brief");

    // Found from the binary's own path, and only from an app's.
    assert_eq!(bundle_beside(&exe), Some(resources.join("mod")));
    assert_eq!(bundle_beside(&tree.0.join(".cargo/bin/armada")), None);

    // An empty home is filled.
    let home = tree.0.join("home");
    let folder = fleet::runtime::mod_dir(&home.to_string_lossy());
    assert!(!is_populated(&folder));
    assert_eq!(filled_from_bundle(&resources.join("mod"), &folder), Ok(true));
    assert!(folder.join("armada/skills/reading-a-brief/SKILL.md").is_file());

    // The same version is left alone, and an update replaces the copy whole.
    assert_eq!(filled_from_bundle(&resources.join("mod"), &folder), Ok(false));
    std::fs::remove_dir_all(resources.join("mod")).unwrap();
    bundled(&resources, "1.0.1", "root-cause");
    assert_eq!(filled_from_bundle(&resources.join("mod"), &folder), Ok(true));
    assert!(folder.join("armada/skills/root-cause/SKILL.md").is_file());
    assert!(!folder.join("armada/skills/reading-a-brief").exists(), "nothing of the old copy kept");
    let beside: Vec<_> = std::fs::read_dir(folder.parent().unwrap()).unwrap().map(|one| one.unwrap().file_name()).collect();
    assert_eq!(beside, ["mod"], "no half copy left beside it");

    // The door offers every operation the rules name.
    let offered = api::offered_without_a_manifest();
    let rules = adapters::SESSION_RULES.replace("mcp__armada-fleet__", "");
    let named = rules
        .split('`')
        .skip(1)
        .step_by(2)
        .flat_map(|span| span.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')))
        .collect::<Vec<_>>();
    let operations: Vec<&str> = api::SERVED.iter().map(|route| route.operation).collect();
    let renamed = |tool: &str| ipc::door::RENAMED.iter().find(|(name, _)| *name == tool).map(|(_, operation)| *operation);
    let mut checked = 0;
    for word in named {
        let operation = renamed(word).unwrap_or(word);
        if operations.contains(&operation) {
            assert!(offered.contains(&operation), "`{word}` is named in the rules and is not offered with no armada.yml");
            checked += 1;
        }
    }
    assert!(checked >= 10, "the rules name operations, or this read nothing: {checked}");
}
