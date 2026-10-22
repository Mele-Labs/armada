//! Share: pieces made into a package, read, added to the person's Kit, and held unbound where
//! their remote is none's. `crate::sharing`.
//!
//! **Two Macs are two Fleets over two homes.** A package made by one is added by the other, so
//! what travels is the text and nothing else.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::Bucket;
use api::Refusal;
use ipc::{
    AddPackage, BindUnboundPiece, FlightControl, FlightControlKind, FlightControls, FlightControlsRead,
    Landed, PackageText, PieceToCommit, PieceKind, PieceToShare, SaveRepositoryFlightControls, SharePieces,
    TileList, WorkflowId,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::red_claim::Reaching;
use crate::repositories::{SavedWorkflow, WorkflowNotSaved};
use crate::tests::repositories::Planted;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

mod manifest;
mod requests;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const REMOTE: &str = "acme/widgets";

const WORKFLOW: &str = "version: 1\nworkflow_id: quick\nname: Quick\nsteps:\n  - id: implement\n    label: \"Implement\"\n    \
                        evidence: {submitted: {type: diff}}\n    mechanical_checks:\n      - type: every_manifest_check\n    \
                        delivers: false\n    advance_gate: auto\n";
const REVIEW_TOOL: &str = "---\nkind: tool\nname: Security\n---\nLook for secrets in the diff.\n";

/// A machine whose repository pushes to `remote`.
struct Remote(&'static str);

impl Reaching for Remote {
    fn slug(&self, _root: &str) -> Option<String> {
        Some(self.0.to_string())
    }
    fn open(&self, _url: &str, _directory: &Path) -> Result<Option<Box<dyn adapter_traits::Store + Send>>, String> {
        Ok(None)
    }
    fn bucket(&self, _address: &adapters::BucketAddress) -> Result<Box<dyn Bucket + Send>, adapter_traits::BucketError> {
        Err(adapter_traits::BucketError::Unsigned(String::from("none on this machine")))
    }
}

struct Mac {
    fleet: Fixture,
    locator: Arc<Planted>,
    root: String,
    kit: PathBuf,
    _home: TempDir,
}

/// A Mac whose one repository pushes to `remote`.
fn mac(remote: &'static str) -> Mac {
    let home = TempDir::new();
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&[]));
    let locator = Arc::new(Planted::nothing());
    fitted.locating = Arc::clone(&locator) as Arc<dyn crate::repositories::Locating>;
    fitted.reaching = Arc::new(Remote(remote));
    let fleet = Fleet::assembled(fitted);
    let root = fleet.repositories().first().expect("a served repository").root().to_string();
    let kit = PathBuf::from(&fleet.host().kit_home);
    Mac { fleet, locator, root, kit, _home: home }
}

fn tile(id: &str, text: &str) -> FlightControl {
    FlightControl {
        id: id.into(),
        name: None,
        glyph: None,
        key: None,
        hidden: false,
        text: Some(text.into()),
        on_text: None,
        off_text: None,
        kind: Some(FlightControlKind::Shell),
    }
}

fn refused_with(refusal: Refusal) -> (u16, String, String) {
    match refusal {
        Refusal::Unacceptable(error) => (422, error.code, error.message),
        Refusal::NotFound(error) => (404, error.code, error.message),
        other => panic!("neither 422 nor 404: {other:?}"),
    }
}

impl Mac {
    async fn keeps_tiles(&self, job: Vec<FlightControl>, deck: Vec<FlightControl>) {
        let controls = FlightControls { version: 1, job, deck };
        self.fleet
            .repository_flight_controls_saved(SaveRepositoryFlightControls { repository: self.root.clone(), controls })
            .await
            .expect("saved");
    }

    async fn tiles(&self) -> FlightControls {
        match self.fleet.repository_flight_controls_read(self.root.clone(), None).await.expect("read").read {
            FlightControlsRead::Read { controls } => controls,
            FlightControlsRead::NotRead { why } => panic!("not read: {why}"),
        }
    }

    fn keeps_review_tool(&self, text: &str) {
        let at = self.kit.join("review-tools").join("security.md");
        std::fs::create_dir_all(at.parent().expect("a folder")).expect("made");
        std::fs::write(at, text).expect("written");
    }

    /// Kit's workflow file, as the repository's catalogue holds it.
    fn keeps_workflow(&self, text: &str) {
        let served = self.fleet.repositories().first().expect("served");
        let file = self.kit.join("workflows").join("quick.yml");
        let written = [config::Written::in_kit(file, text.to_string())];
        let (workflows, _, files) = config::Catalogue::of(written, &config::Roster::offering_nothing()).resolve(served.manifest()).into_parts();
        assert_eq!(files.len(), 1, "the workflow fits the fixture's Manifest");
        served.catalogued(crate::repositories::Catalogued { workflows, left_out: Vec::new(), files });
    }

    fn sharing(&self, piece: PieceToCommit) -> SharePieces {
        SharePieces { pieces: vec![PieceToShare { repository: self.root.clone(), piece }] }
    }

    async fn shared(&self, piece: PieceToCommit) -> String {
        self.fleet.package_made(self.sharing(piece)).await.expect("made").text
    }
}

fn tiles_of(job: &[&str]) -> PieceToCommit {
    PieceToCommit::FlightControls { job: job.iter().map(|id| id.to_string()).collect(), deck: Vec::new() }
}

fn adding(text: &str, replace: bool) -> AddPackage {
    AddPackage { text: text.to_string(), replace }
}

/// **A package carries each piece as stored and its remote, and nothing about the person.**
#[tokio::test]
async fn a_package_holds_each_piece_and_its_remote_and_no_path_user_or_time() {
    let mine = mac(REMOTE);
    mine.keeps_tiles(vec![tile("specs", "make test")], vec![tile("ship", "make ship")]).await;
    mine.keeps_workflow(WORKFLOW);
    mine.keeps_review_tool(REVIEW_TOOL);
    let asked = SharePieces {
        pieces: ["specs", "ship"]
            .into_iter()
            .map(|id| PieceToShare {
                repository: mine.root.clone(),
                piece: match id {
                    "specs" => tiles_of(&["specs"]),
                    _ => PieceToCommit::FlightControls { job: Vec::new(), deck: vec![id.to_string()] },
                },
            })
            .chain([
                PieceToShare { repository: mine.root.clone(), piece: PieceToCommit::Workflow { workflow_id: WorkflowId::carried("quick") } },
                PieceToShare { repository: mine.root.clone(), piece: PieceToCommit::ReviewTool { id: "security".into() } },
            ])
            .collect(),
    };

    let made = mine.fleet.package_made(asked).await.expect("made");

    assert_eq!(made.file_name, "widgets.armada.toml");
    let parsed: toml::Table = toml::from_str(&made.text).expect("it is TOML");
    assert_eq!(parsed.keys().map(String::as_str).collect::<Vec<_>>(), ["piece", "version"]);
    assert_eq!(parsed["version"].as_integer(), Some(1));
    let pieces = parsed["piece"].as_array().expect("one table a piece");
    assert_eq!(pieces.len(), 4);
    let kinds: Vec<&str> = pieces.iter().map(|one| one["kind"].as_str().expect("a kind")).collect();
    assert_eq!(kinds, ["tile", "tile", "workflow", "review_tool"]);
    for one in pieces {
        let table = one.as_table().expect("a table");
        assert_eq!(table["remote"].as_str(), Some(REMOTE));
        for key in table.keys() {
            assert!(["kind", "list", "remote", "tile", "text", "id"].contains(&key.as_str()), "a field no piece is meant to carry: {key}");
        }
    }
    assert_eq!(pieces[0]["tile"]["text"].as_str(), Some("make test"));
    assert_eq!(pieces[0]["list"].as_str(), Some("job"));
    assert_eq!(pieces[1]["list"].as_str(), Some("deck"));
    assert_eq!(pieces[2]["text"].as_str(), Some(WORKFLOW), "the workflow exactly as stored");
    assert_eq!(pieces[3]["text"].as_str(), Some(REVIEW_TOOL), "the review tool exactly as stored");

    // Nothing of this machine: the root, Kit's folder and the home hold the fixture's own directory, and the user is the host's.
    let host = mine.fleet.host();
    for private in [mine.root.as_str(), host.kit_home.as_str(), host.home.as_str(), host.user.as_str(), "kit/"] {
        assert!(!made.text.contains(private), "the package names `{private}`:\n{}", made.text);
    }
    assert!(!made.text.to_lowercase().contains("time") && !made.text.contains("20"), "no timestamp:\n{}", made.text);
}

#[tokio::test]
async fn a_single_piece_names_the_file_for_itself() {
    let mine = mac(REMOTE);
    mine.keeps_tiles(vec![tile("Run specs", "make test")], Vec::new()).await;
    let made = mine.fleet.package_made(mine.sharing(tiles_of(&["Run specs"]))).await.expect("made");
    assert_eq!(made.file_name, "run-specs.armada.toml");
}

#[tokio::test]
async fn a_piece_the_person_does_not_keep_is_refused_and_so_is_no_piece() {
    let mine = mac(REMOTE);
    mine.keeps_tiles(vec![tile("specs", "make test")], Vec::new()).await;

    let cases = [
        (SharePieces { pieces: Vec::new() }, "No pieces are chosen."),
        (mine.sharing(tiles_of(&[])), "No tiles are chosen."),
        (mine.sharing(tiles_of(&["lint"])), "Your tiles for this repository have no `lint` in job."),
        (mine.sharing(PieceToCommit::Workflow { workflow_id: WorkflowId::carried("quick") }), "Your Kit holds no workflow `quick` this repository runs."),
        (mine.sharing(PieceToCommit::ReviewTool { id: "security".into() }), "Your Kit holds no review tool `security`."),
    ];
    for (asked, said) in cases {
        let (status, code, message) = refused_with(mine.fleet.package_made(asked).await.expect_err("refused"));
        assert_eq!((status, code.as_str(), message.as_str()), (422, "fleet.share_not_yours", said));
    }

    let elsewhere = SharePieces { pieces: vec![PieceToShare { repository: "/not/served".into(), piece: tiles_of(&["specs"]) }] };
    let (status, code, _) = refused_with(mine.fleet.package_made(elsewhere).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (404, "fleet.run_memory_repository_not_served"));
}

/// **A colleague reads every piece, command and remote before anything is written**, and what
/// they read is what Add then writes, on their own copy matched by remote.
#[tokio::test]
async fn a_package_is_read_then_added_to_the_colleagues_own_tiles_and_touches_no_other() {
    let mine = mac(REMOTE);
    mine.keeps_tiles(vec![tile("specs", "make test")], vec![FlightControl { kind: Some(FlightControlKind::Page), text: Some("https://example.test".into()), ..tile("docs", "") }]).await;
    let text = mine
        .fleet
        .package_made(SharePieces {
            pieces: vec![PieceToShare { repository: mine.root.clone(), piece: PieceToCommit::FlightControls { job: vec!["specs".into()], deck: vec!["docs".into()] } }],
        })
        .await
        .expect("made")
        .text;

    let theirs = mac(REMOTE);
    let colleagues_own = tile("lint", "make lint");
    theirs.keeps_tiles(vec![colleagues_own.clone()], Vec::new()).await;
    let machine_file = theirs.kit.join("flight-controls.toml");
    assert!(!machine_file.exists());

    let read = theirs.fleet.package_read(PackageText { text: text.clone() }).await.expect("read");
    let [specs, docs] = read.pieces.as_slice() else { panic!("two pieces: {read:?}") };
    assert_eq!((specs.kind, specs.id.as_str(), specs.list, specs.commands.as_slice()), (PieceKind::Tile, "specs", Some(TileList::Job), ["make test".to_string()].as_slice()));
    assert_eq!((specs.remote.as_deref(), specs.repository.as_deref(), specs.exists), (Some(REMOTE), Some(theirs.root.as_str()), false));
    assert!(docs.commands.is_empty(), "a page runs no shell line: {docs:?}");
    assert_eq!(theirs.tiles().await.job, vec![colleagues_own.clone()], "reading wrote nothing");

    let added = theirs.fleet.package_added(adding(&text, false)).await.expect("added");
    assert_eq!(added.pieces.iter().map(|one| (one.id.as_str(), one.landed)).collect::<Vec<_>>(), [("specs", Landed::Added), ("docs", Landed::Added)]);
    assert!(added.pieces.iter().all(|one| one.repository.as_deref() == Some(theirs.root.as_str())));
    let now = theirs.tiles().await;
    assert_eq!(now.job.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["lint", "specs"]);
    assert_eq!(now.job[0], colleagues_own, "a tile the package does not hold is as it was");
    assert_eq!(now.deck[0].id, "docs");
    assert!(!machine_file.exists(), "the machine's tiles are not touched: these are the repository's");
    let again = theirs.fleet.package_read(PackageText { text }).await.expect("read");
    assert!(again.pieces.iter().all(|one| one.exists), "now Kit holds them: {again:?}");
}

/// **An id already present is refused**, with nothing written for the others, unless replace says so.
#[tokio::test]
async fn an_id_kit_already_holds_is_refused_whole_and_replaced_only_when_asked() {
    let mine = mac(REMOTE);
    mine.keeps_tiles(vec![tile("specs", "make test"), tile("fresh", "make fresh")], Vec::new()).await;
    let text = mine.shared(tiles_of(&["specs", "fresh"])).await;

    let theirs = mac(REMOTE);
    theirs.keeps_tiles(vec![tile("specs", "their own command")], Vec::new()).await;
    let before = theirs.tiles().await;

    let (status, code, said) = refused_with(theirs.fleet.package_added(adding(&text, false)).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (422, "fleet.share_exists"));
    assert_eq!(said, "Your Kit already holds the tile `specs` in job for acme/widgets. Add with replace to replace it.");
    assert_eq!(theirs.tiles().await, before, "the piece that was free is not written either");

    let replaced = theirs.fleet.package_added(adding(&text, true)).await.expect("replaced");
    assert_eq!(replaced.pieces.iter().map(|one| (one.id.as_str(), one.landed)).collect::<Vec<_>>(), [("specs", Landed::Replaced), ("fresh", Landed::Added)]);
    let now = theirs.tiles().await;
    assert_eq!(now.job.iter().map(|one| (one.id.as_str(), one.text.as_deref())).collect::<Vec<_>>(), [("specs", Some("make test")), ("fresh", Some("make fresh"))]);
}

/// Each way a package can be wrong is a refusal that says which piece and what.
#[tokio::test]
async fn a_malformed_package_is_refused_naming_what_is_wrong() {
    let theirs = mac(REMOTE);
    let piece = "[[piece]]\nkind = \"tile\"\nlist = \"job\"\nremote = \"acme/widgets\"\n[piece.tile]\nid = \"specs\"\nkind = \"shell\"\ntext = \"make test\"\n";
    let cases: Vec<(String, &str)> = vec![
        ("not toml [".into(), "The package is not one:"),
        ("version = 2\n".into(), "The package is version 2. This Armada reads version 1."),
        ("version = 1\n".into(), "The package holds no pieces."),
        (format!("version = 1\nowner = \"someone\"\n{piece}"), "owner"),
        (format!("version = 1\n{}", piece.replace("remote = \"acme/widgets\"", "remote = \"widgets\"")), "Piece 1: the remote `widgets` is not owner/name."),
        (format!("version = 1\n{}", piece.replace("kind = \"shell\"\n", "")), "Piece 1: job[0].kind is missing"),
        (format!("version = 1\n{piece}{piece}"), "Pieces 1 and 2 are the same piece `specs`."),
        ("version = 1\n[[piece]]\nkind = \"workflow\"\ntext = \"steps: []\\n\"\n".into(), "Piece 1: the workflow has no `workflow_id`."),
        ("version = 1\n[[piece]]\nkind = \"review_tool\"\nid = \"x\"\ntext = \"no front matter\"\n".into(), "Piece 1: the review tool"),
        ("version = 1\n[[piece]]\nkind = \"hook\"\n".into(), "hook"),
    ];
    for (text, says) in cases {
        for refusal in [
            theirs.fleet.package_read(PackageText { text: text.clone() }).await.expect_err("read refused"),
            theirs.fleet.package_added(adding(&text, true)).await.expect_err("add refused"),
        ] {
            let (status, code, said) = refused_with(refusal);
            assert_eq!((status, code.as_str()), (422, "fleet.share_malformed"), "{text}");
            assert!(said.contains(says), "`{said}` should hold `{says}` for:\n{text}");
        }
    }
    assert_eq!(theirs.tiles().await, FlightControls::default(), "nothing was written");
}

/// **A remote that names none of the colleague's repositories holds the piece unbound**, written
/// nowhere, until one is chosen, once.
#[tokio::test]
async fn a_piece_for_an_unknown_remote_waits_unbound_until_a_repository_is_chosen() {
    let mine = mac("acme/gadgets");
    mine.keeps_tiles(vec![tile("specs", "make test")], Vec::new()).await;
    let text = mine.shared(tiles_of(&["specs"])).await;

    let theirs = mac(REMOTE);
    theirs.keeps_tiles(vec![tile("lint", "make lint")], Vec::new()).await;
    let before = theirs.tiles().await;
    let read = theirs.fleet.package_read(PackageText { text: text.clone() }).await.expect("read");
    assert_eq!((read.pieces[0].remote.as_deref(), read.pieces[0].repository.as_deref(), read.pieces[0].exists), (Some("acme/gadgets"), None, false));

    let added = theirs.fleet.package_added(adding(&text, false)).await.expect("added");
    let piece = &added.pieces[0];
    assert_eq!((piece.landed, piece.repository.as_deref()), (Landed::Unbound, None));
    let unbound_id = piece.unbound_id.clone().expect("named, to be bound");
    assert_eq!(theirs.tiles().await, before, "nothing is written for it");

    let again = theirs.fleet.package_added(adding(&text, false)).await.expect("added again");
    assert_eq!(again.pieces[0].unbound_id.as_deref(), Some(unbound_id.as_str()), "the same piece is held once");
    let held = theirs.fleet.unbound_listed();
    assert_eq!(held.pieces.len(), 1);
    assert_eq!((held.pieces[0].id.as_str(), held.pieces[0].commands.as_slice(), held.pieces[0].remote.as_deref()), ("specs", ["make test".to_string()].as_slice(), Some("acme/gadgets")));

    let bound = theirs.fleet.unbound_bound(BindUnboundPiece { unbound_id: unbound_id.clone(), repository: theirs.root.clone(), replace: false }).await.expect("bound");
    assert_eq!((bound.landed, bound.repository.as_deref()), (Landed::Added, Some(theirs.root.as_str())));
    assert_eq!(theirs.tiles().await.job.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["lint", "specs"]);
    assert!(theirs.fleet.unbound_listed().pieces.is_empty(), "bound once, and no longer waiting");

    let (status, code, _) = refused_with(theirs.fleet.unbound_bound(BindUnboundPiece { unbound_id, repository: theirs.root.clone(), replace: false }).await.expect_err("gone"));
    assert_eq!((status, code.as_str()), (404, "fleet.share_no_unbound_piece"));
}

#[tokio::test]
async fn binding_names_a_repository_fleet_serves_an_id_it_made_and_does_not_replace_unasked() {
    let mine = mac("acme/gadgets");
    mine.keeps_tiles(vec![tile("specs", "make test")], Vec::new()).await;
    let text = mine.shared(tiles_of(&["specs"])).await;
    let theirs = mac(REMOTE);
    theirs.keeps_tiles(vec![tile("specs", "their own command")], Vec::new()).await;
    let added = theirs.fleet.package_added(adding(&text, false)).await.expect("added");
    let unbound_id = added.pieces[0].unbound_id.clone().expect("unbound");

    let bind = |unbound_id: &str, repository: &str, replace: bool| BindUnboundPiece { unbound_id: unbound_id.into(), repository: repository.into(), replace };
    let (status, code, _) = refused_with(theirs.fleet.unbound_bound(bind(&unbound_id, "/not/served", false)).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (404, "fleet.run_memory_repository_not_served"));
    for not_an_id in ["../flight-controls", "0123456789abcdef", ""] {
        let (status, code, _) = refused_with(theirs.fleet.unbound_bound(bind(not_an_id, &theirs.root, false)).await.expect_err("refused"));
        assert_eq!((status, code.as_str()), (404, "fleet.share_no_unbound_piece"), "{not_an_id}");
    }
    let (_, code, _) = refused_with(theirs.fleet.unbound_bound(bind(&unbound_id, &theirs.root, false)).await.expect_err("refused"));
    assert_eq!(code, "fleet.share_exists");
    assert_eq!(theirs.fleet.unbound_listed().pieces.len(), 1, "a refusal leaves it waiting");

    let bound = theirs.fleet.unbound_bound(bind(&unbound_id, &theirs.root, true)).await.expect("replaced");
    assert_eq!(bound.landed, Landed::Replaced);
    assert_eq!(theirs.tiles().await.job[0].text.as_deref(), Some("make test"));
}

/// A review tool lands in Kit's folder as the file it was, and replaces the one of that id in place.
#[tokio::test]
async fn a_review_tool_is_written_to_kit_as_its_file_and_replaced_only_when_asked() {
    let mine = mac(REMOTE);
    mine.keeps_review_tool(REVIEW_TOOL);
    let text = mine.shared(PieceToCommit::ReviewTool { id: "security".into() }).await;

    let theirs = mac(REMOTE);
    let read = theirs.fleet.package_read(PackageText { text: text.clone() }).await.expect("read");
    assert_eq!((read.pieces[0].kind, read.pieces[0].name.as_str(), read.pieces[0].commands.is_empty(), read.pieces[0].exists), (PieceKind::ReviewTool, "Security", true, false));

    theirs.fleet.package_added(adding(&text, false)).await.expect("added");
    let file = theirs.kit.join("review-tools").join("security.md");
    assert_eq!(std::fs::read_to_string(&file).expect("in Kit"), REVIEW_TOOL);

    std::fs::write(&file, "---\nkind: tool\nname: Security\n---\nOlder prompt.\n").expect("edited");
    let (_, code, _) = refused_with(theirs.fleet.package_added(adding(&text, false)).await.expect_err("refused"));
    assert_eq!(code, "fleet.share_exists");
    assert!(std::fs::read_to_string(&file).expect("there").contains("Older prompt."), "refused, so untouched");
    let replaced = theirs.fleet.package_added(adding(&text, true)).await.expect("replaced");
    assert_eq!(replaced.pieces[0].landed, Landed::Replaced);
    assert_eq!(std::fs::read_to_string(&file).expect("in Kit"), REVIEW_TOOL);
}

/// A workflow is added as `save_workflow` saves one into Kit, so the repository's Checks decide whether it fits.
#[tokio::test]
async fn a_workflow_is_saved_into_kit_through_the_repositorys_own_check() {
    let mine = mac(REMOTE);
    mine.keeps_workflow(WORKFLOW);
    let text = mine.shared(PieceToCommit::Workflow { workflow_id: WorkflowId::carried("quick") }).await;

    let theirs = mac(REMOTE);
    let read = theirs.fleet.package_read(PackageText { text: text.clone() }).await.expect("read");
    assert_eq!((read.pieces[0].kind, read.pieces[0].id.as_str(), read.pieces[0].name.as_str(), read.pieces[0].exists), (PieceKind::Workflow, "quick", "Quick", false));

    theirs.locator.will_save(Ok(SavedWorkflow { id: theirs.fleet.repositories().first().expect("served").workflows().keys().next().expect("one").clone(), file: "/kit/workflows/quick.json".into(), replaced: false }));
    let added = theirs.fleet.package_added(adding(&text, false)).await.expect("added");
    assert_eq!(added.pieces[0].landed, Landed::Added);
    let saves = theirs.locator.saves();
    assert_eq!(saves.len(), 1);
    assert_eq!((saves[0].scope, saves[0].definition.as_str(), saves[0].overwrite), (ipc::WorkflowScope::Kit, WORKFLOW, false));

    theirs.keeps_workflow(WORKFLOW);
    let (_, code, _) = refused_with(theirs.fleet.package_added(adding(&text, false)).await.expect_err("refused"));
    assert_eq!(code, "fleet.share_exists");
    assert_eq!(theirs.locator.saves().len(), 1, "refused before it was saved");
    assert!(theirs.fleet.package_read(PackageText { text }).await.expect("read").pieces[0].exists);
}

/// A package text holding `kind` pieces of one id, each for its own remote.
fn same_id_for(kind: &str, body: &str, remotes: [&str; 2]) -> String {
    let mut text = String::from("version = 1\n");
    for remote in remotes {
        text.push_str(&format!("[[piece]]\nkind = \"{kind}\"\nremote = \"{remote}\"\n{body}"));
    }
    text
}

/// Kit keeps one workflow and one review tool of an id for every repository, so a package holding one id for
/// two repositories would have the second overwrite the first.
#[tokio::test]
async fn one_workflow_or_review_tool_id_for_two_repositories_is_refused_on_read() {
    let theirs = mac(REMOTE);
    let workflow = format!("text = '''{WORKFLOW}'''\n");
    let tool = format!("id = \"security\"\ntext = '''{REVIEW_TOOL}'''\n");
    let cases = [
        (same_id_for("workflow", &workflow, ["acme/widgets", "acme/gadgets"]), "Pieces 1 and 2 are the same piece `quick`. Kit keeps one of an id for every repository."),
        (same_id_for("review_tool", &tool, ["acme/widgets", "acme/gadgets"]), "Pieces 1 and 2 are the same piece `security`. Kit keeps one of an id for every repository."),
    ];
    for (text, says) in cases {
        for refusal in [
            theirs.fleet.package_read(PackageText { text: text.clone() }).await.expect_err("read refused"),
            theirs.fleet.package_added(adding(&text, true)).await.expect_err("add refused"),
        ] {
            let (status, code, said) = refused_with(refusal);
            assert_eq!((status, code.as_str(), said.as_str()), (422, "fleet.share_malformed", says));
        }
    }
    assert!(!theirs.kit.join("review-tools").join("security.md").exists(), "nothing was written");
    assert!(theirs.locator.saves().is_empty(), "nothing was saved");
}

/// Every workflow is checked before the first is saved, so a refusal for a later one leaves the earlier unwritten.
#[tokio::test]
async fn a_workflow_save_would_refuse_is_found_before_any_workflow_is_saved() {
    let theirs = mac(REMOTE);
    let other = WORKFLOW.replace("quick", "slow");
    let piece = |text: &str| format!("[[piece]]\nkind = \"workflow\"\nremote = \"{REMOTE}\"\ntext = '''{text}'''\n");
    let text = format!("version = 1\n{}{}", piece(WORKFLOW), piece(&other));

    theirs.locator.will_check(vec![Ok(()), Err(WorkflowNotSaved::Unfit { why: "it names a Check the Manifest does not declare".into() })]);
    let (status, code, said) = refused_with(theirs.fleet.package_added(adding(&text, false)).await.expect_err("refused"));
    assert_eq!((status, code.as_str(), said.as_str()), (422, "fleet.workflow_unfit", "it names a Check the Manifest does not declare"));
    assert_eq!(theirs.locator.checks().iter().map(|asked| asked.definition.as_str()).collect::<Vec<_>>(), [WORKFLOW, other.as_str()]);
    assert!(theirs.locator.saves().is_empty(), "the first workflow was not saved either");
}
