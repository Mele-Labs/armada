//! The question put to a small model about one message of his, and what it answered. Pure: `super::reading` runs it.
//!
//! **The model sorts, he corrects, and the corrections teach.** The newest few of his changes ride in the prompt as
//! examples, so a kind he keeps changing stops being sorted that way. An answer that cannot be read leaves the message
//! unsorted rather than guessed at.

use ipc::{AskKind, MessageKind};
use store::KeptCorrection;

/// How many of his corrections go in the prompt as examples.
pub(crate) const EXAMPLES: u32 = 8;
/// The turns before the message that go in the prompt.
pub(crate) const TURNS: usize = 6;
/// The most of one turn, or of his message, the prompt carries.
const TURN_CHARS: usize = 500;
const WORDS_CHARS: usize = 1500;
/// The longest title kept.
pub(crate) const TITLE_CHARS: usize = 80;

pub(crate) const KIND: &str = "kind:";
pub(crate) const ASK_KIND: &str = "ask_kind:";
pub(crate) const ASK: &str = "ask:";
pub(crate) const TITLE: &str = "title:";

/// An ask the Job holds that is not built or dropped: what a message may revise or answer. **The prompt numbers them from 1**
/// and the answer is read back by number, since a small model copies a short number and not a 26 character id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OpenAsk {
    pub id: String,
    pub kind: AskKind,
    pub title: String,
}

/// A turn of the thread before his message: who said it, and what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Turn {
    pub person: bool,
    pub text: String,
}

/// What the model made of a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Sorted {
    pub kind: MessageKind,
    /// For a `new_ask`.
    pub ask_kind: Option<AskKind>,
    /// For a `revision` or an `answer`: an ask of the Job.
    pub ask: Option<String>,
    /// For a `new_ask`.
    pub title: Option<String>,
}

pub(crate) fn kind_word(kind: MessageKind) -> &'static str {
    match kind {
        MessageKind::NewAsk => "new_ask",
        MessageKind::Revision => "revision",
        MessageKind::Answer => "answer",
        MessageKind::Talk => "talk",
    }
}

pub(crate) fn ask_kind_word(kind: AskKind) -> &'static str {
    match kind {
        AskKind::Bug => "bug",
        AskKind::ChangeRequest => "change_request",
        AskKind::Idea => "idea",
        AskKind::Fix => "fix",
    }
}

pub(crate) fn ask_kind_of(word: &str) -> Option<AskKind> {
    match word.trim().to_lowercase().as_str() {
        "bug" => Some(AskKind::Bug),
        "change_request" | "change request" | "change" => Some(AskKind::ChangeRequest),
        "idea" => Some(AskKind::Idea),
        "fix" => Some(AskKind::Fix),
        _ => None,
    }
}

fn kind_of(word: &str) -> Option<MessageKind> {
    match word.trim().to_lowercase().as_str() {
        "new_ask" | "new ask" => Some(MessageKind::NewAsk),
        "revision" => Some(MessageKind::Revision),
        "answer" => Some(MessageKind::Answer),
        "talk" => Some(MessageKind::Talk),
        _ => None,
    }
}

pub(crate) fn cut(text: &str, at: usize) -> String {
    let text = text.trim();
    match text.char_indices().nth(at) {
        Some((end, _)) => format!("{}...", text[..end].trim_end()),
        None => text.to_string(),
    }
}

/// A title for an ask whose model gave none: his first line, cut.
pub(crate) fn title_from(words: &str) -> String {
    let line = words.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or("Untitled");
    cut(line, TITLE_CHARS)
}

/// What the model is asked.
pub(crate) fn question(words: &str, asks: &[OpenAsk], turns: &[Turn], corrections: &[KeptCorrection]) -> String {
    let listed = match asks.is_empty() {
        true => String::from("none"),
        false => asks.iter().enumerate().map(|(at, ask)| format!("- {}: [{}] {}", at + 1, ask_kind_word(ask.kind), ask.title)).collect::<Vec<_>>().join("\n"),
    };
    let before = match turns.is_empty() {
        true => String::from("none"),
        false => turns
            .iter()
            .map(|turn| format!("{}: {}", if turn.person { "person" } else { "agent" }, cut(&turn.text, TURN_CHARS)))
            .collect::<Vec<_>>()
            .join("\n"),
    };
    let examples = match corrections.is_empty() {
        true => String::new(),
        false => {
            let each: Vec<String> = corrections
                .iter()
                .map(|one| {
                    let ask_kind = one.ask_kind.as_deref().map(|kind| format!(", {ASK_KIND} {kind}")).unwrap_or_default();
                    format!("Message: {}\nRight sorting: {KIND} {}{ask_kind}", cut(&one.words, TURN_CHARS), one.kind)
                })
                .collect();
            format!("Messages the person sorted himself, which are right:\n\n{}\n\n", each.join("\n\n"))
        }
    };
    format!(
        "A person sent one message to a coding agent working on a software project. Sort it.\n\n\
         Kinds:\n\
         new_ask: asks for something new: a bug to fix, a change, an idea, or a repair.\n\
         revision: changes or adds to an open ask listed below.\n\
         answer: a pick, an approval or a reply to something the agent asked or showed, joined to the open ask it is about.\n\
         talk: anything else, such as a question, thanks or discussion. It opens nothing.\n\n\
         Ask kinds: bug (something is broken), change_request (something to change or build), idea (something to consider), fix (a repair he names).\n\n\
         Open asks of this job:\n{listed}\n\n\
         The last turns before the message:\n{before}\n\n\
         {examples}\
         The message:\n{words}\n\n\
         Answer in four lines and nothing else:\n\
         {KIND} new_ask, revision, answer or talk\n\
         {ASK_KIND} bug, change_request, idea or fix for a new_ask, else none\n\
         {ASK} the number of the open ask a revision or answer joins, else none\n\
         {TITLE} for a new_ask, a title of at most eight words in the person's own terms, else none",
        words = cut(words, WORDS_CHARS),
    )
}

/// The first number in `said`, as the model writes the ask it means: `2`, `ask 2` or `#2`.
pub(crate) fn number_in(said: &str) -> Option<usize> {
    let digits: String = said.chars().skip_while(|c| !c.is_ascii_digit()).take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The model's answer, read. `None` where it names no kind, which leaves the message unsorted.
///
/// **A revision or an answer that names no open ask is talk**: it cannot be joined to anything, and one press makes it what he meant.
/// A new ask with no kind is a change request, and with no title takes his first line.
pub(crate) fn read_of(reply: &str, words: &str, asks: &[OpenAsk]) -> Option<Sorted> {
    let field = |name: &str| {
        reply.lines().find_map(|line| {
            let line = line.trim().trim_start_matches(['-', '*']).trim();
            line.to_lowercase().strip_prefix(name).map(|_| line[name.len()..].trim().to_string())
        })
    };
    let kind = kind_of(&field(KIND)?)?;
    let none = |word: Option<String>| word.filter(|said| !said.is_empty() && !said.eq_ignore_ascii_case("none"));
    Some(match kind {
        MessageKind::NewAsk => Sorted {
            kind,
            ask_kind: Some(none(field(ASK_KIND)).and_then(|word| ask_kind_of(&word)).unwrap_or(AskKind::ChangeRequest)),
            ask: None,
            title: Some(none(field(TITLE)).map(|title| cut(&title, TITLE_CHARS)).unwrap_or_else(|| title_from(words))),
        },
        MessageKind::Revision | MessageKind::Answer => {
            let joined = none(field(ASK)).and_then(|said| number_in(&said)).and_then(|number| asks.get(number.checked_sub(1)?)).map(|ask| ask.id.clone());
            match joined {
                Some(ask) => Sorted { kind, ask_kind: None, ask: Some(ask), title: None },
                None => Sorted { kind: MessageKind::Talk, ask_kind: None, ask: None, title: None },
            }
        }
        MessageKind::Talk => Sorted { kind, ask_kind: None, ask: None, title: None },
    })
}
