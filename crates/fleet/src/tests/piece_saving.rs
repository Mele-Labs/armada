//! Saving and deleting one Workspace, lifecycle, rule or autonomy default in Kit: the file the
//! writer makes is read back by the piece's own parser as the piece it was made from, a piece that
//! does not parse writes nothing, and a piece is found by what its file declares. `crate::piece_saving`.

use std::path::{Path, PathBuf};

use api::Refusal;
use core_model::rule::Rule;
use core_model::{Lifecycle, ManifestId, Ulid, Workspace};
use ipc::{
    AutonomyToSave, DeleteLifecyclePiece, LifecyclePiece, LifecyclePieceKind, PieceScope, SaveLifecyclePiece,
    WorkspaceToSave,
};

use super::session_host::{rig_placing, Rig};
use crate::lifecycles::wire;
use crate::piece_saving::{PIECE_EXISTS, PIECE_NOT_KEPT, PIECE_UNFIT};

const TICKET: &str = "version: 1
lifecycle_id: ticket
name: Ticket
states:
  - { id: todo, label: To do, read: { tracker: [] } }
  - { id: in_progress, label: In progress, read: { tracker: [] } }
  - { id: closed, label: Closed, read: { tracker: [] } }
transitions:
  - { from: todo, to: in_progress }
  - { from: in_progress, to: closed }
ends: [closed]
";

const PULL_REQUEST: &str = "version: 1
lifecycle_id: pr
name: Pull request
states:
  - { id: draft, label: Draft, read: { forge: draft } }
  - { id: ready, label: Ready, read: { forge: ready } }
  - { id: merged, label: Merged, read: { forge: merged } }
transitions:
  - { from: draft, to: ready }
  - { from: ready, to: merged }
ends: [merged]
";

const DEPLOY: &str = "version: 1
lifecycle_id: deploy
name: Deploy
states:
  - { id: rolling_out, label: Rolling out, read: { watch: running } }
  - { id: healthy, label: Healthy, read: { watch: passed } }
ends: [healthy]
";

const JOB: &str = "version: 1
lifecycle_id: job
name: Job
states:
  - { id: building, label: Building, read: { fleet: 'step_entered:build' } }
  - { id: landed, label: Landed, read: { fleet: landed } }
ends: [landed]
";

/// Words a YAML writer must quote to keep them text, in a name and in the tracker's own words.
const AWKWARD_WORDS: &str = "version: 1
lifecycle_id: awkward
name: 'Yes: no # maybe'
scope: my-store
states:
  - { id: first, label: 'true', read: { tracker: ['123', 'null', 'In Progress: now', '- dash', '''quoted'] } }
  - { id: second, label: '~', read: { tracker: [] } }
ends: [second]
";

const MOVES_TICKETS: &str = "version: 1
rule_id: plan_moves_tickets
name: Move the tickets when planning starts
scope: my-store
when: { step_starts: plan }
if:
  - any: { held: ticket, in: todo }
do:
  - tracker_write: { move: in_progress, on: { held: ticket, in: todo } }
autonomy: acts
on_failure: { retries: 3, then: tell_agent }
";

const EVERYTHING: &str = "version: 1
rule_id: everything
name: Everything
when: { enters: { lifecycle: pr, state: approved } }
if:
  - every: { linked_from: ticket, in: in_review }
  - no: { linking_to: job, in: running }
  - any: { entered: true, in: approved }
  - job: { workflow: feature }
  - job: { step: build }
  - job: { pull_request_opened: true }
do:
  - tracker_write: { assign: alice, on: { entered: true } }
  - tracker_write: { move: closed, on: { linked_from: ticket } }
  - tracker_write: { close: true, on: { linked_from: ticket } }
  - forge_write: { request_reviewers: [alice, bob], on: { entered: true } }
  - forge_write: { mark_ready: true, on: { entered: true } }
  - forge_write: { merge: true, on: { entered: true } }
  - forge_write: { auto_merge: true, on: { entered: true } }
  - draft: { template: 'Ready for {pr}', on: { entered: true } }
  - start_job: { prompt: Look at the deploy, workflow: bug }
  - start_watch: { recipe: deploy-health, values: { sha: abc, url: 'https://x.test/a: b' }, deploy: web }
  - tell_agent: { message: Approved }
  - tell_agent: { wake: true }
  - run_command: { command: lint }
  - run_agent: { skill: review }
  - run_agent: { drone: Summarise the review }
autonomy: asks
on_failure: { retries: 0, then: ask_owner }
";

const NUDGE: &str = "version: 1\nrule_id: nudge\nname: Nudge\nwhen: { job: landed }\ndo:\n  - tell_agent: { wake: true }\n";

/// A prompt that is several lines, and words that read as something else unquoted.
const AWKWARD_RULE: &str = "version: 1
rule_id: awkward_rule
name: 'Review: #1 (yes)'
when: { job: started }
do:
  - start_job: { prompt: \"First line\\nSecond line: with a colon\\n\\n- and a dash\", workflow: 'on' }
  - start_watch: { recipe: checks, values: { sha: '0123', flag: 'true' } }
  - run_command: { command: \"cargo test -- --nocapture # all\" }
";

const WORKSPACE: &str = "version: 1\nworkspace_id: my-store\nname: 'My store: web and api'\nrepositories:\n  - store-web\n  - store-api\n";

const AUTONOMY: &str = "version: 1\nautonomy_id: mine\nscope: my-store\ndefaults:\n  tracker_write: acts\n  forge_write: asks\n  run_command: off\n";

fn kit_folder(rig: &Rig, folder: &str) -> PathBuf {
    PathBuf::from(&rig.fleet.host().kit_home).join(folder)
}

fn plant(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("made");
    std::fs::write(path, text).expect("written");
}

fn saving(piece: LifecyclePiece, overwrite: bool) -> SaveLifecyclePiece {
    SaveLifecyclePiece { piece, overwrite }
}

fn lifecycle_of(text: &str) -> (Lifecycle, LifecyclePiece) {
    let read = config::parse_lifecycle(Path::new("fixture.yml"), text).expect("the parser's own fixture loads");
    let piece = LifecyclePiece::Lifecycle { lifecycle: wire::lifecycle(&read) };
    (read, piece)
}

fn rule_of(text: &str) -> (Rule, LifecyclePiece) {
    let read = config::parse_rule(Path::new("fixture.yml"), text).expect("the parser's own fixture loads");
    let piece = LifecyclePiece::Rule { rule: wire::rule(&read) };
    (read, piece)
}

fn workspace_of(text: &str) -> (Workspace, LifecyclePiece) {
    let read = config::parse_workspace(Path::new("fixture.yml"), text).expect("the parser's own fixture loads");
    let workspace = WorkspaceToSave {
        id: read.id().as_str().to_string(),
        name: read.title().as_str().to_string(),
        repositories: read.repositories().iter().map(ipc::ManifestId::from).collect(),
    };
    (read, LifecyclePiece::Workspace { workspace })
}

fn autonomy_of(text: &str) -> (config::AutonomyPiece, LifecyclePiece) {
    let read = config::parse_autonomy(Path::new("fixture.yml"), text).expect("the parser's own fixture loads");
    let scope = match read.scope() {
        core_model::LifecycleScope::Everywhere => PieceScope::Everywhere,
        core_model::LifecycleScope::Within(id) => PieceScope::Within { id: id.as_str().to_string() },
    };
    let defaults = read.defaults().iter().map(|(kind, autonomy)| (kind.as_wire().to_string(), autonomy.as_wire().to_string())).collect();
    let autonomy = AutonomyToSave { id: read.id().as_str().to_string(), scope, defaults };
    (read, LifecyclePiece::Autonomy { autonomy })
}

fn refused_with(refusal: Refusal) -> (String, String) {
    match refusal {
        Refusal::Unacceptable(error) => (error.code, error.message),
        other => panic!("not a 422: {other:?}"),
    }
}

/// **The writer cannot drift from the parser.** Each fixture is a file the parser's own tests load;
/// it goes to the DTO a read answers, through the writer, to a file, and back through the parser to
/// the same piece. A part the writer dropped or spelled differently reads back as another value.
#[tokio::test]
async fn the_parsers_own_fixtures_come_back_the_same_through_the_writer() {
    let rig = rig_placing(None);
    for fixture in [TICKET, PULL_REQUEST, DEPLOY, JOB, AWKWARD_WORDS] {
        let (read, piece) = lifecycle_of(fixture);
        let saved = rig.fleet.lifecycle_piece_saved(saving(piece, false)).expect("saved");
        let written = std::fs::read_to_string(&saved.file).expect("a file");
        let again = config::parse_lifecycle(Path::new(&saved.file), &written).unwrap_or_else(|why| panic!("{why}\n{written}"));
        assert_eq!(again, read, "{written}");
    }
    for fixture in [MOVES_TICKETS, EVERYTHING, NUDGE, AWKWARD_RULE] {
        let (read, piece) = rule_of(fixture);
        let saved = rig.fleet.lifecycle_piece_saved(saving(piece, false)).expect("saved");
        let written = std::fs::read_to_string(&saved.file).expect("a file");
        let again = config::parse_rule(Path::new(&saved.file), &written).unwrap_or_else(|why| panic!("{why}\n{written}"));
        assert_eq!(again, read, "{written}");
    }
    let (read, piece) = workspace_of(WORKSPACE);
    let saved = rig.fleet.lifecycle_piece_saved(saving(piece, false)).expect("saved");
    let written = std::fs::read_to_string(&saved.file).expect("a file");
    assert_eq!(config::parse_workspace(Path::new(&saved.file), &written).expect("reads"), read, "{written}");
    let (read, piece) = autonomy_of(AUTONOMY);
    let saved = rig.fleet.lifecycle_piece_saved(saving(piece, false)).expect("saved");
    let written = std::fs::read_to_string(&saved.file).expect("a file");
    assert_eq!(config::parse_autonomy(Path::new(&saved.file), &written).expect("reads"), read, "{written}");
}

/// What a person opens reads as the keys the parsers take, in the order a hand-written file has them.
#[tokio::test]
async fn a_saved_file_is_plain_yaml_with_the_keys_a_person_would_write() {
    let rig = rig_placing(None);
    let (_, piece) = rule_of(MOVES_TICKETS);
    let saved = rig.fleet.lifecycle_piece_saved(saving(piece, false)).expect("saved");
    let written = std::fs::read_to_string(&saved.file).expect("a file");
    assert!(written.starts_with("version: 1\nrule_id: plan_moves_tickets\nname: Move the tickets when planning starts\nscope: my-store\nwhen:\n  step_starts: plan\n"), "{written}");
    assert!(!written.contains('!'), "no YAML tags: {written}");
    let (_, piece) = lifecycle_of(TICKET);
    let written = std::fs::read_to_string(rig.fleet.lifecycle_piece_saved(saving(piece, false)).expect("saved").file).expect("a file");
    assert!(!written.contains("scope:") && !written.contains("transitions: []"), "what is absent leaves no key: {written}");
}

#[tokio::test]
async fn a_new_file_is_named_for_the_piece_and_its_scope_and_kits_folder_is_made() {
    let rig = rig_placing(None);
    assert!(!kit_folder(&rig, "lifecycles").exists());
    let (_, everywhere) = lifecycle_of(TICKET);
    let saved = rig.fleet.lifecycle_piece_saved(saving(everywhere, false)).expect("saved");
    assert_eq!((saved.kind, saved.id.as_str(), saved.scope.clone(), saved.replaced), (LifecyclePieceKind::Lifecycle, "ticket", PieceScope::Everywhere, false));
    assert_eq!(PathBuf::from(&saved.file), kit_folder(&rig, "lifecycles").join("ticket.yml"));

    let (_, within) = lifecycle_of(&TICKET.replace("name: Ticket", "name: Ticket\nscope: my-store"));
    let saved = rig.fleet.lifecycle_piece_saved(saving(within, false)).expect("saved");
    assert_eq!(saved.scope, PieceScope::Within { id: "my-store".to_string() });
    assert_eq!(PathBuf::from(&saved.file), kit_folder(&rig, "lifecycles").join("ticket.my-store.yml"));
    let held: Vec<_> = std::fs::read_dir(kit_folder(&rig, "lifecycles")).expect("a folder").map(|entry| entry.expect("an entry").file_name()).collect();
    assert_eq!(held.len(), 2, "two files that rank against each other, and no staged leftover: {held:?}");
}

/// **A piece that does not parse is refused with the parser's own sentence, every fault of it, and
/// nothing is written**: not the folder, not a staged file.
#[tokio::test]
async fn a_piece_that_does_not_parse_is_refused_with_every_fault_and_writes_nothing() {
    let rig = rig_placing(None);
    let (_, piece) = lifecycle_of(TICKET);
    let LifecyclePiece::Lifecycle { mut lifecycle } = piece else { unreachable!("a lifecycle") };
    lifecycle.id = "Not An Id".to_string();
    lifecycle.states[0].read = ipc::StateReadFrom::Forge { fact: "nonsense".to_string() };
    lifecycle.states[1].id = "Bad State".to_string();
    let refusal = rig.fleet.lifecycle_piece_saved(saving(LifecyclePiece::Lifecycle { lifecycle }, false)).expect_err("refused");
    let (code, said) = refused_with(refusal);
    assert_eq!(code, PIECE_UNFIT);
    for fault in ["lifecycle_id", "states[0].read.forge", "states[1].id"] {
        assert!(said.contains(fault), "{fault} is named in: {said}");
    }
    assert!(said.contains("(lifecycle) was refused"), "the parser's own sentence: {said}");
    assert!(!kit_folder(&rig, "lifecycles").exists(), "nothing was created");

    let (_, piece) = rule_of(MOVES_TICKETS);
    let LifecyclePiece::Rule { mut rule } = piece else { unreachable!("a rule") };
    rule.on_failure.retries = 9;
    rule.on_failure.then = "ignore".to_string();
    rule.conditions = vec![ipc::RuleCondition::Instances {
        among: ipc::RuleAmong::Held { lifecycle: "ticket".to_string() },
        quantity: "most".to_string(),
        in_state: "todo".to_string(),
    }];
    let (code, said) = refused_with(rig.fleet.lifecycle_piece_saved(saving(LifecyclePiece::Rule { rule }, false)).expect_err("refused"));
    assert_eq!(code, PIECE_UNFIT);
    for fault in ["on_failure.retries", "on_failure.then", "if[0]"] {
        assert!(said.contains(fault), "{fault} is named in: {said}");
    }
    assert!(!kit_folder(&rig, "rules").exists());

    let none = AutonomyToSave { id: "mine".to_string(), scope: PieceScope::Everywhere, defaults: Default::default() };
    let (code, _) = refused_with(rig.fleet.lifecycle_piece_saved(saving(LifecyclePiece::Autonomy { autonomy: none }, false)).expect_err("refused"));
    assert_eq!(code, PIECE_UNFIT, "a layer that sets nothing is a file someone expected to do something");
    let blank = WorkspaceToSave { id: "  ".to_string(), name: "My store".to_string(), repositories: Vec::new() };
    let (code, _) = refused_with(rig.fleet.lifecycle_piece_saved(saving(LifecyclePiece::Workspace { workspace: blank }, false)).expect_err("refused"));
    assert_eq!(code, PIECE_UNFIT);
    assert!(!kit_folder(&rig, "autonomy").exists() && !kit_folder(&rig, "workspaces").exists());
}

/// **A file is the piece it declares, whatever it is called.** The file that holds the identity is
/// the one replaced, in place, and only with `overwrite`.
#[tokio::test]
async fn replacing_goes_by_what_a_file_declares_and_only_with_overwrite() {
    let rig = rig_placing(None);
    let folder = kit_folder(&rig, "lifecycles");
    let elsewhere = folder.join("work-tickets.yaml");
    plant(&elsewhere, TICKET);
    let (_, replacement) = lifecycle_of(&TICKET.replace("name: Ticket", "name: Ticket, replaced"));

    let (code, said) = refused_with(rig.fleet.lifecycle_piece_saved(saving(replacement.clone(), false)).expect_err("refused"));
    assert_eq!(code, PIECE_EXISTS);
    assert!(said.contains(&elsewhere.display().to_string()) && said.contains("overwrite"), "{said}");
    assert_eq!(std::fs::read_to_string(&elsewhere).expect("there"), TICKET, "nothing was written");

    let saved = rig.fleet.lifecycle_piece_saved(saving(replacement, true)).expect("replaced");
    assert!(saved.replaced);
    assert_eq!(PathBuf::from(&saved.file), elsewhere, "the file that declared it, under its own name");
    assert!(std::fs::read_to_string(&elsewhere).expect("there").contains("Ticket, replaced"));
    assert_eq!(std::fs::read_dir(&folder).expect("a folder").count(), 1, "no second file");
}

/// `ticket` for everywhere and `ticket` within a repository are two pieces: saving one never
/// replaces, or asks to replace, the other.
#[tokio::test]
async fn a_piece_is_its_kind_id_and_scope_and_not_its_id() {
    let rig = rig_placing(None);
    let folder = kit_folder(&rig, "lifecycles");
    plant(&folder.join("ticket.yml"), TICKET);
    let (_, within) = lifecycle_of(&TICKET.replace("name: Ticket", "name: Ticket\nscope: my-store"));
    let saved = rig.fleet.lifecycle_piece_saved(saving(within, false)).expect("a different piece");
    assert!(!saved.replaced);
    assert_eq!(std::fs::read_to_string(folder.join("ticket.yml")).expect("there"), TICKET);
    assert_eq!(PathBuf::from(&saved.file), folder.join("ticket.my-store.yml"));
}

/// A new file never takes the name of a file that declares something else, nor one that declares
/// nothing a parser reads.
#[tokio::test]
async fn a_new_file_never_overwrites_a_file_that_declares_something_else() {
    let rig = rig_placing(None);
    let folder = kit_folder(&rig, "lifecycles");
    plant(&folder.join("ticket.yml"), &TICKET.replace("lifecycle_id: ticket", "lifecycle_id: ticket_v2"));
    plant(&folder.join("ticket-2.yml"), "not: a lifecycle");
    let (_, piece) = lifecycle_of(TICKET);
    let saved = rig.fleet.lifecycle_piece_saved(saving(piece, false)).expect("saved");
    assert!(!saved.replaced);
    assert_eq!(PathBuf::from(&saved.file), folder.join("ticket-3.yml"));
    assert!(std::fs::read_to_string(folder.join("ticket.yml")).expect("there").contains("ticket_v2"));
    assert_eq!(std::fs::read_to_string(folder.join("ticket-2.yml")).expect("there"), "not: a lifecycle");
}

/// A scope is any text and a file's name is not.
#[tokio::test]
async fn a_scope_that_is_not_fit_for_a_file_name_is_made_fit() {
    let rig = rig_placing(None);
    let (_, piece) = rule_of(&NUDGE.replace("name: Nudge", "name: Nudge\nscope: 'a/../b c'"));
    let saved = rig.fleet.lifecycle_piece_saved(saving(piece, false)).expect("saved");
    assert_eq!(PathBuf::from(&saved.file), kit_folder(&rig, "rules").join("nudge.a----b-c.yml"));
    let (_, piece) = workspace_of(&WORKSPACE.replace("my-store", "../../escape"));
    let saved = rig.fleet.lifecycle_piece_saved(saving(piece, false)).expect("saved");
    assert_eq!(PathBuf::from(&saved.file).parent(), Some(kit_folder(&rig, "workspaces").as_path()));
}

/// Two files that declare one piece leave both out of every read; the save that replaces one leaves
/// the piece standing.
#[tokio::test]
async fn a_save_that_replaces_one_of_two_files_for_a_piece_removes_the_other() {
    let rig = rig_placing(None);
    let folder = kit_folder(&rig, "workspaces");
    plant(&folder.join("a.yml"), WORKSPACE);
    plant(&folder.join("b.yml"), WORKSPACE);
    let (_, piece) = workspace_of(WORKSPACE);
    let saved = rig.fleet.lifecycle_piece_saved(saving(piece, true)).expect("replaced");
    assert_eq!(PathBuf::from(&saved.file), folder.join("a.yml"));
    assert!(!folder.join("b.yml").exists());
}

fn deleting(kind: LifecyclePieceKind, id: &str, scope: PieceScope) -> DeleteLifecyclePiece {
    DeleteLifecyclePiece { kind, id: id.to_string(), scope }
}

#[tokio::test]
async fn a_delete_removes_the_file_that_declares_the_piece_and_no_other() {
    let rig = rig_placing(None);
    let folder = kit_folder(&rig, "lifecycles");
    plant(&folder.join("tickets-for-everyone.yml"), TICKET);
    plant(&folder.join("ticket.my-store.yml"), &TICKET.replace("name: Ticket", "name: Ticket\nscope: my-store"));
    let in_the_repository = Path::new(&rig.root).join(".armada/lifecycles/ticket.yml");
    plant(&in_the_repository, TICKET);

    let (code, said) = refused_with(rig.fleet.lifecycle_piece_deleted(deleting(LifecyclePieceKind::Lifecycle, "ticket", PieceScope::Within { id: "elsewhere".into() })).expect_err("refused"));
    assert_eq!((code.as_str(), said.as_str()), (PIECE_NOT_KEPT, "Kit holds no lifecycle `ticket` within `elsewhere`."));
    let (code, said) = refused_with(rig.fleet.lifecycle_piece_deleted(deleting(LifecyclePieceKind::Rule, "ticket", PieceScope::Everywhere)).expect_err("refused"));
    assert_eq!((code.as_str(), said.as_str()), (PIECE_NOT_KEPT, "Kit holds no rule `ticket`."));

    let deleted = rig.fleet.lifecycle_piece_deleted(deleting(LifecyclePieceKind::Lifecycle, "ticket", PieceScope::Everywhere)).expect("deleted");
    assert_eq!(PathBuf::from(&deleted.file), folder.join("tickets-for-everyone.yml"));
    assert_eq!((deleted.kind, deleted.id.as_str(), deleted.scope), (LifecyclePieceKind::Lifecycle, "ticket", PieceScope::Everywhere));
    assert!(!folder.join("tickets-for-everyone.yml").exists());
    assert!(folder.join("ticket.my-store.yml").exists(), "the piece within a scope is another piece");
    assert!(in_the_repository.exists(), "nothing outside Kit's folder is touched");
}

#[tokio::test]
async fn a_workspace_has_no_scope_to_delete_it_by() {
    let rig = rig_placing(None);
    plant(&kit_folder(&rig, "workspaces").join("my-store.yml"), WORKSPACE);
    let scoped = PieceScope::Within { id: "ignored".to_string() };
    let deleted = rig.fleet.lifecycle_piece_deleted(deleting(LifecyclePieceKind::Workspace, "my-store", scoped)).expect("deleted");
    assert_eq!(deleted.scope, PieceScope::Everywhere);
    assert!(!kit_folder(&rig, "workspaces").join("my-store.yml").exists());
}

#[tokio::test]
async fn the_repository_ids_of_a_workspace_survive_the_trip() {
    let rig = rig_placing(None);
    let (read, piece) = workspace_of(WORKSPACE);
    let saved = rig.fleet.lifecycle_piece_saved(saving(piece, false)).expect("saved");
    let again = config::parse_workspace(Path::new(&saved.file), &std::fs::read_to_string(&saved.file).expect("a file")).expect("reads");
    assert_eq!(again.repositories(), [ManifestId::carried(Ulid::carried("store-web")), ManifestId::carried(Ulid::carried("store-api"))]);
    assert_eq!(again, read);
}
