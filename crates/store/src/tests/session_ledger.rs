//! The session ledger: a session survives a reopen, an attachment has one state
//! vocabulary whatever its kind, and the search finds a session by what it holds.

use std::collections::BTreeMap;

use crate::tests::{open, TempDir};
use crate::{
    AttachmentState, Holder, HolderKind, KeptAttachment, KeptSession, SessionFigures,
    SessionSearch, SessionState,
};

fn session(id: &str) -> KeptSession {
    KeptSession {
        id: id.into(),
        harness: "a_harness".into(),
        origin: "terminal".into(),
        manifest_id: Some("armada".into()),
        cwd: "/repos/armada".into(),
        title: Some("fix the ledger".into()),
        state: SessionState::Live,
        started_at: "2026-10-06T09:00:00.000Z".into(),
        last_seen_at: "2026-10-06T09:00:00.000Z".into(),
        last_turn_at: None,
        ended_at: None,
        end_reason: None,
        figures: SessionFigures::default(),
    }
}

fn attachment(holder: Holder, kind: &str, target: &str, at: &str) -> KeptAttachment {
    KeptAttachment {
        holder,
        kind: kind.into(),
        manifest_id: "armada".into(),
        target: target.into(),
        state: AttachmentState::Standing,
        detail: BTreeMap::new(),
        since: at.into(),
        changed_at: at.into(),
    }
}

#[test]
fn a_session_survives_a_reopen_with_its_figures() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut kept = session("s1");
    kept.figures = SessionFigures {
        context_tokens: Some(41_000),
        context_window: Some(200_000),
        cost_micros: Some(1_250_000),
    };
    store.keep_session(&kept).expect("kept");
    drop(store);

    let store = open(&dir);
    assert_eq!(store.session("s1").expect("reads"), Some(kept));
    assert_eq!(store.session("nobody").expect("reads"), None);
}

#[test]
fn a_holder_that_is_a_job_keeps_rows_beside_a_session_that_shares_its_id() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = Holder {
        kind: HolderKind::Job,
        id: "x".into(),
    };
    let by_session = Holder::session("x");
    store
        .attach(&attachment(job.clone(), "slot", "2", "t1"), false)
        .expect("job");
    store
        .attach(&attachment(by_session.clone(), "slot", "3", "t1"), false)
        .expect("session");

    assert_eq!(store.attachments_of(&job).expect("reads").len(), 1);
    assert_eq!(store.attachments_of(&by_session).expect("reads").len(), 1);
    let at_two = store.attachments_at("slot", "2", None).expect("reads");
    assert_eq!(at_two.len(), 1);
    assert_eq!(at_two[0].holder, job);
}

#[test]
fn taking_what_is_held_already_changes_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let holder = Holder::session("s1");
    let first = attachment(holder.clone(), "branch", "fleet/a", "t1");
    assert!(store.attach(&first, true).expect("taken"));
    let again = attachment(holder.clone(), "branch", "fleet/a", "t2");
    assert!(!store.attach(&again, true).expect("taken"));

    let held = store.attachments_of(&holder).expect("reads");
    assert_eq!(held[0].changed_at, "t1");
}

#[test]
fn an_exclusive_kind_gives_back_the_one_it_replaces() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let holder = Holder::session("s1");
    store
        .attach(&attachment(holder.clone(), "branch", "fleet/a", "t1"), true)
        .expect("a");
    store
        .attach(&attachment(holder.clone(), "branch", "fleet/b", "t2"), true)
        .expect("b");
    store
        .attach(&attachment(holder.clone(), "pr", "12", "t2"), true)
        .expect("a pull request is not a branch");

    let state_of = |target: &str| {
        store
            .attachments_of(&holder)
            .expect("reads")
            .into_iter()
            .find(|one| one.target == target)
            .map(|one| one.state)
    };
    assert_eq!(state_of("fleet/a"), Some(AttachmentState::GivenBack));
    assert_eq!(state_of("fleet/b"), Some(AttachmentState::Standing));
    assert_eq!(state_of("12"), Some(AttachmentState::Standing));
}

#[test]
fn a_session_ending_gives_back_what_it_stood_on_and_a_spent_row_stays_spent() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let holder = Holder::session("s1");
    store
        .attach(&attachment(holder.clone(), "slot", "3", "t1"), true)
        .expect("slot");
    store
        .attach(&attachment(holder.clone(), "pr", "12", "t1"), false)
        .expect("pr");
    assert!(store
        .settle(&holder, "pr", "armada", "12", AttachmentState::Spent, "t2")
        .expect("settled"));
    assert!(store.give_back(&holder, None, "t3").expect("given back"));

    let held = store.attachments_of(&holder).expect("reads");
    let slot = held.iter().find(|one| one.kind == "slot").expect("slot");
    let pr = held.iter().find(|one| one.kind == "pr").expect("pr");
    assert_eq!(slot.state, AttachmentState::GivenBack);
    assert_eq!(pr.state, AttachmentState::Spent);
    assert!(!store.give_back(&holder, None, "t4").expect("nothing left"));
}

#[test]
fn who_holds_a_target_lists_standing_holders_before_the_ones_that_let_go() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let old = Holder::session("old");
    let new = Holder::session("new");
    store
        .attach(&attachment(old.clone(), "branch", "fleet/a", "t1"), true)
        .expect("old");
    store.give_back(&old, None, "t2").expect("let go");
    store
        .attach(&attachment(new.clone(), "branch", "fleet/a", "t3"), true)
        .expect("new");

    let holders = store
        .attachments_at("branch", "fleet/a", Some("armada"))
        .expect("reads");
    let order: Vec<(&str, AttachmentState)> = holders
        .iter()
        .map(|one| (one.holder.id.as_str(), one.state))
        .collect();
    assert_eq!(
        order,
        [
            ("new", AttachmentState::Standing),
            ("old", AttachmentState::GivenBack)
        ]
    );
    assert!(store
        .attachments_at("branch", "fleet/a", Some("elsewhere"))
        .expect("reads")
        .is_empty());
}

#[test]
fn the_search_finds_a_session_by_pull_request_branch_job_slot_and_title() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut left = session("left");
    left.title = Some("Ledger intake".into());
    let right = session("right");
    store.keep_session(&left).expect("left");
    store.keep_session(&right).expect("right");
    for (holder, kind, target) in [
        ("left", "pr", "112"),
        ("left", "branch", "fleet/session-ledger"),
        ("left", "job", "01JOBAAAA"),
        ("left", "slot", "3"),
        ("right", "pr", "12"),
    ] {
        store
            .attach(
                &attachment(Holder::session(holder), kind, target, "t1"),
                false,
            )
            .expect("taken");
    }

    let ids = |text: &str| -> Vec<String> {
        store
            .find_sessions(&SessionSearch {
                text: Some(text),
                ..SessionSearch::default()
            })
            .expect("searches")
            .into_iter()
            .map(|one| one.id)
            .collect()
    };
    assert_eq!(ids("#112"), ["left"], "a pull request number is whole");
    assert_eq!(ids("12"), ["right"], "and 12 is not 112");
    assert_eq!(ids("session-ledger"), ["left"]);
    assert_eq!(ids("01JOB"), ["left"]);
    assert_eq!(ids("slot-3"), ["left"]);
    assert_eq!(ids("ledger intake"), ["left"], "a title, in any case");
    assert_eq!(ids("100%"), Vec::<String>::new(), "a % is not a wildcard");
}

#[test]
fn the_search_narrows_by_repository_and_by_state() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut ended = session("ended");
    ended.state = SessionState::Ended;
    let mut elsewhere = session("elsewhere");
    elsewhere.manifest_id = Some("other".into());
    for kept in [ended, elsewhere, session("live")] {
        store.keep_session(&kept).expect("kept");
    }
    let found = |search: SessionSearch<'_>| -> Vec<String> {
        store
            .find_sessions(&search)
            .expect("searches")
            .into_iter()
            .map(|one| one.id)
            .collect()
    };
    assert_eq!(
        found(SessionSearch {
            manifest_id: Some("armada"),
            state: Some(SessionState::Live),
            ..SessionSearch::default()
        }),
        ["live"]
    );
    assert_eq!(found(SessionSearch::default()).len(), 3);
}

/// A need is not exclusive, and its order is when it was taken, then the
/// holder's id: not the order the ledger answers who holds a thing in.
#[test]
fn needs_stand_in_the_order_they_were_taken_and_are_not_exclusive() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = Holder::job("job-b");
    let session = Holder::session("session-a");
    let take = |store: &mut crate::Store, holder: &Holder, target: &str, at: &str| {
        store
            .attach(&attachment(holder.clone(), "need", target, at), false)
            .expect("attached")
    };
    take(
        &mut store,
        &job,
        "protocol-version.toml",
        "2026-10-07T09:00:02.000Z",
    );
    take(
        &mut store,
        &session,
        "protocol-version.toml",
        "2026-10-07T09:00:01.000Z",
    );
    // One holder waits on two paths at once.
    take(
        &mut store,
        &job,
        "docs/INDEX.md",
        "2026-10-07T09:00:03.000Z",
    );

    let standing = store
        .standing_in_order("need", "armada", Some("protocol-version.toml"))
        .expect("reads");
    let order: Vec<&str> = standing.iter().map(|row| row.holder.id.as_str()).collect();
    assert_eq!(order, ["session-a", "job-b"]);
    assert_eq!(
        store
            .standing_in_order("need", "armada", None)
            .expect("reads")
            .len(),
        3,
        "a second path is not a replacement"
    );

    // Equal instants fall to the holder's id.
    take(
        &mut store,
        &Holder::job("job-0"),
        "x",
        "2026-10-07T09:00:09.000Z",
    );
    take(
        &mut store,
        &Holder::job("job-1"),
        "x",
        "2026-10-07T09:00:09.000Z",
    );
    let tied = store
        .standing_in_order("need", "armada", Some("x"))
        .expect("reads");
    assert_eq!(tied[0].holder.id, "job-0");
}

#[test]
fn what_a_holder_held_is_spent_or_given_back_in_one_move() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = Holder::job("job-1");
    for target in ["a", "b"] {
        store
            .attach(
                &attachment(job.clone(), "need", target, "2026-10-07T09:00:00.000Z"),
                false,
            )
            .expect("attached");
    }
    store
        .attach(
            &attachment(job.clone(), "slot", "3", "2026-10-07T09:00:00.000Z"),
            true,
        )
        .expect("attached");

    assert!(store
        .settle_held(
            &job,
            Some("need"),
            AttachmentState::Spent,
            "2026-10-07T10:00:00.000Z"
        )
        .expect("settled"));
    let held = store.attachments_of(&job).expect("reads");
    assert!(held
        .iter()
        .filter(|row| row.kind == "need")
        .all(|row| row.state == AttachmentState::Spent));
    assert!(
        held.iter()
            .any(|row| row.kind == "slot" && row.state == AttachmentState::Standing),
        "a kind that was not named stays"
    );
    assert!(!store
        .settle_held(
            &job,
            Some("need"),
            AttachmentState::Spent,
            "2026-10-07T11:00:00.000Z"
        )
        .expect("settled"));
}
