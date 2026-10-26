//! `read_loadout_item` through a real Fleet: a row the session's Loadout lists is read off disk, one
//! it does not list reads nothing, and Armada's own two texts are answered from what Fleet holds.
//! `docs/practices/protocol.md`, *A Session's Loadout*.

use api::Sessions;
use ipc::{
    LoadoutItem, LoadoutItemAsked, LoadoutItemBody, LoadoutItemFormat, LoadoutItems, LoadoutKind, LoadoutSource,
    SessionFact, SessionId, SessionReport,
};
use testkit::FakeWorkProduct;

use crate::loadout_reading::ID_NOTE;
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

fn report(fact: SessionFact) -> SessionReport {
    SessionReport { harness: "a_harness".into(), session_id: SessionId::carried("s1"), fact }
}

fn item(kind: LoadoutKind, name: &str, source: LoadoutSource) -> LoadoutItem {
    LoadoutItem { kind, name: name.into(), source, from: None, state: None, tokens: None }
}

fn asked(kind: LoadoutKind, name: &str, source: LoadoutSource) -> LoadoutItemAsked {
    LoadoutItemAsked { session_id: "s1".into(), kind, name: name.into(), source, from: None }
}

fn why(body: LoadoutItemBody) -> String {
    match body {
        LoadoutItemBody::NotRead { why } => why,
        read => panic!("read, and should not have been: {read:?}"),
    }
}

#[tokio::test]
async fn a_listed_row_is_read_off_disk_and_one_the_loadout_does_not_list_is_not() {
    let home = TempDir::new();
    let repo = home.path().join("repo");
    // A memory file, because which folder a skill is in is the adapter's to know and its tests'.
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("LAND.md"), "# Land").unwrap();
    std::fs::write(repo.join("WALK.md"), "# Walk").unwrap();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    assert!(fleet.read_loadout_item(asked(LoadoutKind::MemoryFile, "LAND.md", LoadoutSource::Repository)).await.is_err(), "no such session");

    let cwd = repo.to_string_lossy().to_string();
    fleet
        .report_session(report(SessionFact::Started { cwd, title: None, origin: ipc::SessionOrigin::Terminal, mod_version: None }))
        .await
        .unwrap();
    let items = vec![
        item(LoadoutKind::MemoryFile, "LAND.md", LoadoutSource::Repository),
        item(LoadoutKind::McpServer, "a_server", LoadoutSource::Mcp),
        item(LoadoutKind::Rules, "Armada session rules", LoadoutSource::Armada),
        item(LoadoutKind::Injection, "armadaSession", LoadoutSource::Armada),
        item(LoadoutKind::OutputStyle, "Explanatory", LoadoutSource::BuiltIn),
    ];
    fleet.report_session(report(SessionFact::Equipped { loadout: LoadoutItems { items } })).await.unwrap();

    let land = fleet.read_loadout_item(asked(LoadoutKind::MemoryFile, "LAND.md", LoadoutSource::Repository)).await.unwrap();
    assert_eq!(
        land,
        LoadoutItemBody::Read {
            path: Some("LAND.md".into()),
            format: LoadoutItemFormat::Markdown,
            body: "# Land".into(),
            truncated: false
        }
    );
    let walk = fleet.read_loadout_item(asked(LoadoutKind::MemoryFile, "WALK.md", LoadoutSource::Repository)).await.unwrap();
    assert!(why(walk).contains("does not list it"), "on disk, and not in this session's Loadout");

    let rules = fleet.read_loadout_item(asked(LoadoutKind::Rules, "Armada session rules", LoadoutSource::Armada)).await.unwrap();
    assert_eq!(
        rules,
        LoadoutItemBody::Read { path: None, format: LoadoutItemFormat::Text, body: adapters::SESSION_RULES.trim().into(), truncated: false }
    );
    let note = fleet.read_loadout_item(asked(LoadoutKind::Injection, "armadaSession", LoadoutSource::Armada)).await.unwrap();
    let LoadoutItemBody::Read { body, .. } = note else { panic!("the note") };
    assert!(body.starts_with("Your Armada job id is <job id>."));

    why(fleet.read_loadout_item(asked(LoadoutKind::McpServer, "a_server", LoadoutSource::Mcp)).await.unwrap());
    why(fleet.read_loadout_item(asked(LoadoutKind::OutputStyle, "Explanatory", LoadoutSource::BuiltIn)).await.unwrap());
}

/// **Kept by hand**, so this is what notices the two drifting: the mod's `idNote`, with the id as
/// `<job id>` and its four constants written in, is the note Fleet answers.
#[test]
fn the_note_fleet_answers_is_the_one_the_mod_injects() {
    let mod_source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugins/armada/hooks/register.ts"))
        .expect("the mod's hooks");
    let constant = |name: &str| {
        let opening = format!("const {name} = \"");
        let start = mod_source.find(&opening).expect("the constant") + opening.len();
        let end = start + mod_source[start..].find("\";").expect("its end");
        mod_source[start..end].to_string()
    };
    let opening = "function idNote(id: string): string {\n  return `";
    let start = mod_source.find(opening).expect("idNote") + opening.len();
    let end = start + mod_source[start..].find("`\n}").expect("its end");
    let note = mod_source[start..end]
        .replace("${id}", "<job id>")
        .replace("${ASK_IN_WINDOW}", &constant("ASK_IN_WINDOW"))
        .replace("${PROPOSE_DRAFT}", &constant("PROPOSE_DRAFT"))
        .replace("${SHOW_SKETCH}", &constant("SHOW_SKETCH"))
        .replace("${SHOW_FILE}", &constant("SHOW_FILE"))
        .replace("\\`", "`");
    assert_eq!(ID_NOTE, note, "plugins/armada/hooks/register.ts `idNote` and `ID_NOTE` differ");
}
