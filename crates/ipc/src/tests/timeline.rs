//! The timeline's spellings: the kinds Bridge matches on, and what an absent field looks like.

use crate::{
    decode, encode, Delivered, Event, FleetTimeline, Instant, JobId, ManifestId, SessionId,
    SessionFact, StreamMessage, TimelineAppended, TimelineAsk, TimelineEvent, TimelineKind, TimelineSubject, TimelineTone,
};

const ALL: [TimelineKind; 33] = [
    TimelineKind::JobCreated,
    TimelineKind::JobQueued,
    TimelineKind::JobStarted,
    TimelineKind::JobStep,
    TimelineKind::JobCheck,
    TimelineKind::JobJudged,
    TimelineKind::JobFiles,
    TimelineKind::JobRead,
    TimelineKind::JobRan,
    TimelineKind::JobAsked,
    TimelineKind::JobAnswered,
    TimelineKind::JobLanded,
    TimelineKind::JobFailed,
    TimelineKind::SessionStarted,
    TimelineKind::SessionAsked,
    TimelineKind::SessionAnswered,
    TimelineKind::SessionTurn,
    TimelineKind::SessionEdited,
    TimelineKind::SessionRead,
    TimelineKind::SessionRan,
    TimelineKind::SessionSubagent,
    TimelineKind::SessionWaiting,
    TimelineKind::SessionResolved,
    TimelineKind::SessionSpawned,
    TimelineKind::SessionEnded,
    TimelineKind::SlotLeased,
    TimelineKind::SlotReleased,
    TimelineKind::PrOpened,
    TimelineKind::PrChecks,
    TimelineKind::PrQueued,
    TimelineKind::PrMerged,
    TimelineKind::MainRed,
    TimelineKind::MainGreen,
];

/// A kind Fleet keeps as text reads back as the same kind, and these are the
/// names `packages/protocol/src/timeline.ts` lists.
#[test]
fn every_kind_is_kept_under_its_dotted_name_and_reads_back() {
    let names: Vec<String> = ALL.iter().map(|kind| kind.as_wire()).collect();
    assert_eq!(
        names,
        [
            "job.created", "job.queued", "job.started", "job.step", "job.check", "job.judged",
            "job.files", "job.read", "job.ran", "job.asked", "job.answered", "job.landed", "job.failed",
            "session.started", "session.asked", "session.answered", "session.turn", "session.edited",
            "session.read", "session.ran", "session.subagent", "session.waiting", "session.resolved",
            "session.spawned", "session.ended",
            "slot.leased", "slot.released", "pr.opened", "pr.checks", "pr.queued", "pr.merged",
            "main.red", "main.green",
        ]
    );
    for kind in ALL {
        assert_eq!(TimelineKind::from_wire(&kind.as_wire()), Some(kind));
    }
    assert_eq!(TimelineKind::from_wire("job.vanished"), None);
    assert_eq!(TimelineTone::from_wire("ask"), Some(TimelineTone::Ask));
    assert_eq!(TimelineTone::from_wire("loud"), None);
}

fn a_main_red() -> TimelineEvent {
    TimelineEvent {
        id: 41,
        at: Instant::carried("2026-10-12T10:00:00.000Z"),
        kind: TimelineKind::MainRed,
        subject: TimelineSubject::Main,
        title: None,
        tone: TimelineTone::Bad,
        says: "Main red".to_string(),
        detail: None,
        items: Vec::new(),
        manifest_id: None,
        live: false,
        over: false,
    }
}

/// Absent, never null: Bridge types these `field?: T` and compares to `undefined`.
#[test]
fn what_an_entry_does_not_carry_is_left_out_not_sent_as_null() {
    let json = encode(&a_main_red()).expect("plain data");
    assert!(!json.contains("null"), "{json}");
    for left_out in ["title", "detail", "items", "manifest_id", "live", "over"] {
        assert!(!json.contains(&format!("\"{left_out}\"")), "{left_out}: {json}");
    }
    assert!(json.contains(r#""subject":{"kind":"main"}"#), "{json}");
    let asked = encode(&TimelineAsk::default()).expect("plain data");
    assert_eq!(asked, "{}");
    let whole = encode(&FleetTimeline::default()).expect("plain data");
    assert_eq!(whole, r#"{"events":[]}"#, "`cut` is stated only when something was cut");
}

#[test]
fn a_subject_is_tagged_by_what_it_is() {
    let job = TimelineSubject::Job { id: JobId::carried("01JOB") };
    let session = TimelineSubject::Session { id: SessionId::carried("s9") };
    let pull = TimelineSubject::PullRequest { number: 1890, url: None };
    assert_eq!(encode(&job).unwrap(), r#"{"kind":"job","id":"01JOB"}"#);
    assert_eq!(encode(&session).unwrap(), r#"{"kind":"session","id":"s9"}"#);
    assert_eq!(encode(&pull).unwrap(), r#"{"kind":"pull_request","number":1890}"#);
}

/// The live append is an event of its own kind, carrying the entry whole.
#[test]
fn an_appended_entry_travels_as_an_event_and_round_trips() {
    let mut entry = a_main_red();
    entry.manifest_id = Some(ManifestId::carried("armada"));
    entry.detail = Some("components_test".to_string());
    entry.live = true;
    let message = StreamMessage::Event(Delivered {
        cursor: crate::Cursor::at(3),
        event: Event::TimelineAppended(TimelineAppended { event: entry }),
    });
    let json = encode(&message).expect("plain data");
    assert!(json.contains(r#""kind":"timeline.appended""#), "{json}");
    assert!(json.contains(r#""event":{"id":41"#), "{json}");
    assert!(json.contains(r#""manifest_id":"armada""#), "{json}");
    assert_eq!(
        decode::<StreamMessage>("stream message", json.as_bytes()).expect("it round-trips"),
        message
    );
}

/// The mod's two facts the timeline counts, as `plugins/armada/hooks/fleet.ts` spells them.
#[test]
fn a_read_and_a_command_are_told_as_the_mod_spells_them() {
    let read: SessionFact = decode("a fact", br#"{"kind":"read","path":"src/a.rs"}"#).expect("a read");
    let ran: SessionFact = decode("a fact", br#"{"kind":"ran","command":"cargo test"}"#).expect("a command");
    assert_eq!(read, SessionFact::Read { path: "src/a.rs".to_string() });
    assert_eq!(ran, SessionFact::Ran { command: "cargo test".to_string() });
}
