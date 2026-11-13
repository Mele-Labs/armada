//! Listen: with `take_in` set to listen, Jobs are read out loud through Armada Voice and the thread
//! keeps what was said as cards. Nothing here turns it on: `take_in` and the `listen_*` switches are
//! saved by Settings' Ways of working page, and at their defaults (`read`) nothing is spoken.
//!
//! | File | Holds |
//! |---|---|
//! | `mod.rs` | The held preferences, what arrives and the words it is spoken as, the line of cards and how long a card runs |
//! | `speaker.rs` | The helper's `speak` and `voices` off the main thread, and the mock that never starts it |
//! | `engine.rs` | `Listener`, the one entity that owns the line and the speaker |

pub mod engine;
pub mod scenes;
pub mod speaker;

pub use speaker::Speaker;

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;

use ipc::{PlanTaskState, Preferences, SessionRecord, TakeIn, WaitingActKind, WaitingItem, WaitingSource};

/// What `listen_*` and `take_in` say, as the thread reads them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Prefs {
    /// `take_in` is `listen`.
    pub listening: bool,
    pub end_summary: bool,
    pub needs_you: bool,
    pub progress: bool,
    pub operator: bool,
    /// A voice id; empty is the Mac's default voice.
    pub voice: String,
}

impl Prefs {
    pub fn of(preferences: &Preferences) -> Prefs {
        Prefs {
            listening: preferences.take_in == TakeIn::Listen,
            end_summary: preferences.listen_end_summary,
            needs_you: preferences.listen_needs_you,
            progress: preferences.listen_progress,
            operator: preferences.listen_operator,
            voice: preferences.listen_voice.clone(),
        }
    }

    /// Whether a kind of thing is read out: only while he listens, and only the kinds he switched on.
    pub fn speaks(&self, kind: Kind) -> bool {
        self.listening
            && match kind {
                Kind::EndSummary => self.end_summary,
                Kind::NeedsYou => self.needs_you,
                Kind::Progress => self.progress,
                Kind::Operator => self.operator,
            }
    }
}

thread_local! {
    /// What Fleet last said he set: read once a connection, and again after each save.
    static HELD: std::cell::RefCell<Prefs> = std::cell::RefCell::new(Prefs::default());
}

/// What he has set, for anything drawn or spoken on this thread. Preferences Fleet has not served read as
/// the defaults, which listen to nothing.
pub fn held() -> Prefs {
    HELD.with(|held| held.borrow().clone())
}

/// Take Fleet's preferences, on a read or after a save.
pub fn hold(preferences: &Preferences) {
    HELD.with(|held| *held.borrow_mut() = Prefs::of(preferences));
}

/// The four things that can be read out, each with its own switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    EndSummary,
    NeedsYou,
    Progress,
    /// Operator cards. The Operator page has no card feed to read from yet.
    #[allow(dead_code)]
    Operator,
}

/// One thing to read out: the Job it is about, what kind it is, and the words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    pub job: String,
    pub kind: Kind,
    pub words: String,
}

/// An option as it reads inside a phrase: its first letter lower, unless the word is an acronym.
fn inside(option: &str) -> String {
    let mut letters = option.chars();
    match (letters.next(), letters.next()) {
        (Some(first), second) if first.is_uppercase() && !second.is_some_and(char::is_uppercase) => first.to_lowercase().chain(option.chars().skip(1)).collect(),
        _ => option.to_string(),
    }
}

/// Options as one phrase: `a`, `a or b`, `a, b, or c`. The first begins the sentence after the question, so
/// it keeps its capital; the rest read on from it.
fn joined(options: &[&str]) -> String {
    let options: Vec<String> = options
        .iter()
        .enumerate()
        .map(|(at, option)| {
            let option = option.trim().trim_end_matches(['.', '?', '!']);
            if at == 0 { option.to_string() } else { inside(option) }
        })
        .collect();
    match options.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [one, two] => format!("{one} or {two}"),
        [rest @ .., last] => format!("{}, or {last}", rest.join(", ")),
    }
}

/// What a Job's question is read as: its name, the question, then its options.
pub fn needs_sentence(job: &str, question: &str, options: &[&str]) -> String {
    let mut said = format!("{job} asks: {}", question.trim());
    if !options.is_empty() {
        said.push(' ');
        said.push_str(&joined(options));
        said.push('?');
    }
    said
}

/// What a Job that ended is read as: its name, then the summary Fleet carries for the done card.
pub fn end_sentence(job: &str, summary: &str) -> String {
    format!("The {job} Job is done. {}", summary.trim())
}

/// What a plan step that ended is read as.
pub fn progress_sentence(job: &str, step: &str) -> String {
    format!("{job}: {} is done.", step.trim())
}

/// The helper reports no length, so a line's is its words at 120 a minute, never under a second. A Mac speaks
/// faster, so the bar runs slow and jumps to the end when the helper says the line is done.
const MILLIS_PER_WORD: u64 = 500;

pub fn estimate(words: &str) -> Duration {
    let count = words.split_whitespace().count() as u64;
    Duration::from_millis((count * MILLIS_PER_WORD).max(1000))
}

/// A length as `m:ss`, rounded down to the second.
pub fn m_ss(length: Duration) -> String {
    let secs = length.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// The Job's name as it is spoken.
fn name_of(record: &SessionRecord) -> String {
    record.title.clone().filter(|title| !title.trim().is_empty()).unwrap_or_else(|| record.id.as_str().to_string())
}

/// Whether an item is a question the agent put to him: words, with or without choices, and not a walk,
/// a command or a pull request to open.
fn is_question(item: &WaitingItem) -> bool {
    matches!(item.source, WaitingSource::Agent | WaitingSource::AskCard | WaitingSource::WindowAsk)
        && item.steps.is_empty()
        && !item.text.trim().is_empty()
        && !matches!(item.act.as_ref().map(|act| act.kind), Some(WaitingActKind::Walk | WaitingActKind::Run | WaitingActKind::ApprovePr))
}

/// What the Jobs held the last time they were looked at, so only what is new is read out.
#[derive(Default)]
struct Known {
    asks: HashSet<(String, String)>,
    done: HashMap<String, Option<String>>,
    steps: HashSet<(String, String)>,
}

fn known_of(records: &[SessionRecord]) -> Known {
    let mut known = Known::default();
    for record in records {
        let job = record.id.as_str().to_string();
        known.asks.extend(record.waiting_for.iter().filter(|item| is_question(item)).map(|item| (job.clone(), item.id.clone())));
        known.done.insert(job.clone(), record.done_since.as_ref().map(|at| at.as_str().to_string()));
        for group in record.plan.iter().flat_map(|plan| plan.groups.iter()).filter(|group| ended(group)) {
            known.steps.insert((job.clone(), group.id.clone()));
        }
    }
    known
}

fn ended(group: &ipc::SessionPlanGroup) -> bool {
    !group.tasks.is_empty() && group.tasks.iter().all(|task| task.state == PlanTaskState::Done)
}

/// The Jobs as last seen. The first look only remembers them: what was already waiting when he started
/// listening is not read out.
#[derive(Default)]
pub struct Seen {
    known: Option<Known>,
}

impl Seen {
    /// Nothing has been looked at, so nothing is kept.
    pub fn is_idle(&self) -> bool {
        self.known.is_none()
    }

    /// What is new in `records` since the last look, in the order a person would hear it, for the kinds
    /// he has switched on. Not listening forgets what was seen, so listening later starts from then.
    pub fn fresh(&mut self, records: &[SessionRecord], prefs: &Prefs) -> Vec<Said> {
        if !prefs.listening {
            self.known = None;
            return Vec::new();
        }
        let now = known_of(records);
        let Some(before) = self.known.replace(now) else { return Vec::new() };
        let mut said = Vec::new();
        for record in records {
            let job = record.id.as_str();
            let name = name_of(record);
            if prefs.speaks(Kind::NeedsYou) {
                for item in record.waiting_for.iter().filter(|item| is_question(item)) {
                    if before.asks.contains(&(job.to_string(), item.id.clone())) {
                        continue;
                    }
                    let options: Vec<&str> = item.options.iter().map(|option| option.label.as_str()).collect();
                    said.push(Said { job: job.to_string(), kind: Kind::NeedsYou, words: needs_sentence(&name, &item.text, &options) });
                }
            }
            // A Job that was not there before is new to the list, not newly done.
            let Some(was) = before.done.get(job) else { continue };
            let ends = record.done_since.as_ref().map(|at| at.as_str().to_string());
            if prefs.speaks(Kind::EndSummary) && ends.is_some() && ends != *was {
                if let Some(summary) = record.done_summary.as_deref().filter(|text| !text.trim().is_empty()) {
                    said.push(Said { job: job.to_string(), kind: Kind::EndSummary, words: end_sentence(&name, summary) });
                }
            }
            if prefs.speaks(Kind::Progress) {
                for group in record.plan.iter().flat_map(|plan| plan.groups.iter()).filter(|group| ended(group)) {
                    if !before.steps.contains(&(job.to_string(), group.id.clone())) {
                        said.push(Said { job: job.to_string(), kind: Kind::Progress, words: progress_sentence(&name, &group.title) });
                    }
                }
            }
        }
        said
    }
}

/// Why a line could not be spoken. Said as a fact on its card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    HelperMissing,
    VoiceMissing,
    HelperFailed,
}

impl Why {
    pub fn said(self) -> &'static str {
        match self {
            Why::HelperMissing => "Armada Voice is missing",
            Why::VoiceMissing => "The voice is missing",
            Why::HelperFailed => "Armada Voice stopped",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Waiting for the line ahead of it to end.
    Queued,
    Speaking,
    /// Stopped by a press; the line waits on it.
    Paused,
    Spoken,
    /// Never spoken: its words stand and its disc is off.
    Failed(Why),
}

/// One thing spoken, or waiting to be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    pub id: u64,
    pub job: String,
    pub kind: Kind,
    pub words: String,
    /// How long it is estimated to take.
    pub total: Duration,
    /// How much of it has been said.
    pub played: Duration,
    pub state: State,
    /// Which start of it is current: events from an earlier start are ignored.
    pub run: u64,
}

impl Card {
    pub fn failed(&self) -> bool {
        matches!(self.state, State::Failed(_))
    }
}

/// The cards in the order they were made, and the order they are spoken in. One speaks at a time.
#[derive(Default)]
pub struct Line {
    cards: Vec<Card>,
    queue: VecDeque<u64>,
    next_id: u64,
    runs: u64,
}

impl Line {
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn card(&self, id: u64) -> Option<&Card> {
        self.cards.iter().find(|card| card.id == id)
    }

    fn card_mut(&mut self, id: u64) -> Option<&mut Card> {
        self.cards.iter_mut().find(|card| card.id == id)
    }

    /// A new card at the back of the line.
    pub fn add(&mut self, said: Said) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let total = estimate(&said.words);
        self.cards.push(Card { id, job: said.job, kind: said.kind, words: said.words, total, played: Duration::ZERO, state: State::Queued, run: 0 });
        self.queue.push_back(id);
        id
    }

    /// Whether a card is speaking or paused: the line waits on it.
    fn held(&self) -> bool {
        self.cards.iter().any(|card| matches!(card.state, State::Speaking | State::Paused))
    }

    /// The card to speak next, now speaking, with the number of its start. Nothing while one is speaking or
    /// paused, so two never overlap.
    pub fn start_next(&mut self) -> Option<(u64, u64)> {
        if self.held() {
            return None;
        }
        let id = self.queue.pop_front()?;
        self.runs += 1;
        let run = self.runs;
        let card = self.card_mut(id)?;
        card.state = State::Speaking;
        card.played = Duration::ZERO;
        card.run = run;
        Some((id, run))
    }

    /// Time passes for the card that is speaking. It stays under its estimate until the helper says it is
    /// done, so the bar never claims more than is known.
    pub fn tick(&mut self, by: Duration) {
        if let Some(card) = self.cards.iter_mut().find(|card| card.state == State::Speaking) {
            card.played = (card.played + by).min(card.total.saturating_sub(Duration::from_millis(1)));
        }
    }

    /// A card that was started part way through, as a mock one is.
    pub fn set_played(&mut self, id: u64, played: Duration) {
        if let Some(card) = self.card_mut(id) {
            card.played = played.min(card.total.saturating_sub(Duration::from_millis(1)));
        }
    }

    fn current(&mut self, id: u64, run: u64) -> Option<&mut Card> {
        self.card_mut(id).filter(|card| card.run == run && card.state == State::Speaking)
    }

    /// The helper began the card's words.
    pub fn began(&mut self, id: u64, run: u64) {
        let _ = self.current(id, run);
    }

    /// The helper finished the card's words. False for an event of an earlier start.
    pub fn ended(&mut self, id: u64, run: u64) -> bool {
        let Some(card) = self.current(id, run) else { return false };
        card.state = State::Spoken;
        card.played = card.total;
        true
    }

    /// The card could not be spoken. False for an event of an earlier start.
    pub fn failed(&mut self, id: u64, run: u64, why: Why) -> bool {
        let Some(card) = self.current(id, run) else { return false };
        card.state = State::Failed(why);
        true
    }

    /// A press on pause: the card stops where it is and the line waits on it.
    pub fn pause(&mut self, id: u64) -> bool {
        let Some(card) = self.card_mut(id).filter(|card| card.state == State::Speaking) else { return false };
        card.state = State::Paused;
        true
    }

    /// A card that was paused, and another pressed: it goes to the front of what is left.
    fn put_back_paused(&mut self, except: u64) {
        let paused: Vec<u64> = self.cards.iter().filter(|card| card.state == State::Paused && card.id != except).map(|card| card.id).collect();
        for id in paused.into_iter().rev() {
            if let Some(card) = self.card_mut(id) {
                card.state = State::Queued;
                card.played = Duration::ZERO;
            }
            self.queue.push_front(id);
        }
    }

    /// A press on play, or on Again: the card is spoken from its start, next. Whatever was speaking or paused
    /// is stopped (the caller stops the speaker) and goes back behind it. False for a card that cannot be.
    pub fn again(&mut self, id: u64) -> bool {
        if !self.card(id).is_some_and(|card| matches!(card.state, State::Speaking | State::Paused | State::Spoken)) {
            return false;
        }
        // A speaking card pressed again is stopped; one of another card is put back behind this one.
        let other = self.cards.iter().find(|card| card.state == State::Speaking && card.id != id).map(|card| card.id);
        if let Some(other) = other {
            if let Some(card) = self.card_mut(other) {
                card.state = State::Paused;
            }
        }
        self.put_back_paused(id);
        if let Some(card) = self.card_mut(id) {
            card.state = State::Queued;
            card.played = Duration::ZERO;
            card.run = 0;
        }
        self.queue.push_front(id);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(job: &str, kind: Kind, words: &str) -> Said {
        Said { job: job.into(), kind, words: words.into() }
    }

    fn listening() -> Prefs {
        Prefs { listening: true, end_summary: true, needs_you: true, ..Prefs::default() }
    }

    #[test]
    fn a_question_is_read_as_the_job_its_words_and_its_options() {
        assert_eq!(
            needs_sentence("Endpoint cleanup", "What happens to the old endpoint?", &["Keep it a week", "Remove it now"]),
            "Endpoint cleanup asks: What happens to the old endpoint? Keep it a week or remove it now?"
        );
        assert_eq!(
            needs_sentence("Endpoint cleanup", "Which one?", &["Keep it", "Remove it.", "Ask again"]),
            "Endpoint cleanup asks: Which one? Keep it, remove it, or ask again?"
        );
        assert_eq!(needs_sentence("Endpoint cleanup", "Which?", &["Yes", "API only"]), "Endpoint cleanup asks: Which? Yes or API only?");
        assert_eq!(needs_sentence("Endpoint cleanup", "Which?", &["Yes", "NO"]), "Endpoint cleanup asks: Which? Yes or NO?");
        assert_eq!(needs_sentence("Endpoint cleanup", "Keep the old endpoint a week?", &[]), "Endpoint cleanup asks: Keep the old endpoint a week?");
        assert_eq!(joined(&["A", "b", "c", "d"]), "A, b, c, or d");
    }

    #[test]
    fn an_end_and_a_step_are_read_with_the_job_named() {
        assert_eq!(end_sentence("Settings window", "Settings opens in its own window. A walk is waiting."), "The Settings window Job is done. Settings opens in its own window. A walk is waiting.");
        assert_eq!(progress_sentence("Settings window", "Move the panes"), "Settings window: Move the panes is done.");
    }

    #[test]
    fn a_line_runs_by_its_words_and_a_length_reads_as_minutes_and_seconds() {
        assert_eq!(estimate("one"), Duration::from_secs(1));
        assert_eq!(estimate(&["word"; 10].join(" ")), Duration::from_secs(5));
        assert_eq!(estimate(&["word"; 22].join(" ")), Duration::from_secs(11));
        assert_eq!(m_ss(Duration::from_secs(6)), "0:06");
        assert_eq!(m_ss(Duration::from_millis(11_900)), "0:11");
        assert_eq!(m_ss(Duration::from_secs(65)), "1:05");
        assert_eq!(m_ss(Duration::ZERO), "0:00");
    }

    #[test]
    fn each_switch_gates_its_own_kind_and_nothing_speaks_unless_he_listens() {
        let on = Prefs { listening: true, end_summary: false, needs_you: false, progress: false, operator: false, voice: String::new() };
        assert!(!on.speaks(Kind::EndSummary) && !on.speaks(Kind::NeedsYou) && !on.speaks(Kind::Progress) && !on.speaks(Kind::Operator));
        assert!(Prefs { end_summary: true, ..on.clone() }.speaks(Kind::EndSummary));
        assert!(!Prefs { end_summary: true, ..on.clone() }.speaks(Kind::NeedsYou));
        assert!(Prefs { needs_you: true, ..on.clone() }.speaks(Kind::NeedsYou));
        assert!(Prefs { progress: true, ..on.clone() }.speaks(Kind::Progress));
        assert!(Prefs { operator: true, ..on.clone() }.speaks(Kind::Operator));
        let read = Prefs { listening: false, end_summary: true, needs_you: true, progress: true, operator: true, voice: String::new() };
        assert!(![Kind::EndSummary, Kind::NeedsYou, Kind::Progress, Kind::Operator].into_iter().any(|kind| read.speaks(kind)));
        // Fleet's defaults, read: nothing.
        assert!(!Prefs::of(&Preferences::default()).speaks(Kind::NeedsYou));
        assert!(Prefs::of(&Preferences { take_in: TakeIn::Listen, ..Preferences::default() }).speaks(Kind::NeedsYou));
        assert!(!Prefs::of(&Preferences { take_in: TakeIn::Listen, ..Preferences::default() }).speaks(Kind::Progress));
    }

    fn record(id: &str, waiting: &[(&str, &str)], done: Option<&str>) -> SessionRecord {
        let waiting: Vec<serde_json::Value> = waiting
            .iter()
            .map(|(item, text)| serde_json::json!({"id": item, "text": text, "since": "2026-11-13T10:00:00Z", "source": "agent", "options": [{"label": "Keep it"}, {"label": "Remove it"}]}))
            .collect();
        let mut value = serde_json::json!({
            "id": id, "harness": "claude_code", "origin": "bridge", "cwd": "/repos/armada", "title": "Endpoint cleanup", "state": "live",
            "started_at": "2026-11-13T09:00:00Z", "last_seen_at": "2026-11-13T10:00:00Z",
            "usage": {}, "attachments": [],
            "waiting_for": waiting,
        });
        if let Some(done) = done {
            value["done_since"] = done.into();
            value["done_summary"] = "The old endpoint is gone.".into();
        }
        serde_json::from_value(value).expect("a record decodes")
    }

    #[test]
    fn only_what_is_new_since_the_last_look_is_read_and_the_first_look_reads_nothing() {
        let mut seen = Seen::default();
        let prefs = listening();
        assert!(seen.fresh(&[record("a", &[("q1", "Which?")], None)], &prefs).is_empty());
        assert!(seen.fresh(&[record("a", &[("q1", "Which?")], None)], &prefs).is_empty());
        let heard = seen.fresh(&[record("a", &[("q1", "Which?"), ("q2", "And then?")], None)], &prefs);
        assert_eq!(heard, [said("a", Kind::NeedsYou, "Endpoint cleanup asks: And then? Keep it or remove it?")]);
        let ended = seen.fresh(&[record("a", &[], Some("2026-11-13T10:05:00Z"))], &prefs);
        assert_eq!(ended, [said("a", Kind::EndSummary, "The Endpoint cleanup Job is done. The old endpoint is gone.")]);
        assert!(seen.fresh(&[record("a", &[], Some("2026-11-13T10:05:00Z"))], &prefs).is_empty());
    }

    #[test]
    fn a_switch_that_is_off_reads_nothing_of_its_kind_and_not_listening_forgets_what_was_seen() {
        let mut seen = Seen::default();
        let only_ends = Prefs { needs_you: false, ..listening() };
        seen.fresh(&[record("a", &[], None)], &only_ends);
        assert!(seen.fresh(&[record("a", &[("q1", "Which?")], None)], &only_ends).is_empty());
        let read = Prefs::default();
        assert!(seen.fresh(&[record("a", &[("q2", "Which?")], None)], &read).is_empty());
        // Listening again starts from what stands, so a question already waiting is not read.
        assert!(seen.fresh(&[record("a", &[("q2", "Which?")], None)], &listening()).is_empty());
    }

    #[test]
    fn the_line_never_overlaps_and_keeps_its_order() {
        let mut line = Line::default();
        let (a, b, c) = (line.add(said("j", Kind::NeedsYou, "one")), line.add(said("j", Kind::NeedsYou, "two")), line.add(said("j", Kind::EndSummary, "three")));
        let (first, run) = line.start_next().expect("the first card starts");
        assert_eq!(first, a);
        // Nothing starts while one speaks.
        assert_eq!(line.start_next(), None);
        assert_eq!(line.card(b).map(|card| card.state), Some(State::Queued));
        assert!(line.ended(a, run));
        let (second, run) = line.start_next().expect("the second follows");
        assert_eq!(second, b);
        // A late event of the first card's start changes nothing.
        assert!(!line.ended(a, 1));
        assert_eq!(line.start_next(), None);
        assert!(line.ended(b, run));
        assert_eq!(line.start_next().map(|(id, _)| id), Some(c));
    }

    #[test]
    fn a_pause_holds_the_line_and_again_speaks_a_card_next_and_puts_the_paused_one_back() {
        let mut line = Line::default();
        let (a, b, c) = (line.add(said("j", Kind::NeedsYou, "one")), line.add(said("j", Kind::NeedsYou, "two")), line.add(said("j", Kind::NeedsYou, "three")));
        let (_, run) = line.start_next().expect("starts");
        assert!(line.pause(a));
        assert_eq!(line.start_next(), None, "a paused card holds the line");
        // Play: from its start, next.
        assert!(line.again(a));
        assert_eq!(line.start_next().map(|(id, _)| id), Some(a));
        assert!(!line.ended(a, run), "the stopped start's end is ignored");
        // Again on the finished card while another is paused: it goes first, the paused one follows it.
        let run = line.card(a).map(|card| card.run).expect("a run");
        assert!(line.ended(a, run));
        let (b_id, b_run) = line.start_next().expect("b starts");
        assert_eq!(b_id, b);
        assert!(line.pause(b));
        assert!(line.again(a));
        assert_eq!(line.start_next().map(|(id, _)| id), Some(a));
        let a_run = line.card(a).map(|card| card.run).expect("a run");
        assert!(line.ended(a, a_run));
        assert_eq!(line.start_next().map(|(id, _)| id), Some(b));
        assert!(!line.ended(b, b_run));
        let b_run = line.card(b).map(|card| card.run).expect("b run");
        assert!(line.ended(b, b_run));
        assert_eq!(line.start_next().map(|(id, _)| id), Some(c));
    }

    #[test]
    fn the_bar_stays_under_its_estimate_until_the_helper_says_it_is_done() {
        let mut line = Line::default();
        let id = line.add(said("j", Kind::NeedsYou, "one two three four five"));
        let (_, run) = line.start_next().expect("starts");
        line.tick(Duration::from_secs(30));
        assert!(line.card(id).is_some_and(|card| card.played < card.total));
        assert!(line.ended(id, run));
        assert_eq!(line.card(id).map(|card| card.played), line.card(id).map(|card| card.total));
    }
}
