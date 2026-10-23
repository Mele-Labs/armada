//! The shared home's claim: **a mod and a skill added to the home on one Mac read as only in the
//! home on another; Bring puts them in that Mac's mods folder and its harness's skills folder,
//! and they then read the same. An MCP server carried across holds its name and its host and no
//! credential, and reads on the other Mac as needing its key.**
//!
//! Two machines on one remote, every folder held in memory. Nothing reaches a forge, a disk or a
//! process: the home is `testkit::FakeRemote`, and each Mac's copy is a `FakeHome` that sees what
//! the other pushed only once it opens the home.
//!
//! | Proved | Not proved |
//! |---|---|
//! | A mod and a skill added on Mac A are rows only in the home on Mac B | That the home is made private and cloned over HTTPS through the forge CLI's credential. The forge's home adapter in `adapters::shared_home`, against the real forge only |
//! | Bring writes the mod's two files into B's mods folder and the skill into the harness's skills folder, and both then read the same | That a brought mod is listed by Bridge. `fleet::mods` rescans the folder |
//! | A server goes into the home as its name and host: neither the userinfo, the path nor the query of its address. On B it reads as needing its key, and Bring refuses it | That the switches in What syncs are kept. `store::tests::home_kinds` |
//! | | That a row that differs is refused by Add and Bring and settled by a chosen side. `fleet::tests::shared_home` |

use adapter_traits::HomeRepository;
use core_model::{Actor, KitServer, ServerAddress, ServerName, Timestamp};
use fleet::home::{self, Places, Refused};
use ipc::{HomeKind, HomeRow, HomeRowAt, HomeRowState};
use testkit::{FakeHarnessHome, FakeHome, FakeRemote, HeldFolder};

const LOGIN: &str = "nick";
const DUSK_MANIFEST: &str = "name = \"dusk\"\nkind = \"theme\"\nversion = \"1\"\n";
const DUSK_CSS: &str = ":root { --accent: #E8A25C; }\n";
const HUMANIZER: &str = "---\nname: humanizer\ndescription: Rewrite text so it reads like the writer\n---\n\nRemove the tells.\n";
/// What only Mac A's address carries: the account's own key, as a server's URL often holds one.
const KEY: &str = "k3y-0f-the-0wner";

/// One machine: its copy of the home, its mods folder, Kit's home, its harness and Kit's servers.
struct Mac {
    home: FakeHome,
    mods: HeldFolder,
    kit: HeldFolder,
    person: HeldFolder,
    harness: FakeHarnessHome,
    servers: Vec<KitServer>,
}

impl Mac {
    fn on(remote: &FakeRemote) -> Mac {
        let person = HeldFolder::new();
        Mac {
            home: FakeHome::on(remote),
            mods: HeldFolder::new(),
            kit: HeldFolder::new(),
            harness: FakeHarnessHome::over(person.clone()),
            person,
            servers: Vec::new(),
        }
    }

    fn places(&self) -> Places<'_> {
        Places {
            home: &self.home,
            mods: &self.mods,
            kit: &self.kit,
            harness: &self.harness,
            servers: &self.servers,
            repositories: &[],
        }
    }

    fn rows(&self) -> Vec<HomeRow> {
        home::read(&self.places(), HomeKind::ALL).rows
    }
}

fn at(kind: HomeKind, name: &str) -> HomeRowAt {
    HomeRowAt { login: LOGIN.to_string(), kind, name: name.to_string(), repository: None }
}

fn row(rows: &[HomeRow], kind: HomeKind, name: &str) -> HomeRow {
    rows.iter()
        .find(|row| row.kind == kind && row.name == name)
        .cloned()
        .unwrap_or_else(|| panic!("no {} row named {name} in {rows:?}", kind.as_wire()))
}

fn linear() -> KitServer {
    let address = ServerAddress::address(&format!("https://nick:{KEY}@mcp.linear.app/sse?api_key={KEY}")).expect("an address");
    KitServer::added(
        ServerName::named("linear").expect("a plain name"),
        address,
        Timestamp::from_rfc3339("2026-10-23T09:00:00.000Z"),
        Actor::Human,
    )
}

#[test]
fn a_mod_and_a_skill_added_on_one_mac_are_brought_onto_another_and_a_server_carries_no_key() {
    let remote = FakeRemote::of(LOGIN);
    let mut a = Mac::on(&remote);
    a.mods = a.mods.with("dusk/mod.toml", DUSK_MANIFEST).with("dusk/theme.css", DUSK_CSS);
    a.person = a.person.with("harness/skills/humanizer/SKILL.md", HUMANIZER);
    a.servers.push(linear());
    a.home.open().expect("the home is made and cloned on Mac A");
    for added in [at(HomeKind::Mods, "dusk"), at(HomeKind::Skills, "humanizer"), at(HomeKind::McpServers, "linear")] {
        home::add(&a.places(), &added, "mac-a").unwrap_or_else(|why| panic!("{} goes into the home: {}", added.name, why.said()));
    }

    let b = Mac::on(&remote);
    b.home.open().expect("Mac B clones the home");
    let rows = b.rows();
    assert_eq!(row(&rows, HomeKind::Mods, "dusk").state, HomeRowState::Home);
    assert_eq!(row(&rows, HomeKind::Skills, "humanizer").state, HomeRowState::Home);
    let server = row(&rows, HomeKind::McpServers, "linear");
    assert_eq!((server.state, server.key_here), (HomeRowState::Home, Some(false)), "B has no key for it");

    // The home holds the server's name and host, and nothing else of the address anywhere.
    let record = remote.text("mcp_servers/linear.toml").expect("the server is in the home");
    assert!(record.contains("linear") && record.contains("mcp.linear.app"), "{record}");
    for path in remote.paths() {
        let text = remote.text(&path).unwrap_or_default();
        assert!(!text.contains(KEY) && !text.contains("/sse") && !text.contains("nick:"), "{path} carries the address: {text}");
    }

    home::bring(&b.places(), &at(HomeKind::Mods, "dusk")).expect("the mod is brought");
    home::bring(&b.places(), &at(HomeKind::Skills, "humanizer")).expect("the skill is brought");
    assert_eq!(b.mods.text("dusk/mod.toml").as_deref(), Some(DUSK_MANIFEST));
    assert_eq!(b.mods.text("dusk/theme.css").as_deref(), Some(DUSK_CSS));
    assert_eq!(b.person.text("harness/skills/humanizer/SKILL.md").as_deref(), Some(HUMANIZER));

    let rows = b.rows();
    assert_eq!(row(&rows, HomeKind::Mods, "dusk").state, HomeRowState::Same);
    assert_eq!(row(&rows, HomeKind::Skills, "humanizer").state, HomeRowState::Same);
    match home::bring(&b.places(), &at(HomeKind::McpServers, "linear")) {
        Err(Refused::Cannot(_)) => {}
        other => panic!("a server without its key is not brought: {other:?}"),
    }
    assert_eq!(row(&b.rows(), HomeKind::McpServers, "linear").key_here, Some(false));
}
