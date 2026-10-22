//! Kit's Add and Remove: a source looked at through a planted repository, installs run by a runner
//! that records what it was asked, and a skill moved to a scratch home's Trash and back. Nothing
//! here reaches the network or the person's own CLI.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

use super::scratch::Scratch;
use crate::kit_pieces::{
    install, look, uninstall, Asked, Choice, Cli, CommandLine, Failed, Found, Installed, Piece, Runner, SourceReader,
    Trash,
};

/// Repositories by name, each its files; and websites by address.
#[derive(Default)]
struct Planted {
    repositories: BTreeMap<String, BTreeMap<String, String>>,
    pages: BTreeMap<String, String>,
}

impl Planted {
    fn repository(mut self, name: &str, files: &[(&str, &str)]) -> Planted {
        let files = files.iter().map(|(path, text)| (path.to_string(), text.to_string())).collect();
        self.repositories.insert(name.to_string(), files);
        self
    }

    fn page(mut self, url: &str, text: &str) -> Planted {
        self.pages.insert(url.to_string(), text.to_string());
        self
    }
}

impl SourceReader for Planted {
    fn paths(&self, repository: &str) -> Result<Vec<String>, String> {
        self.repositories
            .get(repository)
            .map(|files| files.keys().cloned().collect())
            .ok_or_else(|| String::from("Not Found (HTTP 404)"))
    }

    fn file(&self, repository: &str, path: &str) -> Result<Vec<u8>, String> {
        self.repositories
            .get(repository)
            .and_then(|files| files.get(path))
            .map(|text| text.clone().into_bytes())
            .ok_or_else(|| String::from("Not Found (HTTP 404)"))
    }

    fn page(&self, url: &str) -> Result<String, String> {
        self.pages.get(url).cloned().ok_or_else(|| String::from("it answered 404 Not Found"))
    }
}

const MARKETPLACE: &str = r#"{
  "name": "impeccable",
  "owner": { "name": "Paul" },
  "plugins": [{ "name": "impeccable", "source": "./plugin", "version": "4.7.0" }]
}"#;

fn impeccable() -> Planted {
    Planted::default().repository(
        "pbakaus/impeccable",
        &[
            (".claude-plugin/marketplace.json", MARKETPLACE),
            ("plugin/skills/impeccable/SKILL.md", "---\nname: impeccable\n---\n"),
            ("plugin/skills/impeccable/reference/adapt.md", "# Adapt\n"),
            (".cursor/skills/impeccable/SKILL.md", "---\nname: impeccable\n---\n"),
        ],
    )
}

fn shown(commands: &[CommandLine]) -> Vec<String> {
    commands.iter().map(CommandLine::shown).collect()
}

fn why(found: Found) -> String {
    match found {
        Found::Unknown { why } => why,
        other => panic!("found {other:?}"),
    }
}

#[test]
fn a_repository_with_a_marketplace_is_a_plugin_by_its_own_names() {
    let Found::Piece(Piece::Plugin { name, marketplace, repository, skills, commands }) =
        look("pbakaus/impeccable", &impeccable())
    else {
        panic!("not a plugin");
    };
    assert_eq!((name.as_str(), marketplace.as_str()), ("impeccable", "impeccable"));
    assert_eq!(repository, "pbakaus/impeccable");
    assert_eq!(skills, vec!["impeccable"]);
    assert_eq!(
        shown(&commands),
        vec![
            "claude plugin marketplace add pbakaus/impeccable",
            "claude plugin install impeccable@impeccable",
        ]
    );
}

#[test]
fn a_github_address_is_read_as_its_repository() {
    for said in ["https://github.com/pbakaus/impeccable", "github.com/pbakaus/impeccable.git", "https://github.com/pbakaus/impeccable/tree/main/"] {
        assert!(matches!(look(said, &impeccable()), Found::Piece(Piece::Plugin { .. })), "{said}");
    }
}

#[test]
fn a_website_is_followed_to_the_repository_its_install_line_names() {
    let page = r#"<a href="https://github.com/someone/elsewhere">Star</a>
        <code>/plugin marketplace add pbakaus/impeccable</code>"#;
    let reader = impeccable().page("https://impeccable.style", page);
    let Found::Piece(piece) = look("https://impeccable.style/", &reader) else {
        panic!("not found");
    };
    assert_eq!(piece.repository(), "pbakaus/impeccable");
}

#[test]
fn a_website_with_only_a_link_is_followed_to_the_repository_it_links() {
    let page = r#"<a href="https://github.com/sponsors/x">Sponsor</a> <a href="https://github.com/pbakaus/impeccable">Code</a>"#;
    let reader = impeccable().page("https://impeccable.style", page);
    let Found::Piece(piece) = look("https://impeccable.style", &reader) else {
        panic!("not found");
    };
    assert_eq!(piece.repository(), "pbakaus/impeccable");
}

#[test]
fn a_website_that_names_no_repository_is_unknown_with_why() {
    let reader = Planted::default().page("https://example.com", "<p>hello</p>");
    assert!(why(look("https://example.com", &reader)).contains("names no GitHub repository"));
}

#[test]
fn a_skill_at_a_repositorys_root_is_a_skill_by_its_own_name() {
    let reader = Planted::default().repository("someone/writing", &[("SKILL.md", "---\nname: humanizer\n---\n")]);
    let Found::Piece(Piece::Skill { name, repository, commands }) = look("someone/writing", &reader) else {
        panic!("not a skill");
    };
    assert_eq!((name.as_str(), repository.as_str()), ("humanizer", "someone/writing"));
    assert_eq!(
        shown(&commands),
        vec!["npx skills add someone/writing --global --agent claude-code --skill humanizer --copy --yes"]
    );
}

#[test]
fn one_skill_under_skills_is_that_skill() {
    let reader = Planted::default().repository("someone/tools", &[("skills/docs/SKILL.md", "# Docs\n"), ("README.md", "")]);
    let Found::Piece(Piece::Skill { name, .. }) = look("someone/tools", &reader) else {
        panic!("not a skill");
    };
    assert_eq!(name, "docs");
}

fn several(found: Found) -> (String, Vec<Choice>) {
    match found {
        Found::Several { repository, choices } => (repository, choices),
        other => panic!("found {other:?}"),
    }
}

#[test]
fn several_skills_and_no_marketplace_are_each_a_choice_that_installs_only_itself() {
    let reader = Planted::default().repository(
        "someone/tools",
        &[("skills/docs/SKILL.md", "---\nname: docs\ndescription: Writes docs\n---\n"), ("skills/lint/SKILL.md", "")],
    );
    let (repository, choices) = several(look("someone/tools", &reader));
    assert_eq!(repository, "someone/tools");
    let read: Vec<(&str, Option<&str>, Vec<String>)> = choices
        .iter()
        .map(|one| (one.piece.name(), one.says.as_deref(), shown(one.piece.commands())))
        .collect();
    assert_eq!(
        read,
        vec![
            ("docs", Some("Writes docs"), vec![String::from("npx skills add someone/tools --global --agent claude-code --skill docs --copy --yes")]),
            ("lint", None, vec![String::from("npx skills add someone/tools --global --agent claude-code --skill lint --copy --yes")]),
        ]
    );
}

#[test]
fn a_marketplace_with_several_plugins_offers_each_by_its_own_name_and_skills() {
    let market = r#"{"name":"tools","plugins":[
        {"name":"one","source":"./one","description":"The first"},
        {"name":"two","source":"./two"}]}"#;
    let reader = Planted::default().repository(
        "someone/tools",
        &[(".claude-plugin/marketplace.json", market), ("one/skills/alpha/SKILL.md", "")],
    );
    let (_, choices) = several(look("someone/tools", &reader));
    let Piece::Plugin { name, marketplace, skills, commands, .. } = &choices[0].piece else {
        panic!("not a plugin");
    };
    assert_eq!((name.as_str(), marketplace.as_str(), choices[0].says.as_deref()), ("one", "tools", Some("The first")));
    assert_eq!(skills, &vec![String::from("alpha")]);
    assert_eq!(
        shown(commands),
        vec!["claude plugin marketplace add someone/tools", "claude plugin install one@tools"]
    );
    assert_eq!((choices[1].piece.name(), choices[1].says.as_deref()), ("two", None));
    assert_eq!(shown(choices[1].piece.commands())[1], "claude plugin install two@tools");
}

#[test]
fn a_choice_named_like_a_flag_is_never_passed_to_a_command() {
    let market = r#"{"name":"tools","plugins":[{"name":"one"},{"name":"--two"}]}"#;
    let reader = Planted::default().repository("someone/tools", &[(".claude-plugin/marketplace.json", market)]);
    assert!(why(look("someone/tools", &reader)).contains("will not pass to a command"));
}

#[test]
fn a_repository_with_neither_is_unknown_with_why() {
    let reader = Planted::default().repository("someone/app", &[("README.md", "# App\n")]);
    assert!(why(look("someone/app", &reader)).contains("neither a plugin marketplace"));
}

#[test]
fn a_repository_that_will_not_read_is_unknown_with_why() {
    let why = why(look("someone/missing", &Planted::default()));
    assert!(why.contains("could not be read on GitHub: Not Found"), "{why}");
}

#[test]
fn a_source_that_is_neither_a_repository_nor_an_address_is_unknown() {
    assert!(why(look("not a source", &Planted::default())).contains("not a repository"));
    assert!(why(look("--flag/x", &Planted::default())).contains("not a repository"));
}

#[test]
fn a_marketplace_named_like_a_flag_is_never_passed_to_a_command() {
    let market = r#"{"name":"--force","plugins":[{"name":"one"}]}"#;
    let reader = Planted::default().repository("someone/tools", &[(".claude-plugin/marketplace.json", market)]);
    assert!(why(look("someone/tools", &reader)).contains("will not pass to a command"));
}

/// A CLI that keeps a set of marketplaces, records every command, and fails the ones it is told to.
struct Recorded {
    asked: RefCell<Vec<String>>,
    marketplaces: RefCell<BTreeSet<String>>,
    failing: Vec<&'static str>,
}

impl Recorded {
    fn new(held: &[&str], failing: &[&'static str]) -> Recorded {
        Recorded {
            asked: RefCell::new(Vec::new()),
            marketplaces: RefCell::new(held.iter().map(|one| one.to_string()).collect()),
            failing: failing.to_vec(),
        }
    }

    fn asked(&self) -> Vec<String> {
        self.asked.borrow().clone()
    }
}

impl Runner for Recorded {
    fn run(&self, line: &CommandLine) -> Result<String, Failed> {
        let shown = line.shown();
        self.asked.borrow_mut().push(shown.clone());
        if self.failing.iter().any(|fails| shown.starts_with(fails)) {
            return Err(Failed { exit_code: 1, said: format!("{shown} broke") });
        }
        let args: Vec<&str> = line.args.iter().map(String::as_str).collect();
        match args.as_slice() {
            ["plugin", "marketplace", "list", "--json"] => {
                let names: Vec<String> =
                    self.marketplaces.borrow().iter().map(|name| format!("{{\"name\":\"{name}\"}}")).collect();
                Ok(format!("[{}]", names.join(",")))
            }
            ["plugin", "marketplace", "add", _] => {
                self.marketplaces.borrow_mut().insert(String::from("broken-plugin"));
                Ok(String::new())
            }
            ["plugin", "marketplace", "remove", name] => {
                self.marketplaces.borrow_mut().remove(*name);
                Ok(String::new())
            }
            _ => Ok(String::new()),
        }
    }
}

fn broken_plugin() -> Piece {
    let market = r#"{"name":"broken-plugin","plugins":[{"name":"broken","source":"./"}]}"#;
    let reader = Planted::default().repository("someone/broken", &[(".claude-plugin/marketplace.json", market)]);
    match look("someone/broken", &reader) {
        Found::Piece(piece) => piece,
        other => panic!("found {other:?}"),
    }
}

fn asked() -> Asked {
    Asked::when(true).expect("asked")
}

#[test]
fn a_plugin_install_runs_the_commands_the_look_showed_in_order() {
    let cli = Recorded::new(&[], &[]);
    assert_eq!(install(asked(), &broken_plugin(), &cli), Installed::Done);
    assert_eq!(
        cli.asked(),
        vec![
            "claude plugin marketplace list --json",
            "claude plugin marketplace add someone/broken",
            "claude plugin install broken@broken-plugin",
        ]
    );
}

#[test]
fn a_failed_plugin_install_takes_back_the_marketplace_it_added() {
    let cli = Recorded::new(&[], &["claude plugin install"]);
    let installed = install(asked(), &broken_plugin(), &cli);
    assert_eq!(
        installed,
        Installed::Failed {
            command: String::from("claude plugin install broken@broken-plugin"),
            exit_code: 1,
            error: String::from("claude plugin install broken@broken-plugin broke"),
            taken_back: vec![String::from("the broken-plugin marketplace")],
        }
    );
    assert!(cli.marketplaces.borrow().is_empty(), "left {:?}", cli.marketplaces.borrow());
    assert_eq!(cli.asked().last().map(String::as_str), Some("claude plugin marketplace remove broken-plugin"));
}

#[test]
fn a_marketplace_the_person_already_had_is_never_taken_back() {
    let cli = Recorded::new(&["broken-plugin"], &["claude plugin install"]);
    let Installed::Failed { taken_back, .. } = install(asked(), &broken_plugin(), &cli) else {
        panic!("installed");
    };
    assert!(taken_back.is_empty());
    assert!(cli.marketplaces.borrow().contains("broken-plugin"));
    assert!(!cli.asked().iter().any(|one| one.contains("marketplace remove")));
}

#[test]
fn a_marketplace_that_will_not_add_stops_the_install_and_says_so() {
    let cli = Recorded::new(&[], &["claude plugin marketplace add"]);
    let Installed::Failed { command, taken_back, .. } = install(asked(), &broken_plugin(), &cli) else {
        panic!("installed");
    };
    assert_eq!(command, "claude plugin marketplace add someone/broken");
    assert!(taken_back.is_empty());
    assert!(!cli.asked().iter().any(|one| one.starts_with("claude plugin install")));
}

#[test]
fn a_skill_install_runs_its_one_line_and_a_failure_names_it() {
    let reader = Planted::default().repository("someone/writing", &[("SKILL.md", "---\nname: humanizer\n---\n")]);
    let Found::Piece(piece) = look("someone/writing", &reader) else {
        panic!("not found");
    };
    let cli = Recorded::new(&[], &["npx"]);
    let Installed::Failed { command, .. } = install(asked(), &piece, &cli) else {
        panic!("installed");
    };
    assert_eq!(command, "npx skills add someone/writing --global --agent claude-code --skill humanizer --copy --yes");
    assert_eq!(cli.asked(), vec![command]);
}

#[test]
fn a_plugin_is_uninstalled_by_its_name_and_marketplace() {
    let cli = Recorded::new(&[], &[]);
    uninstall("impeccable@impeccable", &cli).expect("uninstalled");
    assert_eq!(cli.asked(), vec!["claude plugin uninstall impeccable@impeccable"]);
}

#[test]
fn the_cli_runs_each_argument_as_itself_on_the_path_fleet_gives_it() {
    let scratch = Scratch::new();
    let log = scratch.path().join("log");
    let bin = scratch.folder("bin");
    let program = scratch.file("bin/claude", &format!("#!/bin/sh\nfor a in \"$@\"; do echo \"[$a]\" >> {}; done\n", log.display()));
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("executable");
    let cli = Cli {
        path: format!("{}:/usr/bin:/bin", bin.display()),
        home: scratch.path().to_path_buf(),
        within: Duration::from_secs(30),
    };
    uninstall("a b@c", &cli).expect("ran");
    assert_eq!(std::fs::read_to_string(log).expect("the log"), "[plugin]\n[uninstall]\n[a b@c]\n");
}

#[test]
fn a_cli_that_exits_non_zero_comes_back_with_its_code_and_words() {
    let scratch = Scratch::new();
    let bin = scratch.folder("bin");
    let program = scratch.file("bin/claude", "#!/bin/sh\necho no such plugin >&2\nexit 3\n");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("executable");
    let cli = Cli { path: bin.display().to_string(), home: scratch.path().to_path_buf(), within: Duration::from_secs(30) };
    assert_eq!(uninstall("x@y", &cli), Err(Failed { exit_code: 3, said: String::from("no such plugin") }));
}

fn skill_in(home: &Path, name: &str) -> std::path::PathBuf {
    let folder = home.join(".claude/skills").join(name);
    std::fs::create_dir_all(&folder).expect("the skill");
    std::fs::write(folder.join("SKILL.md"), format!("---\nname: {name}\n---\n")).expect("its file");
    folder
}

#[test]
fn a_skill_moves_to_the_trash_and_is_put_back() {
    let scratch = Scratch::new();
    let home = scratch.path();
    let folder = skill_in(home, "humanizer");
    let trash = Trash::of(home);

    let trashed = trash.put(home, &folder).expect("trashed");
    assert_eq!(trashed, home.join(".Trash/humanizer"));
    assert!(!folder.exists());
    assert!(trashed.join("SKILL.md").is_file());

    trash.take_back(home, &trashed, &folder).expect("put back");
    assert!(folder.join("SKILL.md").is_file());
    assert!(!trashed.exists());
}

#[test]
fn a_second_skill_of_the_same_name_takes_a_new_place_in_the_trash() {
    let scratch = Scratch::new();
    let home = scratch.path();
    let trash = Trash::of(home);
    let first = trash.put(home, &skill_in(home, "docs")).expect("first");
    let second = trash.put(home, &skill_in(home, "docs")).expect("second");
    assert_ne!(first, second);
    assert_eq!(second, home.join(".Trash/docs 2"));
}

#[test]
fn a_folder_outside_the_persons_skills_is_never_trashed() {
    let scratch = Scratch::new();
    let home = scratch.path();
    let elsewhere = scratch.folder("project");
    let plugin_skill = scratch.folder(".claude/plugins/cache/x/skills/y");
    let trash = Trash::of(home);
    for folder in [elsewhere.clone(), plugin_skill.clone(), home.join(".claude/skills/../../project"), home.join(".claude/skills")] {
        assert!(trash.put(home, &folder).is_err(), "{}", folder.display());
    }
    assert!(elsewhere.is_dir() && plugin_skill.is_dir());
}

#[test]
fn putting_back_refuses_anything_not_in_the_trash_or_a_place_outside_the_skills() {
    let scratch = Scratch::new();
    let home = scratch.path();
    let trash = Trash::of(home);
    let trashed = trash.put(home, &skill_in(home, "docs")).expect("trashed");
    assert!(trash.take_back(home, &trashed, &home.join("Documents/docs")).is_err());
    let loose = scratch.folder("loose");
    assert!(trash.take_back(home, &loose, &home.join(".claude/skills/loose")).is_err());
    skill_in(home, "docs");
    assert!(trash.take_back(home, &trashed, &home.join(".claude/skills/docs")).is_err());
    assert!(trashed.is_dir());
}
