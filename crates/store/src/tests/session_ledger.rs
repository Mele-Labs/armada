//! The session ledger: a session survives a reopen, an attachment has one state
//! vocabulary whatever its kind, and the search finds a session by what it holds.

use std::collections::BTreeMap;

use crate::migrations::MIGRATIONS;
use crate::tests::{open, TempDir};
use crate::{
    AttachmentState, Holder, KeptAttachment, KeptSession, SessionFigures,
    SessionSearch, SessionState, Store,
};

const VERSIONS: &str = "session_ledger.versions";
/// Reads the versions' table, so a build without that table has none of it either.
const ORIGINS: &str = "session_ledger.version_origins";

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
        mod_version: None,
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
fn the_search_finds_a_session_by_pull_request_branch_slot_and_title() {
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

/// A non-exclusive kind keeps a holder's other rows, and its order is when each
/// was taken, then the holder's id: not the order the ledger answers who holds a
/// thing in.
#[test]
fn rows_stand_in_the_order_they_were_taken_and_a_second_does_not_replace_the_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let held_by = Holder::session("session-b");
    let session = Holder::session("session-a");
    let take = |store: &mut crate::Store, holder: &Holder, target: &str, at: &str| {
        store
            .attach(&attachment(holder.clone(), "pr", target, at), false)
            .expect("attached")
    };
    take(
        &mut store,
        &held_by,
        "41",
        "2026-10-07T09:00:02.000Z",
    );
    take(
        &mut store,
        &session,
        "41",
        "2026-10-07T09:00:01.000Z",
    );
    // One holder holds two at once.
    take(
        &mut store,
        &held_by,
        "42",
        "2026-10-07T09:00:03.000Z",
    );

    let standing = store
        .standing_in_order("pr", "armada", Some("41"))
        .expect("reads");
    let order: Vec<&str> = standing.iter().map(|row| row.holder.id.as_str()).collect();
    assert_eq!(order, ["session-a", "session-b"]);
    assert_eq!(
        store
            .standing_in_order("pr", "armada", None)
            .expect("reads")
            .len(),
        3,
        "a second target is not a replacement"
    );

    // Equal instants fall to the holder's id.
    take(
        &mut store,
        &Holder::session("session-0"),
        "43",
        "2026-10-07T09:00:09.000Z",
    );
    take(
        &mut store,
        &Holder::session("session-1"),
        "43",
        "2026-10-07T09:00:09.000Z",
    );
    let tied = store
        .standing_in_order("pr", "armada", Some("43"))
        .expect("reads");
    assert_eq!(tied[0].holder.id, "session-0");
}

#[test]
fn what_a_holder_held_is_spent_or_given_back_in_one_move() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let held_by = Holder::session("session-1");
    for target in ["41", "42"] {
        store
            .attach(
                &attachment(held_by.clone(), "pr", target, "2026-10-07T09:00:00.000Z"),
                false,
            )
            .expect("attached");
    }
    store
        .attach(
            &attachment(held_by.clone(), "slot", "3", "2026-10-07T09:00:00.000Z"),
            true,
        )
        .expect("attached");

    assert!(store
        .settle_held(
            &held_by,
            Some("pr"),
            AttachmentState::Spent,
            "2026-10-07T10:00:00.000Z"
        )
        .expect("settled"));
    let held = store.attachments_of(&held_by).expect("reads");
    assert!(held
        .iter()
        .filter(|row| row.kind == "pr")
        .all(|row| row.state == AttachmentState::Spent));
    assert!(
        held.iter()
            .any(|row| row.kind == "slot" && row.state == AttachmentState::Standing),
        "a kind that was not named stays"
    );
    assert!(!store
        .settle_held(
            &held_by,
            Some("pr"),
            AttachmentState::Spent,
            "2026-10-07T11:00:00.000Z"
        )
        .expect("settled"));
}

#[test]
fn only_an_ended_session_that_was_never_anything_is_forgotten_as_blank() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let blank = |id: &str| KeptSession {
        manifest_id: None,
        cwd: String::new(),
        title: None,
        state: SessionState::Ended,
        ..session(id)
    };
    store.keep_session(&blank("blank")).expect("kept");
    store
        .keep_session(&KeptSession {
            state: SessionState::Live,
            ..blank("live")
        })
        .expect("kept");
    store
        .keep_session(&KeptSession {
            title: Some("named".into()),
            ..blank("titled")
        })
        .expect("kept");
    store
        .keep_session(&KeptSession {
            last_turn_at: Some("2026-10-06T09:01:00.000Z".into()),
            ..blank("turned")
        })
        .expect("kept");
    store.keep_session(&blank("holding")).expect("kept");
    store
        .attach(&attachment(Holder::session("holding"), "branch", "main", "t1"), false)
        .expect("held");

    assert_eq!(store.forget_blank_sessions().expect("forgets"), 1);
    assert_eq!(store.session("blank").expect("reads"), None);
    for kept in ["live", "titled", "turned", "holding"] {
        assert!(store.session(kept).expect("reads").is_some(), "{kept} is kept");
    }
}

pub(super) fn hosting(session_id: &str, manifest: &str, helm: bool) -> crate::KeptHosting {
    crate::KeptHosting {
        session_id: session_id.into(),
        manifest_id: manifest.into(),
        model: Some("opus".into()),
        effort: None,
        mode: "auto".into(),
        ran: false,
        lease_slot: None,
        lease_branch: None,
        fork_of: None,
        helm,
        setup: false,
        workflow: None,
        proposed: String::new(),
        turn_model: None,
        turn_effort: None,
        paused_at: None,
        paused_resuming: false,
        auto_land: false,
        mod_job: false,
        start_held: false,
        workflow_chosen: false,
    }
}

/// A repository's Helm Session is found by its repository, live; an ended one is not live, but is
/// the latest, so the next starts on what it was tuned to; and an ordinary hosted Session is never
/// either.
#[test]
fn a_repositorys_helm_session_is_found_live_and_the_latest_is_found_ended() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (id, at, state) in [
        ("old", "2026-10-13T09:00:00.000Z", SessionState::Ended),
        ("new", "2026-10-13T10:00:00.000Z", SessionState::Live),
        ("plain", "2026-10-13T11:00:00.000Z", SessionState::Live),
        ("other", "2026-10-13T12:00:00.000Z", SessionState::Live),
    ] {
        let mut kept = session(id);
        kept.origin = "bridge".into();
        kept.started_at = at.into();
        kept.state = state;
        store.keep_session(&kept).expect("kept");
    }
    store.keep_hosting(&hosting("old", "armada", true)).expect("kept");
    store.keep_hosting(&hosting("new", "armada", true)).expect("kept");
    store.keep_hosting(&hosting("plain", "armada", false)).expect("kept");
    store.keep_hosting(&hosting("other", "elsewhere", true)).expect("kept");

    let live = store.live_helm_of("armada").expect("read").expect("one is live");
    assert_eq!(live.session_id, "new");
    assert!(live.helm);
    assert_eq!(store.live_helm_of("elsewhere").expect("read").expect("its own").session_id, "other");
    assert!(store.live_helm_of("nowhere").expect("read").is_none());

    // Ended, it is not live, and it is still the latest.
    let mut ended = session("new");
    ended.origin = "bridge".into();
    ended.started_at = "2026-10-13T10:00:00.000Z".into();
    ended.state = SessionState::Ended;
    store.keep_session(&ended).expect("kept");
    assert!(store.live_helm_of("armada").expect("read").is_none());
    let latest = store.latest_helm_of("armada").expect("read").expect("the newest");
    assert_eq!((latest.session_id.as_str(), latest.model.as_deref()), ("new", Some("opus")));
}

fn story(store: &Store, holder: &Holder) -> Vec<(String, String, AttachmentState, String)> {
    store
        .attachment_versions(holder)
        .expect("reads")
        .into_iter()
        .map(|v| (v.kind, v.target, v.state, v.at))
        .collect()
}

#[test]
fn an_artifact_keeps_each_version_it_stood_in_and_a_repeat_adds_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let holder = Holder::session("s1");
    let mut file = attachment(holder.clone(), "artifact", "notes.md", "t1");
    store.attach(&file, false).expect("taken");
    file.detail = BTreeMap::from([("lines".to_string(), "40".to_string())]);
    file.changed_at = "t2".into();
    assert!(store.attach(&file, false).expect("taken"));
    assert!(store
        .give_back(&holder, Some("artifact"), "t3")
        .expect("given back"));

    let versions = store.attachment_versions(&holder).expect("reads");
    let seen: Vec<_> = versions.iter().map(|v| (v.state, v.at.as_str())).collect();
    assert_eq!(
        seen,
        [
            (AttachmentState::Standing, "t1"),
            (AttachmentState::Standing, "t2"),
            (AttachmentState::GivenBack, "t3")
        ]
    );
    assert!(versions[0].detail.is_empty());
    assert_eq!(versions[1].detail.get("lines").map(String::as_str), Some("40"));

    // The same detail and state again changes the row not at all, so adds no version.
    let mut again = file.clone();
    again.state = AttachmentState::GivenBack;
    again.changed_at = "t4".into();
    assert!(!store.attach(&again, false).expect("taken"));
    assert_eq!(store.attachment_versions(&holder).expect("reads").len(), 3);
}

#[test]
fn every_writer_that_moves_a_row_leaves_a_version_and_one_that_moves_nothing_leaves_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let holder = Holder::session("s1");
    let other = Holder::session("s2");
    store
        .attach(&attachment(holder.clone(), "pr", "41", "t1"), false)
        .expect("taken");
    store
        .attach(&attachment(holder.clone(), "slot", "3", "t1"), true)
        .expect("taken");
    store
        .attach(&attachment(holder.clone(), "subagent", "a", "t1"), false)
        .expect("taken");
    store
        .attach(&attachment(other.clone(), "pr", "41", "t1"), false)
        .expect("taken");
    // Exclusive: slot 4 gives slot 3 back.
    store
        .attach(&attachment(holder.clone(), "slot", "4", "t2"), true)
        .expect("taken");
    // Follow: the forge says the pull request merged, with a new detail.
    let pr = store.attachments_of(&holder).expect("reads")
        .into_iter()
        .find(|row| row.kind == "pr")
        .expect("the pull request");
    let merged = BTreeMap::from([("state".to_string(), "merged".to_string())]);
    store.follow(&pr, AttachmentState::Spent, &merged, "t3").expect("followed");
    // Settle one, settle all, stop a subagent: stamped again where it already reads spent.
    store
        .settle(&holder, "slot", "armada", "4", AttachmentState::Spent, "t4")
        .expect("settled");
    store.subagent_stopped(&holder, "armada", "a", "t5").expect("stopped");
    store.subagent_stopped(&holder, "armada", "a", "t6").expect("stopped");

    let read: Vec<_> = story(&store, &holder)
        .into_iter()
        .map(|(kind, target, state, at)| (kind, target, state.as_text(), at))
        .collect();
    let want = |kind: &str, target: &str, state: &'static str, at: &str| {
        (kind.to_string(), target.to_string(), state, at.to_string())
    };
    assert_eq!(
        read,
        [
            want("pr", "41", "standing", "t1"),
            want("slot", "3", "standing", "t1"),
            want("subagent", "a", "standing", "t1"),
            want("slot", "3", "given_back", "t2"),
            want("slot", "4", "standing", "t2"),
            want("pr", "41", "spent", "t3"),
            want("slot", "4", "spent", "t4"),
            want("subagent", "a", "spent", "t5"),
        ],
        "oldest first; the second stop moved only changed_at"
    );
    assert_eq!(story(&store, &other).len(), 1, "another holder's history is its own");
}

#[test]
fn a_row_kept_before_versions_were_is_seeded_with_one_version_at_its_since() {
    let dir = TempDir::new();
    let before: Vec<_> = MIGRATIONS
        .iter()
        .filter(|m| m.name != VERSIONS && m.name != ORIGINS)
        .copied()
        .collect();
    let mut old = Store::open_with(&dir.db(), &before).expect("a build before versions");
    let holder = Holder::session("s1");
    old.attach(&attachment(holder.clone(), "artifact", "notes.md", "t1"), false)
        .expect("taken");
    old.give_back(&holder, None, "t2").expect("given back");
    drop(old);

    let store = open(&dir);
    assert_eq!(
        story(&store, &holder),
        [("artifact".to_string(), "notes.md".to_string(), AttachmentState::GivenBack, "t1".to_string())]
    );
}

#[test]
fn forgetting_a_blank_session_forgets_its_versions_and_only_its_own() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let blank = KeptSession {
        manifest_id: None,
        cwd: String::new(),
        title: None,
        state: SessionState::Ended,
        ..session("blank")
    };
    store.keep_session(&blank).expect("kept");
    // A version whose row is gone: blank means nothing is held, so only a leftover can be here.
    for holder_id in ["blank", "elsewhere"] {
        store.keep_session_message(holder_id, "m1").expect("a focus");
        store
            .conn
            .execute(
                "INSERT INTO ledger_attachment_versions
                 (holder_kind, holder_id, kind, manifest_id, target, state, detail, at)
                 VALUES ('session', ?1, 'artifact', '', 'x', 'standing', '{}', 't1')",
                [holder_id],
            )
            .expect("a leftover");
    }

    assert_eq!(store.forget_blank_sessions().expect("forgets"), 1);
    assert!(story(&store, &Holder::session("blank")).is_empty());
    assert_eq!(story(&store, &Holder::session("elsewhere")).len(), 1);
    let kept = |table: &str| -> i64 {
        store.conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)).expect("a count")
    };
    assert_eq!(kept("ledger_attachment_version_origins"), 1, "where its versions came from goes with them");
    assert_eq!(kept("session_focus"), 1);
}
