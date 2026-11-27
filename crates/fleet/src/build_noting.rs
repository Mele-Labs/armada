//! The notes of the build the owner runs: what each offered branch told him to look at, read for the branches the
//! running preview holds, marked looked at, and answered with his words. `docs/concepts/fleet.md`, *Offering a
//! branch for the preview*. The offers a note is kept from are `crate::previewing`.
//!
//! **A note outlives its offer**: the build he loaded still holds the branch after its Job ended or its tip
//! landed. Feedback goes to the Job where it is live, as his message, and starts a new Job where it has ended.
//!
//! **A Ready to try card is a note seen from the thread of the Job that offered it**, with what the offer gave the
//! card beside it: what he approved, the shots and his words. Its answer is the note's verdict, so the two never
//! disagree. `crates/ipc/operations/answer_ready_to_try.toml`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    AnswerReadyToTry, ArtifactForm, BuildNote, BuildNoteFeedbackSent, BuildNotes, DispatchJob, DispatchMessage, DispatchThen, Event,
    Instant, ManifestId, MarkBuildNoteLookedAt, NoteVerdict, PreviewStep, ReadyAnswer, ReadyApproved, ReadyState, SendBuildNoteFeedback,
    SendSessionMessage, SessionId, StartSession, ThreadItem, ThreadReadyToTry, WireError,
};
use store::{KeptApproval, KeptBuildNote, SessionState};

use crate::thread_keeping::Change;

use crate::daemon::Fleet;

const NOTE_UNKNOWN: &str = "fleet.build_note_unknown";
const WORDS_EMPTY: &str = "fleet.build_note_words_empty";
const NO_REPOSITORY: &str = "fleet.build_note_no_repository";
const CARD_UNKNOWN: &str = "fleet.ready_to_try_unknown";
const CARD_SETTLED: &str = "fleet.ready_to_try_settled";
const REMIND_PAST: &str = "fleet.ready_to_try_remind_past";
const NOT_IN_BUILD: &str = "fleet.ready_not_in_build";

/// What the Job is told when he says its offer looks good: land it.
fn landing_text(note: &KeptBuildNote) -> String {
    format!(
        "The owner tried {} at {} in his preview and says it looks good: \"{}\". Land it.",
        note.branch,
        note.tip.get(..7).unwrap_or(&note.tip),
        note.about
    )
}

/// What the Job is told when he skips the preview: he approved the look on the walk, so land it untried.
fn skipped_text(note: &KeptBuildNote) -> String {
    format!(
        "The owner skipped the preview of {} at {}: he approved the look on the walk and did not try it. Land it: \"{}\".",
        note.branch,
        note.tip.get(..7).unwrap_or(&note.tip),
        note.about
    )
}

/// A card's id, which the thread holds it by.
fn card_id(branch: &str, tip: &str) -> String {
    format!("try:{branch}@{tip}")
}

fn form_read(word: &str) -> ArtifactForm {
    match word {
        "draft" => ArtifactForm::Draft,
        "page" => ArtifactForm::Page,
        _ => ArtifactForm::Sketch,
    }
}

/// Where a card stands, read in the contract's order: what he said, then what the Job did, then his reminder.
/// `newest` is the card being the Job's newest offer of its branch.
fn card_state(note: &KeptBuildNote, newest: bool, now: &str) -> ReadyState {
    if verdict_read(note.verdict.as_deref()).is_some() {
        ReadyState::Answered
    } else if note.withdrawn_at.is_some() {
        ReadyState::Withdrawn
    } else if !newest {
        ReadyState::Replaced
    } else if note.later_until.as_deref().is_some_and(|until| until > now) {
        ReadyState::Later
    } else {
        ReadyState::Open
    }
}

/// The one item that waits on him while any of the Job's cards is open: id and artifact `try:<session>`, text
/// `Ready to try`, since the oldest open card, opening the Job. It goes once no card is open. A card he dismissed
/// the row for (`dismissed` holds its id) does not raise it, so a card offered later does.
pub(crate) fn ready_items(notes: Vec<KeptBuildNote>, session: &str, now: &str, dismissed: &[String]) -> Option<ipc::WaitingItem> {
    let since = cards_of(notes, &|_, _| false, now)
        .into_iter()
        .filter(|card| card.state == ReadyState::Open && !dismissed.contains(&card.id))
        .map(|card| card.at)
        .min()?;
    let id = format!("try:{session}");
    Some(ipc::WaitingItem {
        order: 0,
        why: String::new(),
        approves: false,
        text: "Ready to try".into(),
        since,
        source: ipc::WaitingSource::ReadyToTry,
        act: Some(ipc::WaitingAct { kind: ipc::WaitingActKind::Message, target: session.to_string(), on: None }),
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
        artifact: Some(id.clone()),
        id,
    })
}

/// The Job's notes as its cards. `in_build` says whether the running build holds a branch at a tip.
fn cards_of(notes: Vec<KeptBuildNote>, in_build: &dyn Fn(&str, &str) -> bool, now: &str) -> Vec<ThreadReadyToTry> {
    let mut newest: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
    for note in &notes {
        let at = (note.offered_at.as_str(), note.tip.as_str());
        let kept = newest.entry(note.branch.as_str()).or_insert(at);
        if at > *kept {
            *kept = at;
        }
    }
    let newest: BTreeSet<(String, String)> = newest.into_iter().map(|(branch, (_, tip))| (branch.to_string(), tip.to_string())).collect();
    notes
        .into_iter()
        .map(|note| {
            let state = card_state(&note, newest.contains(&(note.branch.clone(), note.tip.clone())), now);
            ThreadReadyToTry {
                id: card_id(&note.branch, &note.tip),
                at: Instant::carried(note.back_at.as_deref().unwrap_or(&note.offered_at)),
                offered_at: Instant::carried(&note.offered_at),
                in_build: in_build(&note.branch, &note.tip),
                verdict: verdict_read(note.verdict.as_deref()),
                skipped: note.verdict.as_deref() == Some(SKIPPED_KEPT),
                later_until: note.later_until.as_deref().filter(|_| state == ReadyState::Later).map(Instant::carried),
                state,
                look: note.look,
                steps: note.steps.into_iter().map(|step| PreviewStep { say: step.say, act: step.act }).collect(),
                approved: note.card.approved.map(|one| ReadyApproved {
                    artifact: one.artifact,
                    title: one.title,
                    form: form_read(&one.form),
                    revision: one.revision,
                    approved_at: one.approved_at.as_deref().map(Instant::carried),
                }),
                pictures: note.card.pictures,
                words: note.card.words,
                branch: note.branch,
                tip: note.tip,
                about: note.about,
            }
        })
        .collect()
}

/// What the Job is told, led by which note it answers.
fn answer_text(note: &KeptBuildNote, words: &str) -> String {
    format!("On your preview note \"{}\": {words}", note.about)
}

/// What the Job is told when the owner marks its note looked at, or says what he made of it.
fn looked_text(note: &KeptBuildNote, verdict: Option<NoteVerdict>) -> String {
    let named = format!("your preview note \"{}\" on {} at {}", note.about, note.branch, note.tip.get(..7).unwrap_or(&note.tip));
    match verdict {
        None => format!("The owner looked at {named}."),
        Some(NoteVerdict::Good) => format!("The owner says {named} is good."),
        Some(NoteVerdict::NeedsWork) => format!("The owner says {named} needs work. His words on it follow in a message: wait for them before you change it."),
    }
}

/// The verdict as the store keeps it, which is as the wire spells it.
fn verdict_kept(verdict: NoteVerdict) -> &'static str {
    match verdict {
        NoteVerdict::Good => "good",
        NoteVerdict::NeedsWork => "needs_work",
    }
}

/// What the note keeps for his Skip: a `good` that was landed untried, read as `good` everywhere but the card.
const SKIPPED_KEPT: &str = "good_skipped";

fn verdict_read(kept: Option<&str>) -> Option<NoteVerdict> {
    match kept? {
        "good" | SKIPPED_KEPT => Some(NoteVerdict::Good),
        "needs_work" => Some(NoteVerdict::NeedsWork),
        _ => None,
    }
}

/// What a new Job starts from where the note's Job has ended: his words, then what he was looking at.
fn started_brief(note: &KeptBuildNote, title: &str, words: &str) -> String {
    let mut brief = format!("His feedback on a preview note of the Job \"{title}\", which has ended:\n\n{words}\n\nThe note said: {}", note.about);
    if let Some(look) = &note.look {
        brief.push_str(&format!("\nWhat to look at: {look}"));
    }
    for (at, step) in note.steps.iter().enumerate() {
        let act = step.act.as_deref().map(|act| format!(" ({act})")).unwrap_or_default();
        brief.push_str(&format!("\n{}. {}{act}", at + 1, step.say));
    }
    brief.push_str(&format!("\n\nThe branch is {} at {}.", note.branch, note.tip));
    brief
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `get_build_notes`: the note of each (branch, tip) the running preview build holds, in the order the branches
    /// were offered. Empty, and no `commit`, off the preview.
    pub(crate) async fn build_notes_read(&self) -> BuildNotes {
        let Some((commit, tips)) = self.preview_build() else {
            return BuildNotes { commit: None, notes: Vec::new() };
        };
        let store = self.store().lock().await;
        let mut kept: Vec<KeptBuildNote> = tips.iter().filter_map(|(branch, tip)| store.build_note_of(branch, tip).ok().flatten()).collect();
        kept.sort_by(|one, other| (&one.offered_at, &one.branch).cmp(&(&other.offered_at, &other.branch)));
        let notes = kept
            .into_iter()
            .map(|one| {
                let session = store.session(&one.session_id).ok().flatten();
                BuildNote {
                    title: session.as_ref().and_then(|row| row.title.clone()).unwrap_or_else(|| one.session_id.clone()),
                    job_ended: session.as_ref().is_none_or(|row| row.state == SessionState::Ended),
                    session_id: SessionId::carried(one.session_id.as_str()),
                    looked_at: one.looked_at.is_some(),
                    verdict: verdict_read(one.verdict.as_deref()),
                    branch: one.branch,
                    tip: one.tip,
                    about: one.about,
                    look: one.look,
                    steps: one.steps.into_iter().map(|step| PreviewStep { say: step.say, act: step.act }).collect(),
                }
            })
            .collect();
        BuildNotes { commit: Some(commit), notes }
    }

    /// Publish the notes where they differ from the ones last sent. The first answer after Fleet starts is sent.
    /// **The cards of every Job a change touched move with them**, since a card says whether the build holds it.
    pub(crate) async fn build_notes_moved(self: &Arc<Self>) {
        for session in self.build_notes_published().await {
            self.ready_cards_moved(&session).await;
        }
    }

    /// Publish the notes where they differ from the ones last sent, and name the Jobs whose notes the change
    /// touched. None where nothing changed.
    pub(crate) async fn build_notes_published(&self) -> BTreeSet<String> {
        let now = self.build_notes_read().await;
        let before = {
            let mut sent = self.previewing().notes_sent.lock().expect("not poisoned");
            let changed = sent.as_ref() != Some(&now);
            changed.then(|| sent.replace(now.clone()))
        };
        let Some(before) = before else { return BTreeSet::new() };
        let mut touched: BTreeSet<String> = before.iter().flat_map(|one| one.notes.iter()).chain(now.notes.iter()).map(|note| note.session_id.as_str().to_string()).collect();
        // A new build can hold a branch no list names, because it landed on the main the build stands on.
        if before.as_ref().map(|one| &one.commit) != Some(&now.commit) {
            touched.extend(self.store().lock().await.build_note_sessions().unwrap_or_default());
        }
        self.publish(Event::BuildNotesChanged(now));
        touched
    }

    /// The Job's Ready to try cards as they stand now, oldest offer first. None where the store will not read.
    pub(crate) async fn ready_cards_of(&self, session: &str) -> Vec<ThreadReadyToTry> {
        let notes = self.store().lock().await.build_notes_of_session(session).unwrap_or_default();
        if notes.is_empty() {
            return Vec::new();
        }
        let build = self.preview_build();
        cards_of(notes, &|branch, tip| self.build_holds(build.as_ref(), branch, tip), self.now().as_str())
    }

    /// Each Ready to try card in `items` of a Job held from another Mac, read against the build he runs here: that
    /// Mac read it against its own build.
    pub(crate) fn ready_cards_restamped(&self, items: &mut [ThreadItem]) {
        let mut cards = items.iter_mut().filter_map(|item| match item {
            ThreadItem::ReadyToTry(card) => Some(card),
            _ => None,
        }).peekable();
        if cards.peek().is_none() {
            return;
        }
        let build = self.preview_build();
        for card in cards {
            card.in_build = self.build_holds(build.as_ref(), &card.branch, &card.tip);
        }
    }

    /// His answer on a card of a Job held from another Mac goes there only where the build he runs here holds the
    /// card's branch at its tip.
    pub(crate) fn ready_in_build_here(&self, asked: &AnswerReadyToTry) -> Result<(), Refusal> {
        let build = self.preview_build();
        if self.build_holds(build.as_ref(), &asked.branch, &asked.tip) {
            return Ok(());
        }
        let said = "Your build does not hold this branch yet. Refresh preview first.";
        Err(Refusal::IllegalMove(WireError::raised(NOT_IN_BUILD, said, self.run_id())))
    }

    /// The Job's cards moved: its thread, where a client holds it, is folded again with them, and its record, whose
    /// Needs you holds each open card, is published.
    pub(crate) async fn ready_cards_moved(self: &Arc<Self>, session: &str) {
        let cards = self.ready_cards_of(session).await;
        self.job_thread_changed(session, vec![Change::Ready(cards)]).await;
        let _ = self.published_hosted(session).await;
    }

    /// Bring back each card whose reminder came: drawn at that time, open again. **A tick of the notice loop
    /// calls it**; the store writes only where one is due.
    pub(crate) async fn ready_cards_come_back(self: &Arc<Self>) {
        let now = self.now().as_str().to_string();
        let due = self.store().lock().await.build_notes_come_back(&now).unwrap_or_default();
        let sessions: BTreeSet<String> = due.into_iter().map(|(session, ..)| session).collect();
        for session in sessions {
            self.ready_cards_moved(&session).await;
        }
    }

    /// `answer_ready_to_try`: his answer on a card that is open or later. **Nothing changes on a refusal**: the
    /// words of a rework are sent before its verdict is kept, so a send that fails leaves the card as it was.
    pub(crate) async fn ready_to_try_answered(self: &Arc<Self>, asked: AnswerReadyToTry) -> Result<ThreadReadyToTry, Refusal> {
        let session = asked.session_id.as_str();
        let card = self.ready_card(session, &asked.branch, &asked.tip).await?;
        let settled = match card.state {
            ReadyState::Open | ReadyState::Later => None,
            ReadyState::Answered => Some("answered"),
            ReadyState::Withdrawn => Some("withdrawn"),
            ReadyState::Replaced => Some("replaced by a newer offer"),
        };
        if let Some(how) = settled {
            return Err(Refusal::IllegalMove(WireError::raised(CARD_SETTLED, format!("that card is {how}"), self.run_id())));
        }
        let at = self.now().as_str().to_string();
        match asked.answer {
            ReadyAnswer::LooksGood | ReadyAnswer::Skip => {
                let skipped = matches!(asked.answer, ReadyAnswer::Skip);
                let note = self.store().lock().await.build_note_of(&asked.branch, &asked.tip).map_err(|why| self.ledger_fault(why))?;
                let note = note.ok_or_else(|| self.card_unknown(&asked.branch, &asked.tip))?;
                self.store().lock().await.keep_build_note_looked_at(&asked.branch, &asked.tip, &at, Some(if skipped { SKIPPED_KEPT } else { verdict_kept(NoteVerdict::Good) })).map_err(|why| self.ledger_fault(why))?;
                self.looked_good(session, &asked.branch, &asked.tip, &note.about, &at, skipped).await;
                // His looks-good is his Approve, as an item the Job raised asking for it is: it closes the Job's first
                // open `you_approved` gate. A preview names no plan group, so no group is picked for it. His Skip is the same.
                let how = if skipped { "Ready to try, skipped" } else { "Ready to try" };
                let approval = KeptApproval { item_id: card_id(&asked.branch, &asked.tip), text: format!("{how}: {}", note.about), at: at.clone() };
                self.store().lock().await.keep_approval(session, &approval).map_err(|why| self.ledger_fault(why))?;
                // Nothing on the Job's row moved for the keeper to hear, as an item going from its list does, so it is swept now.
                self.gates_kept(session).await;
                let live = self.store().lock().await.session(session).ok().flatten().is_some_and(|row| row.state != SessionState::Ended);
                if live {
                    let said = if skipped { skipped_text(&note) } else { landing_text(&note) };
                    let _ = self.told_news_by_fleet(session, &said, ipc::Notice::BuildMoved).await;
                }
            }
            ReadyAnswer::NeedsRework { words } => {
                let feedback = SendBuildNoteFeedback { branch: asked.branch.clone(), tip: asked.tip.clone(), words };
                self.build_note_answered(feedback).await?;
                self.store()
                    .lock()
                    .await
                    .keep_build_note_looked_at(&asked.branch, &asked.tip, &at, Some(verdict_kept(NoteVerdict::NeedsWork)))
                    .map_err(|why| self.ledger_fault(why))?;
            }
            ReadyAnswer::Remind { until } => {
                let millis = crate::operator::utc_millis(until.as_str())
                    .ok_or_else(|| self.hosted_refusal(REMIND_PAST, "a time to remind him at: that one could not be read"))?;
                if millis <= self.now().epoch_millis().unwrap_or(0) {
                    return Err(self.hosted_refusal(REMIND_PAST, "a time to remind him at that has not passed"));
                }
                let until = crate::clock::rfc3339_utc(millis);
                self.store().lock().await.keep_build_note_later(&asked.branch, &asked.tip, &until).map_err(|why| self.ledger_fault(why))?;
            }
        }
        self.build_notes_moved().await;
        self.ready_cards_moved(session).await;
        self.ready_card(session, &asked.branch, &asked.tip).await
    }

    async fn ready_card(&self, session: &str, branch: &str, tip: &str) -> Result<ThreadReadyToTry, Refusal> {
        let cards = self.ready_cards_of(session).await;
        cards.into_iter().find(|card| card.branch == branch && card.tip == tip).ok_or_else(|| self.card_unknown(branch, tip))
    }

    fn card_unknown(&self, branch: &str, tip: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(CARD_UNKNOWN, format!("that job's thread holds no ready to try card for {branch} at {tip}"), self.run_id()))
    }

    /// `mark_build_note_looked_at`: keep the mark and the verdict against the (branch, tip), and answer the notes now.
    pub(crate) async fn build_note_looked_at(self: &Arc<Self>, asked: MarkBuildNoteLookedAt) -> Result<BuildNotes, Refusal> {
        let at = self.now().as_str().to_string();
        let before = self.store().lock().await.build_note_of(&asked.branch, &asked.tip).map_err(|why| self.ledger_fault(why))?;
        let verdict = asked.verdict.map(verdict_kept);
        let marked = self.store().lock().await.keep_build_note_looked_at(&asked.branch, &asked.tip, &at, verdict).map_err(|why| self.ledger_fault(why))?;
        if !marked {
            return Err(self.build_note_unknown(&asked.branch, &asked.tip));
        }
        // The note's Job is told the first mark and each change of verdict, where it is live, as it is told the rest
        // of its preview news. A mark that says nothing new tells it nothing.
        let news = before.clone().filter(|note| note.looked_at.is_none() || (verdict.is_some() && verdict_read(note.verdict.as_deref()) != asked.verdict));
        if let (Some(note), Some(NoteVerdict::Good)) = (&before, asked.verdict) {
            self.looked_good(&note.session_id, &asked.branch, &asked.tip, &note.about, &at, false).await;
        }
        if let Some(note) = news {
            let live = self.store().lock().await.session(&note.session_id).ok().flatten().is_some_and(|row| row.state != SessionState::Ended);
            if live {
                let said = asked.verdict.or_else(|| verdict_read(note.verdict.as_deref()));
                let _ = self.told_news_by_fleet(&note.session_id, &looked_text(&note, said), ipc::Notice::BuildMoved).await;
            }
        }
        self.build_notes_moved().await;
        if let Some(note) = before {
            self.ready_cards_moved(&note.session_id).await;
        }
        Ok(self.build_notes_read().await)
    }

    /// `send_build_note_feedback`: his words to the note's Job where it is live, else to a new Job started at once.
    pub(crate) async fn build_note_answered(self: &Arc<Self>, asked: SendBuildNoteFeedback) -> Result<BuildNoteFeedbackSent, Refusal> {
        let words = asked.words.trim().to_string();
        if words.is_empty() {
            return Err(self.hosted_refusal(WORDS_EMPTY, "words on the note: there are none to send"));
        }
        let (note, session) = {
            let store = self.store().lock().await;
            let note = store.build_note_of(&asked.branch, &asked.tip).map_err(|why| self.ledger_fault(why))?;
            let note = note.ok_or_else(|| self.build_note_unknown(&asked.branch, &asked.tip))?;
            let session = store.session(&note.session_id).map_err(|why| self.ledger_fault(why))?;
            (note, session)
        };
        let live = session.as_ref().is_some_and(|row| row.state != SessionState::Ended);
        if live {
            let terminal = self.terminal_session(&note.session_id).await?;
            let sent = SendSessionMessage {
                session_id: SessionId::carried(note.session_id.as_str()),
                text: answer_text(&note, &words),
                attachments: Vec::new(),
                mentions: Vec::new(),
                sketches: Vec::new(),
            };
            Arc::clone(self).send_known(terminal, sent).await?;
            return Ok(BuildNoteFeedbackSent { session_id: SessionId::carried(note.session_id.as_str()), started: false });
        }
        let title = session.as_ref().and_then(|row| row.title.clone()).unwrap_or_else(|| note.session_id.clone());
        let manifest = match session.and_then(|row| row.manifest_id) {
            Some(manifest) => manifest,
            None => self.preview_place().map(|(_, served)| served.manifest().id().as_str().to_string()).ok_or_else(|| {
                self.hosted_refusal(NO_REPOSITORY, "the repository the note's Job worked in: Fleet serves none with a preview")
            })?,
        };
        let brief = started_brief(&note, &title, &words);
        let start = StartSession {
            project: None,
            manifest_id: ManifestId::carried(&manifest),
            title: None,
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: Some(brief.clone()),
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
            workflow: None,
        };
        let then = vec![DispatchThen::Message(DispatchMessage { text: brief, attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })];
        let record = self.dispatch_started(DispatchJob { start, kind: None, then }).await?;
        Ok(BuildNoteFeedbackSent { session_id: record.id, started: true })
    }

    fn build_note_unknown(&self, branch: &str, tip: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(NOTE_UNKNOWN, format!("no preview note is kept for {branch} at {tip}"), self.run_id()))
    }
}
