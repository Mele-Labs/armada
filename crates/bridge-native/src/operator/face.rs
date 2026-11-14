//! What the page draws for each thing Fleet serves, as words, a registry glyph and a hue: the kind chip
//! of an item, its mark in the list, a time said the way the sketch says it. Nothing here is worked
//! out from more than the one field it reads.

use chrono::{DateTime, Local};
use gpui::SharedString;
use ipc::operator_page::{OperatorAnswer, OperatorMain, OperatorThen, OperatorTurnItem, OperatorTurnKind};
use ipc::{HeldChecks, Instant, MainCiState, OperatorConflict, OperatorHeld, WaitingActKind, WaitingSource};

use crate::look::{token, Token};

/// The kind of an item of Your turn: the chip's word, the glyph that marks its row, and the hue both take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kind {
    pub word: &'static str,
    pub glyph: &'static str,
    pub hue: Token,
    pub ground: Token,
}

const OPERATOR: (Token, Token) = (token::OPERATOR, token::OPERATOR_MUTED);
const REVIEW: (Token, Token) = (token::STATUS_AWAITING_REVIEW, token::STATUS_AWAITING_REVIEW_BG);
const WALK: (Token, Token) = (token::ACCENT_HOVER, token::ACCENT_MUTED);
const PULL: (Token, Token) = (token::STATUS_REJECTED, token::STATUS_REJECTED_BG);
const FAILED: (Token, Token) = (token::STATUS_COMPLETED_FAILED, token::STATUS_COMPLETED_FAILED_BG);
const REPAIR: (Token, Token) = (token::STATUS_AWAITING_REPAIR, token::STATUS_AWAITING_REPAIR_BG);

const fn kind(word: &'static str, glyph: &'static str, (hue, ground): (Token, Token)) -> Kind {
    Kind { word, glyph, hue, ground }
}

/// What he may call a dispatch, as a chip: its word, its registry glyph and its hue. A hint: the Operator still decides.
pub fn dispatch_kind(kind: ipc::DispatchKind) -> Kind {
    match kind {
        ipc::DispatchKind::Idea => self::kind("Idea", "lightbulb", REVIEW),
        ipc::DispatchKind::Feature => self::kind("Feature", "sparkles", WALK),
        ipc::DispatchKind::Bug => self::kind("Bug", "bug", FAILED),
    }
}

/// What the chip's tooltip says: the word is a hint and nothing is decided by it.
pub fn dispatch_kind_tip(kind: ipc::DispatchKind) -> &'static str {
    match kind {
        ipc::DispatchKind::Idea => "A hint that this is an idea. The Operator decides what happens to it",
        ipc::DispatchKind::Feature => "A hint that this is a feature. The Operator decides what happens to it",
        ipc::DispatchKind::Bug => "A hint that this is a bug. The Operator decides what happens to it",
    }
}

/// The three kinds he may pick, in the order the chips stand.
pub const DISPATCH_KINDS: [ipc::DispatchKind; 3] = [ipc::DispatchKind::Idea, ipc::DispatchKind::Feature, ipc::DispatchKind::Bug];

/// A red main nobody is on is a go-ahead whose item id is `red:<repository>:<commit>`.
pub fn is_red_main(turn: &OperatorTurnItem) -> bool {
    turn.item.id.starts_with("red:")
}

/// The chip of a Job's own ask, which a `job_conflict` or `job_held` item of a Job's Needs you takes as the page does.
pub fn ask_kind(source: WaitingSource) -> Option<Kind> {
    match source {
        WaitingSource::JobConflict => Some(CONFLICT),
        WaitingSource::JobHeld => Some(HELD),
        _ => None,
    }
}

const CONFLICT: Kind = kind("Conflict", "triangle-alert", REPAIR);
const HELD: Kind = kind("Held?", "git-pull-request", PULL);

/// What a Job sent the Operator, where this item is one: the card draws it in place of the item's own words.
#[derive(Clone, Copy)]
pub enum JobAsk<'a> {
    Conflict(&'a OperatorConflict),
    Held(&'a OperatorHeld),
}

pub fn job_ask(turn: &OperatorTurnItem) -> Option<JobAsk<'_>> {
    match (&turn.conflict, &turn.held) {
        (Some(conflict), _) => Some(JobAsk::Conflict(conflict)),
        (None, Some(held)) => Some(JobAsk::Held(held)),
        _ => None,
    }
}

/// A Job on a card as its chip says it: its title, else its address where it had none.
pub fn job_named(job: &ipc::OperatorAskJob) -> SharedString {
    if job.title.trim().is_empty() { job.address.clone().into() } else { job.title.clone().into() }
}

/// A held? card's title: the pull request, else the branch, which is what it asks to land.
pub fn held_title(held: &OperatorHeld) -> String {
    match (held.pull, &held.branch) {
        (Some(pull), _) => format!("Land #{pull}, or is it held?"),
        (None, Some(branch)) => format!("Land {branch}, or is it held?"),
        (None, None) => "Land it, or is it held?".to_string(),
    }
}

/// How a pull request's checks stand, as a registry glyph, its hue and its tooltip.
pub fn checks_said(checks: HeldChecks) -> (&'static str, Token, &'static str) {
    match checks {
        HeldChecks::Passed => ("shield-check", token::STATUS_COMPLETED_SUCCESS, "Checks passed"),
        HeldChecks::Pending => ("circle-dot", token::STATUS_RUNNING, "Checks running"),
        HeldChecks::Failed => ("shield-x", token::STATUS_COMPLETED_FAILED, "Checks failed"),
    }
}

/// What each outcome of a Job's ask does, for its tooltip, in the order Fleet lists them.
pub fn ask_does(ask: JobAsk<'_>, at: usize) -> Option<String> {
    match (ask, at) {
        (JobAsk::Conflict(conflict), 0) => Some(format!("{} is told to change its side", job_named(&conflict.raised_by))),
        (JobAsk::Conflict(conflict), 1) => Some(format!("{} is told to change its side", job_named(&conflict.with))),
        (JobAsk::Conflict(_), 2) => Some("Both Jobs are told to hold landing until you rule".to_string()),
        (JobAsk::Held(held), 0) => Some(format!("{} is told to land it", job_named(&held.asked_by))),
        (JobAsk::Held(held), 1) => Some(format!("{} is told to hold it", job_named(&held.asked_by))),
        _ => None,
    }
}

pub fn kind_of(turn: &OperatorTurnItem) -> Kind {
    if is_red_main(turn) {
        return kind("Red main", "circle-x", FAILED);
    }
    match job_ask(turn) {
        Some(JobAsk::Conflict(_)) => return CONFLICT,
        Some(JobAsk::Held(_)) => return HELD,
        None => {}
    }
    if let Some(ask) = ask_kind(turn.item.source) {
        return ask;
    }
    if turn.kind == OperatorTurnKind::GoAhead {
        return kind("Go-ahead", "bot", OPERATOR);
    }
    let item = &turn.item;
    match (item.source, item.act.as_ref().map(|act| act.kind)) {
        (WaitingSource::Plan, _) => kind("Plan", "list-checks", REVIEW),
        (WaitingSource::Walk, _) | (_, Some(WaitingActKind::Walk)) => kind("Walk", "eye", WALK),
        (_, Some(WaitingActKind::ApprovePr)) => kind("Pull request", "git-pull-request", PULL),
        (WaitingSource::Sketch, _) => kind("Sketch", "pencil-ruler", REVIEW),
        (WaitingSource::Document, _) => kind("Document", "file-text", REVIEW),
        (WaitingSource::Draft, _) => kind("Draft", "file-pen-line", REVIEW),
        (WaitingSource::Permission, _) => kind("Permission", "shield", REVIEW),
        (WaitingSource::AskCard, _) | (WaitingSource::WindowAsk, _) => kind("Question", "message-circle-question-mark", REVIEW),
        _ if !item.options.is_empty() => kind("Question", "message-circle-question-mark", REVIEW),
        (_, Some(WaitingActKind::Run)) => kind("Command", "terminal", REVIEW),
        (_, Some(WaitingActKind::Message)) => kind("Message", "square-pen", REVIEW),
        _ => kind("Needs you", "bell-ring", REVIEW),
    }
}

/// A go-ahead to file his note as an issue: the one item that offers Thens.
fn files(turn: &OperatorTurnItem) -> bool {
    !turn.then.is_empty()
}

/// An item's name in the list: the Job's, or the item's own words where no Job holds it.
pub fn title_of(turn: &OperatorTurnItem) -> SharedString {
    match job_ask(turn) {
        Some(JobAsk::Conflict(conflict)) => return conflict.breaks.clone().into(),
        Some(JobAsk::Held(held)) => return held_title(held).into(),
        None => {}
    }
    match &turn.job {
        Some(job) => job.clone().into(),
        None if files(turn) => format!("File your note: {}", turn.item.text).into(),
        None => turn.item.text.clone().into(),
    }
}

/// The open card's title, which says what it asks where the list's name is his note.
pub fn card_title(turn: &OperatorTurnItem) -> SharedString {
    if turn.job.is_none() && files(turn) { "File your note as an issue".into() } else { title_of(turn) }
}

/// What the card says it asks, under the title, where the title is the Job's: the item's words.
pub fn asks_of(turn: &OperatorTurnItem) -> Option<SharedString> {
    if job_ask(turn).is_some() {
        return None;
    }
    turn.job.as_ref().map(|_| turn.item.text.clone().into())
}

/// His note, on a go-ahead to file, which the card writes after when he left it: `Your note from 11:52:`.
pub fn note_of(turn: &OperatorTurnItem) -> Option<(&Instant, String)> {
    (turn.job.is_none() && files(turn)).then(|| (&turn.item.since, format!("\"{}\"", turn.item.text)))
}

/// The Operator's pick, where it read one: an empty `pick` is none.
pub fn pick_of(turn: &OperatorTurnItem) -> Option<SharedString> {
    turn.read.as_ref().map(|read| read.pick.trim()).filter(|pick| !pick.is_empty()).map(|pick| SharedString::from(pick.to_string()))
}

/// The word on the pick's button: the option it names, exactly as labelled, Later, or else "My pick",
/// whose sentence goes in its tooltip, never in the button (his ruling, 12 Nov). `chosen` is his own
/// pick among the options. With no pick, Yes.
pub fn pick_word(turn: &OperatorTurnItem, chosen: Option<&str>) -> SharedString {
    let pick = chosen.map(SharedString::from).or_else(|| pick_of(turn));
    match pick {
        None => "Yes".into(),
        Some(pick) if is_later(&pick) || turn.item.options.iter().any(|option| option.label == pick.as_ref()) => pick,
        Some(_) => "My pick".into(),
    }
}

/// The pick's sentence, where its button cannot say it: a pick that is neither an option nor Later.
pub fn pick_sentence(turn: &OperatorTurnItem) -> Option<SharedString> {
    pick_of(turn).filter(|pick| !is_later(pick) && !turn.item.options.iter().any(|option| option.label == pick.as_ref()))
}

/// Whether the Operator's pick is Later, which a Yes does.
pub fn picks_later(turn: &OperatorTurnItem) -> bool {
    pick_of(turn).is_some_and(|pick| is_later(&pick))
}

fn is_later(pick: &str) -> bool {
    pick.eq_ignore_ascii_case("later")
}

/// The Later menu: each choice's id, its words, and when it brings the item back, from `now`. A
/// morning is 09:00; next week is Monday's.
pub fn later_choices(now: DateTime<Local>) -> [(&'static str, &'static str, DateTime<Local>); 4] {
    let nine = |days: u64| {
        (now.date_naive() + chrono::Days::new(days)).and_hms_opt(9, 0, 0).and_then(|at| at.and_local_timezone(Local).earliest()).unwrap_or(now + chrono::Duration::days(days as i64))
    };
    let to_monday = 7 - u64::from(chrono::Datelike::weekday(&now).num_days_from_monday());
    [
        ("later_hour", "1 hour", now + chrono::Duration::hours(1)),
        ("later_hours", "3 hours", now + chrono::Duration::hours(3)),
        ("later_morning", "Tomorrow morning", nine(1)),
        ("later_week", "Next week", nine(to_monday)),
    ]
}

/// When a Later item comes back, as the menu and its tooltip say it, in his clock: `15:05`, `tomorrow 09:00`,
/// `Mon 16 Nov 09:00`.
pub fn back_at(at: DateTime<Local>, now: DateTime<Local>) -> String {
    let time = crate::settings::time::clock(at);
    let days = (at.date_naive() - now.date_naive()).num_days();
    match days {
        0 => time,
        1 => format!("tomorrow {time}"),
        _ => format!("{} {time}", at.format("%a %-d %b")),
    }
}

/// What a Later item's mark says: when it comes back, or the rest of today where Fleet gave no time.
pub fn later_said(turn: &OperatorTurnItem, now: DateTime<Local>) -> String {
    let at = turn.later_until.as_ref().and_then(|at| DateTime::parse_from_rfc3339(at.as_str()).ok());
    match at {
        Some(at) => format!("Later, until {}", back_at(at.with_timezone(&Local), now)),
        None => "Later today".into(),
    }
}

/// Whether the card offers the item's options to choose from: more than one, and no Then in their place.
pub fn chooses(turn: &OperatorTurnItem) -> bool {
    turn.then.is_empty() && turn.item.options.len() > 1
}

/// The answers drawn on the open item, in order, as the approved sketch offers them: a go-ahead Fleet can
/// keep a rule for takes Yes, Always here, Always and No; anything else takes its pick alone.
pub fn answers_of(turn: &OperatorTurnItem) -> Vec<OperatorAnswer> {
    // A go-ahead to file is one row of outcomes, Don't file among them; a rule is offered after he answers.
    if !turn.then.is_empty() {
        vec![OperatorAnswer::Yes, OperatorAnswer::No]
    } else if turn.keeps {
        vec![OperatorAnswer::Yes, OperatorAnswer::AlwaysHere, OperatorAnswer::Always, OperatorAnswer::No]
    } else {
        vec![OperatorAnswer::Yes]
    }
}

/// Each answer's key, as the registry binds it in the call scope: none is a digit, which picks a Then.
pub fn answer_key(answer: OperatorAnswer) -> &'static str {
    match answer {
        OperatorAnswer::Yes => "A",
        OperatorAnswer::AlwaysHere => "H",
        OperatorAnswer::Always => "⇧A",
        OperatorAnswer::No => "D",
    }
}

/// A plan is approved here, and changes to it are sent from its Job, where it is: Send changes opens that Job.
pub fn sends_changes(turn: &OperatorTurnItem) -> bool {
    turn.item.source == WaitingSource::Plan && turn.session_id.is_some()
}

pub fn answer_word(answer: OperatorAnswer) -> &'static str {
    match answer {
        OperatorAnswer::Yes => "Yes",
        OperatorAnswer::AlwaysHere => "Always here",
        OperatorAnswer::Always => "Always",
        OperatorAnswer::No => "No",
    }
}

/// A Then offered on a go-ahead to file, in the sketch's words and glyph.
/// What an answer's tooltip says it does on this item: the act Yes does, by the Then picked or the
/// Operator's pick, and where Always here and Always keep it as a rule. A decision's says what it asks.
pub fn answer_tip(turn: &OperatorTurnItem, answer: OperatorAnswer, then: Option<OperatorThen>, option: Option<&str>) -> String {
    if turn.kind == OperatorTurnKind::Decision {
        return turn.item.text.clone();
    }
    let hand_to = turn.then.iter().find_map(|offer| offer.hand_to.as_ref()).map(|to| to.job.as_str());
    let act = then.map(|then| then_said(then, hand_to).0.to_string()).or_else(|| option.map(str::to_string)).or_else(|| pick_of(turn).map(|pick| pick.to_string())).unwrap_or_else(|| "Do my pick".into());
    let here = turn.repository.as_deref().unwrap_or("this project");
    match answer {
        OperatorAnswer::Yes => format!("{act}, this once"),
        OperatorAnswer::AlwaysHere => format!("{act}, and do the same in {here} from now on without asking"),
        OperatorAnswer::Always => format!("{act}, and do the same in every project from now on without asking"),
        OperatorAnswer::No => "Do nothing, and it leaves Your turn".into(),
    }
}

pub fn then_said(then: OperatorThen, hand_to: Option<&str>) -> (SharedString, &'static str) {
    match then {
        OperatorThen::JustFile => ("File it".into(), "file"),
        OperatorThen::FileAndStart => ("File and start a Job".into(), "play"),
        OperatorThen::FileAndHand => (format!("File and hand to {}", hand_to.unwrap_or("the Job on these files")).into(), "git-branch"),
    }
}

/// What an outcome of a go-ahead to file does, for its tooltip.
pub fn then_does(then: OperatorThen, hand_to: Option<&str>) -> String {
    match then {
        OperatorThen::JustFile => "File the note as an issue".into(),
        OperatorThen::FileAndStart => "File the note as an issue, and start a Job on it".into(),
        OperatorThen::FileAndHand => format!("File the note as an issue, and hand it to {}", hand_to.unwrap_or("the Job on these files")),
    }
}

/// `Wednesday 11 November`.
pub fn day(at: &DateTime<Local>) -> SharedString {
    at.format("%A %-d %B").to_string().into()
}

/// A main's pill: what its tooltip says, and its mark and hue where Fleet has read a state that has one.
/// A main nothing ran on, or that Fleet has not read yet, is a plain pill.
pub fn main_said(main: &OperatorMain, since: Option<&str>) -> (String, Option<(&'static str, Token)>) {
    let repository = &main.repository;
    match main.state {
        Some(MainCiState::Red) => (
            since.map_or(format!("{repository} main is red"), |since| format!("{repository} main is red since {since}")),
            Some(("circle-x", token::STATUS_COMPLETED_FAILED)),
        ),
        Some(MainCiState::Green) => (format!("{repository} main is green"), Some(("circle-check", token::STATUS_COMPLETED_SUCCESS))),
        Some(MainCiState::Running) => (format!("{repository} main's checks are running"), Some(("circle-dot", token::STATUS_RUNNING))),
        Some(MainCiState::NothingRan) => (format!("Nothing ran on {repository} main"), None),
        None => (format!("{repository} main is not read yet"), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(wire: &str) -> OperatorTurnItem {
        ipc::decode("a turn item", wire.as_bytes()).expect("a turn item")
    }

    const PLAN: &str = r#"{"id":"turn:1","kind":"decision","session_id":"s-1","job":"Quick dispatch","item":{"id":"plan","text":"Approve plan version 2","since":"2026-11-11T12:00:00Z","source":"plan"},"read":{"read":"It adds the chips.","pick":"Approve"},"raised_at":"2026-11-11T12:00:00Z"}"#;
    const RED: &str = r#"{"id":"turn:2","kind":"go_ahead","item":{"id":"red:storefront:abc","text":"storefront main is red","since":"2026-11-11T12:00:00Z","source":"agent"},"raised_at":"2026-11-11T12:00:00Z","keeps":true}"#;

    #[test]
    fn a_decision_offers_its_pick_alone_and_a_kept_go_ahead_offers_always_and_no_too() {
        assert_eq!(answers_of(&turn(PLAN)), vec![OperatorAnswer::Yes]);
        assert_eq!(answers_of(&turn(RED)), vec![OperatorAnswer::Yes, OperatorAnswer::AlwaysHere, OperatorAnswer::Always, OperatorAnswer::No]);
        assert!(sends_changes(&turn(PLAN)));
        assert!(!sends_changes(&turn(RED)));
    }

    #[test]
    fn each_answer_says_what_it_does_on_the_item() {
        const NOTE: &str = r#"{"id":"turn:3","kind":"go_ahead","repository":"armada","item":{"id":"note","text":"the slot meter flickers","since":"2026-11-11T11:52:00Z","source":"agent"},"read":{"read":"No issue covers it.","pick":"File and hand to Slot pool controls"},"then":[{"then":"just_file"},{"then":"file_and_hand","hand_to":{"session_id":"s-5","job":"Slot pool controls"},"pick":true}],"raised_at":"2026-11-11T12:00:00Z","keeps":true}"#;
        let note = turn(NOTE);
        assert_eq!(answer_tip(&note, OperatorAnswer::Yes, Some(OperatorThen::FileAndHand), None), "File and hand to Slot pool controls, this once");
        assert_eq!(answer_tip(&note, OperatorAnswer::Yes, Some(OperatorThen::JustFile), None), "File it, this once");
        assert_eq!(answer_tip(&note, OperatorAnswer::AlwaysHere, Some(OperatorThen::FileAndHand), None), "File and hand to Slot pool controls, and do the same in armada from now on without asking");
        assert_eq!(answer_tip(&note, OperatorAnswer::No, None, None), "Do nothing, and it leaves Your turn");
        assert_eq!(answer_tip(&turn(PLAN), OperatorAnswer::Yes, None, None), "Approve plan version 2");
    }

    #[test]
    fn each_kind_names_its_chip_and_a_red_main_reads_as_one() {
        assert_eq!(kind_of(&turn(PLAN)).word, "Plan");
        assert_eq!(kind_of(&turn(RED)).word, "Red main");
        assert_eq!(title_of(&turn(PLAN)), "Quick dispatch");
        assert_eq!(asks_of(&turn(PLAN)).as_deref(), Some("Approve plan version 2"));
        assert_eq!(title_of(&turn(RED)), "storefront main is red");
        assert_eq!(asks_of(&turn(RED)), None);
    }

    #[test]
    fn a_main_says_its_state_and_a_main_not_read_is_a_plain_pill() {
        let main = |wire: &str| -> OperatorMain { ipc::decode("a main", wire.as_bytes()).expect("a main") };
        let red = main(r#"{"manifest_id":"s","repository":"storefront","state":"red","commit":"abc","since":"2026-11-11T11:24:00-05:00"}"#);
        assert_eq!(main_said(&red, Some("11:24")), ("storefront main is red since 11:24".into(), Some(("circle-x", token::STATUS_COMPLETED_FAILED))));
        assert_eq!(main_said(&main(r#"{"manifest_id":"a","repository":"armada","state":"running"}"#), None).1, Some(("circle-dot", token::STATUS_RUNNING)));
        assert_eq!(main_said(&main(r#"{"manifest_id":"a","repository":"armada","state":"nothing_ran"}"#), None), ("Nothing ran on armada main".into(), None));
        assert_eq!(main_said(&main(r#"{"manifest_id":"a","repository":"armada"}"#), None), ("armada main is not read yet".into(), None));
    }

    #[test]
    fn an_empty_pick_is_none() {
        let mut none = turn(PLAN);
        none.read.as_mut().expect("a read").pick = " ".into();
        assert_eq!(pick_of(&none), None);
        assert_eq!(pick_of(&turn(PLAN)).as_deref(), Some("Approve"));
    }
    #[test]
    fn a_job_ask_is_drawn_only_where_its_field_is_set_and_leads_with_what_it_says() {
        let conflict = r#"{"raised_by":{"session_id":"s-1","address":"s-9c3e41a0","title":"Upgrade the router"},"with":{"session_id":"s-2","address":"s-c41d7e88","title":""},"files":["a.rs"],"wire":[],"mine":"Moves the loop","theirs":"Adds a loop","breaks":"A 503 retries twice"}"#;
        let held = r#"{"branch":"armada/router","asked_by":{"session_id":"s-1","address":"s-9c3e41a0","title":"Upgrade the router"}}"#;
        let with = |field: &str, wire: &str| turn(&format!(r#"{{"id":"turn:40","kind":"decision","session_id":"s-1","job":"Upgrade the router","item":{{"id":"x","text":"Should it land?","since":"2026-11-11T12:00:00Z","source":"agent"}},"raised_at":"2026-11-11T12:00:00Z","{field}":{wire}}}"#));
        let (plain, asked_conflict, asked_held) = (turn(PLAN), with("conflict", conflict), with("held", held));
        assert!(job_ask(&plain).is_none() && job_ask(&asked_conflict).is_some() && job_ask(&asked_held).is_some());
        assert_eq!((kind_of(&asked_conflict).word, kind_of(&asked_conflict).glyph), ("Conflict", "triangle-alert"));
        assert_eq!((kind_of(&asked_held).word, kind_of(&asked_held).glyph), ("Held?", "git-pull-request"));
        assert_eq!(kind_of(&plain).word, "Plan");
        assert_eq!(title_of(&asked_conflict), "A 503 retries twice");
        assert_eq!(title_of(&asked_held), "Land armada/router, or is it held?", "the branch where there is no pull request");
        assert_eq!(asks_of(&asked_conflict), None);
        assert_eq!(asks_of(&plain).as_deref(), Some("Approve plan version 2"));
        let Some(JobAsk::Conflict(conflict)) = job_ask(&asked_conflict) else { panic!("a conflict") };
        assert_eq!(job_named(&conflict.with), "s-c41d7e88", "a Job with no title is named by its address");
    }

    #[test]
    fn a_pull_requests_title_names_its_number_and_checks_are_a_glyph_with_words() {
        let held: OperatorHeld = ipc::decode("a held", br#"{"pull":3671,"checks":"failed","asked_by":{"session_id":"s-1","address":"s-9c3e41a0","title":"Upgrade the router"}}"#).expect("a held");
        assert_eq!(held_title(&held), "Land #3671, or is it held?");
        assert_eq!(checks_said(HeldChecks::Passed).2, "Checks passed");
        assert_eq!(checks_said(HeldChecks::Pending).2, "Checks running");
        assert_eq!(checks_said(HeldChecks::Failed), ("shield-x", token::STATUS_COMPLETED_FAILED, "Checks failed"));
        assert_eq!(ask_does(JobAsk::Held(&held), 1).as_deref(), Some("Upgrade the router is told to hold it"));
        assert_eq!(ask_does(JobAsk::Held(&held), 2), None);
    }
}
