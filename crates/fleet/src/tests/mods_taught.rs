//! The kinds a teaching makes: Words, Fleet and Tweak, and the Commands kind beside them. Each accepts what it should, refuses
//! with a reason what it should not, is listed like any mod, and is made by a scaffold.

use std::path::Path;

use api::Mods;
use ipc::{AddressFound, Does, FleetFile, FleetRule, ModKind, RuleScope, ScaffoldMod};
use testkit::FakeWorkProduct;

use crate::mods::scaffold::{self, Taught};
use crate::mods::{commands, examine, fleet_rules, scan, stylesheet, words};
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

const JIRA: &str = "name = \"Jira key\"\npattern = \"[A-Z][A-Z0-9]+-\\\\d+\"\nexample = \"ARM-123\"\n\n\
    [[address]]\nrepository = \"armada\"\ntemplate = \"https://acme.example.net/browse/{match}\"\nfound = \"settings\"\n\n\
    [[address]]\nrepository = \"*\"\ntemplate = \"https://other.example.net/{match}\"\nfound = \"pull_request #12\"\n";

fn mod_toml(name: &str, kind: &str) -> String {
    format!("name = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n")
}

fn a_taught_mod(dir: &Path, name: &str, kind: &str, file: &str, text: &str) {
    let at = dir.join(name);
    std::fs::create_dir_all(&at).expect("a folder");
    std::fs::write(at.join("mod.toml"), mod_toml(name, kind)).expect("mod.toml");
    std::fs::write(at.join(file), text).expect("the file");
}

fn said(problems: &[String], wanted: &str) -> bool {
    problems.iter().any(|one| one.contains(wanted))
}

#[test]
fn a_words_file_with_addresses_is_accepted_and_reads_back_typed() {
    assert_eq!(words::problems(JIRA), Vec::<String>::new());
    let file = words::read(JIRA).expect("reads");
    assert_eq!(file.name, "Jira key");
    assert_eq!(file.address[0].found, AddressFound::Settings);
    assert_eq!(file.address[1].found, AddressFound::PullRequest(12));
}

#[test]
fn a_words_file_that_bridge_could_not_run_is_refused_with_its_reason() {
    let with = |change: &dyn Fn(&str) -> String| change(JIRA);
    let cases: Vec<(String, &str)> = vec![
        (with(&|t| t.replace("[A-Z][A-Z0-9]+-\\\\d+", "BLOG-\\\\d+")), "does not match all of"),
        (with(&|t| t.replace("[A-Z][A-Z0-9]+-\\\\d+", "([A-Z]+)+-\\\\d+")), "repeats a group"),
        (with(&|t| t.replace("[A-Z][A-Z0-9]+-\\\\d+", "(?i)arm-\\\\d+")), "cannot read"),
        (with(&|t| t.replace("https://acme.example.net/browse/{match}", "ftp://acme.example.net/{match}")), "not an http or https address"),
        (with(&|t| t.replace("https://acme.example.net/browse/{match}", "https://acme.example.net/browse/")), "no `{match}`"),
        (with(&|t| t.replace("https://other.example.net/{match}", "https://{match}.example.net/")), "host"),
        (with(&|t| t.replace("\"*\"", "\"armada\"")), "two addresses"),
        (with(&|t| t.replace("\"settings\"", "\"guessed\"")), "`guessed` is not"),
        (with(&|t| t.replace("name = \"Jira key\"", "name = \"\"")), "`name` is 1 to"),
        (with(&|t| format!("{t}\nowner = \"me\"\n")), "unknown field"),
        ("this is not toml".to_string(), "words.toml line 1"),
    ];
    for (text, wanted) in cases {
        let found = words::problems(&text);
        assert!(said(&found, wanted), "{text:?} should say {wanted:?}, got {found:?}");
    }
}

#[test]
fn a_fleet_file_names_notices_that_exist_and_places_that_parse() {
    let good = "[[rule]]\nnotice = \"behind_base\"\ndoes = \"batch\"\nwhere = \"everywhere\"\n\n\
        [[rule]]\nnotice = \"behind_base\"\ndoes = \"quiet\"\nwhere = \"job 84ed9507\"\n";
    assert_eq!(fleet_rules::problems(good), Vec::<String>::new());
    assert_eq!(fleet_rules::problems(""), Vec::<String>::new(), "no rule is a file that changes nothing");
    let rule = |notice: &str, does: &str, place: &str| format!("[[rule]]\nnotice = \"{notice}\"\ndoes = \"{does}\"\nwhere = \"{place}\"\n");
    let cases = [
        (rule("not_a_notice", "tell", "everywhere"), "is not a notice Fleet sends"),
        (rule("behind_base", "shout", "everywhere"), "shout"),
        (rule("behind_base", "tell", "somewhere"), "is not `job <id>`"),
        (rule("behind_base", "tell", "job "), "is not `job <id>`"),
        (format!("{0}{0}", rule("behind_base", "tell", "repo armada")), "ruled twice"),
        ("[[rule]]\nnotice = \"behind_base\"\ndoes = \"tell\"\n".to_string(), "missing field `where`"),
    ];
    for (text, wanted) in cases {
        let found = fleet_rules::problems(&text);
        assert!(said(&found, wanted), "{text:?} should say {wanted:?}, got {found:?}");
    }
}

#[test]
fn a_bad_matches_or_a_bad_condition_makes_the_fleet_file_invalid_and_says_why() {
    let rule = |extra: &str| format!("[[rule]]\nnotice = \"behind_base\"\ndoes = \"batch\"\nwhere = \"everywhere\"\n{extra}\n");
    let good = [
        rule("matches = \"behind \\\\d+\""),
        rule("when = [{ if = \"blocks_merge\", does = \"tell\" }, { if = \"owner_away\", does = \"quiet\" }]"),
        format!("{}{}", rule("matches = \"lint\""), rule("matches = \"fmt\"")),
    ];
    for text in good {
        assert_eq!(fleet_rules::problems(&text), Vec::<String>::new(), "{text}");
    }
    let long = "a".repeat(201);
    let cases = [
        (rule("matches = \"(a+)+\""), "repeats a group"),
        (rule("matches = \"(a|aa)*\""), "repeats a group"),
        (rule("matches = \"(?i)lint\""), "a flag, a name or a lookaround"),
        (rule("matches = \"(?=lint)\""), "a flag, a name or a lookaround"),
        (rule("matches = \"\""), "empty or over 200 characters"),
        (rule("matches = \"a*\""), "matches the empty string"),
        (rule("matches = \"[\""), "does not compile"),
        (rule(&format!("matches = \"{long}\"")), "over 200 characters"),
        (rule("when = [{ if = \"is_friday\", does = \"tell\" }]"), "is_friday"),
        (rule("when = [{ if = \"working\", does = \"shout\" }]"), "shout"),
        (rule("when = [{ if = \"working\", does = \"tell\" }, { if = \"working\", does = \"quiet\" }]"), "tested twice"),
        (format!("{0}{0}", rule("matches = \"lint\"")), "ruled twice"),
    ];
    for (text, wanted) in cases {
        let found = fleet_rules::problems(&text);
        assert!(said(&found, wanted), "{text:?} should say {wanted:?}, got {found:?}");
    }
}

#[test]
fn a_mod_with_a_bad_matches_is_invalid_in_the_list_with_the_reason() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    let head = "[[rule]]\nnotice = \"behind_base\"\ndoes = \"batch\"\nwhere = \"everywhere\"\n";
    a_taught_mod(&mods, "fine", "fleet", "fleet.toml", &format!("{head}matches = \"behind\"\n"));
    a_taught_mod(&mods, "hangs", "fleet", "fleet.toml", &format!("{head}matches = \"(a+)+\"\n"));

    let list = scan(&mods, &Default::default(), &Default::default());
    let row = |name: &str| list.mods.iter().find(|one| one.name == name).expect(name).clone();
    assert!(row("fine").valid);
    let hangs = row("hangs");
    assert!(!hangs.valid && hangs.reason.expect("why").contains("repeats a group"));
}

#[test]
fn the_nearest_place_wins_and_a_later_file_wins_a_tie() {
    let file = |text: &str| fleet_rules::read(text).expect("reads");
    let rule = |does: &str, place: &str| format!("[[rule]]\nnotice = \"behind_base\"\ndoes = \"{does}\"\nwhere = \"{place}\"\n");
    let everywhere = file(&rule("quiet", "everywhere"));
    let repo = file(&rule("batch", "repo armada"));
    let job = file(&rule("call", "job s1"));
    let later_everywhere = file(&rule("tell", "everywhere"));
    let files = [job.clone(), repo.clone(), everywhere.clone()];
    assert_eq!(fleet_rules::rule_for(&files, "behind_base", "", "s1", "armada").map(|rule| rule.does), Some(Does::Call));
    assert_eq!(fleet_rules::rule_for(&files, "behind_base", "", "s2", "armada").map(|rule| rule.does), Some(Does::Batch));
    assert_eq!(fleet_rules::rule_for(&files, "behind_base", "", "s2", "other").map(|rule| rule.does), Some(Does::Quiet));
    assert_eq!(fleet_rules::rule_for(&files, "other_notice", "", "s1", "armada").map(|rule| rule.does), None);
    assert_eq!(fleet_rules::rule_for(&[everywhere, later_everywhere], "behind_base", "", "s2", "other").map(|rule| rule.does), Some(Does::Tell));
}

#[test]
fn a_tweak_holds_token_declarations_and_names_its_own_file_in_a_problem() {
    assert_eq!(stylesheet::problems_in(":root {\n  --accent: #E0954A;\n}\n", "tweak.css"), Vec::<String>::new());
    let found = stylesheet::problems_in(":root {\n  --accent: url(x);\n  color: red;\n}\n", "tweak.css");
    assert!(found[0].starts_with("tweak.css line 2:") && said(&found, "`url(`"), "{found:?}");
    assert!(said(&found, "`color` is not a design token"), "{found:?}");
    assert!(stylesheet::problems(":root { color: red; }")[0].starts_with("theme.css line 1:"));
}

#[test]
fn each_new_kind_is_read_from_its_own_file_and_a_wrong_file_is_that_mods_problem() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_taught_mod(&mods, "jira", "words", "words.toml", JIRA);
    a_taught_mod(&mods, "quiet", "fleet", "fleet.toml", "[[rule]]\nnotice = \"behind_base\"\ndoes = \"quiet\"\nwhere = \"everywhere\"\n");
    a_taught_mod(&mods, "warm", "tweak", "tweak.css", ":root { --accent: #E0954A; }");
    a_taught_mod(&mods, "bad-tweak", "tweak", "tweak.css", ":root { --accent: url(x); }");
    a_taught_mod(&mods, "bad-words", "words", "words.toml", "pattern = 1");
    a_taught_mod(&mods, "no-file", "fleet", "theme.css", ":root {}");

    let list = scan(&mods, &Default::default(), &Default::default());
    let row = |name: &str| list.mods.iter().find(|one| one.name == name).expect(name).clone();
    for (name, kind) in [("jira", ModKind::Words), ("quiet", ModKind::Fleet), ("warm", ModKind::Tweak)] {
        let one = row(name);
        assert_eq!((one.kind, one.valid, one.reason), (Some(kind), true, None), "{name}");
    }
    assert!(row("bad-tweak").reason.expect("why").starts_with("tweak.css line 1"));
    assert!(row("bad-words").reason.expect("why").starts_with("words.toml line"));
    assert_eq!(row("no-file").reason.as_deref(), Some("fleet.toml is missing"));
    assert!(examine(&mods, "jira").files.is_some() && examine(&mods, "bad-words").files.is_none());
}

#[test]
fn a_mod_row_lists_what_it_was_taught_on() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    let toml = format!("{}taught_on = \"The Cockpit's merge line header\"\nentry = \"41\"\n", mod_toml("tidy", "tweak"));
    let at = mods.join("tidy");
    std::fs::create_dir_all(&at).expect("a folder");
    std::fs::write(at.join("mod.toml"), toml).expect("mod.toml");
    std::fs::write(at.join("tweak.css"), ":root {}\n").expect("css");
    let list = scan(&mods, &Default::default(), &Default::default());
    assert_eq!((list.mods[0].kind, list.mods[0].valid), (Some(ModKind::Tweak), true));
    assert_eq!(list.mods[0].taught_on.as_deref(), Some("The Cockpit's merge line header"));
    assert_eq!(examine(&mods, "tidy").manifest.entry.as_deref(), Some("41"));

    for (key, why) in [("taught_on", "longer than 120"), ("entry", "`entry` must be")] {
        let long = if key == "taught_on" { "x".repeat(121) } else { "a b".to_string() };
        let (_, found) = crate::mods::manifest::read(&format!("{}{key} = \"{long}\"\n", mod_toml("tidy", "tweak")), "tidy");
        assert!(said(&found, why), "{key}: {found:?}");
    }
}

#[test]
fn a_scaffold_of_each_new_kind_is_valid_at_once_and_a_taught_one_carries_its_text_and_origin() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    for (name, kind) in [("w", ModKind::Words), ("f", ModKind::Fleet), ("t", ModKind::Tweak)] {
        scaffold::make(&mods, name, None, kind, None).expect("made");
        let found = examine(&mods, name);
        assert!(found.valid(), "{name}: {:?}", found.problems);
    }
    let taught = Taught { text: ":root {\n  --accent: #E0954A;\n}\n", taught_on: Some("a \"row\""), entry: Some("41") };
    scaffold::make_taught(&mods, "warm", Some("Warmer"), ModKind::Tweak, Some(&taught)).expect("made");
    let found = examine(&mods, "warm");
    assert!(found.valid(), "{:?}", found.problems);
    assert_eq!((found.manifest.taught_on.as_deref(), found.manifest.entry.as_deref()), (Some("a \"row\""), Some("41")));
    assert_eq!(found.files.expect("valid").payload.text(), taught.text);
    let many = "x".repeat(121);
    let long = Taught { taught_on: Some(&many), ..taught };
    assert!(scaffold::make_taught(&mods, "long", None, ModKind::Tweak, Some(&long)).is_err());
    assert!(!mods.join("long").exists(), "nothing left behind");
}

#[tokio::test]
async fn fleet_scaffolds_validates_and_lists_a_tweak_and_hands_bridge_its_css() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let made = fleet
        .scaffold_mod(ScaffoldMod { name: "warm".into(), description: None, kind: Some(ModKind::Tweak), session_id: None })
        .await
        .expect("scaffolded");
    std::fs::write(Path::new(&made.path).join("tweak.css"), ":root { --accent: #E0954A; }\n").expect("edited");
    let checked = fleet.validate_mod("warm".into()).await.expect("checked");
    assert!(checked.valid && checked.css.as_deref() == Some(":root { --accent: #E0954A; }\n"), "{checked:?}");
    let words = fleet
        .scaffold_mod(ScaffoldMod { name: "keys".into(), description: None, kind: Some(ModKind::Words), session_id: None })
        .await
        .expect("scaffolded");
    assert!(Path::new(&words.path).join("words.toml").is_file());
    let list = fleet.list_mods().await.expect("listed");
    assert_eq!(list.mods.iter().filter(|one| one.valid).count(), 2);
}

#[test]
fn a_mod_row_names_the_logbook_entry_that_made_it_and_a_hand_made_mod_names_none() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_taught_mod(&mods, "tidy", "tweak", "tweak.css", ":root {}\n");
    std::fs::write(mods.join("tidy/mod.toml"), format!("{}entry = \"41\"\n", mod_toml("tidy", "tweak"))).expect("mod.toml");
    a_taught_mod(&mods, "by-hand", "tweak", "tweak.css", ":root {}\n");
    // A mod whose files are wrong still says where it came from: forgetting the entry finds it so.
    a_taught_mod(&mods, "broken", "tweak", "tweak.css", ":root { color: red; }");
    std::fs::write(mods.join("broken/mod.toml"), format!("{}entry = \"42\"\n", mod_toml("broken", "tweak"))).expect("mod.toml");

    let list = scan(&mods, &Default::default(), &Default::default());
    let entry = |name: &str| list.mods.iter().find(|one| one.name == name).expect(name).entry.clone();
    assert_eq!(entry("tidy").as_deref(), Some("41"));
    assert_eq!(entry("by-hand"), None);
    assert_eq!(entry("broken").as_deref(), Some("42"));
}

#[tokio::test]
async fn validating_a_taught_mod_hands_bridge_what_it_holds_and_a_broken_one_holds_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let mods = fleet.mods_dir();
    let rules = "[[rule]]\nnotice = \"behind_base\"\ndoes = \"quiet\"\nwhere = \"everywhere\"\n";
    a_taught_mod(&mods, "jira", "words", "words.toml", JIRA);
    a_taught_mod(&mods, "quiet", "fleet", "fleet.toml", rules);
    a_taught_mod(&mods, "warm", "tweak", "tweak.css", ":root { --accent: #E0954A; }\n");
    a_taught_mod(&mods, "bad-words", "words", "words.toml", "pattern = 1");
    a_taught_mod(&mods, "bad-fleet", "fleet", "fleet.toml", "[[rule]]\nnotice = \"not_a_notice\"\ndoes = \"tell\"\nwhere = \"everywhere\"\n");

    let jira = fleet.validate_mod("jira".into()).await.expect("checked");
    assert_eq!(jira.words, Some(words::read(JIRA).expect("reads")), "{jira:?}");
    assert!(jira.valid && jira.fleet.is_none() && jira.css.is_none() && jira.layout.is_none(), "{jira:?}");
    let wire = ipc::encode(&jira).expect("plain data");
    assert!(wire.contains(r#""found":"pull_request #12""#) && wire.contains(r#""repository":"armada""#), "{wire}");

    let quiet = fleet.validate_mod("quiet".into()).await.expect("checked");
    let rule = FleetRule { notice: "behind_base".into(), does: Does::Quiet, scope: RuleScope::Everywhere, matches: None, when: Vec::new() };
    assert_eq!(quiet.fleet, Some(FleetFile { rule: vec![rule] }), "{quiet:?}");
    assert!(quiet.valid && quiet.words.is_none() && quiet.css.is_none(), "{quiet:?}");

    let warm = fleet.validate_mod("warm".into()).await.expect("checked");
    assert!(warm.css.is_some() && warm.words.is_none() && warm.fleet.is_none(), "{warm:?}");
    for name in ["bad-words", "bad-fleet"] {
        let broken = fleet.validate_mod(name.into()).await.expect("answers for a broken mod");
        assert!(!broken.valid && broken.words.is_none() && broken.fleet.is_none(), "{name}: {broken:?}");
    }
}

const ROWS: &str = "[[command]]\nname = \"Review a pull request for security\"\naliases = [\"sec\", \"audit\"]\nhotkey = \"⌥⌘S\"\n\
    prompt = \"Review {pr} for security issues in {repo}, then {pr} again\"\n\n\
    [[command]]\nname = \"Open the board\"\nurl = \"https://grafana.example.com/d/{repo}\"\n\n\
    [[command]]\nname = \"Audit dependencies\"\nshell = \"pnpm audit\"\n\n\
    [[command]]\nname = \"Add a task\"\nchain = [\"nav:projects\", \"act:add_task\"]\n";

/// One `[[command]]` of `name` holding `rest`, a line or more.
fn one(name: &str, rest: &str) -> String {
    format!("[[command]]\nname = \"{name}\"\n{rest}\n")
}

fn refused(text: &str, wanted: &str) {
    let found = commands::problems(text);
    assert!(said(&found, wanted), "{text:?} should say {wanted:?}, got {found:?}");
    assert!(found.iter().all(|one| one.starts_with("commands.toml")), "{found:?}");
}

#[test]
fn a_commands_file_with_each_kind_of_row_is_accepted_and_reads_back_with_its_blanks() {
    assert_eq!(commands::problems(ROWS), Vec::<String>::new());
    let file = commands::read(ROWS).expect("reads");
    assert_eq!(file.command.len(), 4);
    let sec = &file.command[0];
    assert_eq!((sec.name.as_str(), sec.aliases.as_slice(), sec.hotkey.as_deref()), ("Review a pull request for security", &["sec".to_string(), "audit".to_string()][..], Some("⌥⌘S")));
    assert_eq!(sec.blanks, ["pr", "repo"], "once each, in the order they first appear");
    assert_eq!(file.command[1].blanks, ["repo"]);
    assert_eq!(file.command[2].shell.as_deref(), Some("pnpm audit"));
    assert!(file.command[2].blanks.is_empty());
    assert_eq!(file.command[3].chain.as_deref(), Some(&["nav:projects".to_string(), "act:add_task".to_string()][..]));
}

#[test]
fn a_commands_file_with_no_command_is_refused() {
    refused("", "at least one `[[command]]`");
    refused("# nothing\n", "at least one `[[command]]`");
}

#[test]
fn a_command_with_no_name_is_refused() {
    refused(&one("", "shell = \"ls\""), "command 1 has no name");
    refused(&one("   ", "shell = \"ls\""), "command 1 has no name");
}

#[test]
fn a_command_must_do_exactly_one_of_prompt_url_shell_or_chain() {
    refused(&one("Nothing", ""), "`Nothing` does nothing");
    refused(&one("Two", "prompt = \"a\"\nshell = \"b\""), "`Two` does more than one");
    refused(&one("All", "prompt = \"a\"\nurl = \"https://x.example\"\nshell = \"b\"\nchain = [\"nav:x\"]"), "`All` does more than one");
    refused(&one("Blank", "shell = \"  \""), "`Blank` has an empty shell");
}

#[test]
fn a_blank_must_be_spelled_like_a_name_and_only_prompt_url_and_shell_take_one() {
    refused(&one("Loud", "prompt = \"Review {Pr}\""), "`Loud` has `{Pr}`");
    refused(&one("Digit", "url = \"https://x.example/{1}\""), "`Digit` has `{1}`");
    refused(&one("Open", "shell = \"echo {oops\""), "`Open` has a `{` with no `}`");
    refused(&one("Linked", "chain = [\"nav:{repo}\"]"), "`Linked` takes no blanks in a chain");
    let braces = one("Awk", "shell = \"awk '{print $1} {a,b}' {pr}\"");
    assert_eq!(commands::problems(&braces), Vec::<String>::new(), "braces that hold no name are text");
    assert_eq!(commands::read(&braces).expect("reads").command[0].blanks, ["print", "pr"]);
}

#[test]
fn a_chain_must_name_a_row_and_every_one_must_be_a_row_id() {
    refused(&one("Empty", "chain = []"), "`Empty` has an empty chain");
    refused(&one("Holes", "chain = [\"nav:projects\", \"\"]"), "`Holes` has an empty row id in its chain");
}

#[test]
fn two_commands_may_not_share_a_name() {
    refused(&format!("{}{}", one("Same", "shell = \"a\""), one("Same", "shell = \"b\"")), "`Same` is named twice");
}

#[test]
fn a_hotkey_is_a_chord_of_modifiers_and_at_most_one_key() {
    for good in ["⌥⌘S", "S", "⇧⌘R", "⌘[", "Esc", "⌘F5", "⌃⌥⇧⌘K", "⌥↑"] {
        let text = one("Keyed", &format!("shell = \"a\"\nhotkey = \"{good}\""));
        assert_eq!(commands::problems(&text), Vec::<String>::new(), "{good}");
    }
    for (bad, wanted) in [("", "empty hotkey"), ("  ", "empty hotkey"), ("⌘SD", "one key"), ("⌘S⌘", "one key"), ("⌘ S", "one key"), ("⌘Nope", "one key")] {
        let text = one("Keyed", &format!("shell = \"a\"\nhotkey = \"{bad}\""));
        refused(&text, wanted);
    }
}

#[test]
fn a_commands_file_that_is_not_the_toml_described_is_refused_with_its_line() {
    refused("this is not toml", "commands.toml line 1");
    refused(&one("Extra", "shell = \"a\"\nowner = \"me\""), "unknown field");
}

#[test]
fn a_commands_mod_is_listed_validated_and_handed_to_bridge_checked() {
    let home = TempDir::new();
    let mods = home.path().join("mods");
    a_taught_mod(&mods, "rows", "commands", "commands.toml", ROWS);
    a_taught_mod(&mods, "bad-rows", "commands", "commands.toml", &one("Two", "prompt = \"a\"\nshell = \"b\""));
    a_taught_mod(&mods, "no-rows", "commands", "words.toml", ROWS);
    let list = scan(&mods, &Default::default(), &Default::default());
    let row = |name: &str| list.mods.iter().find(|one| one.name == name).expect(name).clone();
    let good = row("rows");
    assert_eq!((good.kind, good.valid, good.reason), (Some(ModKind::Commands), true, None));
    assert!(row("bad-rows").reason.expect("why").starts_with("commands.toml: `Two` does more than one"));
    assert_eq!(row("no-rows").reason.as_deref(), Some("commands.toml is missing"));
    assert!(examine(&mods, "rows").files.is_some() && examine(&mods, "bad-rows").files.is_none());
}

#[tokio::test]
async fn fleet_scaffolds_a_commands_mod_that_validates_and_hands_bridge_its_rows() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let made = fleet
        .scaffold_mod(ScaffoldMod { name: "rows".into(), description: None, kind: Some(ModKind::Commands), session_id: None })
        .await
        .expect("scaffolded");
    let starter = fleet.validate_mod("rows".into()).await.expect("checked");
    assert!(starter.valid && starter.commands.is_some(), "a starter is a file that passes: {starter:?}");
    std::fs::write(Path::new(&made.path).join("commands.toml"), ROWS).expect("edited");
    let checked = fleet.validate_mod("rows".into()).await.expect("checked");
    assert!(checked.valid && checked.css.is_none() && checked.words.is_none(), "{checked:?}");
    let rows = checked.commands.expect("rows");
    assert_eq!((rows.command.len(), rows.command[0].blanks.clone()), (4, vec!["pr".to_string(), "repo".to_string()]));
}
