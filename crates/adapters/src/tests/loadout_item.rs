//! One Loadout row's file, off a planted home and repository on disk: each kind resolves to the one
//! file it implies, and every name that could reach another is refused before anything is read.

use std::fs;
use std::path::{Path, PathBuf};

use adapter_traits::{HarnessSetup, ItemAsked, ItemKind, ItemRead, ItemWhose};

use crate::{ExistingSetup, Home};

/// One test's home and repository, removed when the test ends and cleared before use.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("armada-adapters-item-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).unwrap();
        fs::create_dir_all(dir.join("repo")).unwrap();
        Scratch(dir)
    }
    fn home(&self) -> PathBuf {
        self.0.join("home")
    }
    fn repo(&self) -> PathBuf {
        self.0.join("repo")
    }
    fn plant(&self, at: &Path, text: &str) {
        fs::create_dir_all(at.parent().unwrap()).unwrap();
        fs::write(at, text).unwrap();
    }
    fn read(&self, kind: ItemKind, name: &str, whose: ItemWhose<'_>) -> ItemRead {
        self.capped(kind, name, whose, 256 * 1024)
    }
    fn capped(&self, kind: ItemKind, name: &str, whose: ItemWhose<'_>, cap: usize) -> ItemRead {
        let home = self.home().to_string_lossy().to_string();
        let root = self.repo().to_string_lossy().to_string();
        ExistingSetup::over(Home::at(&home), &home).read_item(&ItemAsked { kind, name, whose, root: &root, cap })
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn read(answer: ItemRead) -> (String, bool, String, bool) {
    match answer {
        ItemRead::Read { path, markdown, body, truncated } => (path, markdown, body, truncated),
        ItemRead::NotRead { why } => panic!("not read: {why}"),
    }
}

fn why(answer: ItemRead) -> String {
    match answer {
        ItemRead::NotRead { why } => why,
        ItemRead::Read { path, .. } => panic!("read {path}, and should not have been"),
    }
}

#[test]
fn a_skill_is_its_skill_md_in_the_repository_the_home_or_the_plugin() {
    let at = Scratch::new("skills");
    at.plant(&at.repo().join(".claude/skills/land/SKILL.md"), "# Land");
    at.plant(&at.home().join(".claude/skills/walk/SKILL.md"), "# Walk");
    at.plant(&at.home().join(".claude/skills/synced/set-1/tidy/SKILL.md"), "---\nname: tidy-up\n---\n# Tidy");
    let plugin = at.0.join("cache/productivity");
    at.plant(&plugin.join("skills/start/SKILL.md"), "# Start");
    at.plant(
        &at.home().join(".claude/plugins/installed_plugins.json"),
        &format!(r#"{{"plugins":{{"productivity@market":[{{"installPath":"{}"}}]}}}}"#, plugin.display()),
    );

    assert_eq!(
        read(at.read(ItemKind::Skill, "land", ItemWhose::Repository)),
        (".claude/skills/land/SKILL.md".into(), true, "# Land".into(), false)
    );
    assert_eq!(read(at.read(ItemKind::Skill, "walk", ItemWhose::Kit)).0, "~/.claude/skills/walk/SKILL.md");
    assert_eq!(
        read(at.read(ItemKind::Skill, "tidy-up", ItemWhose::Kit)).0,
        "~/.claude/skills/synced/set-1/tidy/SKILL.md",
        "a synced skill, by the name its front matter gives"
    );
    assert_eq!(read(at.read(ItemKind::Skill, "start", ItemWhose::Plugin("productivity"))).0, "skills/start/SKILL.md");
    assert!(why(at.read(ItemKind::Skill, "start", ItemWhose::Plugin("absent"))).contains("not installed"));
    assert!(why(at.read(ItemKind::Skill, "gone", ItemWhose::Kit)).contains("No skill called `gone`"));
}

#[test]
fn an_agent_and_a_style_are_their_markdown_file_by_file_name_or_front_matter() {
    let at = Scratch::new("markdown");
    at.plant(&at.home().join(".claude/agents/rust-engineer.md"), "# Rust");
    at.plant(&at.repo().join(".claude/output-styles/terse.md"), "---\nname: Terse\n---\nShort.");

    assert_eq!(read(at.read(ItemKind::Subagent, "rust-engineer", ItemWhose::Kit)).0, "~/.claude/agents/rust-engineer.md");
    assert_eq!(
        read(at.read(ItemKind::OutputStyle, "Terse", ItemWhose::Repository)).0,
        ".claude/output-styles/terse.md"
    );
}

#[test]
fn a_memory_file_is_read_where_the_mod_reported_it_and_only_under_the_repository_or_the_home() {
    let at = Scratch::new("memory");
    at.plant(&at.repo().join("CLAUDE.md"), "# Repo");
    at.plant(&at.repo().join("docs/CLAUDE.md"), "# Docs");
    at.plant(&at.home().join(".claude/CLAUDE.md"), "# Mine");
    at.plant(&at.0.join("elsewhere/CLAUDE.md"), "# Not yours");

    assert_eq!(read(at.read(ItemKind::MemoryFile, "CLAUDE.md", ItemWhose::Repository)).0, "CLAUDE.md");
    assert_eq!(read(at.read(ItemKind::MemoryFile, "~/.claude/CLAUDE.md", ItemWhose::Kit)).2, "# Mine");
    let absolute = at.repo().join("docs/CLAUDE.md").to_string_lossy().to_string();
    assert_eq!(read(at.read(ItemKind::MemoryFile, &absolute, ItemWhose::Repository)).0, "docs/CLAUDE.md");
    let in_home = at.home().join(".claude/CLAUDE.md").to_string_lossy().to_string();
    assert_eq!(read(at.read(ItemKind::MemoryFile, &in_home, ItemWhose::Kit)).0, "~/.claude/CLAUDE.md");

    let outside = at.0.join("elsewhere/CLAUDE.md").to_string_lossy().to_string();
    assert!(why(at.read(ItemKind::MemoryFile, &outside, ItemWhose::Repository)).contains("only under"));
}

#[test]
fn a_name_that_could_climb_or_name_a_second_folder_is_refused_before_anything_is_read() {
    let at = Scratch::new("climb");
    at.plant(&at.0.join("secret/SKILL.md"), "secret");
    at.plant(&at.0.join("secret.md"), "secret");
    at.plant(&at.home().join(".claude/skills/a/b/SKILL.md"), "# Nested");

    for name in ["..", ".", "", "a/b", "../secret", "a\\b", "/etc/passwd"] {
        why(at.read(ItemKind::Skill, name, ItemWhose::Kit));
        why(at.read(ItemKind::Subagent, name, ItemWhose::Repository));
        why(at.read(ItemKind::OutputStyle, name, ItemWhose::Kit));
    }
    for plugin in ["..", "a/b", "name@market"] {
        why(at.read(ItemKind::Skill, "start", ItemWhose::Plugin(plugin)));
    }
    let climbing = format!("{}/../secret.md", at.repo().display());
    for name in ["../secret.md", "docs/../../secret.md", "~/.claude/../../secret.md", "~/secret.md", "~other/x", climbing.as_str(), "/etc/hosts"] {
        why(at.read(ItemKind::MemoryFile, name, ItemWhose::Repository));
    }
}

#[test]
fn a_link_out_of_the_folder_reads_where_it_stays_in_the_home_and_nothing_beyond_it() {
    let at = Scratch::new("links");
    at.plant(&at.0.join("outside/SKILL.md"), "outside");
    at.plant(&at.home().join("dotfiles/skills/mine/SKILL.md"), "# Mine");
    at.plant(&at.repo().join("real.md"), "# Real");
    fs::create_dir_all(at.home().join(".claude/skills")).unwrap();
    std::os::unix::fs::symlink(at.0.join("outside"), at.home().join(".claude/skills/out")).unwrap();
    std::os::unix::fs::symlink(at.home().join("dotfiles/skills/mine"), at.home().join(".claude/skills/mine")).unwrap();
    std::os::unix::fs::symlink("/etc", at.home().join(".claude/skills/etc")).unwrap();
    std::os::unix::fs::symlink(at.0.join("outside/SKILL.md"), at.repo().join("CLAUDE.md")).unwrap();
    std::os::unix::fs::symlink(at.repo().join("real.md"), at.repo().join("AGENTS.md")).unwrap();

    assert_eq!(
        read(at.read(ItemKind::Skill, "mine", ItemWhose::Kit)),
        ("~/.claude/skills/mine/SKILL.md".into(), true, "# Mine".into(), false),
        "a dotfiles link that stays in the home"
    );
    assert!(why(at.read(ItemKind::Skill, "out", ItemWhose::Kit)).contains("a link to outside"));
    why(at.read(ItemKind::Skill, "etc", ItemWhose::Kit));
    fs::create_dir_all(at.home().join(".claude/agents")).unwrap();
    std::os::unix::fs::symlink("/etc/hosts", at.home().join(".claude/agents/hosts.md")).unwrap();
    assert!(why(at.read(ItemKind::Subagent, "hosts", ItemWhose::Kit)).contains("a link to outside"), "a link to /etc");
    assert!(why(at.read(ItemKind::MemoryFile, "CLAUDE.md", ItemWhose::Repository)).contains("a link to outside"));
    assert_eq!(read(at.read(ItemKind::MemoryFile, "AGENTS.md", ItemWhose::Repository)).2, "# Real");
}

#[test]
fn a_body_past_the_cap_is_cut_on_a_character_and_says_so() {
    let at = Scratch::new("cap");
    at.plant(&at.repo().join("CLAUDE.md"), "abé");
    let (_, _, body, truncated) = read(at.capped(ItemKind::MemoryFile, "CLAUDE.md", ItemWhose::Repository, 3));
    assert_eq!((body.as_str(), truncated), ("ab", true), "the é is two bytes and the cap cuts it");
    let (_, _, body, truncated) = read(at.capped(ItemKind::MemoryFile, "CLAUDE.md", ItemWhose::Repository, 4));
    assert_eq!((body.as_str(), truncated), ("abé", false));
}
