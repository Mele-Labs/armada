//! The shared home's adapters over planted folders: a folder written and refused, the harness's
//! plugins read with where each marketplace comes from, and each profile's copy of its home named
//! by its own origin. Nothing reaches the forge or runs a process.

use adapter_traits::{FileRead, HarnessHome, HomeFailure, HomePlace, HomeRepositories, KeptFolder, PluginRecord, Secret};

use super::github_api::{ok, serve, Reply, Seen, Server};
use super::scratch::Scratch;
use crate::github_api::GitHubApi;
use crate::shared_home::{credential, not_held, ForgeHome};
use crate::{AgentHome, Folder, ForgeHomes};

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
fn each_profiles_home_is_its_own_clone_and_acts_as_that_login_alone() {
    let scratch = Scratch::new();
    let homes = ForgeHomes::in_dir(scratch.folder("home"));
    let (personal, work) = (homes.of("nick", &[]).expect("a login"), homes.of("nick-work", &[]).expect("a login"));
    assert_eq!(personal.here(), None, "no clone yet");
    scratch.file("home/nick/.git/config", "[core]\n\tbare = false\n[remote \"origin\"]\n\turl = https://github.com/nick/armada-home.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n");
    assert_eq!(personal.here().as_deref(), Some("nick/armada-home"));
    assert_eq!(work.here(), None, "one profile's clone is not another's");
    scratch.file("home/nick-work/.git/config", "[remote \"origin\"]\n\turl = https://github.com/nick/armada-home.git\n");
    assert_eq!(work.here(), None, "a clone of another login's home is not taken for this one's");
    scratch.file("home/nick-work/.git/config", "[remote \"origin\"]\n\turl = git@github.com:nick-work/armada-home.git\n");
    assert_eq!(work.here(), None, "only over HTTPS");
    for not_a_login in ["", "../nick", "nick/work", "-nick", "nick work"] {
        assert!(homes.of(not_a_login, &[]).is_err(), "{not_a_login:?} is refused");
    }

    let credential = credential("nick-work");
    assert_eq!(credential[1], "credential.helper=", "every helper configured is cleared first");
    assert!(credential[3].contains("gh auth token --hostname github.com --user nick-work"), "{}", credential[3]);
    let refused = not_held("nick-work");
    assert!(refused.contains("nick-work") && refused.contains("`gh auth login`"), "{refused}");
}

fn owners(logins: &[&str]) -> Vec<String> {
    logins.iter().map(|login| login.to_string()).collect()
}

#[test]
fn a_clone_owned_by_an_organisation_of_the_login_is_its_home_and_one_owned_by_any_other_is_not() {
    let scratch = Scratch::new();
    let homes = ForgeHomes::in_dir(scratch.folder("home"));
    scratch.file("home/nick/.git/config", "[remote \"origin\"]\n\turl = https://github.com/mele-labs/kit.git\n");
    assert_eq!(homes.of("nick", &owners(&["nick", "mele-labs"])).expect("a login").here().as_deref(), Some("mele-labs/kit"));
    assert_eq!(homes.of("nick", &owners(&["nick", "acme"])).expect("a login").here(), None, "mele-labs is not one of its organisations");
    assert_eq!(homes.of("nick", &[]).expect("a login").here(), None, "no organisations, only the login");
}

fn at(owner: &str, name: &str) -> HomePlace {
    HomePlace { owner: owner.to_string(), name: name.to_string() }
}

/// `nick`'s home, asked of a forge on this machine that answers by `answer`.
fn forge(answer: impl Fn(&Seen) -> Reply + Send + Sync + 'static) -> (Server, ForgeHome) {
    let server = serve(answer);
    let api = GitHubApi::against(&server.url, || Ok(Secret::new("token".to_string())));
    let home = ForgeHome::through(Scratch::new().folder("home/nick"), "nick", &owners(&["nick", "mele-labs"]), api);
    (server, home)
}

fn reply(status: u16, body: &str) -> Reply {
    Reply { status, headers: Vec::new(), body: body.to_string() }
}

#[test]
fn a_home_made_as_the_login_is_a_private_user_repository_marked_so_another_mac_can_find_it() {
    let (server, home) = forge(|seen| match (seen.method.as_str(), seen.target.as_str()) {
        ("GET", "/repos/nick/armada-home") => reply(404, "{}"),
        ("POST", "/user/repos") => reply(201, "{}"),
        ("PUT", "/repos/nick/armada-home/topics") => ok("{}"),
        other => panic!("{other:?}"),
    });
    home.ensured(&at("nick", "armada-home")).expect("made");
    let seen = server.seen();
    let made = seen.iter().find(|it| it.method == "POST").expect("created");
    for part in ["\"name\":\"armada-home\"", "\"private\":true", "\"auto_init\":true"] {
        assert!(made.body.contains(part), "{part} in {}", made.body);
    }
    let marked = seen.last().expect("marked");
    assert_eq!((marked.method.as_str(), marked.body.as_str()), ("PUT", r#"{"names":["armada-home"]}"#));
}

#[test]
fn a_home_in_an_organisation_is_made_through_that_organisation_and_refused_by_name_where_the_login_may_not() {
    let (server, home) = forge(|seen| match (seen.method.as_str(), seen.target.as_str()) {
        ("GET", "/repos/mele-labs/kit") => reply(404, "{}"),
        ("POST", "/orgs/mele-labs/repos") => reply(201, "{}"),
        ("PUT", "/repos/mele-labs/kit/topics") => ok("{}"),
        other => panic!("{other:?}"),
    });
    home.ensured(&at("mele-labs", "kit")).expect("made");
    assert!(server.seen().iter().any(|it| it.method == "POST" && it.target == "/orgs/mele-labs/repos"));

    let (_, home) = forge(|seen| match seen.method.as_str() {
        "GET" => reply(404, "{}"),
        _ => reply(403, r#"{"message":"You are not permitted to create a repository in this organization"}"#),
    });
    let refused = home.ensured(&at("mele-labs", "kit")).expect_err("refused");
    assert_eq!(
        refused.said(),
        "nick cannot make repositories in mele-labs. Ask an owner of mele-labs to allow it, or choose nick"
    );
    assert!(matches!(refused, HomeFailure::Declined(_)));
}

#[test]
fn a_home_that_is_there_is_taken_where_the_login_may_push_to_it_and_refused_by_name_where_it_may_not() {
    let (server, home) = forge(|_| ok(r#"{"topics":["armada-home"],"permissions":{"admin":false,"push":true,"pull":true}}"#));
    home.ensured(&at("mele-labs", "kit")).expect("taken");
    assert_eq!(server.seen().len(), 1, "nothing is made, nothing marked: {:?}", server.seen());

    let (_, home) = forge(|_| ok(r#"{"permissions":{"admin":false,"push":false,"pull":true}}"#));
    let refused = home.ensured(&at("mele-labs", "kit")).expect_err("refused");
    assert_eq!(refused.said(), "mele-labs/kit exists and nick cannot write to it. Choose another name");
}

#[test]
fn the_home_found_is_a_marked_repository_owned_by_the_login_or_its_organisation_the_logins_own_first() {
    let listing = r#"[
        {"name": "notes", "owner": {"login": "nick"}, "topics": []},
        {"name": "kit", "owner": {"login": "mele-labs"}, "topics": ["armada-home"]},
        {"name": "armada-home", "owner": {"login": "Nick"}, "topics": ["armada-home", "kit"]},
        {"name": "theirs", "owner": {"login": "stranger"}, "topics": ["armada-home"]}
    ]"#;
    let (server, home) = forge(move |_| ok(listing));
    assert_eq!(home.listed().expect("listed"), Some(at("Nick", "armada-home")));
    assert_eq!(server.seen()[0].target, "/user/repos?affiliation=owner,organization_member&per_page=100");

    let (_, home) = forge(|_| ok(r#"[{"name": "armada-home", "owner": {"login": "stranger"}, "topics": ["armada-home"]}, {"name": "x", "owner": {"login": "nick"}}, {"name": "y", "owner": {"login": "nick"}, "topics": ["rust"]}]"#));
    assert_eq!(home.listed().expect("listed"), None, "another owner's, and ones without the topic, are not its home");
}

#[test]
fn a_home_that_is_there_without_the_topic_is_marked_keeping_its_other_topics_so_a_retry_heals_it() {
    let (server, home) = forge(|seen| match seen.method.as_str() {
        "GET" => ok(r#"{"topics":["rust"],"permissions":{"admin":false,"push":true,"pull":true}}"#),
        _ => ok("{}"),
    });
    home.ensured(&at("mele-labs", "kit")).expect("taken");
    let seen = server.seen();
    let marked = seen.last().expect("marked");
    assert_eq!((marked.method.as_str(), marked.target.as_str()), ("PUT", "/repos/mele-labs/kit/topics"));
    assert_eq!(marked.body, r#"{"names":["rust","armada-home"]}"#);
    assert!(seen.iter().all(|it| it.method != "POST"), "nothing is made: {seen:?}");
}
