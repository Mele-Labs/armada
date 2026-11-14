//! The cards of a thread: a sketch, a file, a document and a changed value, each read off the ledger or
//! the timeline and placed among the rows.

use std::collections::{BTreeMap, HashMap};

use ipc::{
    Attachment, CardSettled, ChangedValue, Instant, RawScene, SketchHow, ThreadDocument, ThreadItem, ThreadSketch,
    TimelineEvent, TimelineKind, TimelineSubject,
};

use super::{Entry, Piece, Revisions};

pub(super) fn form_of(one: &Attachment) -> Option<&str> {
    (one.kind == "artifact").then(|| one.detail.get("form").map(String::as_str)).flatten()
}

/// The card a `show_sketch` call is. **Only a call the ledger holds a sketch for draws one**: Fleet keeps
/// the sketch it accepted under the call's title, so a call it refused is no card.
pub(super) fn sketch_card_of(id: &str, at: &Instant, text: &str, ledger: &[Attachment]) -> Option<ThreadItem> {
    let call = ipc::sketch_call_of(text)?;
    let sketch = format!("sketch:{}", call.title);
    let held = ledger.iter().find(|one| form_of(one) == Some("sketch") && one.target == sketch)?;
    // The call is the agent's words, without the `kept` Fleet wrote on the sketch it kept.
    let drawing = match held.detail.get("scene").and_then(|scene| ipc::scene_of_detail(scene)) {
        Some(kept) => ipc::kept_from(call.scene, &kept),
        None => call.scene,
    };
    Some(ThreadItem::Sketch(ThreadSketch {
        id: id.to_string(),
        at: at.clone(),
        sketch,
        title: call.title,
        drawing,
        against: call.against,
        against_drawing: None,
        replaced: false,
        asked: false,
        withdrawn: false,
        settled: None,
        revisions: Vec::new(),
        for_line: None,
    }))
}

/// The item a `show_file` call is, where the ledger holds the file. **Opened by the ledger's own path**,
/// the canonical one Fleet kept: the call's text is the path as the agent gave it, which may be relative.
pub(super) fn file_card_of(id: &str, at: &Instant, text: &str, ledger: &[Attachment]) -> Option<ThreadItem> {
    let path = ipc::file_call_of(text)?;
    let given = path.strip_prefix("./").unwrap_or(&path);
    let held = ledger.iter().find(|one| {
        form_of(one) == Some("file") && (one.target == path || one.target.ends_with(&format!("/{given}")))
    })?;
    Some(ThreadItem::File {
        id: id.to_string(),
        at: at.clone(),
        path: held.target.clone(),
        name: title_of(held),
    })
}

pub(super) fn title_of(one: &Attachment) -> String {
    one.detail
        .get("title")
        .cloned()
        .unwrap_or_else(|| one.target.rsplit('/').next().unwrap_or(&one.target).to_string())
}

/// How the person answered a sketch or a document put to him. A how Fleet does not write reads as open.
pub(super) fn settled_of(detail: &BTreeMap<String, String>) -> Option<CardSettled> {
    let how = match detail.get("settled").map(String::as_str) {
        Some("approved") => SketchHow::Approved,
        Some("sent_back") => SketchHow::SentBack,
        _ => return None,
    };
    let at = Instant::carried(detail.get("settled_at")?);
    let words = detail.get("words").filter(|words| !words.is_empty()).cloned();
    Some(CardSettled { how, at, words })
}

/// A card is replaced where a later card holds the same sketch. What it was drawn against is the latest
/// card before it with the title it names, as it was drawn then, else the sketch the ledger keeps the
/// earlier scene of. **What he was asked, how he answered, and the drawings kept under the title belong
/// to the sketch as it stands**, so the latest card holds them.
pub(super) fn with_sketch_state(entries: &mut [Entry<'_>], ledger: &[Attachment], revisions: &Revisions) {
    fn card<'e>(entry: &'e Entry<'_>) -> Option<&'e ThreadSketch> {
        match &entry.piece {
            Piece::Card(ThreadItem::Sketch(card)) => Some(card),
            _ => None,
        }
    }
    let cards = (0..entries.len()).filter(|at| card(&entries[*at]).is_some()).collect::<Vec<_>>();
    let mut last: HashMap<&str, &str> = HashMap::new();
    for at in &cards {
        let one = card(&entries[*at]).expect("indexed as a card");
        last.insert(&one.sketch, &one.id);
    }
    let mut states = Vec::new();
    for (k, at) in cards.iter().enumerate() {
        let one = card(&entries[*at]).expect("indexed as a card");
        let held = ledger.iter().find(|held| form_of(held) == Some("sketch") && held.target == one.sketch);
        let earlier = one.against.as_deref().and_then(|against| {
            let before = cards[..k].iter().rev().filter_map(|j| card(&entries[*j])).find(|one| one.title == against);
            before.map(|one| one.drawing.clone()).or_else(|| {
                let (title, scene) = ipc::against_of_detail(held?.detail.get("against")?)?;
                (title == against).then_some(scene)
            })
        });
        let latest = last.get(one.sketch.as_str()) == Some(&one.id.as_str());
        let answer = held.filter(|_| latest).map(|held| {
            (
                held.detail.get("asked").is_some_and(|asked| asked == "true"),
                held.detail.get("withdrawn").is_some_and(|withdrawn| withdrawn == "true"),
                settled_of(&held.detail),
                revisions_of(held, revisions),
                ipc::ArtifactFor::of_detail(&held.detail),
            )
        });
        states.push((*at, earlier, !latest, answer));
    }
    for (at, earlier, replaced, answer) in states {
        let Piece::Card(ThreadItem::Sketch(card)) = &mut entries[at].piece else { continue };
        card.against_drawing = earlier;
        card.replaced = replaced;
        if let Some((asked, withdrawn, settled, kept, for_line)) = answer {
            (card.asked, card.withdrawn, card.settled, card.revisions, card.for_line) = (asked, withdrawn, settled, kept, for_line);
        }
    }
}

/// The scenes of every drawing kept under a sketch's title, oldest first: none until all of them read.
pub(super) fn revisions_of(held: &Attachment, revisions: &Revisions) -> Vec<RawScene> {
    let paths = held.detail.get("revisions").map(|kept| ipc::revision_paths_of_detail(kept)).unwrap_or_default();
    paths.iter().map(|path| revisions.get(path).cloned()).collect::<Option<Vec<_>>>().unwrap_or_default()
}

/// Whether `at` is after `kept`. **A time that does not read as one is never later**, so a card follows it.
pub(super) fn later_than(at: &Instant, kept: &Instant) -> bool {
    reads(at) && at > kept
}

/// Whether an instant reads as a time, which a mock's words do not.
pub(super) fn reads(at: &Instant) -> bool {
    let shaped = at.as_str().as_bytes();
    shaped.len() >= 20 && shaped[4] == b'-' && shaped[10] == b'T' && shaped[..4].iter().all(u8::is_ascii_digit)
}

/// The card a Job's document is: its newest revision, read off the artifact and never off a write's
/// tool row, so a page a script changed has one too. A page whose revision does not read has none.
pub(super) fn document_card_of(one: &Attachment) -> Option<ThreadDocument> {
    if form_of(one) != Some("document") {
        return None;
    }
    let detail = &one.detail;
    let given = detail.get("revision").filter(|revision| !revision.is_empty());
    let settled = given.and_then(|_| settled_of(detail));
    let kept = match detail.get("kept_at").filter(|kept| !kept.is_empty()) {
        Some(kept) => Instant::carried(kept),
        None => match (&settled, given) {
            (None, _) => one.changed_at.clone(),
            (Some(_), Some(revision)) if revision == "1" => one.since.clone(),
            (Some(settled), _) => settled.at.clone(),
        },
    };
    let revision = match given {
        Some(revision) => revision.parse::<u32>().ok()?,
        None => detail.get("revisions").map_or(0, |kept| ipc::revisions_of_detail(kept)) as u32,
    };
    if revision < 1 {
        return None;
    }
    Some(ThreadDocument {
        id: format!("document:{}:r{revision}", one.target),
        at: kept,
        page: one.target.clone(),
        title: title_of(one),
        revision,
        asked: given.is_some() && detail.get("asked").is_some_and(|asked| asked == "true"),
        settled,
        for_line: ipc::ArtifactFor::of_detail(detail),
    })
}

/// Each document's card, where its revision was kept: before the first item later than that.
pub(super) fn with_documents(entries: &mut Vec<Entry<'_>>, ledger: &[Attachment]) {
    let mut cards = ledger.iter().filter_map(document_card_of).collect::<Vec<_>>();
    cards.sort_by(|a, b| a.at.cmp(&b.at));
    for card in cards {
        let later = entries.iter().position(|entry| later_than(&entry.at, &card.at)).unwrap_or(entries.len());
        entries.insert(later, Entry { at: card.at.clone(), from_row: false, piece: Piece::Card(ThreadItem::Document(card)) });
    }
}

/// What a timeline entry that changed a Job's workflow, model or effort says: the value's name and what
/// it was and became.
pub(super) fn change_of_event(kind: TimelineKind, says: &str) -> Option<(ChangedValue, String, String)> {
    let what = match kind {
        TimelineKind::SessionWorkflow => ChangedValue::Workflow,
        TimelineKind::SessionModel => ChangedValue::Model,
        TimelineKind::SessionEffort => ChangedValue::Effort,
        _ => return None,
    };
    let word = says.len() - says.trim_start_matches(|c: char| c.is_ascii_lowercase()).len();
    let after = says[word..].strip_prefix(':').filter(|_| word > 0)?.trim_start();
    if after.contains('\n') {
        return None;
    }
    let (at, arrow) = [("→", after.find('→')), ("->", after.find("->"))]
        .into_iter()
        .filter_map(|(arrow, at)| Some((at?, arrow)))
        .min()?;
    Some((what, after[..at].trim_end().to_string(), after[at + arrow.len()..].trim_start().to_string()))
}

/// The values the person changed through a panel, which the agent was told of, as items of the thread
/// where they happened: before the first row written after them.
pub(super) fn with_changes(entries: &mut Vec<Entry<'_>>, session: &ipc::SessionId, timeline: &[TimelineEvent]) {
    let mut changes = timeline
        .iter()
        .filter(|event| matches!(&event.subject, TimelineSubject::Session { id } if id == session))
        .filter_map(|event| {
            let (what, from, to) = change_of_event(event.kind, &event.says)?;
            let item = ThreadItem::Change { id: format!("event:{}", event.id), at: event.at.clone(), what, from, to };
            Some((event.at.clone(), item))
        })
        .collect::<Vec<_>>();
    changes.sort_by(|a, b| a.0.cmp(&b.0));
    for (at, change) in changes {
        let later = entries.iter().position(|entry| entry.from_row && later_than(&entry.at, &at)).unwrap_or(entries.len());
        entries.insert(later, Entry { at, from_row: false, piece: Piece::Card(change) });
    }
}

