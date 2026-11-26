//! What the page draws for each thing Fleet serves, as words, a registry glyph and a hue: the kind chip
//! of an item, its mark in the list, a time said the way the sketch says it. Nothing here is worked
//! out from more than the one field it reads.

use chrono::{DateTime, Local};
use gpui::SharedString;
use ipc::operator_page::{OperatorFeedKind, OperatorMain, OperatorThen, OperatorTurnItem, OperatorTurnKind};
use ipc::{MainCiState, OperatorCardKind, WaitingActKind, WaitingSource};

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

const fn kind(word: &'static str, glyph: &'static str, (hue, ground): (Token, Token)) -> Kind {
    Kind { word, glyph, hue, ground }
}

/// What a dispatch is marked as, drawn: its word, its registry glyph and its hue. A hint: the Operator still decides.
pub fn dispatch_kind(kind: ipc::DispatchKind) -> Kind {
    match kind {
        ipc::DispatchKind::Idea => self::kind("Idea", "lightbulb", REVIEW),
        ipc::DispatchKind::Feature => self::kind("Feature", "sparkles", WALK),
        ipc::DispatchKind::Bug => self::kind("Bug", "bug", FAILED),
    }
}

/// A red main nobody is on is a go-ahead whose item id is `red:<repository>:<commit>`.
pub fn is_red_main(turn: &OperatorTurnItem) -> bool {
    turn.item.id.starts_with("red:")
}

/// What a card the Operator raised is called on its chip: a thing it raises, or a decision it brings with its read.
pub fn card_kind(kind: OperatorCardKind) -> Kind {
    match kind {
        OperatorCardKind::Raise => self::kind("Raised", "flag", OPERATOR),
        OperatorCardKind::Deal => self::kind("To deal with", "list-todo", OPERATOR),
    }
}

/// A card he picked that came back to the front: the Operator reported it failed, or nothing settled it in time.
pub fn returned(turn: &OperatorTurnItem) -> bool {
    turn.card.as_ref().is_some_and(|card| card.returned.is_some())
}

/// The first line of a returned card's read: what he picked, and why it came back.
pub fn returned_lead(turn: &OperatorTurnItem) -> Option<SharedString> {
    let card = turn.card.as_ref()?;
    let back = card.returned.as_ref()?;
    Some(match &card.picked {
        Some(picked) => format!("You picked {}. {}", picked.option, back.why).into(),
        None => back.why.clone().into(),
    })
}

/// A card as Fleet serves it once he has pressed `option` at `at`: picked, with no steps yet and nothing returned.
/// The page shows it so at once, before Fleet's page says so.
pub fn picked_card(turn: &OperatorTurnItem, option: &str, at: &str) -> Option<OperatorTurnItem> {
    let card = turn.card.as_ref()?;
    let picked = ipc::OperatorCardPicked { option: option.to_string(), at: ipc::Instant::carried(at) };
    Some(OperatorTurnItem { card: Some(ipc::OperatorCard { picked: Some(picked), steps: Vec::new(), returned: None, ..card.clone() }), ..turn.clone() })
}

/// The option he picked on a card the Operator is working on.
pub fn picked_option(turn: &OperatorTurnItem) -> Option<SharedString> {
    turn.card.as_ref()?.picked.as_ref().map(|picked| picked.option.clone().into())
}

/// One entry of the feed: its glyph, its hue and the word its tooltip says.
pub fn feed_kind(kind: OperatorFeedKind) -> (&'static str, Token, &'static str) {
    match kind {
        OperatorFeedKind::Dispatched => ("rocket", token::STATUS_RUNNING, "Started a Job"),
        OperatorFeedKind::Merged => ("git-merge", token::STATUS_PILOTED, "Merged"),
        OperatorFeedKind::Ended => ("circle-check", token::STATUS_COMPLETED_SUCCESS, "A Job ended"),
        OperatorFeedKind::Asked => ("bot", token::OPERATOR, "A Job asked"),
        OperatorFeedKind::Told => ("bot", token::OPERATOR, "A Job told the Operator"),
        OperatorFeedKind::Settled => ("check", token::ACCENT, "Settled"),
    }
}

/// A question: an Operator card's chip, and a Job's own question as Needs you draws it on the same card.
pub const QUESTION: Kind = kind("Question", "message-circle-question-mark", REVIEW);

/// The Approve a Job raises when it thinks its step is done, on a step that ends on his Approve: the card Needs you draws for it.
pub const STEP_DONE: Kind = kind("Step done", "circle-check", REVIEW);

pub fn kind_of(turn: &OperatorTurnItem) -> Kind {
    if returned(turn) {
        return kind("Did not settle", "circle-alert", FAILED);
    }
    if let Some(card) = &turn.card {
        return card_kind(card.kind);
    }
    if is_red_main(turn) {
        return kind("Red main", "circle-x", FAILED);
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
        (WaitingSource::ReadyToTry, _) => kind("Ready to try", "bell-ring", REVIEW),
        (WaitingSource::Permission, _) => kind("Permission", "shield", REVIEW),
        (WaitingSource::AskCard, _) | (WaitingSource::WindowAsk, _) => QUESTION,
        _ if !item.options.is_empty() => QUESTION,
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
    turn.job.as_ref().map(|_| turn.item.text.clone().into())
}

/// The Operator's pick, where it read one: an empty `pick` is none.
pub fn pick_of(turn: &OperatorTurnItem) -> Option<SharedString> {
    turn.read.as_ref().map(|read| read.pick.trim()).filter(|pick| !pick.is_empty()).map(|pick| SharedString::from(pick.to_string()))
}

/// The word on the pick's button: the option it names, exactly as labelled, Later, or else "My pick",
/// whose sentence goes in its tooltip, never in the button (his ruling, 12 Nov). `chosen` is his own
/// pick among the options. With no pick, Yes.
pub fn pick_word(turn: &OperatorTurnItem, chosen: Option<&str>) -> SharedString {
    if goes_to_look(turn) {
        return "Go look".into();
    }
    if goes_to_review(turn) {
        return "Review".into();
    }
    let pick = chosen.map(SharedString::from).or_else(|| pick_of(turn));
    match pick {
        None => "Yes".into(),
        Some(pick) if is_later(&pick) || turn.item.options.iter().any(|option| option.label == pick.as_ref()) => pick,
        Some(_) => "My pick".into(),
    }
}

/// The pick's sentence, where its button cannot say it: a pick that is neither an option nor Later.
pub fn pick_sentence(turn: &OperatorTurnItem) -> Option<SharedString> {
    // The pick of an item read in the Review panel stands there, under the Operator's name.
    if goes_to_review(turn) {
        return None;
    }
    pick_of(turn).filter(|pick| !is_later(pick) && !turn.item.options.iter().any(|option| option.label == pick.as_ref()))
}

/// A walk a Job holds, with no pick of Later: its press opens the Job to look at it, and says no Yes.
pub fn goes_to_look(turn: &OperatorTurnItem) -> bool {
    kind_of(turn).word == "Walk" && turn.session_id.is_some() && !picks_later(turn)
}

/// A plan, sketch, page or draft a Job holds, with no pick of Later: its press opens the Review panel over the page with it
/// chosen, where it is read and answered, and says no Yes.
pub fn goes_to_review(turn: &OperatorTurnItem) -> bool {
    turn.card.is_none()
        && turn.session_id.is_some()
        && matches!(turn.item.source, WaitingSource::Plan | WaitingSource::Sketch | WaitingSource::Document | WaitingSource::Draft)
        && !returned(turn)
        && !picks_later(turn)
}

/// A Job's Ready to try: its answers are the shared card's own, drawn on the page, so the page's keys answer nothing.
pub fn answers_on_card(turn: &OperatorTurnItem) -> bool {
    turn.card.is_none() && turn.session_id.is_some() && turn.item.source == WaitingSource::ReadyToTry && !returned(turn)
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
    let held = crate::settings::time::held();
    let time = crate::settings::time::time_of(at, ipc::time_format::TimeFormat { seconds: false, ..held });
    let days = (at.date_naive() - now.date_naive()).num_days();
    match days {
        0 => time,
        1 => format!("tomorrow {time}"),
        _ => format!("{} {time}", at.format("%a %-d %b")),
    }
}

/// Whether the card offers the item's options to choose from: more than one, and no Then in their place.
pub fn chooses(turn: &OperatorTurnItem) -> bool {
    turn.then.is_empty() && turn.item.options.len() > 1
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
    const WALK: &str = r#"{"id":"turn:3","kind":"decision","session_id":"s-2","job":"Native Settings","item":{"id":"walk","text":"armada://walk/native-settings-pages","since":"2026-11-11T12:00:00Z","source":"walk"},"raised_at":"2026-11-11T12:00:00Z"}"#;

    #[test]
    fn a_walk_a_job_holds_says_go_look_not_yes() {
        let walk = turn(WALK);
        assert!(goes_to_look(&walk));
        assert_eq!(pick_word(&walk, None).as_ref(), "Go look");
        assert!(!goes_to_look(&turn(PLAN)), "a plan still takes its pick");
    }

    #[test]
    fn a_plan_a_job_holds_says_review_and_a_build_to_try_answers_on_its_own_card() {
        let plan = turn(PLAN);
        assert!(goes_to_review(&plan));
        assert_eq!(pick_word(&plan, None).as_ref(), "Review");
        assert_eq!(pick_sentence(&plan), None, "the pick stands in the Review panel");
        let mut later = turn(PLAN);
        later.read.as_mut().expect("a read").pick = "Later".into();
        assert!(!goes_to_review(&later), "a pick of Later is still done by Yes");
        assert!(!goes_to_review(&turn(WALK)) && !goes_to_review(&turn(RED)));
        let build = turn(r#"{"id":"turn:4","kind":"decision","session_id":"s-3","job":"Native Bridge foundation","item":{"id":"try:s-3","text":"Ready to try","since":"2026-11-11T12:00:00Z","source":"ready_to_try"},"raised_at":"2026-11-11T12:00:00Z"}"#);
        assert!(answers_on_card(&build) && !goes_to_review(&build));
        assert!(!answers_on_card(&plan));
        assert_eq!(kind_of(&build).word, "Ready to try", "its chip in Next up names the kind whole");
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
    fn a_card_names_its_kind_on_its_chip_and_a_feed_entry_its_glyph() {
        assert_eq!(card_kind(OperatorCardKind::Raise).word, "Raised");
        assert_eq!(card_kind(OperatorCardKind::Deal).word, "To deal with");
        assert_eq!(feed_kind(OperatorFeedKind::Merged).0, "git-merge");
        assert_eq!(feed_kind(OperatorFeedKind::Dispatched).0, "rocket");
    }

    #[test]
    fn an_empty_pick_is_none() {
        let mut none = turn(PLAN);
        none.read.as_mut().expect("a read").pick = " ".into();
        assert_eq!(pick_of(&none), None);
        assert_eq!(pick_of(&turn(PLAN)).as_deref(), Some("Approve"));
    }
}
