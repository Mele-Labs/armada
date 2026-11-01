//! A Judge's reading between the store's rows and the wire, and the message a sent note is.
//! `judge_serving` is what reads and writes them.

use ipc::{
    Instant, JudgeArtifact, JudgeKind, JudgeNote, JudgeNoteHow, JudgeNoteSettled, JudgeReading, JudgeState,
};
use store::{KeptJudgeNote, KeptJudgeReading};

pub(crate) const READING: &str = "reading";
pub(crate) const DONE: &str = "done";
pub(crate) const FAILED: &str = "failed";

/// What a reading whose Fleet stopped before it ended reads as.
pub(crate) const DIED: &str = "Fleet restarted before the reading finished";

pub(crate) const KINDS: [(JudgeKind, &str); 12] = [
    (JudgeKind::Job, "job"),
    (JudgeKind::Plan, "plan"),
    (JudgeKind::PullRequest, "pull_request"),
    (JudgeKind::Commit, "commit"),
    (JudgeKind::File, "file"),
    (JudgeKind::Page, "page"),
    (JudgeKind::Doc, "doc"),
    (JudgeKind::Sketch, "sketch"),
    (JudgeKind::Command, "command"),
    (JudgeKind::Subagent, "subagent"),
    (JudgeKind::Issue, "issue"),
    (JudgeKind::Note, "note"),
];

pub(crate) fn kind_word(kind: JudgeKind) -> &'static str {
    KINDS.iter().find(|(held, _)| *held == kind).map_or("job", |(_, word)| word)
}

pub(crate) fn kind_of(word: &str) -> JudgeKind {
    KINDS.iter().find(|(_, held)| *held == word).map_or(JudgeKind::Job, |(kind, _)| *kind)
}

fn state_of(word: &str) -> JudgeState {
    match word {
        READING => JudgeState::Reading,
        DONE => JudgeState::Done,
        _ => JudgeState::Failed,
    }
}

pub(crate) fn how_word(how: JudgeNoteHow) -> &'static str {
    match how {
        JudgeNoteHow::Sent => "sent",
        JudgeNoteHow::Dismissed => "dismissed",
    }
}

fn how_of(word: &str) -> JudgeNoteHow {
    match word {
        "sent" => JudgeNoteHow::Sent,
        _ => JudgeNoteHow::Dismissed,
    }
}

/// The reading as the wire carries it.
pub(crate) fn reading_of(kept: &KeptJudgeReading) -> JudgeReading {
    JudgeReading {
        id: kept.id.clone(),
        artifact: JudgeArtifact { kind: kind_of(&kept.kind), id: kept.artifact_id.clone(), title: kept.title.clone() },
        prompt: kept.prompt.clone(),
        state: state_of(&kept.state),
        asked_at: Instant::carried(&kept.asked_at),
        finished_at: kept.finished_at.as_deref().map(Instant::carried),
        failed: kept.failed.clone(),
        read: kept.read,
        notes: kept.notes.iter().map(note_of).collect(),
        cost_micros: kept.cost_micros,
    }
}

fn note_of(kept: &KeptJudgeNote) -> JudgeNote {
    JudgeNote {
        id: kept.id.clone(),
        about: kept.about.clone(),
        claim: kept.claim.clone(),
        r#where: kept.place.clone(),
        expected: kept.expected.clone(),
        found: kept.found.clone(),
        impact: kept.impact.clone(),
        cites: kept.cites.clone(),
        settled: kept
            .settled
            .as_ref()
            .map(|(how, at)| JudgeNoteSettled { how: how_of(how), at: Instant::carried(at) }),
    }
}

/// A note as it is kept.
pub(crate) fn note_kept(note: &JudgeNote) -> KeptJudgeNote {
    KeptJudgeNote {
        id: note.id.clone(),
        about: note.about.clone(),
        claim: note.claim.clone(),
        place: note.r#where.clone(),
        expected: note.expected.clone(),
        found: note.found.clone(),
        impact: note.impact.clone(),
        cites: note.cites.clone(),
        settled: None,
    }
}

/// What the agent is told of the artifact a note is about, in its own terms: the plan it wrote, or the
/// kind and title. **No id**: the agent knows its work by what it called it.
fn artifact_told(reading: &KeptJudgeReading) -> String {
    let titled = |noun: &str| match reading.title.trim() {
        "" => format!("the {noun}"),
        title => format!("the {noun} \"{title}\""),
    };
    match kind_of(&reading.kind) {
        JudgeKind::Job => String::from("your work on this Job"),
        JudgeKind::Plan => String::from("your plan"),
        JudgeKind::PullRequest => titled("pull request"),
        JudgeKind::Commit => titled("commit"),
        JudgeKind::File => titled("file"),
        JudgeKind::Page => titled("page"),
        JudgeKind::Doc => titled("document"),
        JudgeKind::Sketch => titled("sketch"),
        JudgeKind::Command => titled("command"),
        JudgeKind::Subagent => titled("subagent's report"),
        JudgeKind::Issue => titled("issue"),
        JudgeKind::Note => titled("note"),
    }
}

/// What the agent is told when the person sends a note: the part it is about as the person reads it,
/// then the claim and what it rests on.
pub(crate) fn message(reading: &KeptJudgeReading, note: &KeptJudgeNote) -> String {
    let part = match note.place.as_deref().map(str::trim).filter(|place| !place.is_empty()) {
        Some(place) => format!(", at {place}"),
        None => String::new(),
    };
    let mut said = format!("A Judge read {}{part} and raised this:\n\n{}", artifact_told(reading), note.claim);
    for (label, part) in [("Expected", &note.expected), ("Found", &note.found), ("Impact", &note.impact)] {
        if let Some(part) = part.as_deref().filter(|part| !part.is_empty()) {
            said.push_str(&format!("\n{label}: {part}"));
        }
    }
    if !note.cites.is_empty() {
        said.push_str(&format!("\n\nIt points at {}.", note.cites.join(", ")));
    }
    said
}
