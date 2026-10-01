//! The one rule Armada writes into a person's own settings, asserted on the
//! bytes — because the schema is the agent CLI's and a key this spells wrongly
//! is a rule that silently never applies. `#1389`.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::{personal_settings, remember_the_rule, Remembered, PERSONAL_SETTINGS};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// One test's directory, removed when the test ends.
///
/// **Cleared before use as well as after.** nextest runs each test in its own
/// process, so the counter is always zero and the name is the pid alone. Before
/// this removed anything, 1,161 were left in the temp dir, and a reused pid
/// opened the half-written file another test had planted two days earlier.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new() -> Scratch {
        let dir = std::env::temp_dir().join(format!(
            "armada-adapters-settings-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        Scratch(dir)
    }

    fn settings(&self) -> std::path::PathBuf {
        self.0.join(PERSONAL_SETTINGS)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_rule_lands_under_permissions_allow() {
    let scratch = Scratch::new();
    let at = scratch.settings();

    let written = remember_the_rule(&at, "Bash(gh issue list:*)").expect("the file written");

    assert_eq!(written, Remembered::Written);
    let back = std::fs::read_to_string(&at).expect("the file read back");
    assert_eq!(
        back,
        "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(gh issue list:*)\"\n    ]\n  }\n}\n"
    );
}

/// **The file is the person's**, so everything they wrote in it survives — the
/// guarantee `publish_the_agents_door` makes about a repository's `.mcp.json`,
/// one file over.
#[test]
fn every_other_key_and_every_other_rule_comes_back_out() {
    let scratch = Scratch::new();
    let at = scratch.settings();
    std::fs::create_dir_all(at.parent().expect("a parent")).expect("the directory");
    std::fs::write(
        &at,
        "{\"permissions\":{\"allow\":[\"Bash(git push)\"],\"deny\":[\"WebFetch\"]},\"model\":\"opus\"}",
    )
    .expect("the file planted");

    remember_the_rule(&at, "Edit").expect("the file written");

    let back = std::fs::read_to_string(&at).expect("the file read back");
    assert!(back.contains("\"Bash(git push)\""), "{back}");
    assert!(back.contains("\"Edit\""), "{back}");
    assert!(back.contains("\"deny\""), "{back}");
    assert!(back.contains("\"opus\""), "{back}");
}

/// A second allow of the same rule moves no mtime and offers a person no diff
/// of nothing.
#[test]
fn a_rule_already_there_is_not_written_again() {
    let scratch = Scratch::new();
    let at = scratch.settings();

    remember_the_rule(&at, "Bash(gh:*)").expect("the file written");
    let again = remember_the_rule(&at, "Bash(gh:*)").expect("the second answer");

    assert_eq!(again, Remembered::AlreadyThere);
}

/// **A file somebody is in the middle of editing is left alone.** The call
/// still runs — `fleet::helm` settles it as allowed-but-not-remembered — and
/// nothing here replaces what would not parse.
#[test]
fn a_settings_file_that_will_not_parse_is_left_exactly_as_it_was() {
    let scratch = Scratch::new();
    let at = scratch.settings();
    std::fs::create_dir_all(at.parent().expect("a parent")).expect("the directory");
    std::fs::write(&at, "{\"permissions\": {").expect("the file planted");

    let refused = remember_the_rule(&at, "Edit");

    assert!(refused.is_err());
    assert_eq!(
        std::fs::read_to_string(&at).expect("the file read back"),
        "{\"permissions\": {"
    );
}

/// **Not `~/.claude/settings.json` and not `.claude/settings.json`.** The first
/// is theirs across every repository and every tool; the second is committed
/// and is everybody's.
#[test]
fn the_file_is_this_person_s_own_in_this_repository() {
    assert_eq!(PERSONAL_SETTINGS, ".claude/settings.local.json");
    assert_eq!(
        personal_settings("/repos/armada"),
        std::path::Path::new("/repos/armada/.claude/settings.local.json")
    );
}
