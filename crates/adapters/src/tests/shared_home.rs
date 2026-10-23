//! The shared home's adapters over planted folders: a folder written and refused, the harness's
//! plugins read with where each marketplace comes from, and the home's copy named by its own
//! origin. Nothing reaches the forge or runs a process.

use adapter_traits::{FileRead, HarnessHome, HomeRepository, KeptFolder, PluginRecord};

use super::scratch::Scratch;
use crate::{AgentHome, Folder, ForgeHome};

const INSTALLED: &str = r#"{"version": 2, "plugins": {
  "tidy@official": [{"scope": "user", "installPath": "/x"}],
  "local-only@mine": [{"scope": "user"}],
  "shop-only@official": [{"scope": "project", "projectPath": "/work/shop"}]
}}"#;
const MARKETPLACES: &str = r#"{
  "official": {"source": {"source": "github", "repo": "acme/plugins"}},
  "mine": {"source": {"source": "directory", "path": "/Users/user/plugins"}}
}"#;

#[test]
fn a_folder_writes_with_its_folders_made_and_refuses_a_path_that_climbs() {
    let scratch = Scratch::new();
    let folder = Folder::at(scratch.folder("kept"));
    folder.write("skills/tone/SKILL.md", "warm\n").expect("written");
    assert_eq!(folder.read("skills/tone/SKILL.md"), FileRead::Bytes(b"warm\n".to_vec()));
    assert!(folder.write("../outside.md", "no").is_err());
    assert!(!scratch.folder("").join("outside.md").exists());
    folder.remove("skills/tone/SKILL.md").expect("removed");
    folder.remove("skills/tone/SKILL.md").expect("a second remove is not an error");
    assert_eq!(folder.read("skills/tone/SKILL.md"), FileRead::Absent);
}

#[test]
fn the_harness_lists_the_persons_plugins_and_a_forge_marketplace_as_their_source() {
    let scratch = Scratch::new();
    scratch.file(".claude/plugins/installed_plugins.json", INSTALLED);
    scratch.file(".claude/plugins/known_marketplaces.json", MARKETPLACES);
    let harness = AgentHome::at(scratch.folder(""), "/usr/bin");
    let plugins = harness.plugins().expect("read");
    assert_eq!(
        plugins,
        vec![
            PluginRecord { name: "local-only@mine".to_string(), marketplace: "mine".to_string(), source: None },
            PluginRecord { name: "tidy@official".to_string(), marketplace: "official".to_string(), source: Some("acme/plugins".to_string()) },
        ],
        "one installed for a project only does not travel"
    );
    assert_eq!(harness.memory_dir("/Users/user/code/shop"), ".claude/projects/-Users-user-code-shop/memory");
}

#[test]
fn the_homes_copy_is_named_by_its_own_origin_and_only_over_https() {
    let scratch = Scratch::new();
    let over_https = ForgeHome::in_dir(scratch.folder("https"));
    assert_eq!(over_https.here(), None, "no clone yet");
    scratch.file("https/.git/config", "[core]\n\tbare = false\n[remote \"origin\"]\n\turl = https://github.com/nick/armada-home.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n");
    assert_eq!(over_https.here().as_deref(), Some("nick/armada-home"));
    scratch.file("ssh/.git/config", "[remote \"origin\"]\n\turl = git@github.com:nick/armada-home.git\n");
    assert_eq!(ForgeHome::in_dir(scratch.folder("ssh")).here(), None);
}
