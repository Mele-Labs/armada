//! Kit against the shared home, kind by kind: each kind goes into the home from one machine and
//! comes out on another the same, a row that differs is settled only by a side picked, and
//! nothing carrying a credential leaves. Two machines on one `FakeRemote`, folders in memory; and
//! one machine's two profiles, each on a home of its own.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{HomeFailure, HomeRepository, PluginRecord};
use core_model::{Actor, KitServer, ServerAddress, ServerName, Timestamp};
use ipc::{HomeKind, HomeLeft, HomeNamed, HomeRow, HomeRowAt, HomeRowState, HomeSide, ModKind};
use testkit::{FakeHarnessHome, FakeHome, FakeHomes, FakeRemote, HeldFolder};

use crate::home::{self, secrets, Places, Refused, Repository};

const LOGIN: &str = "nick";
const NOTE: &str = "---\nname: walks\ndescription: Show a walk, never a picture\nmetadata:\n  originSessionId: aaaa\n  modified: 2026-10-01T00:00:00Z\n---\n\nA walk link on its own line.\n";

struct Mac {
    home: Arc<dyn HomeRepository>,
    mods: HeldFolder,
    kit: HeldFolder,
    person: HeldFolder,
    harness: FakeHarnessHome,
    servers: Vec<KitServer>,
    repositories: Vec<Repository>,
}

impl Mac {
    fn on(remote: &FakeRemote, root: &str) -> Mac {
        Mac::homed(Arc::new(FakeHome::on(remote)), root)
    }

    /// A machine acting on `home`, one of its profiles'.
    fn homed(home: Arc<dyn HomeRepository>, root: &str) -> Mac {
        let person = HeldFolder::new();
        Mac {
            home,
            mods: HeldFolder::new(),
            kit: HeldFolder::new(),
            harness: FakeHarnessHome::over(person.clone()),
            person,
            servers: Vec::new(),
            repositories: vec![Repository { name: "shop".to_string(), root: root.to_string() }],
        }
    }

    fn opened(self) -> Mac {
        self.home.open().expect("the home opens");
        self
    }

    fn places(&self) -> Places<'_> {
        Places {
            home: self.home.as_ref(),
            mods: &self.mods,
            kit: &self.kit,
            harness: &self.harness,
            servers: &self.servers,
            repositories: &self.repositories,
        }
    }

    fn rows(&self) -> Vec<HomeRow> {
        home::read(&self.places(), HomeKind::ALL).rows
    }

    /// Sync, with the kinds in `on` switched on, every mod taken as valid.
    fn synced(&self, on: &[HomeKind]) -> Result<Vec<HomeLeft>, Refused> {
        home::sync(&self.places(), on, "mac-a", &|_, _| Ok(()))
    }

    /// The memory folder of the one repository it serves.
    fn memory(&self) -> String {
        use adapter_traits::HarnessHome;
        self.harness.memory_dir(&self.repositories[0].root)
    }
}

fn at(kind: HomeKind, name: &str) -> HomeRowAt {
    let repository = (kind == HomeKind::Memory).then(|| "shop".to_string());
    HomeRowAt { login: LOGIN.to_string(), kind, name: name.to_string(), repository }
}

fn state(rows: &[HomeRow], kind: HomeKind, name: &str) -> HomeRowState {
    rows.iter()
        .find(|row| row.kind == kind && row.name == name)
        .map(|row| row.state)
        .unwrap_or_else(|| panic!("no {} row named {name} in {rows:?}", kind.as_wire()))
}

fn server(name: &str, url: &str) -> KitServer {
    KitServer::added(
        ServerName::named(name).expect("a plain name"),
        ServerAddress::address(url).expect("an address"),
        Timestamp::from_rfc3339("2026-10-23T09:00:00.000Z"),
        Actor::Human,
    )
}

fn plugin() -> PluginRecord {
    PluginRecord { name: "tidy@market".to_string(), marketplace: "market".to_string(), source: Some("acme/market".to_string()) }
}

/// One of every kind that travels as files or a line, on Mac A.
fn every_kind(remote: &FakeRemote) -> Mac {
    let mut a = Mac::on(remote, "/Users/user/code/shop");
    a.mods = a.mods.with("dusk/mod.toml", "name = \"dusk\"\nkind = \"theme\"\n").with("dusk/theme.css", ":root {}\n");
    let memory = a.memory();
    a.person = a
        .person
        .with("harness/skills/humanizer/SKILL.md", "---\ndescription: Rewrite text\n---\nBody\n")
        .with("harness/skills/humanizer/scripts/check.sh", "echo ok\n")
        .with("harness/agents/reviewer.md", "A reviewer\n")
        .with("harness/commands/ship.md", "Ship it\n")
        .with("harness/INSTRUCTIONS.md", "# How I work\n")
        .with(&format!("{memory}/walks.md"), NOTE)
        .with(&format!("{memory}/INDEX.md"), "- [Other](other.md)\n");
    a.kit = a.kit.with("workflows/bug.yml", "name: bug\n").with("allowed-commands", "cargo check\talways allow\n");
    a.harness = FakeHarnessHome::over(a.person.clone()).with_plugin(plugin());
    a
}

const EVERY: &[(HomeKind, &str)] = &[
    (HomeKind::Mods, "dusk"),
    (HomeKind::Skills, "humanizer"),
    (HomeKind::SubAgents, "reviewer"),
    (HomeKind::Commands, "ship"),
    (HomeKind::GlobalInstructions, "INSTRUCTIONS.md"),
    (HomeKind::Workflows, "bug"),
    (HomeKind::Allowlist, "cargo check"),
    (HomeKind::Memory, "walks"),
    (HomeKind::Plugins, "tidy@market"),
];

#[test]
fn each_kind_added_on_one_machine_is_only_in_the_home_on_another_and_the_same_once_brought() {
    let remote = FakeRemote::of(LOGIN);
    let a = every_kind(&remote).opened();
    for (kind, name) in EVERY {
        home::add(&a.places(), &at(*kind, name), "mac-a").unwrap_or_else(|why| panic!("{name} is added: {}", why.said()));
    }
    let rows = a.rows();
    for (kind, name) in EVERY {
        assert_eq!(state(&rows, *kind, name), HomeRowState::Same, "{name} on A once added");
    }
    assert!(remote.paths().contains(&"skills/humanizer/scripts/check.sh".to_string()), "{:?}", remote.paths());
    assert!(remote.paths().contains(&"memory/shop/walks.md".to_string()), "{:?}", remote.paths());
    assert!(!remote.paths().iter().any(|path| path.ends_with("INDEX.md")), "the index stays: {:?}", remote.paths());

    let b = Mac::on(&remote, "/Users/user/work/shop").opened();
    let rows = b.rows();
    for (kind, name) in EVERY {
        assert_eq!(state(&rows, *kind, name), HomeRowState::Home, "{name} on B before Bring");
        home::bring(&b.places(), &at(*kind, name)).unwrap_or_else(|why| panic!("{name} is brought: {}", why.said()));
    }
    let rows = b.rows();
    for (kind, name) in EVERY {
        assert_eq!(state(&rows, *kind, name), HomeRowState::Same, "{name} on B once brought");
    }
    assert_eq!(b.person.text("harness/skills/humanizer/scripts/check.sh").as_deref(), Some("echo ok\n"));
    assert_eq!(b.kit.text("allowed-commands").as_deref(), Some("cargo check\talways allow\n"));
    assert_eq!(b.kit.text("workflows/bug.yml").as_deref(), Some("name: bug\n"));
    let index = b.person.text(&format!("{}/INDEX.md", b.memory())).expect("the note is listed");
    assert!(index.contains("(walks.md)") && index.contains("Show a walk"), "{index}");
    assert_eq!(b.harness.installed(), vec![plugin()]);
}

#[test]
fn a_row_that_differs_is_refused_by_add_and_bring_and_settled_only_by_a_chosen_side() {
    let remote = FakeRemote::of(LOGIN);
    let mut a = Mac::on(&remote, "/a/shop");
    a.person = a.person.with("harness/skills/tone/SKILL.md", "warm\n").with("harness/skills/tone/extra.md", "A's\n");
    let a = a.opened();
    home::add(&a.places(), &at(HomeKind::Skills, "tone"), "mac-a").expect("added");
    let mut b = Mac::on(&remote, "/b/shop");
    b.person = b.person.with("harness/skills/tone/SKILL.md", "cool\n");
    let b = b.opened();

    let rows = b.rows();
    let row = rows.iter().find(|row| row.name == "tone").expect("a row");
    assert_eq!(row.state, HomeRowState::Differs);
    assert_eq!(row.here.as_deref(), Some("cool\n"));
    assert!(row.there.as_deref().is_some_and(|there| there.contains("warm") && there.contains("A's")), "{row:?}");
    assert!(matches!(home::add(&b.places(), &at(HomeKind::Skills, "tone"), "mac-b"), Err(Refused::Differs(_))));
    assert!(matches!(home::bring(&b.places(), &at(HomeKind::Skills, "tone")), Err(Refused::Differs(_))));
    assert_eq!(b.person.text("harness/skills/tone/SKILL.md").as_deref(), Some("cool\n"), "Bring wrote nothing");

    home::choose(&b.places(), &at(HomeKind::Skills, "tone"), HomeSide::Here, "mac-b").expect("B's side goes in");
    assert_eq!(remote.text("skills/tone/SKILL.md").as_deref(), Some("cool\n"));
    assert_eq!(remote.text("skills/tone/extra.md"), None, "what B's side has not is taken out of the home");
    assert_eq!(state(&b.rows(), HomeKind::Skills, "tone"), HomeRowState::Same);

    a.home.pull().expect("A pulls");
    home::choose(&a.places(), &at(HomeKind::Skills, "tone"), HomeSide::There, "mac-a").expect("the home's side comes down");
    assert_eq!(a.person.text("harness/skills/tone/SKILL.md").as_deref(), Some("cool\n"));
    assert_eq!(a.person.text("harness/skills/tone/extra.md"), None);
    assert_eq!(state(&a.rows(), HomeKind::Skills, "tone"), HomeRowState::Same);
}

#[test]
fn a_memory_note_reads_the_same_whatever_stamps_and_line_endings_each_machine_wrote() {
    let remote = FakeRemote::of(LOGIN);
    let mut a = Mac::on(&remote, "/a/shop");
    let note = format!("{}/walks.md", a.memory());
    a.person = a.person.with(&note, NOTE);
    let a = a.opened();
    home::add(&a.places(), &at(HomeKind::Memory, "walks"), "mac-a").expect("added");
    let mut b = Mac::on(&remote, "/b/shop");
    let restamped = NOTE.replace("aaaa", "bbbb").replace("2026-10-01", "2026-10-20").replace('\n', "\r\n");
    let note = format!("{}/walks.md", b.memory());
    b.person = b.person.with(&note, &format!("{restamped}\r\n"));
    let b = b.opened();
    assert_eq!(state(&b.rows(), HomeKind::Memory, "walks"), HomeRowState::Same);

    let mut c = Mac::on(&remote, "/c/shop");
    let note = format!("{}/walks.md", c.memory());
    c.person = c.person.with(&note, &NOTE.replace("its own line", "a line"));
    assert_eq!(state(&c.opened().rows(), HomeKind::Memory, "walks"), HomeRowState::Differs, "the body counts");
}

#[test]
fn sync_adds_every_row_only_here_in_one_push_and_leaves_a_credential_and_a_row_that_differs() {
    let remote = FakeRemote::of(LOGIN);
    let mut b = Mac::on(&remote, "/b/shop");
    b.person = b.person.with("harness/skills/tone/SKILL.md", "warm\n");
    let b = b.opened();
    home::add(&b.places(), &at(HomeKind::Skills, "tone"), "mac-b").expect("B's tone is in the home");

    let mut a = Mac::on(&remote, "/a/shop");
    a.person = a
        .person
        .with("harness/skills/humanizer/SKILL.md", "Rewrite text\n")
        .with("harness/skills/tone/SKILL.md", "cool\n")
        .with("harness/skills/deploy/SKILL.md", "Push to https://bot:hunter2@git.acme.dev/shop.git\n")
        .with("harness/commands/ship.md", "Ship it\n")
        .with("harness/agents/reviewer.md", "A reviewer, its kind off\n");
    a.servers.push(server("linear", "https://bot:hunter2@mcp.linear.app/sse"));
    let before = remote.published().len();

    let left = a.synced(&[HomeKind::Skills, HomeKind::Commands, HomeKind::McpServers]).expect("Sync answers");

    let published = remote.published();
    assert_eq!(published.len(), before + 1, "one push: {published:?}");
    let mut pushed = published.last().cloned().unwrap_or_default();
    pushed.sort();
    assert_eq!(pushed, ["commands/ship.md", "mcp_servers/linear.toml", "skills/humanizer/SKILL.md"].map(str::to_string));
    assert_eq!(remote.text("mcp_servers/linear.toml").as_deref(), Some("name = \"linear\"\nhost = \"mcp.linear.app\"\n"));
    assert_eq!(remote.text("skills/tone/SKILL.md").as_deref(), Some("warm\n"), "a row that differs is not touched");
    assert_eq!(a.person.text("harness/skills/tone/SKILL.md").as_deref(), Some("cool\n"));
    assert_eq!(remote.text("skills/deploy/SKILL.md"), None);
    assert_eq!(remote.text("sub_agents/reviewer.md"), None, "a kind switched off stays");

    match left.as_slice() {
        [HomeLeft { kind: HomeKind::Skills, name, repository: None, why }] => {
            assert_eq!(name, "deploy");
            assert!(why.contains("deploy") && why.contains("git.acme.dev") && !why.contains("hunter2"), "{why}");
        }
        other => panic!("only deploy stays: {other:?}"),
    }
    let rows = a.rows();
    assert_eq!(state(&rows, HomeKind::Skills, "humanizer"), HomeRowState::Same);
    assert_eq!(state(&rows, HomeKind::Skills, "deploy"), HomeRowState::Here);
    assert_eq!(state(&rows, HomeKind::Skills, "tone"), HomeRowState::Differs);

    a.synced(&[HomeKind::Skills, HomeKind::Commands, HomeKind::McpServers]).expect("Sync again");
    assert_eq!(remote.published().len(), before + 1, "nothing new, nothing pushed");
}

#[test]
fn a_server_is_the_same_by_its_host_and_a_machine_holding_it_has_its_key() {
    let remote = FakeRemote::of(LOGIN);
    let mut a = Mac::on(&remote, "/a/shop");
    a.servers.push(server("linear", "https://mcp.linear.app/sse?api_key=k"));
    let a = a.opened();
    home::add(&a.places(), &at(HomeKind::McpServers, "linear"), "mac-a").expect("added");
    assert_eq!(remote.text("mcp_servers/linear.toml").as_deref(), Some("name = \"linear\"\nhost = \"mcp.linear.app\"\n"));

    let mut b = Mac::on(&remote, "/b/shop");
    b.servers.push(server("linear", "https://mcp.linear.app/other"));
    let rows = b.opened().rows();
    let row = rows.iter().find(|row| row.name == "linear").expect("a row");
    assert_eq!((row.state, row.key_here), (HomeRowState::Same, Some(true)));
}

#[test]
fn a_piece_carrying_a_credential_in_an_address_stays_and_is_named_without_it() {
    let remote = FakeRemote::of(LOGIN);
    let mut a = Mac::on(&remote, "/a/shop");
    a.person = a
        .person
        .with("harness/skills/deploy/SKILL.md", "Push to https://bot:hunter2@git.acme.dev/shop.git\n")
        .with("harness/skills/fetch/SKILL.md", "curl 'https://api.acme.dev/v1?limit=5&access_token=abc123'\n")
        .with("harness/skills/plain/SKILL.md", "Read https://docs.acme.dev/guide?page=2#top\n");
    a.kit = a.kit.with("allowed-commands", "curl https://u:p@x.dev/\n");
    let a = a.opened();
    for name in ["deploy", "fetch"] {
        match home::add(&a.places(), &at(HomeKind::Skills, name), "mac-a") {
            Err(Refused::Secret(said)) => assert!(!said.contains("hunter2") && !said.contains("abc123") && said.contains("acme.dev"), "{said}"),
            other => panic!("{name} is refused: {other:?}"),
        }
    }
    assert!(matches!(home::add(&a.places(), &at(HomeKind::Allowlist, "curl https://u:p@x.dev/"), "mac-a"), Err(Refused::Secret(_))));
    home::add(&a.places(), &at(HomeKind::Skills, "plain"), "mac-a").expect("an address with no credential goes");
    assert_eq!(remote.paths(), vec!["skills/plain/SKILL.md".to_string()]);
    assert_eq!(secrets::leak("see https://acme.dev/?q=token"), None, "a value named token is not one");
}

#[test]
fn every_kind_is_off_until_turned_on_and_a_kind_off_has_no_rows() {
    let kinds = home::kinds(LOGIN, &BTreeMap::new());
    assert_eq!(kinds.kinds.len(), HomeKind::ALL.len());
    assert!(kinds.kinds.iter().all(|kind| !kind.on));

    let remote = FakeRemote::of(LOGIN);
    let a = every_kind(&remote).opened();
    let on = home::on(&BTreeMap::from([("skills".to_string(), true), ("mods".to_string(), false)]));
    assert_eq!(on, vec![HomeKind::Skills]);
    let rows = home::read(&a.places(), &on).rows;
    assert!(!rows.is_empty() && rows.iter().all(|row| row.kind == HomeKind::Skills), "{rows:?}");

    let unopened = every_kind(&remote);
    let read = home::read(&unopened.places(), HomeKind::ALL);
    assert_eq!((read.home, read.rows.len()), (None, 0), "no copy here is no rows");
}

#[test]
fn each_profile_has_a_home_of_its_own_and_a_login_the_forge_here_does_not_hold_is_refused_by_name() {
    let (personal, work) = (FakeRemote::of("nick"), FakeRemote::of("nick-work"));
    let homes = FakeHomes::new().holding(&personal).holding(&work);
    let profiles = ["nick", "nick-work", "nick-contract"].map(str::to_string);
    let of = |login: &str| home::home_of(&homes, login, &profiles);

    let mut mine = Mac::homed(of("nick").expect("a profile"), "/Users/user/code/shop");
    mine.person = mine.person.with("harness/skills/tone/SKILL.md", "warm\n");
    mine.synced(&[]).expect("the personal home opens");
    home::add(&mine.places(), &at(HomeKind::Skills, "tone"), "mac-a").expect("added");
    let theirs = Mac::homed(of("nick-work").expect("a profile"), "/Users/user/code/shop");
    theirs.synced(&[]).expect("the work home opens");

    assert_eq!(personal.paths(), vec!["skills/tone/SKILL.md".to_string()]);
    assert!(work.paths().is_empty(), "the work home is not the personal one's: {:?}", work.paths());
    let named = |mac: &Mac| home::read(&mac.places(), HomeKind::ALL).home;
    assert_eq!(named(&mine), Some(HomeNamed { account: "nick".to_string(), repository: "armada-home".to_string() }));
    assert_eq!(named(&theirs).map(|home| home.account).as_deref(), Some("nick-work"));
    assert!(!theirs.rows().iter().any(|row| row.name == "tone"), "{:?}", theirs.rows());

    let contract = of("nick-contract").expect("a profile, though the forge here does not hold it");
    match Mac::homed(Arc::clone(&contract), "/Users/user/code/shop").synced(&[]) {
        Err(Refused::Home(HomeFailure::NotSignedIn(said))) => assert!(said.contains("nick-contract"), "{said}"),
        other => panic!("refused by name: {other:?}"),
    }
    assert_eq!(contract.here(), None, "no copy, so its set-up row still waits");
    assert!(matches!(of("stranger"), Err(Refused::NoProfile(_))), "not a profile here");
}

#[test]
fn a_mod_only_the_home_has_carries_the_kind_its_mod_toml_names() {
    let remote = FakeRemote::of(LOGIN);
    let mut a = Mac::on(&remote, "/Users/user/code/shop");
    a.mods = a
        .mods
        .with("dusk/mod.toml", "name = \"dusk\"\nkind = \"theme\"\n")
        .with("dusk/theme.css", ":root {}\n")
        .with("deck/mod.toml", "name = \"deck\"\nkind = \"layout\"\n")
        .with("deck/layout.json", "{}\n");
    let a = a.opened();
    for name in ["dusk", "deck"] {
        home::add(&a.places(), &at(HomeKind::Mods, name), "mac-a").unwrap_or_else(|why| panic!("{name} is added: {}", why.said()));
    }

    let b = Mac::on(&remote, "/Users/user/work/shop").opened();
    let rows = b.rows();
    let kind_of = |name: &str| rows.iter().find(|row| row.kind == HomeKind::Mods && row.name == name).and_then(|row| row.mod_kind);
    assert_eq!(state(&rows, HomeKind::Mods, "deck"), HomeRowState::Home);
    assert_eq!(kind_of("deck"), Some(ModKind::Layout), "{rows:?}");
    assert_eq!(kind_of("dusk"), Some(ModKind::Theme), "{rows:?}");
    assert!(rows.iter().filter(|row| row.kind != HomeKind::Mods).all(|row| row.mod_kind.is_none()), "{rows:?}");
}
