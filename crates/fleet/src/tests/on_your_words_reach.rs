//! What a Job on your words may call through the agent door: the denylist `#1150` replaced `#73`'s
//! allowlist with, checked against the door's own tables.

use ipc::door::{ON_YOUR_WORDS, REACHABLE};

use crate::on_your_words::{may, Authority, RESERVED, UNASKED};

/// Acts the door offers that stay a person's, independent of the ruling that
/// granted a Job on your words the rest (`#1150`). Mirrors [`RESERVED`] so a test failure
/// reads as this list going stale rather than as the const under test.
const THE_PERSONS: &[&str] = RESERVED;

/// A command marked `Yes` is a Job's under `Acting` unless `THE_PERSONS` names
/// it — the denylist's whole shape, checked against the door's own set rather
/// than asserted about `RESERVED` alone.
#[test]
fn every_act_the_door_offers_is_decided_for_a_job_or_the_person() {
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

/// A name reserved from a Job on your words has to be a command the door actually offers —
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
            "`{act}` is not one `Yes` offers, so reserving it from a Job on your words reserves nothing"
        );
    }
}

/// The door offers `On your words` rows to a Job on your words alone, so an acting one
/// must be allowed every row of it, and a read-only one only its reads —
/// the checkout's runs (`#1288`). `#1150`.
#[test]
fn on_your_words_is_an_acting_authoritys_and_only_its_reads_a_read_only_ones() {
    assert!(
        !ON_YOUR_WORDS.is_empty(),
        "the inventory has rows reading `On your words`"
    );
    for row in ON_YOUR_WORDS {
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
/// It is a `On your words` command by shape — it is a POST — and a read-only Fleet
/// that withheld it would leave every uncovered call refused with nobody asked,
/// which is the state the issue exists to end. What comes back through it is a
/// person's own answer, so it grants nothing this predicate could be guarding.
#[test]
fn the_permission_tool_is_offered_however_far_this_machine_lets_a_job_act() {
    let tool = ON_YOUR_WORDS
        .iter()
        .find(|row| row.operation == ipc::door::ASKS_A_PERSON)
        .expect("the inventory offers the permission tool to Job on your words");

    assert_eq!(tool.kind, "command");
    assert!(may(Authority::Acting, tool));
    assert!(may(Authority::ReadOnly, tool));
}


#[test]
fn a_read_only_authority_may_read_and_nothing_else() {
    for row in REACHABLE {
        assert_eq!(may(Authority::ReadOnly, row), row.kind == "query");
        if row.kind == "query" {
            assert!(may(Authority::Acting, row), "`{}` is a read", row.operation);
        }
    }
}

/// What a Job on your words may do on a Studio unasked is commands, offered to it and to no other
/// agent, that an acting authority may call and a read-only one may not. **Named from [`UNASKED`]**,
/// so a call renamed in the inventory fails here first.
#[test]
fn every_unasked_call_is_a_command_offered_to_a_job_on_your_words_alone() {
    for call in UNASKED {
        let row = ON_YOUR_WORDS
            .iter()
            .find(|row| row.operation == *call)
            .unwrap_or_else(|| panic!("`{call}` does not read `On your words`"));
        assert_eq!(row.kind, "command", "`{call}`");
        assert!(may(Authority::Acting, row), "`{call}`");
        assert!(!may(Authority::ReadOnly, row), "`{call}`");
        assert!(
            !REACHABLE.iter().any(|row| row.operation == *call),
            "`{call}` reaches an agent that is not a Job on your words"
        );
    }
}

/// **Every act on a Studio a Job on your words is offered is decided one way or the other.**
/// A command a later step adds — a Run node (`#1289`), a write-up or a dispatch
/// (`#1291`), a scout (`#1292`) — lands here: in [`UNASKED`] if `studio.md`
/// lets a Job on your words take it unasked, or in `ON_AN_ASK` below if it waits for one, and
/// the note's last sentence already covers it.
#[test]
fn every_studio_act_offered_to_a_job_is_unasked_or_waits_for_an_ask() {
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
        .chain(ON_YOUR_WORDS)
        .filter(|row| row.kind == "command" && row.operation.contains("studio"))
    {
        assert!(
            UNASKED.contains(&row.operation) || ON_AN_ASK.contains(&row.operation),
            "`{}` is offered to a Job on your words and nothing says whether it may be called unasked",
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

/// **A Job on your words reads the runs it can start.** `start_checkout_run` is a Job's on an
/// ask, and its end arrives on a stream a session never receives, so the reads
/// that report it are offered to the same session.
#[test]
fn a_job_reads_the_checkout_runs_it_can_start() {
    let offered = |operation: &str| {
        REACHABLE
            .iter()
            .chain(ON_YOUR_WORDS)
            .find(|row| row.operation == operation)
    };
    assert!(offered("start_checkout_run").is_some());
    for read in [
        "list_checkout_runs",
        "get_checkout_run_sheet",
        "get_checkout_run_output",
    ] {
        let row = offered(read).unwrap_or_else(|| panic!("`{read}` is offered to no Job on your words"));
        assert_eq!(row.kind, "query", "`{read}`");
        assert!(may(Authority::ReadOnly, row), "`{read}`");
    }
}


/// **A Job on your words passes words to a live Job on the person's ask.** `tell_session` is offered
/// to a Job on your words alone, and an acting authority may call it and a read-only one may not.
/// It is not destructive, pushes nothing and writes nothing off this machine, so it needs no card.
#[test]
fn telling_a_session_is_an_acting_jobs_alone() {
    let row = ON_YOUR_WORDS
        .iter()
        .find(|row| row.operation == "tell_session")
        .expect("`tell_session` reads `On your words`");
    assert_eq!(row.kind, "command");
    assert!(may(Authority::Acting, row));
    assert!(!may(Authority::ReadOnly, row));
    assert!(
        !REACHABLE
            .iter()
            .any(|row| row.operation == "tell_session"),
        "it reaches an agent that is not a Job on your words"
    );
}
