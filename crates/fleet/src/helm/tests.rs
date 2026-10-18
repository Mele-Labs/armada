//! Helm's note, pinned whole, and the denylist `#1150` replaced `#73`'s
//! allowlist with.
//!
//! **The snapshots pin the copy on purpose**, unlike `tests::briefing`: `#940`'s
//! claim is the assembled text. An edit to section 5a of the Agent Prompt
//! Contract lands here in the same change.

use std::path::Path;

use config::Manifest;
use ipc::door::{HELM_ONLY, REACHABLE};

use super::reach::{RESERVED, UNASKED};
use super::{brief, may, Authority, Voice};

/// Acts the door offers that stay a person's, independent of the ruling that
/// granted Helm the rest (`#1150`). Mirrors [`RESERVED`] so a test failure
/// reads as this list going stale rather than as the const under test.
const THE_PERSONS: &[&str] = RESERVED;

/// A Manifest carrying a Command, so a test can see its line is not told.
fn a_manifest() -> Manifest {
    Manifest::parse(
        Path::new("/work/armada/armada.yml"),
        "version: 1\nid: armada\ncommands:\n  release:\n    run: ./scripts/publish-everything\n",
    )
    .expect("a Manifest")
}

const ACTING_IN_A_TERSE_VOICE: &str = "\
You are Helm, docked in Armada over repository armada, read from \
/work/armada. A person asks you about the work in it. The Fleet tools reach what Armada knows \
of this repository: its Jobs, Drones, pull requests and what is waiting on the person. \
Saying that you did something does not do it. Only a tool call does.

To find a Job, call list_jobs and match what the person said against its titles. It lists \
the live ones and takes state=ended for finished ones. Ask the person which only when two of them \
fit.

To pass words to a running Job, call tell_session with its id and the words. It sends them as \
the person's own message would, and the thread shows where they went.

To change a setting the person asks for, find its key and values with get_settings, then save it \
with save_settings. Limits, sleep mode, mods and Kit servers have their own tools.

Before what the person typed, one line says where they are in Bridge: the screen, the \
repository picked, and the Studio and node open on the Studios screen. It names ids and nothing \
more.

On a Studio you may add a node that starts proposed, propose an edge and name an untitled Studio \
without being asked (add_studio_node, propose_studio_edge and rename_studio). Every other act on \
one waits for the person's ask.

VOICE

Terse.

This sets how long and how formal your answers are. It changes nothing else in this note.";

const A_READ_ONLY_HELM: &str = "\
You are Helm, docked in Armada over repository armada, read from \
/work/armada. A person asks you about the work in it. The Fleet tools reach what Armada knows \
of this repository: its Jobs, Drones, pull requests and what is waiting on the person. \
Saying that you did something does not do it. Only a tool call does.

Before what the person typed, one line says where they are in Bridge: the screen, the \
repository picked, and the Studio and node open on the Studios screen. It names ids and nothing \
more.

This machine is set so that Helm only reads. You may call every tool that reads and none that \
acts. Where an act would help, say which and why, and leave it to the person.";

#[test]
fn the_note_for_an_acting_helm_reads_as_the_contract_draws_it() {
    let voice = Voice::said("Terse.");
    let note = brief(&a_manifest(), Authority::Acting, voice.as_ref());
    assert_eq!(note.as_str(), ACTING_IN_A_TERSE_VOICE);
}

/// Read-only says it reads, is not told how to pass words on or what a Studio allows unasked, and
/// no Voice renders no Voice block.
#[test]
fn a_read_only_helm_with_no_voice_is_told_it_only_reads() {
    let note = brief(&a_manifest(), Authority::ReadOnly, None);
    assert_eq!(note.as_str(), A_READ_ONLY_HELM);
    assert!(!note.as_str().contains("VOICE"));
    assert!(!note.as_str().contains("tell_session"));
    assert!(!note.as_str().contains("add_studio_node"));
}

/// **A note and not a brief**: what the Helm Session is told is short, and none of what the old
/// brief spent most of its words on is in it.
#[test]
fn the_note_is_short_and_carries_none_of_what_the_brief_did() {
    let note = brief(&a_manifest(), Authority::Acting, None);
    let note = note.as_str();
    assert!(note.split_whitespace().count() < 250, "{} words", note.split_whitespace().count());
    for gone in [
        "AUTHORING A WORKFLOW",
        "advance_gate",
        "workflow_id",
        "never on your own initiative",
        "one observation",
        "get_events_since",
        "THE CHECKOUT",
        "no branch, no review and no undo",
    ] {
        assert!(!note.contains(gone), "`{gone}` went with the brief");
    }
}

/// Never told anything of the Manifest past its id and folder.
#[test]
fn the_note_does_not_quote_the_manifest() {
    let note = brief(&a_manifest(), Authority::Acting, None);
    assert!(!note.as_str().contains("publish-everything"));
    assert!(!note.as_str().contains("release"));
}

/// A command marked `Yes` is Helm's under `Acting` unless `THE_PERSONS` names
/// it — the denylist's whole shape, checked against the door's own set rather
/// than asserted about `RESERVED` alone.
#[test]
fn every_act_the_door_offers_is_decided_for_helm_or_the_person() {
    for row in REACHABLE.iter().filter(|row| row.kind == "command") {
        assert_eq!(
            may(Authority::Acting, row),
            !THE_PERSONS.contains(&row.operation),
            "`{}` disagrees with the reserved list",
            row.operation
        );
    }
    for kept in THE_PERSONS {
        let row = REACHABLE
            .iter()
            .find(|row| row.operation == *kept)
            .unwrap_or_else(|| panic!("`{kept}` is no longer offered, so this list is stale"));
        assert!(!may(Authority::Acting, row));
    }
}

/// A name reserved from Helm has to be a command the door actually offers —
/// reserving a name nobody offers would decide nothing.
#[test]
fn every_reserved_act_is_an_operation_the_door_offers() {
    let inventory = {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ipc/operations");
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .expect("the inventory")
            .map(|entry| entry.expect("an entry").path())
            .collect();
        files.sort();
        let mut text = String::from("\n");
        for file in files {
            text.push_str(&std::fs::read_to_string(file).expect("an operation"));
            text.push('\n');
        }
        text
    };
    for act in THE_PERSONS {
        let (_, row) = inventory
            .split_once(&format!("\n[operations.{act}]\n"))
            .unwrap_or_else(|| panic!("`{act}` is not in the inventory"));
        let row = row.split("\n[operations.").next().unwrap_or(row);
        assert!(
            row.contains("agent_access = \"Yes\""),
            "`{act}` is not one `Yes` offers, so reserving it from Helm reserves nothing"
        );
    }
}

/// The door offers `Helm only` rows to a Helm session alone, so an acting one
/// must be allowed every row of it, and a read-only one only its reads —
/// the checkout's runs (`#1288`). `#1150`.
#[test]
fn helm_only_is_an_acting_helms_and_only_its_reads_a_read_only_ones() {
    assert!(
        !HELM_ONLY.is_empty(),
        "the inventory has rows reading `Helm only`"
    );
    for row in HELM_ONLY {
        assert!(may(Authority::Acting, row), "`{}`", row.operation);
        assert_eq!(
            may(Authority::ReadOnly, row),
            row.kind == "query" || row.operation == ipc::door::ASKS_A_PERSON,
            "`{}`",
            row.operation
        );
    }
}

/// `#1389`: **the permission tool is not an act, so no authority decides it.**
/// It is a `Helm only` command by shape — it is a POST — and a read-only Fleet
/// that withheld it would leave every uncovered call refused with nobody asked,
/// which is the state the issue exists to end. What comes back through it is a
/// person's own answer, so it grants nothing this predicate could be guarding.
#[test]
fn the_permission_tool_is_offered_however_far_this_machine_lets_helm_act() {
    let tool = HELM_ONLY
        .iter()
        .find(|row| row.operation == ipc::door::ASKS_A_PERSON)
        .expect("the inventory offers the permission tool to a Helm session");

    assert_eq!(tool.kind, "command");
    assert!(may(Authority::Acting, tool));
    assert!(may(Authority::ReadOnly, tool));
}


#[test]
fn a_read_only_helm_may_read_and_nothing_else() {
    for row in REACHABLE {
        assert_eq!(may(Authority::ReadOnly, row), row.kind == "query");
        if row.kind == "query" {
            assert!(may(Authority::Acting, row), "`{}` is a read", row.operation);
        }
    }
}

/// `docs/concepts/studio.md`, *Helm on a Studio*: what Helm may do unasked is
/// commands, offered to a Helm session and to no other agent, that an acting
/// Helm may call and a read-only one may not. **Named in the note from
/// [`UNASKED`]**, so a call renamed in the inventory fails here first.
#[test]
fn every_unasked_call_is_a_command_offered_to_helm_alone() {
    let note = brief(&a_manifest(), Authority::Acting, None);
    for call in UNASKED {
        let row = HELM_ONLY
            .iter()
            .find(|row| row.operation == *call)
            .unwrap_or_else(|| panic!("`{call}` does not read `Helm only`"));
        assert_eq!(row.kind, "command", "`{call}`");
        assert!(may(Authority::Acting, row), "`{call}`");
        assert!(!may(Authority::ReadOnly, row), "`{call}`");
        assert!(note.as_str().contains(call), "the note names `{call}`");
        assert!(
            !REACHABLE.iter().any(|row| row.operation == *call),
            "`{call}` reaches an agent that is not Helm"
        );
    }
}

/// **Every act on a Studio Helm is offered is decided one way or the other.**
/// A command a later step adds — a Run node (`#1289`), a write-up or a dispatch
/// (`#1291`), a scout (`#1292`) — lands here: in [`UNASKED`] if `studio.md`
/// lets Helm take it unasked, or in `ON_AN_ASK` below if it waits for one, and
/// the note's last sentence already covers it.
#[test]
fn every_studio_act_offered_to_helm_is_unasked_or_waits_for_an_ask() {
    // `#1289`: starting a run spends and changes files, so it waits for an
    // ask, as a scout does — `studio.md`'s table puts both in the asked column.
    // `#1291`: writing up and dispatching are the two it names beside them.
    // `#1345`: a server is a run held rather than one that ends, and it spends
    // and holds a port, so it waits for an ask exactly as a run does.
    const ON_AN_ASK: &[&str] = &[
        "start_scout",
        "start_studio_run",
        "start_studio_server",
        "write_up_studio_node",
        "dispatch_studio_draft",
    ];
    for row in REACHABLE
        .iter()
        .chain(HELM_ONLY)
        .filter(|row| row.kind == "command" && row.operation.contains("studio"))
    {
        assert!(
            UNASKED.contains(&row.operation) || ON_AN_ASK.contains(&row.operation),
            "`{}` is offered to Helm and nothing says whether it may be called unasked",
            row.operation
        );
    }
    for asked in ON_AN_ASK {
        assert!(
            !UNASKED.contains(asked),
            "`{asked}` is in both columns at once"
        );
    }
}

/// **Helm reads the runs it can start.** `start_checkout_run` is Helm's on an
/// ask, and its end arrives on a stream a session never receives, so the reads
/// that report it are offered to the same session.
#[test]
fn helm_reads_the_checkout_runs_it_can_start() {
    let offered = |operation: &str| {
        REACHABLE
            .iter()
            .chain(HELM_ONLY)
            .find(|row| row.operation == operation)
    };
    assert!(offered("start_checkout_run").is_some());
    for read in [
        "list_checkout_runs",
        "get_checkout_run_sheet",
        "get_checkout_run_output",
    ] {
        let row = offered(read).unwrap_or_else(|| panic!("`{read}` is offered to no Helm"));
        assert_eq!(row.kind, "query", "`{read}`");
        assert!(may(Authority::ReadOnly, row), "`{read}`");
    }
}


/// **Helm passes words to a live Session on the person's ask.** `tell_session` is offered to a Helm
/// session alone, an acting Helm may call it and a read-only one may not, and the acting note
/// says so where the read-only one does not. It is not destructive, pushes nothing and writes
/// nothing off this machine, so it is in none of `What Helm asks about`'s classes and needs no card.
#[test]
fn telling_a_session_is_an_acting_helms_alone_and_the_note_says_so() {
    let row = HELM_ONLY
        .iter()
        .find(|row| row.operation == "tell_session")
        .expect("`tell_session` reads `Helm only`");
    assert_eq!(row.kind, "command");
    assert!(may(Authority::Acting, row));
    assert!(!may(Authority::ReadOnly, row));
    assert!(
        !REACHABLE
            .iter()
            .any(|row| row.operation == "tell_session"),
        "it reaches an agent that is not Helm"
    );
    assert!(brief(&a_manifest(), Authority::Acting, None)
        .as_str()
        .contains("call tell_session"));
    assert!(!brief(&a_manifest(), Authority::ReadOnly, None)
        .as_str()
        .contains("tell_session"));
}
