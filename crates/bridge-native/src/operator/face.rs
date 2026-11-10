//! What the page draws for each thing Fleet serves, as words, a registry glyph and a hue: the kind chip
//! of an item, its mark in the list, a time said the way the sketch says it. Nothing here is worked
//! out from more than the one field it reads.

use chrono::{DateTime, Local};
use gpui::SharedString;
use ipc::operator_page::{OperatorAnswer, OperatorThen, OperatorTurnItem, OperatorTurnKind};
use ipc::{Instant, WaitingActKind, WaitingSource};

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

/// A red main nobody is on is a go-ahead whose item id is `red:<repository>:<commit>`.
pub fn is_red_main(turn: &OperatorTurnItem) -> bool {
    turn.item.id.starts_with("red:")
}

pub fn kind_of(turn: &OperatorTurnItem) -> Kind {
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
        (WaitingSource::Permission, _) => kind("Permission", "shield", REVIEW),
        (WaitingSource::AskCard, _) | (WaitingSource::WindowAsk, _) => kind("Question", "message-circle-question", REVIEW),
        _ if !item.options.is_empty() => kind("Question", "message-circle-question", REVIEW),
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

/// What the card says it asks, under the title: the item's words where the title is the Job's, and
/// his note with when he left it on a go-ahead to file.
pub fn asks_of(turn: &OperatorTurnItem) -> Option<SharedString> {
    match &turn.job {
        Some(_) => Some(turn.item.text.clone().into()),
        None if files(turn) => Some(format!("Your note from {}: \"{}\"", clock(&turn.item.since), turn.item.text).into()),
        None => None,
    }
}

/// The Operator's pick, where it read one: an empty `pick` is none.
pub fn pick_of(turn: &OperatorTurnItem) -> Option<SharedString> {
    turn.read.as_ref().map(|read| read.pick.trim()).filter(|pick| !pick.is_empty()).map(|pick| SharedString::from(pick.to_string()))
}

/// The answers drawn on the open item, in order, as the approved sketch offers them: a go-ahead Fleet can
/// keep a rule for takes Yes, Always here, Always and No; anything else takes its pick alone.
pub fn answers_of(turn: &OperatorTurnItem) -> Vec<OperatorAnswer> {
    if turn.keeps {
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
pub fn then_said(then: OperatorThen, hand_to: Option<&str>) -> (SharedString, &'static str) {
    match then {
        OperatorThen::JustFile => ("Just file it".into(), "file"),
        OperatorThen::FileAndStart => ("File and start a Job".into(), "play"),
        OperatorThen::FileAndHand => (format!("File and hand to {}", hand_to.unwrap_or("the Job on these files")).into(), "git-branch"),
    }
}

fn local(at: &Instant) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(at.as_str()).ok().map(|at| at.with_timezone(&Local))
}

/// `13:05`, in this Mac's time.
pub fn clock(at: &Instant) -> SharedString {
    local(at).map(|at| at.format("%H:%M").to_string()).unwrap_or_default().into()
}

/// `Wednesday 11 November`.
pub fn day(at: &DateTime<Local>) -> SharedString {
    at.format("%A %-d %B").to_string().into()
}

/// The head's pills, beside the date. `operator_page` serves none of them: the week is the title bar's
/// reading, and a red main and the slots are drawn only where something hands them over (the walk's mock).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Standing {
    /// Each repository's main: its name and whether it is red.
    pub mains: Vec<(SharedString, bool)>,
    /// The week's share of the plan used, in whole percent.
    pub week: Option<u32>,
    /// Slots held, and slots there are.
    pub slots: Option<(u32, u32)>,
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
    fn each_kind_names_its_chip_and_a_red_main_reads_as_one() {
        assert_eq!(kind_of(&turn(PLAN)).word, "Plan");
        assert_eq!(kind_of(&turn(RED)).word, "Red main");
        assert_eq!(title_of(&turn(PLAN)), "Quick dispatch");
        assert_eq!(asks_of(&turn(PLAN)).as_deref(), Some("Approve plan version 2"));
        assert_eq!(title_of(&turn(RED)), "storefront main is red");
        assert_eq!(asks_of(&turn(RED)), None);
    }

    #[test]
    fn an_empty_pick_is_none() {
        let mut none = turn(PLAN);
        none.read.as_mut().expect("a read").pick = " ".into();
        assert_eq!(pick_of(&none), None);
        assert_eq!(pick_of(&turn(PLAN)).as_deref(), Some("Approve"));
    }
}
