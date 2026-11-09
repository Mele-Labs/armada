//! What a Job's record and its thread's rows must keep true on the wire: the marks left off when
//! they do not apply, the `sent` row, a question option's preview and the card a call is put to a
//! person on.

use crate::{
    decode, encode, Instant, ManifestId, SessionOrigin, SessionRecord, SessionRow, SessionState, SessionTag,
    SessionUsage, TagKind, ThoughtKind, ThoughtTarget,
};

pub(super) fn a_record() -> SessionRecord {
    let at = Instant::carried("2026-10-13T09:00:00.000Z");
    SessionRecord {
        id: crate::SessionId::carried("01RECORD"),
        harness: "hosted".into(),
        origin: SessionOrigin::Bridge,
        manifest_id: Some(ManifestId::carried("armada")),
        cwd: "/repos/armada".into(),
        title: Some("A Job".into()),
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
        mod_version: None,
        tools_missing: None,
        running: Vec::new(),
        waiting_for: Vec::new(),
        done_since: None,
        done_summary: None,
        so_far: None,
        standing: None,
        held_until: None,
        context_breakdown: None,
        loadout_summary: None,
        on_your_words: false,
        r#mod: false,
        from_job: None,
        from_tab: None,
        mod_of_bay: false,
        auto_land: false,
        activity: None,
        build: None,
        workflow: None,
        proposal: None,
        plan: None,
        touches: Vec::new(),
        workflow_step: None,
        judge_readings: Vec::new(),
        closed: Vec::new(),
        waits: None,
        later: Vec::new(),
        held_open_by: Vec::new(),
        stopped: None,
        stop_told_by: None,
        turn_since: None,
        last_heard_at: None,
        started: None,
        workflow_visits: Vec::new(),
        skipped_steps: Vec::new(),
        workflow_waiting_on: Vec::new(),
        workflow_copy: None,
        in_flight: Vec::new(),
        mark: None,
        viewed_at: None,
    }
}

/// The mark is on a Job's record that is a mod of a bay and on no other.
#[test]
fn a_mod_of_a_bay_is_said_on_the_record_and_left_off_every_other() {
    let of_a_bay = SessionRecord { r#mod: true, mod_of_bay: true, ..a_record() };
    let json = encode(&of_a_bay).expect("plain data");
    assert!(json.contains(r#""mod_of_bay":true"#), "{json}");
    assert_eq!(decode::<SessionRecord>("a record", json.as_bytes()).expect("round-trips"), of_a_bay);

    let json = encode(&a_record()).expect("plain data");
    assert!(!json.contains("mod_of_bay"), "{json}");
    assert!(!decode::<SessionRecord>("a record", json.as_bytes()).expect("decodes").mod_of_bay);
}

#[test]
fn the_on_your_words_mark_is_left_off_every_record_that_is_not_on_your_words() {
    let on = SessionRecord { on_your_words: true, ..a_record() };
    let json = encode(&on).expect("plain data");
    assert!(json.contains(r#""on_your_words":true"#), "{json}");
    assert_eq!(decode::<SessionRecord>("a record", json.as_bytes()).expect("round-trips"), on);

    let json = encode(&a_record()).expect("plain data");
    assert!(!json.contains("on_your_words"), "{json}");
    assert!(!decode::<SessionRecord>("a record", json.as_bytes()).expect("decodes").on_your_words);
}

#[test]
fn a_start_says_it_is_on_your_words_only_when_it_is_and_an_older_start_still_decodes() {
    let plain = decode::<crate::StartSession>("a start", br#"{"manifest_id":"armada"}"#).expect("older start");
    assert_eq!(plain.on_your_words, None);
    let json = encode(&plain).expect("plain data");
    assert!(!json.contains("on_your_words"), "{json}");

    let on = decode::<crate::StartSession>("a start", br#"{"manifest_id":"armada","on_your_words":true}"#).expect("a start");
    assert_eq!(on.on_your_words, Some(true));
    assert!(encode(&on).expect("plain data").contains(r#""on_your_words":true"#));
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

fn a_reach_card(kind: Option<crate::CallKind>) -> crate::CallInFlight {
    crate::CallInFlight {
        call: "call-1".into(),
        manifest_id: ManifestId::carried("armada"),
        asked_at: Instant::carried("2026-10-26T10:00:00Z"),
        tool: "Bash".into(),
        detail: "node drive.mjs --out ~/Desktop/shot.png".into(),
        truncated: false,
        length: None,
        whole: None,
        rule: "Bash(node drive.mjs:*)".into(),
        offers: vec![crate::CallAnswer::AllowOnce, crate::CallAnswer::AllowLikeThisForSession],
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
    assert_eq!(decode::<crate::CallInFlight>("a card", json.as_bytes()).expect("round-trips"), card);

    let json = encode(&a_reach_card(None)).expect("plain data");
    assert!(!json.contains(r#""kind""#), "{json}");
}

/// The answer that allows calls of a kind travels under its own word, with the width chosen.
#[test]
fn allowing_calls_like_this_round_trips_with_its_kind() {
    let answer = decode::<crate::CallAnswer>("an answer", br#""allow_like_this_for_session""#).expect("a word");
    assert_eq!(answer, crate::CallAnswer::AllowLikeThisForSession);
    assert_eq!(encode(&answer).expect("plain data"), r#""allow_like_this_for_session""#);

    let said = crate::AnswerSessionAsk {
        session_id: crate::SessionId::carried("s1"),
        call: "call-1".into(),
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
