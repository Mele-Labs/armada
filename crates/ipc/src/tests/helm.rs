//! What Helm's wire must keep true: an ask names where the person is and, for the message that
//! starts the Session, what it runs on; the Session is found by `HelmSession`, which leaves a
//! Session that is not there out. A blank message is Fleet's to refuse, and `helm_box.rs` holds
//! what rides with one.

use crate::{
    decode, encode, AskHelm, HelmContext, HelmScreen, HelmSession, Instant, ManifestId,
    SessionOrigin, SessionRecord, SessionRow, SessionState, SessionTag, SessionThread, SessionUsage,
    StudioId, StudioNodeId, TagKind, ThoughtKind, ThoughtTarget,
};

#[test]
fn a_message_with_something_in_it_decodes() {
    let asked = decode::<AskHelm>("a message to Helm", br#"{"text":"why did job 12 stall?"}"#)
        .expect("plain data");
    assert_eq!(asked.text, "why did job 12 stall?");
}

/// An older Bridge asks with no `context` at all — `#1075`'s protocol minor
/// must still decode it.
#[test]
fn an_ask_with_no_context_still_decodes() {
    let asked = decode::<AskHelm>("a message to Helm", br#"{"text":"why did job 12 stall?"}"#)
        .expect("plain data");
    assert_eq!(asked.context, None);
}

#[test]
fn an_asks_context_round_trips() {
    let asked = AskHelm {
        text: "what is this stuck on?".to_string(),
        attachments: Vec::new(),
        mentions: Vec::new(),
        model: None,
        effort: None,
        context: Some(HelmContext {
            screen: HelmScreen::Overview,
            picked: Some(ManifestId::carried("armada")),
            studio: None,
            node: None,
        }),
    };
    let json = encode(&asked).expect("plain data");
    assert!(json.contains(r#""screen":"overview""#), "{json}");
    assert!(
        !json.contains("studio"),
        "an absent field is left off: {json}"
    );
    assert_eq!(
        decode::<AskHelm>("a message to Helm", json.as_bytes()).expect("round-trips"),
        asked
    );
}

/// `#1287`: a Studio, and the node selected on it, ride on the ask; a context
/// from a Bridge built before 14.7 names neither and still decodes.
#[test]
fn a_studios_context_round_trips_and_an_older_one_still_decodes() {
    let asked = AskHelm {
        text: "what does this connect to?".to_string(),
        attachments: Vec::new(),
        mentions: Vec::new(),
        model: None,
        effort: None,
        context: Some(HelmContext {
            screen: HelmScreen::Studio,
            picked: Some(ManifestId::carried("armada")),
            studio: Some(StudioId::carried("01STUDIO00000000000000000A")),
            node: Some(StudioNodeId::carried("01NODE0000000000000000000B")),
        }),
    };
    let json = encode(&asked).expect("plain data");
    assert!(json.contains(r#""screen":"studio""#), "{json}");
    assert!(
        json.contains(r#""node":"01NODE0000000000000000000B""#),
        "{json}"
    );
    assert_eq!(
        decode::<AskHelm>("a message to Helm", json.as_bytes()).expect("round-trips"),
        asked
    );

    let older = decode::<AskHelm>(
        "a message to Helm",
        br#"{"text":"hi","context":{"screen":"overview"}}"#,
    )
    .expect("an older context");
    let context = older.context.expect("a context");
    assert_eq!((context.studio, context.node), (None, None));
}

/// 23.34: the Workflow creator is a screen Helm is told about.
#[test]
fn the_workflows_screen_round_trips() {
    let asked = decode::<AskHelm>(
        "a message to Helm",
        br#"{"text":"add a step","context":{"screen":"workflows","picked":"armada"}}"#,
    )
    .expect("the workflows screen decodes");
    assert_eq!(
        asked.context.expect("a context").screen,
        HelmScreen::Workflows
    );
}

/// The tuning rides only on the message that starts the Session, and an ask without it carries
/// neither field.
#[test]
fn an_asks_tuning_is_left_out_when_absent_and_round_trips_when_set() {
    let plain = decode::<AskHelm>("a message to Helm", br#"{"text":"hi"}"#).expect("plain data");
    assert_eq!((plain.model.as_deref(), plain.effort.as_deref()), (None, None));
    assert_eq!(encode(&plain).expect("plain data"), r#"{"text":"hi"}"#);

    let tuned = decode::<AskHelm>(
        "a message to Helm",
        br#"{"text":"hi","model":"opus","effort":"high"}"#,
    )
    .expect("plain data");
    assert_eq!((tuned.model.as_deref(), tuned.effort.as_deref()), (Some("opus"), Some("high")));
    let json = encode(&tuned).expect("plain data");
    assert_eq!(decode::<AskHelm>("a message to Helm", json.as_bytes()).expect("round-trips"), tuned);
}

fn a_helm_record() -> SessionRecord {
    let at = Instant::carried("2026-10-13T09:00:00.000Z");
    SessionRecord {
        id: crate::SessionId::carried("01HELM"),
        harness: "hosted".into(),
        origin: SessionOrigin::Bridge,
        manifest_id: Some(ManifestId::carried("armada")),
        cwd: "/repos/armada".into(),
        title: Some("Helm".into()),
        state: SessionState::Live,
        started_at: at.clone(),
        last_seen_at: at,
        last_turn_at: None,
        ended_at: None,
        end_reason: None,
        usage: SessionUsage::default(),
        attachments: Vec::new(),
        hosted: None,
        terminal: None,
        mod_out_of_date: false,
        tools_missing: None,
        waiting_for: Vec::new(),
        done_since: None,
        done_summary: None,
        standing: None,
        held_until: None,
        context_breakdown: None,
        loadout_summary: None,
        helm: true,
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
        held_open_by: Vec::new(),
        started: None,
        workflow_visits: Vec::new(),
        skipped_steps: Vec::new(),
        workflow_waiting_on: Vec::new(),
        workflow_copy: None,
    }
}

/// The mark is on a Helm Session's record and on no other: an ordinary Session leaves it out.
#[test]
fn the_helm_mark_is_left_off_every_record_that_is_not_helms() {
    let helm = a_helm_record();
    let json = encode(&helm).expect("plain data");
    assert!(json.contains(r#""helm":true"#), "{json}");
    assert_eq!(decode::<SessionRecord>("a record", json.as_bytes()).expect("round-trips"), helm);

    let ordinary = SessionRecord { helm: false, ..helm };
    let json = encode(&ordinary).expect("plain data");
    assert!(!json.contains("helm"), "{json}");
    assert!(!decode::<SessionRecord>("a record", json.as_bytes()).expect("decodes").helm);
}

#[test]
fn a_helm_session_is_left_out_when_there_is_none_and_carries_its_thread_when_there_is() {
    let none = HelmSession { manifest_id: ManifestId::carried("armada"), session: None };
    let json = encode(&none).expect("plain data");
    assert_eq!(json, r#"{"manifest_id":"armada"}"#);
    assert_eq!(decode::<HelmSession>("a Helm session", json.as_bytes()).expect("round-trips"), none);

    let some = HelmSession {
        manifest_id: ManifestId::carried("armada"),
        session: Some(SessionThread { session: a_helm_record(), rows: Vec::new() }),
    };
    let json = encode(&some).expect("plain data");
    assert_eq!(decode::<HelmSession>("a Helm session", json.as_bytes()).expect("round-trips"), some);
}

/// The `sent` row is told apart by its `kind` and leaves what it did not carry out.
#[test]
fn the_sent_row_round_trips_and_leaves_empty_lists_out() {
    let target = ThoughtTarget {
        kind: ThoughtKind::Session,
        id: "01LISTVIEWS".into(),
        title: "Sessions list views".into(),
        manifest_id: Some(ManifestId::carried("armada")),
        slot: None,
    };
    let bare = SessionRow::Sent {
        id: "r1".into(),
        at: Instant::carried("2026-10-13T09:00:00.000Z"),
        text: "retries should back off".into(),
        target: target.clone(),
        files: Vec::new(),
        tags: Vec::new(),
    };
    let json = encode(&bare).expect("plain data");
    assert!(json.starts_with(r#"{"kind":"sent","id":"r1","#), "{json}");
    assert!(!json.contains("files") && !json.contains("tags") && !json.contains("null"), "{json}");
    assert_eq!(decode::<SessionRow>("a row", json.as_bytes()).expect("round-trips"), bare);

    let carrying = SessionRow::Sent {
        id: "r2".into(),
        at: Instant::carried("2026-10-13T09:00:00.000Z"),
        text: "see this".into(),
        target,
        files: vec![crate::SentFile { id: "f1".into(), name: "shot.png".into(), media_type: "image/png".into() }],
        tags: vec![SessionTag { kind: TagKind::Issue, id: "214".into(), title: "#214 The title".into(), url: None }],
    };
    let json = encode(&carrying).expect("plain data");
    assert!(json.contains(r#""files":[{"id":"f1""#) && json.contains(r#""tags":[{"kind":"issue""#), "{json}");
    assert_eq!(decode::<SessionRow>("a row", json.as_bytes()).expect("round-trips"), carrying);
    assert_eq!(carrying.id(), "r2");
}

/// The agent's own spelling of an option, preview and all, survives a parse and leaves again
/// under the same key; an option without one leaves no key.
#[test]
fn a_question_option_keeps_its_preview() {
    let tool = br#"{"question":"Use these checks?","header":"Checks","multiSelect":false,"options":[
        {"label":"Yes","description":"Run them","preview":"cargo test\npnpm test"},
        {"label":"No","description":"Skip them"}]}"#;
    let asked = decode::<crate::AskedQuestion>("a question", tool).expect("plain data");
    assert_eq!(asked.options[0].preview, "cargo test\npnpm test");
    assert_eq!(asked.options[1].preview, "");

    let json = encode(&asked).expect("plain data");
    assert!(json.contains(r#""preview":"cargo test\npnpm test""#), "{json}");
    assert_eq!(json.matches("preview").count(), 1, "no key for an option without one: {json}");
    assert_eq!(decode::<crate::AskedQuestion>("a question", json.as_bytes()).expect("round-trips"), asked);
}

fn a_reach_card(kind: Option<crate::CallKind>) -> crate::HelmCallInFlight {
    crate::HelmCallInFlight {
        call: "helm-1".into(),
        manifest_id: ManifestId::carried("armada"),
        asked_at: Instant::carried("2026-10-26T10:00:00Z"),
        tool: "Bash".into(),
        detail: "node drive.mjs --out ~/Desktop/shot.png".into(),
        truncated: false,
        length: None,
        whole: None,
        rule: "Bash(node drive.mjs:*)".into(),
        offers: vec![crate::HelmCallAnswer::AllowOnce, crate::HelmCallAnswer::AllowLikeThisForSession],
        holding_for_seconds: 600,
        questions: Vec::new(),
        reaches: Some("/Users/user/Desktop/shot.png".into()),
        reaches_to: Some(crate::ReachesTo::Write),
        kind,
    }
}

/// A reach card over a Bash call names its program and script, and a card with no kind leaves
/// the key out, so a Fleet's other cards read as they did.
#[test]
fn a_reach_cards_kind_is_sent_when_set_and_left_out_when_not() {
    let kind = crate::CallKind { program: "node".into(), script: Some("drive.mjs".into()) };
    let card = a_reach_card(Some(kind));
    let json = encode(&card).expect("plain data");
    assert!(json.contains(r#""kind":{"program":"node","script":"drive.mjs"}"#), "{json}");
    assert_eq!(decode::<crate::HelmCallInFlight>("a card", json.as_bytes()).expect("round-trips"), card);

    let json = encode(&a_reach_card(None)).expect("plain data");
    assert!(!json.contains(r#""kind""#), "{json}");
}

/// The answer that allows calls of a kind travels under its own word, with the width chosen.
#[test]
fn allowing_calls_like_this_round_trips_with_its_kind() {
    let answer = decode::<crate::HelmCallAnswer>("an answer", br#""allow_like_this_for_session""#).expect("a word");
    assert_eq!(answer, crate::HelmCallAnswer::AllowLikeThisForSession);
    assert_eq!(encode(&answer).expect("plain data"), r#""allow_like_this_for_session""#);

    let said = crate::AnswerSessionAsk {
        session_id: crate::SessionId::carried("s1"),
        call: "helm-1".into(),
        answer,
        note: None,
        answers: Vec::new(),
        folder: Some("/Users/user/Desktop".into()),
        kind: Some(crate::CallKind { program: "node".into(), script: None }),
    };
    let json = encode(&said).expect("plain data");
    assert!(json.contains(r#""kind":{"program":"node"}"#), "{json}");
    assert_eq!(decode::<crate::AnswerSessionAsk>("an answer", json.as_bytes()).expect("round-trips"), said);
}
