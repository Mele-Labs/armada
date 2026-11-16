//! A Kit piece's file, read, saved and opened, against a fixture's own home: only a piece a fresh
//! inventory names has a file, a save over a file that moved is refused, and a plugin's piece reads
//! and is never saved. Nothing here starts a program.

use std::path::{Path, PathBuf};

use ipc::{
    GetKitFile, KitFileKind, KitInventory, KitOpenKind, KitPlace, OpenKitPiece, SaveKitFile, SetupItem, SetupKindRow,
    WhatWasRead, WireValue,
};
use testkit::FakeWorkProduct;

use crate::kit_files::{piece_named, Unnamed};
use crate::kit_open;
use crate::tests::tmp::TempDir;

/// A skill of the person's own, in the folder the harness keeps them in.
fn skill(home: &TempDir, name: &str, body: &str) -> PathBuf {
    let folder = adapters::kit_pieces::skills_folder(home.path()).join(name);
    std::fs::create_dir_all(&folder).expect("the skill's folder");
    std::fs::write(folder.join("SKILL.md"), body).expect("its file");
    folder.join("SKILL.md")
}

/// An installed plugin that brings one skill.
fn plugin_skill(home: &TempDir, plugin: &str, name: &str) -> PathBuf {
    let plugins = adapters::kit_pieces::skills_folder(home.path()).with_file_name("plugins");
    let install = plugins.join("cache").join(plugin);
    let folder = install.join("skills").join(name);
    std::fs::create_dir_all(&folder).expect("the plugin's skill folder");
    std::fs::write(folder.join("SKILL.md"), format!("---\nname: {name}\n---\nfrom {plugin}\n")).expect("its file");
    let listed = format!(
        r#"{{"plugins":{{"{plugin}@market":[{{"version":"1","installPath":"{}"}}]}}}}"#,
        install.display()
    );
    std::fs::write(plugins.join("installed_plugins.json"), listed).expect("the install list");
    folder.join("SKILL.md")
}

fn getting(name: &str) -> GetKitFile {
    GetKitFile { kind: KitFileKind::Skills, name: name.to_string() }
}

fn saving(name: &str, read: &str, text: &str) -> SaveKitFile {
    SaveKitFile { kind: KitFileKind::Skills, name: name.to_string(), read: read.to_string(), text: text.to_string() }
}

fn row(name: &str, source: &str) -> SetupItem {
    SetupItem { name: name.to_string(), says: None, source: source.to_string(), style: None, plugin: None, uninstall: None }
}

fn holding(kind: &str, items: Vec<SetupItem>) -> KitInventory {
    KitInventory {
        harness: String::from("h"),
        home: String::from("/home/user"),
        present: true,
        kinds: vec![SetupKindRow { kind: kind.to_string(), read: WhatWasRead::Read { items, unreadable: Vec::new() } }],
    }
}

#[test]
fn a_skills_file_is_the_one_in_its_folder_and_anything_else_the_row_names_is_the_file() {
    let skills = holding("skills", vec![row("docs", "/home/user/.setup/skills/docs")]);
    let found = piece_named(&skills, KitOpenKind::Skills, "docs").expect("named once");
    assert_eq!(found.place, Path::new("/home/user/.setup/skills/docs"));
    assert_eq!(found.file, Path::new("/home/user/.setup/skills/docs/SKILL.md"));

    let relative = holding("commands", vec![row("ship", ".setup/commands/ship.md")]);
    let found = piece_named(&relative, KitOpenKind::Commands, "ship").expect("a row under the home");
    assert_eq!(found.file, Path::new("/home/user/.setup/commands/ship.md"));
}

#[test]
fn a_name_the_inventory_does_not_hold_once_names_no_file() {
    let twice = holding("skills", vec![row("docs", "/home/user/a/docs"), row("docs", "/home/user/b/docs")]);
    assert_eq!(piece_named(&twice, KitOpenKind::Skills, "docs"), Err(Unnamed::Several(2)));
    for name in ["lint", "../../etc/passwd", "/etc/passwd", ""] {
        assert_eq!(piece_named(&twice, KitOpenKind::Skills, name), Err(Unnamed::NotListed), "{name}");
    }
    assert_eq!(piece_named(&twice, KitOpenKind::Commands, "docs"), Err(Unnamed::NotListed));

    let built_in = holding("output_styles", vec![row("Explanatory", "built in to the harness"), row("Armada job rules", "Armada")]);
    for name in ["Explanatory", "Armada job rules"] {
        assert_eq!(piece_named(&built_in, KitOpenKind::OutputStyles, name), Err(Unnamed::NoFile), "{name}");
    }
}

#[tokio::test]
async fn a_file_is_read_and_saved_and_read_back_as_saved() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let file = skill(&home, "docs", "---\nname: docs\n---\nfirst\n");

    let read = fleet.kit_file_read(getting("docs")).await.expect("read");
    assert_eq!(read.path, file.display().to_string());
    assert_eq!(read.text, "---\nname: docs\n---\nfirst\n");
    assert_eq!(read.plugin, None);

    let edited = "---\nname: docs\n---\nsecond\n";
    let saved = fleet.kit_file_saved(saving("docs", &read.text, edited)).await.expect("saved");
    assert_eq!(saved.text, edited);
    assert_eq!(std::fs::read_to_string(&file).expect("on disk"), edited);

    let again = fleet.kit_file_read(getting("docs")).await.expect("read again");
    assert_eq!(again.text, edited);
    fleet.kit_file_saved(saving("docs", &again.text, "third\n")).await.expect("a second save from the second read");
}

#[tokio::test]
async fn a_save_over_a_file_that_moved_since_the_read_is_refused_and_leaves_it() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let file = skill(&home, "docs", "---\nname: docs\n---\nwhat was read\n");
    let read = fleet.kit_file_read(getting("docs")).await.expect("read");

    let elsewhere = "---\nname: docs\n---\nsomebody else's edit\n";
    std::fs::write(&file, elsewhere).expect("the file moves");
    let refused = fleet.kit_file_saved(saving("docs", &read.text, "my edit\n")).await.expect_err("moved");
    assert_eq!(refused.status(), 409);
    assert_eq!(refused.error().code, "fleet.kit_file_moved");
    assert_eq!(refused.error().fields.get("on_disk"), Some(&WireValue::Str(elsewhere.to_string())));
    assert_eq!(std::fs::read_to_string(&file).expect("on disk"), elsewhere);
}

#[tokio::test]
async fn a_piece_a_plugin_brought_reads_and_is_never_saved() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let file = plugin_skill(&home, "impeccable", "polish");

    let read = fleet.kit_file_read(getting("polish")).await.expect("a plugin's skill reads");
    assert_eq!(read.plugin.as_deref(), Some("impeccable"));
    assert!(read.text.contains("from impeccable"));

    let refused = fleet.kit_file_saved(saving("polish", &read.text, "mine\n")).await.expect_err("refused");
    assert_eq!(refused.status(), 422);
    assert_eq!(refused.error().code, "fleet.kit_file_from_plugin");
    assert_eq!(
        refused.error().message,
        "polish comes from the plugin impeccable, and the plugin's next update overwrites it, so nothing was saved."
    );
    assert_eq!(std::fs::read_to_string(&file).expect("on disk"), read.text);
}

#[tokio::test]
async fn a_save_for_a_name_the_inventory_does_not_hold_writes_nothing() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let elsewhere = home.path().join("projects/notes.md");
    std::fs::create_dir_all(elsewhere.parent().expect("a folder")).expect("a folder that is no piece");
    std::fs::write(&elsewhere, "mine\n").expect("a file that is no piece");

    for name in ["notes", "../projects/notes.md", &elsewhere.display().to_string()] {
        let refused = fleet.kit_file_saved(saving(name, "mine\n", "overwritten\n")).await.expect_err(name);
        assert_eq!(refused.error().code, "fleet.kit_file_not_listed", "{name}");
    }
    assert_eq!(std::fs::read_to_string(&elsewhere).expect("on disk"), "mine\n");
}

#[tokio::test]
async fn a_piece_not_named_is_not_opened_and_a_plugin_is_not_opened_in_an_editor() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    plugin_skill(&home, "impeccable", "polish");

    let nowhere = OpenKitPiece { kind: KitOpenKind::Skills, name: String::from("/etc/passwd"), place: KitPlace::Finder };
    let refused = fleet.kit_piece_opened(nowhere).await.expect_err("not listed");
    assert_eq!(refused.error().code, "fleet.kit_file_not_listed");

    let plugin = OpenKitPiece { kind: KitOpenKind::Plugins, name: String::from("impeccable@market"), place: KitPlace::Editor };
    let refused = fleet.kit_piece_opened(plugin).await.expect_err("Finder only");
    assert_eq!(refused.error().code, "fleet.kit_open_plugin_in_editor");
}

fn no_env(_: &str) -> Option<String> {
    None
}

#[test]
fn the_editor_is_the_setting_then_visual_then_editor_then_the_system_opener() {
    let env = |name: &str| match name {
        "VISUAL" => Some(String::from("visual -w")),
        "EDITOR" => Some(String::from("editor")),
        _ => None,
    };
    let chosen = kit_open::choose(env, "  code -g {file}:{line} ");
    assert_eq!((chosen.command.as_str(), chosen.args.as_slice()), ("code", &[String::from("-g"), String::from("{file}:{line}")][..]));
    assert_eq!(kit_open::choose(env, "   ").command, "visual");
    assert_eq!(kit_open::choose(|name| (name == "EDITOR").then(|| String::from("editor")), "").command, "editor");
    assert_eq!(kit_open::choose(no_env, "").command, "/usr/bin/open");
}

#[test]
fn an_editor_that_is_not_there_is_refused_in_words_and_never_falls_through() {
    let missing = kit_open::in_editor("no-such-editor-here", no_env, "/usr/bin:/bin", "/f.md").unwrap_err();
    assert_eq!(missing, "editor.command names no-such-editor-here, and nothing on PATH is called that.");
    let absolute = kit_open::in_editor("/no/such/editor", no_env, "", "/f.md").unwrap_err();
    assert_eq!(absolute, "editor.command names /no/such/editor, and nothing executable is there.");
    let relative = kit_open::in_editor("bin/editor", no_env, "", "/f.md").unwrap_err();
    assert!(relative.contains("which is relative"), "{relative}");
}

#[test]
fn the_path_goes_where_file_stands_and_last_otherwise_and_finder_reveals_it() {
    let sh = kit_open::in_editor("sh -c {file}:{line}", no_env, "/bin:/usr/bin", "/a b.md").expect("sh is on PATH");
    assert_eq!(sh.argv, ["-c", "/a b.md:1"]);
    let last = kit_open::in_editor("sh -x", no_env, "/bin:/usr/bin", "/a.md").expect("sh is on PATH");
    assert_eq!(last.argv, ["-x", "/a.md"]);
    let finder = kit_open::in_finder("/a.md");
    assert_eq!((finder.program.as_path(), finder.argv.as_slice()), (Path::new("/usr/bin/open"), &[String::from("-R"), String::from("/a.md")][..]));
}
