//! A person's setup held in memory: what is read is the subject, and nothing
//! is written to read it.
//!
//! The one case that touches a real directory is [`a_home_that_is_not_there`],
//! which needs a path that exists nowhere.

use std::collections::BTreeMap;

use adapter_traits::{
    FileEntry, FileRead, HarnessSetup, SetupFiles, SetupItem, SetupKind, StyleApplied, StyleFrom,
    WhatWasRead,
};

use crate::{ExistingSetup, Home};

struct Held(BTreeMap<String, String>);

impl Held {
    fn of(files: &[(&str, &str)]) -> Held {
        Held(
            files
                .iter()
                .map(|(path, text)| (path.to_string(), text.to_string()))
                .collect(),
        )
    }
}

impl SetupFiles for Held {
    fn read(&self, path: &str) -> FileRead {
        match self.0.get(path) {
            Some(text) => FileRead::Bytes(text.as_bytes().to_vec()),
            None => FileRead::Absent,
        }
    }

    fn entries(&self, dir: &str) -> Result<Vec<FileEntry>, String> {
        let prefix = format!("{dir}/");
        let mut found: BTreeMap<String, bool> = BTreeMap::new();
        for path in self.0.keys() {
            if let Some(rest) = path.strip_prefix(&prefix) {
                match rest.split_once('/') {
                    Some((child, _)) => found.insert(child.to_string(), true),
                    None => found.insert(rest.to_string(), false),
                };
            }
        }
        match found.is_empty() {
            true => Err(format!("{dir} is not a directory")),
            false => Ok(found
                .into_iter()
                .map(|(name, is_dir)| FileEntry { name, is_dir })
                .collect()),
        }
    }
}

fn read(files: &[(&str, &str)]) -> adapter_traits::Inventory {
    ExistingSetup::over(Held::of(files), "/home/user").read()
}

fn items(files: &[(&str, &str)], kind: SetupKind) -> Vec<SetupItem> {
    match read(files).kind(kind).cloned() {
        Some(WhatWasRead::Read { items, .. }) => items,
        other => panic!("{kind:?} was {other:?}"),
    }
}

fn unreadable(files: &[(&str, &str)], kind: SetupKind) -> Vec<String> {
    match read(files).kind(kind).cloned() {
        Some(WhatWasRead::Read { unreadable, .. }) => {
            unreadable.into_iter().map(|one| one.source).collect()
        }
        other => panic!("{kind:?} was {other:?}"),
    }
}

const A_SKILL: &str = "---\nname: humanizer\ndescription: Rewrite text\n---\n\n# Humanizer\n";

#[test]
fn a_skill_is_named_and_described_by_its_own_front_matter() {
    let read = items(
        &[(".claude/skills/humanizer/SKILL.md", A_SKILL)],
        SetupKind::Skills,
    );
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].name, "humanizer");
    assert_eq!(read[0].says.as_deref(), Some("Rewrite text"));
    assert_eq!(read[0].source, "/home/user/.claude/skills/humanizer");
}

#[test]
fn a_folder_of_skills_is_read_through_rather_than_reported_as_one() {
    let read = items(
        &[
            (".claude/skills/humanizer/SKILL.md", A_SKILL),
            (
                ".claude/skills/synced/from-the-web/SKILL.md",
                "---\nname: docs\ndescription: Read the docs\n---\n",
            ),
        ],
        SetupKind::Skills,
    );
    let named: Vec<&str> = read.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(named, vec!["humanizer", "docs"]);
}

#[test]
fn a_skill_whose_front_matter_will_not_parse_is_named_rather_than_skipped() {
    let files = [
        (".claude/skills/broken/SKILL.md", "---\nname: [oh\n---\n"),
        (".claude/skills/humanizer/SKILL.md", A_SKILL),
    ];
    assert_eq!(items(&files, SetupKind::Skills).len(), 1);
    assert_eq!(
        unreadable(&files, SetupKind::Skills),
        vec!["/home/user/.claude/skills/broken/SKILL.md"]
    );
}

#[test]
fn front_matter_that_never_closes_is_a_reason_and_not_a_panic() {
    let files = [(".claude/skills/half/SKILL.md", "---\nname: half\n")];
    assert_eq!(items(&files, SetupKind::Skills).len(), 0);
    assert_eq!(unreadable(&files, SetupKind::Skills).len(), 1);
}

#[test]
fn a_skill_without_front_matter_is_named_by_its_folder() {
    let read = items(
        &[(".claude/skills/plain/SKILL.md", "# Plain\n")],
        SetupKind::Skills,
    );
    assert_eq!(read[0].name, "plain");
    assert_eq!(read[0].says, None);
}

#[test]
fn a_namespaced_command_carries_the_folder_in_its_name() {
    let read = items(
        &[(".claude/commands/git/sync.md", "do the thing\n")],
        SetupKind::Commands,
    );
    assert_eq!(read[0].name, "git:sync");
}

#[test]
fn plugins_come_off_the_file_the_harness_keeps_them_in() {
    let read = items(
        &[(
            ".claude/plugins/installed_plugins.json",
            r#"{"version":2,"plugins":{"github@official":[{"scope":"user","version":"1.0.0","installPath":"/home/user/.claude/plugins/cache/github"}]}}"#,
        )],
        SetupKind::Plugins,
    );
    assert_eq!(read[0].name, "github@official");
    assert_eq!(read[0].says.as_deref(), Some("version 1.0.0"));
    assert_eq!(read[0].source, "/home/user/.claude/plugins/cache/github");
}

#[test]
fn a_plugin_file_that_will_not_parse_is_named_and_takes_nothing_else_down() {
    let files = [
        (".claude/plugins/installed_plugins.json", "{ not json"),
        (".claude/skills/humanizer/SKILL.md", A_SKILL),
    ];
    assert_eq!(unreadable(&files, SetupKind::Plugins).len(), 1);
    assert_eq!(items(&files, SetupKind::Skills).len(), 1);
}

const A_PLUGIN: &str = r#"{"plugins":{"armada@mods":[{"installPath":"/home/user/.claude/plugins/cache/armada"}]}}"#;

fn with_plugin<'a>(mut files: Vec<(&'a str, &'a str)>) -> Vec<(&'a str, &'a str)> {
    files.push((".claude/plugins/installed_plugins.json", A_PLUGIN));
    files
}

fn plugin_of<'a>(items: &'a [SetupItem], name: &str) -> &'a Option<String> {
    &items.iter().find(|item| item.name == name).unwrap().plugin
}

#[test]
fn a_plugins_skills_agents_and_commands_are_read_where_it_is_installed() {
    let files = with_plugin(vec![
        (".claude/skills/humanizer/SKILL.md", A_SKILL),
        (".claude/agents/mine.md", "---\ndescription: Mine\n---\n"),
        (".claude/commands/mine.md", "mine\n"),
        (
            ".claude/plugins/cache/armada/skills/walk/SKILL.md",
            "---\nname: walk\ndescription: Show a walk\n---\n",
        ),
        (
            ".claude/plugins/cache/armada/agents/judge.md",
            "---\nname: judge\ndescription: Vetoes\n---\n",
        ),
        (".claude/plugins/cache/armada/commands/ship/it.md", "ship\n"),
    ]);
    let skills = items(&files, SetupKind::Skills);
    assert_eq!(skills.len(), 2);
    assert_eq!(plugin_of(&skills, "humanizer"), &None);
    assert_eq!(plugin_of(&skills, "walk").as_deref(), Some("armada"));
    let walk = skills.iter().find(|item| item.name == "walk").unwrap();
    assert_eq!(walk.says.as_deref(), Some("Show a walk"));
    assert_eq!(walk.source, "/home/user/.claude/plugins/cache/armada/skills/walk");

    let agents = items(&files, SetupKind::SubAgents);
    assert_eq!(agents.len(), 2);
    assert_eq!(plugin_of(&agents, "mine"), &None);
    assert_eq!(plugin_of(&agents, "judge").as_deref(), Some("armada"));

    let commands = items(&files, SetupKind::Commands);
    assert_eq!(commands.len(), 2);
    assert_eq!(plugin_of(&commands, "mine"), &None);
    assert_eq!(plugin_of(&commands, "ship:it").as_deref(), Some("armada"));
    // The Plugins row itself is not a thing a plugin brought.
    assert_eq!(plugin_of(&items(&files, SetupKind::Plugins), "armada@mods"), &None);
}

#[test]
fn a_plugin_skill_that_will_not_read_is_named_under_skills_and_the_rest_stand() {
    let files = with_plugin(vec![
        (".claude/skills/humanizer/SKILL.md", A_SKILL),
        (".claude/plugins/cache/armada/skills/broken/SKILL.md", "---\nname: [oh\n---\n"),
        (".claude/plugins/cache/armada/skills/walk/SKILL.md", "---\nname: walk\n---\n"),
    ]);
    let named: Vec<String> = items(&files, SetupKind::Skills)
        .into_iter()
        .map(|item| item.name)
        .collect();
    assert_eq!(named, vec!["humanizer", "walk"]);
    assert_eq!(
        unreadable(&files, SetupKind::Skills),
        vec!["/home/user/.claude/plugins/cache/armada/skills/broken/SKILL.md"]
    );
}

/// Every place a key sits in a connected server, planted at once: the argument
/// list, the query string, the userinfo, the environment, and the directory a
/// program is kept in.
const KEYS_EVERYWHERE: &str = r#"{
  "oauthAccount": {"emailAddress": "someone@example.com"},
  "mcpServers": {
    "tracker": {
      "command": "/opt/sk-live-SECRET/bin/tracker-server",
      "args": ["-y", "@scope/tracker", "--key=SECRET"],
      "env": {"API_KEY": "SECRET"}
    },
    "docs": {"type": "http", "url": "https://user:SECRET@docs.example.com/mcp?token=SECRET"}
  }
}"#;

/// **The whole safety of this seam, as a test.** A server a person connected
/// crosses as its name and as the program or host it is at, and not one of the
/// five places a credential hides survives — so what is drawn could not start
/// the server it names.
#[test]
fn a_connected_server_crosses_without_one_place_a_key_hides() {
    let read = items(&[(".claude.json", KEYS_EVERYWHERE)], SetupKind::McpServers);
    let said = format!("{read:?}");
    for place in [
        "SECRET",       // the argument list, the query, the userinfo, the environment
        "sk-live",      // the directory the program is kept in
        "--key",        // any argument at all
        "token",        // the query string, key and value alike
        "user:",        // the userinfo
        "emailAddress", // the account the file also holds
    ] {
        assert!(!said.contains(place), "`{place}` survived the read: {said}");
    }
    let named: Vec<&str> = read.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(named, vec!["docs", "tracker"]);
    // What a person is left with: the host, and the program's own file name.
    assert_eq!(read[0].says.as_deref(), Some("docs.example.com"));
    assert_eq!(read[1].says.as_deref(), Some("tracker-server"));
}

/// A port is not a secret and is dropped anyway: it is noise beside the host,
/// and keeping it would mean keeping the one thing after the `:`.
#[test]
fn a_host_crosses_without_its_scheme_or_its_port() {
    let read = items(
        &[(
            ".claude.json",
            r#"{"mcpServers":{"near":{"type":"http","url":"http://127.0.0.1:8931/mcp"},
                              "six":{"type":"http","url":"http://[::1]:8931/mcp"}}}"#,
        )],
        SetupKind::McpServers,
    );
    assert_eq!(read[0].says.as_deref(), Some("127.0.0.1"));
    assert_eq!(read[1].says.as_deref(), Some("[::1]"));
}

/// Neither key is a file a person wrote by hand having a bad day — it is a
/// server this adapter cannot describe, and it is still one they have.
#[test]
fn a_server_with_neither_a_program_nor_an_address_is_still_listed() {
    let read = items(
        &[(".claude.json", r#"{"mcpServers":{"odd":{"type":"sse"}}}"#)],
        SetupKind::McpServers,
    );
    assert_eq!(read[0].name, "odd");
    assert_eq!(read[0].says.as_deref(), Some("connected outside Armada"));
}

#[test]
fn the_global_agent_file_is_measured_and_never_carried() {
    let read = items(
        &[(".claude/CLAUDE.md", "\n# How I work\n\nBe direct.\n")],
        SetupKind::AgentFile,
    );
    assert_eq!(read.len(), 1);
    assert_eq!(
        read[0].says.as_deref(),
        Some("4 lines, opening # How I work")
    );
    assert!(!format!("{read:?}").contains("Be direct"));
}

/// Two kinds are answered as not read rather than drawn as empty, which is the
/// difference between *you have none* and *nothing looked*.
#[test]
fn the_kinds_nothing_reads_yet_say_so() {
    let read = read(&[]);
    assert_eq!(read.kinds.len(), SetupKind::ALL.len());
    for kind in [SetupKind::Allowlist, SetupKind::Models] {
        match read.kind(kind) {
            Some(WhatWasRead::NotRead { why }) => assert!(why.contains("#41")),
            other => panic!("{kind:?} was {other:?}"),
        }
    }
}

#[test]
fn a_home_with_nothing_in_it_is_read_as_nothing_rather_than_as_a_failure() {
    let read = read(&[]);
    assert!(!read.present);
    assert_eq!(
        read.kind(SetupKind::Skills),
        Some(&WhatWasRead::Read {
            items: Vec::new(),
            unreadable: Vec::new()
        })
    );
}

#[test]
fn a_home_that_is_not_there_is_an_answer() {
    let read = ExistingSetup::over(Home::at("/nowhere/at/all"), "/nowhere/at/all").read();
    assert!(!read.present);
    assert_eq!(read.home, "/nowhere/at/all/.claude");
    assert_eq!(read.kinds.len(), SetupKind::ALL.len());
}

/// A path built above the home reads nothing, whatever a caller asks for.
#[test]
fn nothing_above_the_home_is_readable_through_it() {
    let home = Home::at("/nowhere/at/all");
    assert_eq!(
        home.read("../etc/passwd"),
        FileRead::Unreadable("above the home it is read from".to_string())
    );
    assert!(home.entries("../..").is_err());
}

/// A server named for the program it runs says it once. Measured on the
/// owner's machine: his `gitnexus` server runs a binary called `gitnexus`, and
/// the row read `gitnexus gitnexus`.
#[test]
fn a_server_named_for_its_own_program_does_not_say_it_twice() {
    let read = items(
        &[(
            ".claude.json",
            r#"{"mcpServers":{"gitnexus":{"command":"/opt/bin/gitnexus"}}}"#,
        )],
        SetupKind::McpServers,
    );
    assert_eq!(read[0].name, "gitnexus");
    assert_eq!(read[0].says, None);
}

// Output styles.

const USER_STYLE: &str = "---\nname: bluf\ndescription: Bottom line first\n---\n\nSECRET BODY the person wrote.\n";

fn styles(files: &[(&str, &str)], repository: &[(&str, &str)]) -> adapter_traits::Inventory {
    let repository: Vec<(String, String)> = repository
        .iter()
        .map(|(path, text)| (path.to_string(), text.to_string()))
        .collect();
    ExistingSetup::over(Held::of(files), "/home/user")
        .in_repositories(move |_| {
            let held: Vec<(&str, &str)> = repository
                .iter()
                .map(|(path, text)| (path.as_str(), text.as_str()))
                .collect();
            Box::new(Held::of(&held))
        })
        .read_across(&["/work/armada".to_string()])
}

fn style_items(read: &adapter_traits::Inventory) -> Vec<SetupItem> {
    match read.kind(SetupKind::OutputStyles).cloned() {
        Some(WhatWasRead::Read { items, .. }) => items,
        other => panic!("output styles were {other:?}"),
    }
}

fn applied(items: &[SetupItem], name: &str) -> (StyleFrom, StyleApplied) {
    let facts = items
        .iter()
        .find(|item| item.name == name)
        .unwrap_or_else(|| panic!("no style {name} in {items:?}"))
        .style
        .as_ref()
        .expect("a style row carries its facts");
    (facts.from, facts.applied)
}

#[test]
fn styles_are_read_from_the_person_the_repository_and_a_plugin() {
    let read = styles(
        &[
            (".claude/output-styles/bluf.md", USER_STYLE),
            (
                ".claude/plugins/installed_plugins.json",
                r#"{"plugins":{"armada@mods":[{"installPath":"/home/user/.claude/plugins/cache/armada"}]}}"#,
            ),
            (
                ".claude/plugins/cache/armada/output-styles/Armada.md",
                "---\nname: Armada\ndescription: Facts only\n---\nbody\n",
            ),
        ],
        &[(
            ".claude/output-styles/terse.md",
            "---\ndescription: One line each\n---\nbody\n",
        )],
    );
    let items = style_items(&read);
    assert_eq!(applied(&items, "bluf").0, StyleFrom::Person);
    assert_eq!(applied(&items, "Armada").0, StyleFrom::Plugin);
    // The plugin's name, without its marketplace.
    let armada = items.iter().find(|item| item.name == "Armada").unwrap();
    assert_eq!(armada.style.as_ref().unwrap().plugin.as_deref(), Some("armada"));
    // No name in the front matter: the file's own name.
    assert_eq!(applied(&items, "terse").0, StyleFrom::Repository);
    let bluf = items.iter().find(|item| item.name == "bluf").unwrap();
    assert_eq!(bluf.says.as_deref(), Some("Bottom line first"));
    assert_eq!(bluf.source, "/home/user/.claude/output-styles/bluf.md");
    let terse = items.iter().find(|item| item.name == "terse").unwrap();
    assert_eq!(terse.source, "/work/armada/.claude/output-styles/terse.md");
    assert!(items
        .iter()
        .all(|item| item.style.as_ref().unwrap().applied == StyleApplied::Available));
}

#[test]
fn local_settings_win_over_project_and_project_over_user() {
    let both = [
        (".claude/output-styles/a.md", "---\nname: a\n---\n"),
        (".claude/output-styles/b.md", "---\nname: b\n---\n"),
        (".claude/output-styles/c.md", "---\nname: c\n---\n"),
    ];
    let user = |files: &mut Vec<(&'static str, &'static str)>| {
        files.push((".claude/settings.json", r#"{"outputStyle":"a","env":{"K":"v"}}"#));
    };
    let mut files = both.to_vec();
    user(&mut files);

    let alone = styles(&files, &[]);
    assert_eq!(applied(&style_items(&alone), "a").1, StyleApplied::Active);

    let project = styles(&files, &[(".claude/settings.json", r#"{"outputStyle":"b"}"#)]);
    let items = style_items(&project);
    assert_eq!(applied(&items, "b").1, StyleApplied::Active);

    let local = styles(
        &files,
        &[
            (".claude/settings.json", r#"{"outputStyle":"b"}"#),
            (".claude/settings.local.json", r#"{"outputStyle":"c"}"#),
        ],
    );
    let items = style_items(&local);
    assert_eq!(applied(&items, "c").1, StyleApplied::Active);
    // The repository's pick replaces the user's there, and the user's still
    // stands for everywhere else.
    assert_eq!(applied(&items, "a").1, StyleApplied::Active);
    assert_eq!(applied(&items, "b").1, StyleApplied::Available);
}

#[test]
fn a_built_in_style_is_a_row_only_when_it_is_the_one_set() {
    let none = styles(&[], &[]);
    assert!(style_items(&none).is_empty());

    let set = styles(&[(".claude/settings.json", r#"{"outputStyle":"Explanatory"}"#)], &[]);
    let items = style_items(&set);
    assert_eq!(items.len(), 1);
    assert_eq!(
        applied(&items, "Explanatory"),
        (StyleFrom::BuiltIn, StyleApplied::Active)
    );
}

#[test]
fn an_unreadable_style_or_settings_file_is_named_with_its_reason() {
    let read = styles(
        &[
            (".claude/output-styles/half.md", "---\nname: half\n"),
            (".claude/settings.json", "{not json"),
        ],
        &[],
    );
    match read.kind(SetupKind::OutputStyles).cloned() {
        Some(WhatWasRead::Read { items, unreadable }) => {
            assert!(items.is_empty());
            let sources: Vec<_> = unreadable.iter().map(|one| one.source.as_str()).collect();
            assert!(sources.contains(&"/home/user/.claude/output-styles/half.md"));
            assert!(sources.contains(&"/home/user/.claude/settings.json"));
            assert!(unreadable.iter().all(|one| !one.why.is_empty()));
        }
        other => panic!("output styles were {other:?}"),
    }
}

#[test]
fn a_style_is_never_carried_as_its_body() {
    let read = styles(
        &[
            (".claude/output-styles/bluf.md", USER_STYLE),
            (".claude/settings.json", r#"{"outputStyle":"bluf","env":{"TOKEN":"hunter2"}}"#),
        ],
        &[],
    );
    let text = format!("{read:?}");
    assert!(!text.contains("SECRET BODY"));
    assert!(!text.contains("hunter2"));
}
