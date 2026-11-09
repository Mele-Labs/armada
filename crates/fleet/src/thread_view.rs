//! A Job's thread as a screen draws it: the rows Fleet keeps, folded once for every client.
//!
//! Pure. The input is what `get_session` answers and the Job's timeline entries; the output is the
//! head and the items of the whole thread, which [`window`] cuts to whole items. This is a port of
//! what Bridge worked out in TypeScript (`rowsOfThread`, `session-thread-rows`, `thread-cards`,
//! `withChanges` and `itemsOf`), with the rules kept as they were. **Two departures**: a `plan` and a
//! `delivery` row, which the TypeScript left out of the thread, stay as `Row` items for the screen to
//! decide, and no row is worded here, so a settled ask is its `Ask` row and not a line of text.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use ipc::{
    Attachment, AttachmentState, CallInFlight, CardSettled, ChangedValue, FleetAct, FleetActKind, FleetActState,
    Instant, Lapsed, OpenCall, SessionAskState, SessionRow, SessionState, SessionThread, SessionVoice, SketchHow,
    ThreadAsk, ThreadDocument, ThreadHead, ThreadItem, ThreadSketch, ThreadTrail, TimelineEvent, TimelineKind,
    TimelineSubject, TrailBay, TrailOpened,
};

/// What a person says in place of an answer they did not wait for; a question it answers is no longer lapsed.
const LAPSE_ANSWER: &str = "Answering your earlier question";
const FLEET: &str = "fleet";

/// The thread of a Job: its head and all of its items, oldest first.
pub(crate) fn folded(thread: &SessionThread, timeline: &[TimelineEvent]) -> (ThreadHead, Vec<ThreadItem>) {
    (head_of(thread), items_of(thread, timeline))
}

/// Whole items: the newest `limit` before the item that holds `before`, and the id to read the older
/// ones by. `None` is a `before` no item holds.
pub(crate) fn window(
    items: &[ThreadItem],
    before: Option<&str>,
    limit: Option<u32>,
) -> Option<(Vec<ThreadItem>, Option<String>)> {
    let end = match before {
        None => items.len(),
        Some(id) => items.iter().position(|item| holds(item, id))?,
    };
    let most = limit.unwrap_or(ipc::THREAD_ITEMS_SERVED).clamp(1, ipc::THREAD_ITEMS_MOST) as usize;
    let start = end.saturating_sub(most);
    let older = (start > 0).then(|| items[start].id().to_string());
    Some((items[start..end].to_vec(), older))
}

/// Whether the item is `id` or folds the row `id`.
fn holds(item: &ThreadItem, id: &str) -> bool {
    item.id() == id
        || match item {
            ThreadItem::Calls { rows, .. } | ThreadItem::Checks { rows, .. } => rows.iter().any(|row| row.id() == id),
            ThreadItem::FleetTurn { notice, rows, .. } => {
                notice.id() == id || rows.iter().any(|row| row.id() == id)
            }
            _ => false,
        }
}

/// What a client holding `before` must do to hold `after`: the first item of `before` that differs and
/// the items from the matching place of `after` to its end. `None` where nothing differs.
pub(crate) fn tail_of(before: &[ThreadItem], after: &[ThreadItem]) -> Option<(Option<String>, Vec<ThreadItem>)> {
    let same = before.iter().zip(after).take_while(|(held, now)| held == now).count();
    if same == before.len() && same == after.len() {
        return None;
    }
    let from = before.get(same).map(|item| item.id().to_string());
    Some((from, after[same..].to_vec()))
}

// ---- the items

/// A row on its way to the fold, and when it was written.
struct Entry<'a> {
    at: Instant,
    /// Whether a row of the thread wrote it, which is what the timeline places a change by.
    from_row: bool,
    piece: Piece<'a>,
}

enum Piece<'a> {
    Row(&'a SessionRow),
    Card(ThreadItem),
}

fn items_of(thread: &SessionThread, timeline: &[TimelineEvent]) -> Vec<ThreadItem> {
    let record = &thread.session;
    let ledger = &record.attachments;
    let mut entries = thread
        .rows
        .iter()
        .filter(|row| {
            !matches!(row, SessionRow::BrowserReminder { .. } | SessionRow::Ask { state: SessionAskState::Waiting, .. })
        })
        .map(|row| {
            let card = match row {
                SessionRow::Tool { id, at, text, .. } => {
                    sketch_card_of(id, at, text, ledger).or_else(|| file_card_of(id, at, text, ledger))
                }
                _ => None,
            };
            Entry {
                at: row.at().clone(),
                from_row: true,
                piece: card.map_or(Piece::Row(row), Piece::Card),
            }
        })
        .collect::<Vec<_>>();
    with_changes(&mut entries, &record.id, timeline);
    with_sketch_state(&mut entries, ledger);
    with_documents(&mut entries, ledger);
    let last_at = entries.last().map(|entry| entry.at.clone());
    let mut items = fold(entries);
    if let Some(failed) = record.hosted.as_ref().and_then(|hosted| hosted.start_failed.as_ref()) {
        let at = last_at
            .or_else(|| record.last_turn_at.clone())
            .unwrap_or_else(|| Instant::carried("1970-01-01T00:00:00.000Z"));
        items.push(ThreadItem::NotStarted {
            id: String::from("not-started"),
            at,
            cause: failed.cause.clone(),
            log: failed.log.clone(),
        });
    }
    items
}

/// A turn Fleet started folds into one item: its notice, then the agent's replies and tool runs until
/// the next row of any other kind. The person's message is theirs, not Fleet's turn, so it ends the fold.
/// A run of calls, or of one Check, folds into one; a card ends either.
fn fold(entries: Vec<Entry<'_>>) -> Vec<ThreadItem> {
    let mut items: Vec<ThreadItem> = Vec::new();
    let mut entries = entries.into_iter().peekable();
    while let Some(entry) = entries.next() {
        let row = match entry.piece {
            Piece::Card(card) => {
                items.push(card);
                continue;
            }
            Piece::Row(row) => row,
        };
        if is_fleets(row) {
            let mut reply: Vec<&SessionRow> = Vec::new();
            while let Some(Entry { piece: Piece::Row(next), .. }) = entries.peek() {
                if matches!(next, SessionRow::Tool { .. } | SessionRow::Message { from: SessionVoice::Agent, .. }) {
                    reply.push(next);
                    entries.next();
                } else {
                    break;
                }
            }
            // A turn ends in words meant for the person: its last agent message is drawn outside the fold.
            let spoken = reply.iter().rposition(|one| matches!(one, SessionRow::Message { .. }));
            let handled = !reply.is_empty();
            let said = spoken.map(|at| reply.remove(at));
            items.push(ThreadItem::FleetTurn {
                id: row.id().to_string(),
                notice: row.clone(),
                rows: reply.into_iter().cloned().collect(),
                handled,
            });
            if let Some(said) = said {
                items.push(ThreadItem::Row { row: said.clone() });
            }
            continue;
        }
        match row {
            SessionRow::Check { id, name, .. } => match items.last_mut() {
                Some(ThreadItem::Checks { rows, .. })
                    if matches!(rows.first(), Some(SessionRow::Check { name: first, .. }) if first == name) =>
                {
                    rows.push(row.clone());
                }
                _ => items.push(ThreadItem::Checks { id: id.clone(), rows: vec![row.clone()] }),
            },
            SessionRow::Tool { id, at, subagent: Some(subagent), .. } => {
                items.push(ThreadItem::Subagent { id: id.clone(), at: at.clone(), subagent: subagent.clone() });
            }
            SessionRow::Tool { id, .. } => match items.last_mut() {
                Some(ThreadItem::Calls { rows, .. }) => rows.push(row.clone()),
                _ => items.push(ThreadItem::Calls { id: id.clone(), names: Vec::new(), rows: vec![row.clone()] }),
            },
            other => items.push(ThreadItem::Row { row: other.clone() }),
        }
    }
    for item in &mut items {
        if let ThreadItem::Calls { names, rows, .. } = item {
            *names = names_of(rows);
        }
    }
    items
}

/// A notice Fleet wrote: a message from the session named `fleet`.
fn is_fleets(row: &SessionRow) -> bool {
    matches!(row, SessionRow::Message { from: SessionVoice::Session { id, .. }, .. } if id == FLEET)
}

/// The tools a run holds, each named once, in the order they were first called.
fn names_of(rows: &[SessionRow]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for row in rows {
        let SessionRow::Tool { text, .. } = row else { continue };
        let name = text.split(char::is_whitespace).next().unwrap_or("");
        if !name.is_empty() && !names.iter().any(|held| held == name) {
            names.push(name.to_string());
        }
    }
    names
}

// ---- cards

fn form_of(one: &Attachment) -> Option<&str> {
    (one.kind == "artifact").then(|| one.detail.get("form").map(String::as_str)).flatten()
}

/// The card a `show_sketch` call is. **Only a call the ledger holds a sketch for draws one**: Fleet keeps
/// the sketch it accepted under the call's title, so a call it refused is no card.
fn sketch_card_of(id: &str, at: &Instant, text: &str, ledger: &[Attachment]) -> Option<ThreadItem> {
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
    }))
}

/// The item a `show_file` call is, where the ledger holds the file. **Opened by the ledger's own path**,
/// the canonical one Fleet kept: the call's text is the path as the agent gave it, which may be relative.
fn file_card_of(id: &str, at: &Instant, text: &str, ledger: &[Attachment]) -> Option<ThreadItem> {
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

fn title_of(one: &Attachment) -> String {
    one.detail
        .get("title")
        .cloned()
        .unwrap_or_else(|| one.target.rsplit('/').next().unwrap_or(&one.target).to_string())
}

/// How the person answered a sketch or a document put to him. A how Fleet does not write reads as open.
fn settled_of(detail: &BTreeMap<String, String>) -> Option<CardSettled> {
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
/// earlier scene of. **What he was asked, and how he answered, belong to the sketch as it stands**, so
/// the latest card holds them.
fn with_sketch_state(entries: &mut [Entry<'_>], ledger: &[Attachment]) {
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
            )
        });
        states.push((*at, earlier, !latest, answer));
    }
    for (at, earlier, replaced, answer) in states {
        let Piece::Card(ThreadItem::Sketch(card)) = &mut entries[at].piece else { continue };
        card.against_drawing = earlier;
        card.replaced = replaced;
        if let Some((asked, withdrawn, settled)) = answer {
            (card.asked, card.withdrawn, card.settled) = (asked, withdrawn, settled);
        }
    }
}

/// Whether `at` is after `kept`. **A time that does not read as one is never later**, so a card follows it.
fn later_than(at: &Instant, kept: &Instant) -> bool {
    let shaped = at.as_str().as_bytes();
    let reads = shaped.len() >= 20 && shaped[4] == b'-' && shaped[10] == b'T' && shaped[..4].iter().all(u8::is_ascii_digit);
    reads && at > kept
}

/// The card a Job's document is: its newest revision, read off the artifact and never off a write's
/// tool row, so a page a script changed has one too. A page whose revision does not read has none.
fn document_card_of(one: &Attachment) -> Option<ThreadDocument> {
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
    })
}

/// Each document's card, where its revision was kept: before the first item later than that.
fn with_documents(entries: &mut Vec<Entry<'_>>, ledger: &[Attachment]) {
    let mut cards = ledger.iter().filter_map(document_card_of).collect::<Vec<_>>();
    cards.sort_by(|a, b| a.at.cmp(&b.at));
    for card in cards {
        let later = entries.iter().position(|entry| later_than(&entry.at, &card.at)).unwrap_or(entries.len());
        entries.insert(later, Entry { at: card.at.clone(), from_row: false, piece: Piece::Card(ThreadItem::Document(card)) });
    }
}

/// What a timeline entry that changed a Job's workflow, model or effort says: the value's name and what
/// it was and became.
fn change_of_event(kind: TimelineKind, says: &str) -> Option<(ChangedValue, String, String)> {
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
fn with_changes(entries: &mut Vec<Entry<'_>>, session: &ipc::SessionId, timeline: &[TimelineEvent]) {
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

// ---- the head

fn head_of(thread: &SessionThread) -> ThreadHead {
    let record = &thread.session;
    let rows = &thread.rows;
    let asked = record
        .hosted
        .as_ref()
        .and_then(|hosted| hosted.asked.as_ref())
        .or_else(|| record.terminal.as_ref().and_then(|terminal| terminal.asked.as_ref()))
        .map(ask_of);
    let open_call = match (&record.turn_since, rows.last()) {
        (Some(_), Some(SessionRow::Tool { text, at, .. })) => Some(OpenCall { text: text.clone(), since: at.clone() }),
        _ => None,
    };
    ThreadHead {
        asked,
        open_call,
        lapsed: if record.state == SessionState::Ended { None } else { lapsed_of(rows) },
        fleet_acts: fleet_acts_of(rows, record),
        trail: trail_of(record),
    }
}

/// The call the Job waits on, with the command a person reads: the argument whole where `detail` was cut.
fn ask_of(ask: &CallInFlight) -> ThreadAsk {
    let command = match (&ask.whole, ask.detail.is_empty()) {
        (Some(whole), _) => whole.clone(),
        (None, false) => ask.detail.clone(),
        (None, true) => ask.tool.clone(),
    };
    ThreadAsk { command, call: ask.clone() }
}

/// The questions of every ask that nobody answered in time, that was refused, or that was kept (because he
/// was walking a page, or nobody answered inside the hold), oldest first, under the newest one's id.
/// **A kept one outlasts what he says**: only a message that answers it clears it. An unanswered or refused
/// one is cleared by anything he says after it, and a question asked again later is left to the later ask.
fn lapsed_of(rows: &[SessionRow]) -> Option<Lapsed> {
    let mut said = false;
    let mut later: HashSet<&str> = HashSet::new();
    let mut id: Option<&str> = None;
    let mut lapsed = VecDeque::new();
    for row in rows.iter().rev() {
        match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => {
                if text.starts_with(LAPSE_ANSWER) {
                    break;
                }
                said = true;
            }
            SessionRow::Ask { id: row_id, ask, state, .. } if !ask.questions.is_empty() => {
                let kept = *state == SessionAskState::Kept
                    || (!said && matches!(state, SessionAskState::Unanswered | SessionAskState::Refused));
                for asked in &ask.questions {
                    if kept && !later.contains(asked.question.as_str()) {
                        lapsed.push_front(asked.clone());
                        id = id.or(Some(row_id.as_str()));
                    }
                    later.insert(&asked.question);
                }
            }
            _ => {}
        }
    }
    Some(Lapsed { id: id?.to_string(), questions: lapsed.into() })
}

/// The kind of act a Fleet word is, by the notice it was sent for. Fleet's background model reads write
/// no row, so they are none of these.
fn fleet_act_kind_of(notice: Option<&str>) -> FleetActKind {
    match notice {
        Some("stop_nudge" | "rail_nudge") => FleetActKind::Nudge,
        Some("rail_step") => FleetActKind::Step,
        Some("slot_free" | "slot_kept" | "slot_stranded") => FleetActKind::Slot,
        Some("limit_wake" | "sign_in_wake") => FleetActKind::Restart,
        _ => FleetActKind::Told,
    }
}

/// Everything Fleet did, does or will do for a Job, read from what is kept and never from the agent: each
/// word and note of its own in the thread is over, the nudge coming and the repair running are read off
/// the record. **The two sentences of a nudge and a repair are Fleet's own words.**
fn fleet_acts_of(rows: &[SessionRow], record: &ipc::SessionRecord) -> Vec<FleetAct> {
    let mut acts: Vec<FleetAct> = rows
        .iter()
        .filter_map(|row| match row {
            SessionRow::Message { id, at, from: SessionVoice::Session { title, .. }, text, notice, .. }
                if title == "Fleet" =>
            {
                let line = text.lines().next().unwrap_or("");
                Some(FleetAct {
                    id: id.clone(),
                    at: at.clone(),
                    state: FleetActState::Over,
                    kind: fleet_act_kind_of(notice.as_deref()),
                    said: line.strip_prefix("Fleet: ").unwrap_or(line).to_string(),
                    why: None,
                    until: None,
                })
            }
            _ => None,
        })
        .collect();
    if let Some(stopped) = record.stopped.as_ref().filter(|stopped| stopped.repair == Some(ipc::StopRepair::Running)) {
        acts.push(FleetAct {
            id: String::from("repair"),
            at: stopped.at.clone(),
            state: FleetActState::UnderWay,
            kind: FleetActKind::Restart,
            said: String::from("Repair Job running"),
            why: Some(String::from("The restart failed")),
            until: None,
        });
    }
    if let Some(told_by) = &record.stop_told_by {
        acts.push(FleetAct {
            id: String::from("stop-nudge"),
            at: told_by.clone(),
            state: FleetActState::Coming,
            kind: FleetActKind::Nudge,
            said: String::from("Tell it to carry on or name what it waits on"),
            why: Some(String::from("Its turn ended with nothing running and nothing asked")),
            until: Some(told_by.clone()),
        });
    }
    acts
}

/// Where the work has been, off the ledger rows including the ones the ledger drops: a slot no longer
/// standing is a stay that ended (`changed_at`), and a pull request let go was still opened at its `since`.
fn trail_of(record: &ipc::SessionRecord) -> ThreadTrail {
    let mut bays = record
        .attachments
        .iter()
        .filter(|one| one.kind == "slot")
        .filter_map(|one| {
            Some(TrailBay {
                slot: one.target.parse().ok()?,
                from: one.since.clone(),
                until: (one.state != AttachmentState::Standing).then(|| one.changed_at.clone()),
            })
        })
        .collect::<Vec<_>>();
    bays.sort_by(|a, b| a.from.cmp(&b.from));
    let mut opened: BTreeMap<u64, Instant> = BTreeMap::new();
    for one in record.attachments.iter().filter(|one| one.kind == "pr") {
        if let Ok(number) = one.target.parse() {
            opened.insert(number, one.since.clone());
        }
    }
    ThreadTrail {
        started: record.started_at.clone(),
        bays,
        opened: opened.into_iter().map(|(number, at)| TrailOpened { number, at }).collect(),
    }
}
