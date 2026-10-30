//! A Session as an agent lists it. `docs/practices/protocol.md`, *An agent lists
//! Sessions in a line each*.

use std::collections::BTreeMap;

use crate::{
    decode, encode, Attachment, AttachmentState, HostedFacts, Instant, ManifestId, SessionId,
    SessionList, SessionMode, SessionOrigin, SessionRecord, SessionState, SessionSummaries,
    SessionTurn, SessionUsage, SessionView, TurnState,
};

fn at() -> Instant {
    Instant::carried("2026-10-13T09:00:00.000Z")
}

fn held(kind: &str, target: &str, state: AttachmentState, detail: &[(&str, &str)]) -> Attachment {
    Attachment {
        kind: kind.to_string(),
        manifest_id: Some(ManifestId::carried("armada")),
        target: target.to_string(),
        state,
        detail: detail
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>(),
        since: at(),
        changed_at: at(),
    }
}

fn session(attachments: Vec<Attachment>) -> SessionRecord {
    SessionRecord {
        id: SessionId::carried("s1"),
        harness: "a-harness".to_string(),
        origin: SessionOrigin::Terminal,
        manifest_id: Some(ManifestId::carried("armada")),
        cwd: "/work/armada".to_string(),
        title: Some("Main is red".to_string()),
        state: SessionState::Live,
        started_at: at(),
        last_seen_at: at(),
        last_turn_at: Some(at()),
        ended_at: None,
        end_reason: None,
        usage: SessionUsage::default(),
        attachments,
        hosted: None,
        terminal: None,
        mod_out_of_date: false,
        tools_missing: None,
        waiting_for: Vec::new(),
        done_since: None,
        standing: None,
        held_until: None,
        context_breakdown: None,
        loadout_summary: None,
        helm: false,
        r#mod: false,
        auto_land: false,
        activity: None,
        build: None,
        workflow: None,
        proposal: None,
        plan: None,
        touches: Vec::new(),
        workflow_step: None,
        judging: false,
        judge: None,
        settled: Vec::new(),
        closed: Vec::new(),
        waits: None,
        later: Vec::new(),
        held_open_by: None,
        started: None,
        workflow_visits: Vec::new(),
        skipped_steps: Vec::new(),
        workflow_waiting_on: Vec::new(),
        workflow_copy: None,
    }
}

/// **The line says what is held now.** A merged pull request and a slot given back are the
/// ledger's history, which `get_session` reads.
#[test]
fn a_summary_holds_what_is_standing_and_none_of_the_history() {
    let record = session(vec![
        held("slot", "3", AttachmentState::Standing, &[]),
        held("slot", "2", AttachmentState::GivenBack, &[]),
        held("branch", "helm/sees-sessions", AttachmentState::Standing, &[]),
        held("pr", "2201", AttachmentState::Spent, &[("title", "Old"), ("state", "merged")]),
        held(
            "pr",
            "2230",
            AttachmentState::Standing,
            &[("title", "Say main is red"), ("state", "open"), ("body", "x".repeat(5_000).as_str())],
        ),
        held("subagent", "a1", AttachmentState::Standing, &[]),
    ]);
    let line = &SessionSummaries::of(&SessionList { sessions: vec![record] }).sessions[0];
    assert_eq!(line.slot.as_deref(), Some("3"));
    assert_eq!(line.branch.as_deref(), Some("helm/sees-sessions"));
    assert_eq!(line.pull_requests.len(), 1);
    assert_eq!(line.pull_requests[0].number, 2230);
    assert_eq!(line.pull_requests[0].title, "Say main is red");
    assert_eq!(line.pull_requests[0].state.as_deref(), Some("open"));
    assert_eq!(line.turn, None, "a terminal Session reports no turn start");
    let wire = encode(line).expect("a summary encodes");
    assert!(!wire.contains("xxxx"), "a pull request's body reached the line: {wire}");
}

#[test]
fn a_hosted_sessions_turn_is_working_or_idle() {
    let hosted = |turn| HostedFacts {
        turn,
        asked: None,
        model: None,
        resolved_model: None,
        effort: None,
        mode: SessionMode::Auto,
        running: true,
        commands: Vec::new(),
        paused: None,
        start: None,
        start_failed: None,
    };
    let mut record = session(Vec::new());
    record.origin = SessionOrigin::Bridge;
    record.hosted = Some(hosted(SessionTurn::Working { woken_by: None }));
    assert_eq!(SessionSummaries::of(&SessionList { sessions: vec![record.clone()] }).sessions[0].turn, Some(TurnState::Working));
    record.hosted = Some(hosted(SessionTurn::Idle));
    assert_eq!(SessionSummaries::of(&SessionList { sessions: vec![record] }).sessions[0].turn, Some(TurnState::Idle));
}

#[test]
fn a_title_the_length_of_a_prompt_is_cut_and_a_short_one_is_not() {
    let mut record = session(Vec::new());
    record.title = Some("é".repeat(500));
    let cut = SessionSummaries::of(&SessionList { sessions: vec![record] });
    let title = cut.sessions[0].title.as_deref().expect("a title");
    assert_eq!(title.chars().count(), 161);
    assert!(title.ends_with('…'));
    let short = SessionSummaries::of(&SessionList { sessions: vec![session(Vec::new())] });
    assert_eq!(short.sessions[0].title.as_deref(), Some("Main is red"));
}

#[test]
fn a_summary_round_trips_and_leaves_an_empty_field_out() {
    let list = SessionSummaries::of(&SessionList {
        sessions: vec![session(vec![held("slot", "3", AttachmentState::Standing, &[])])],
    });
    let wire = encode(&list).expect("a list encodes");
    assert!(!wire.contains("pull_requests") && !wire.contains("\"jobs\"") && !wire.contains("null"), "{wire}");
    let back: SessionSummaries = decode("a summary list", wire.as_bytes()).expect("a list decodes");
    assert_eq!(back, list);
}

/// The route's default is the wide row: Bridge and Pocket send no `view`.
#[test]
fn a_view_that_is_not_named_is_the_whole_record() {
    assert_eq!(SessionView::default(), SessionView::Full);
    let named: SessionView = decode("a view", br#""summary""#).expect("a view decodes");
    assert_eq!(named, SessionView::Summary);
}
