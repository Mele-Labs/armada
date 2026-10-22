//! Every ask of a reply's close, one item each with the act its words name, and the follow-ups the
//! agent keeps for itself. `docs/spikes/reply-sentence-readers.md` has the two heads measured.
//!
//! **One sentence, one read.** The asks reader of [`super::asks_model`] reads the whole close for one
//! sentence; a reply asks several things at once, so here each sentence of the close is read alone by
//! two heads, and what it says is turned into an act by rules. Pure below [`read`]: the probabilities
//! come in, so every rule is tested without a model.

use adapter_traits::{NotEmbedded, TextModel, TextReading};

use super::asks::{self, Closing};
use super::clarity;

/// Whether the sentence readers are in force. **They are not**: both heads are measured and the owner
/// has not picked. While off, a reply raises one item by the asks reader, as before.
pub(crate) const IN_FORCE: bool = false;

/// The probability at and above which a sentence asks the person: the most recall at which the dev
/// half's five folds by reply (1,564 sentences, 125 asks) kept precision at 0.90, for `sentence_asks`
/// trained 8 epochs. Held out, it read 0.90 precision and 0.20 recall at this point, 21 Oct 2026.
pub(crate) const ASKS_AT: f32 = 0.85;

/// The probability at and above which a sentence is a follow-up the agent keeps: chosen the same way
/// on the same folds (217 follow-ups), for `sentence_follows_up`. Held out, 0.86 precision and 0.53 recall.
pub(crate) const FOLLOWS_UP_AT: f32 = 0.81;

/// What pressing the item does, read from the sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Act {
    /// A shell command fenced right after the sentence.
    Run(String),
    /// A pull request the sentence names with approve, merge or land: `#N`.
    ApprovePr(String),
    /// A reply in the Session's box, or one of `Ask::options`.
    Answer,
}

/// One thing the reply asks of the person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Ask {
    pub sentence: String,
    /// Its place among the reply's asks: the `n` of `replied:<ended>:<n>`.
    pub index: usize,
    pub act: Act,
    /// The choices an `Answer` offers, where the sentence lists them; empty where it does not.
    pub options: Vec<String>,
}

/// What the readers made of one reply.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Extracted {
    pub asks: Vec<Ask>,
    /// The follow-ups the agent keeps for itself, as said.
    pub later: Vec<String>,
}

/// The sentences the heads read: the close with its headings left out.
pub(crate) fn candidates(reply: &str) -> Vec<Closing> {
    asks::closing(reply)
}

/// The reply read by both heads, a sentence at a time.
pub(crate) fn read(model: &dyn TextModel, reply: &str) -> Result<Extracted, NotEmbedded> {
    let close = candidates(reply);
    let texts: Vec<&str> = close.iter().map(|one| one.sentence.as_str()).collect();
    let asks = model.read(TextReading::SentenceAsks, &texts)?;
    let later = model.read(TextReading::SentenceFollowsUp, &texts)?;
    let clear = if clarity::IN_FORCE { Some(model.read(TextReading::ItemClear, &texts)?) } else { None };
    Ok(extracted(reply, &close, &asks, &later, clear.as_deref()))
}

/// From the close's sentences and each one's two probabilities. The rules' own sentence is the first
/// ask where the rules call it, so nothing the asks reader catches today is lost.
///
/// `clear` is each sentence's probability of being clear to him, from the `item_clear` head, and is
/// `Some` only where [`clarity::IN_FORCE`]. Then an ask is a candidate that must read as clear, by the
/// rules and by the head, and one that does not is left out: a reply whose asks are all unclear raises
/// no item, and the others are numbered again among themselves.
pub(crate) fn extracted(reply: &str, close: &[Closing], asks: &[f32], later: &[f32], clear: Option<&[f32]>) -> Extracted {
    let rules = asks::asking_line(reply);
    let mut found: Vec<&str> = Vec::new();
    if let Some(line) = &rules {
        found.push(line);
    }
    for (one, score) in close.iter().zip(asks) {
        if *score >= ASKS_AT && !found.contains(&one.sentence.as_str()) {
            found.push(&one.sentence);
        }
    }
    let asked: Vec<Ask> = found
        .iter()
        .enumerate()
        .map(|(index, sentence)| {
            let at = close.iter().position(|one| one.sentence == *sentence);
            let act = at.and_then(|at| close[at].command.clone()).map_or_else(|| act_of(sentence), |command| Act::Run(command.trim().to_string()));
            let options = if act == Act::Answer { options_of(sentence, close, at) } else { Vec::new() };
            Ask { sentence: sentence.to_string(), index, act, options }
        })
        .collect();
    let asked = match clear {
        Some(clear) => keep_clear(asked, close, clear),
        None => asked,
    };
    let later = close
        .iter()
        .zip(asks)
        .zip(later)
        .filter(|((one, asks), later)| **later >= FOLLOWS_UP_AT && **asks < ASKS_AT && !found.contains(&one.sentence.as_str()))
        .map(|((one, _), _)| one.sentence.clone())
        .collect();
    Extracted { asks: asked, later }
}

/// The asks that read as clear, numbered again among themselves: the rules find nothing wrong with the
/// sentence, and where the head read it, it does not find it unclear. The rules' own sentence may not be
/// one of the close's, and then only the rules read it.
fn keep_clear(asked: Vec<Ask>, close: &[Closing], clear: &[f32]) -> Vec<Ask> {
    asked
        .into_iter()
        .filter(|ask| {
            let runs = matches!(ask.act, Act::Run(_));
            let head = close.iter().position(|one| one.sentence == ask.sentence).and_then(|at| clear.get(at));
            clarity::unclear(&ask.sentence, runs).is_none() && head.is_none_or(|clear| 1.0 - clear < clarity::UNCLEAR_AT)
        })
        .enumerate()
        .map(|(index, ask)| Ask { index, ..ask })
        .collect()
}

/// A pull request to approve where the sentence names one with approve, merge or land; else an answer.
fn act_of(sentence: &str) -> Act {
    let said = sentence.to_lowercase();
    let names = ["approve", "merge", "land"].iter().any(|word| said.contains(word));
    let number = sentence.split(|c: char| c.is_whitespace() || matches!(c, '(' | '[' | ',' | ';')).find_map(|word| {
        let digits = word.strip_prefix('#')?.trim_end_matches(|c: char| !c.is_ascii_digit());
        (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then(|| format!("#{digits}"))
    });
    match number {
        Some(number) if names => Act::ApprovePr(number),
        _ => Act::Answer,
    }
}

/// The choices a question offers: the alternatives of an "X, or Y?", each without the question's
/// lead, or the items of the list the sentence closes. Empty where neither shape fits.
pub(crate) fn options_of(sentence: &str, close: &[Closing], at: Option<usize>) -> Vec<String> {
    let choices = alternatives(sentence);
    if !choices.is_empty() {
        return choices;
    }
    at.map(|at| listed_before(close, at)).unwrap_or_default()
}

/// What a question stem says before the choice, in lower case; the longer first, so "do you want me to"
/// is taken whole.
const LEADS: &[&str] = &[
    "do you want me to ",
    "would you like me to ",
    "do you want it ",
    "do you want them ",
    "do you want the ",
    "do you want ",
    "would you like ",
    "would you rather ",
    "would you prefer ",
    "want me to ",
    "shall i ",
    "should i ",
    "shall we ",
    "should we ",
    "do i ",
    "can i ",
    "could i ",
    "may i ",
    "is it ",
    "is this ",
    "is that ",
    "which is it: ",
    "which: ",
    "which one: ",
];

/// What a question starts with when its whole first clause is the stem and the choices follow a comma.
const QUESTION_WORDS: &[&str] = &["which ", "what ", "how ", "where ", "who ", "do ", "does ", "should ", "would ", "is ", "are ", "can ", "could ", "shall ", "will "];

/// "X, or Y?" and "X, Y, or Z?": the alternatives, each trimmed of the lead and capitalised.
fn alternatives(sentence: &str) -> Vec<String> {
    let body = sentence.trim().trim_end_matches(['*', '"', '\'', ')']);
    let Some(body) = body.strip_suffix('?') else {
        return Vec::new();
    };
    let Some(or) = body.rfind(" or ") else {
        return Vec::new();
    };
    let (left, right) = (body[..or].trim().trim_end_matches(','), body[or + 4..].trim());
    let mut parts: Vec<&str> = left.split(", ").map(str::trim).filter(|part| !part.is_empty()).collect();
    parts.push(right);
    if parts.len() < 2 {
        return Vec::new();
    }
    let first = parts[0].to_lowercase();
    let lead = LEADS.iter().find(|lead| first.starts_with(*lead)).map_or(0, |lead| lead.len());
    // "Should the registry be read once at start": the lead ends after the verb that follows "be".
    let lead = if lead == 0 && (first.starts_with("should ") || first.starts_with("is ") || first.starts_with("does ")) {
        first.find(" be ").and_then(|be| first[be + 4..].find(' ').map(|verb| be + 4 + verb + 1)).unwrap_or(0)
    } else {
        lead
    };
    // "Which page size do you want, 20 or 50?": a stem no lead names, before two or more choices.
    let stem = lead == 0 && parts.len() > 2 && QUESTION_WORDS.iter().any(|word| first.starts_with(word));
    if stem {
        parts.remove(0);
    } else {
        parts[0] = parts[0][lead..].trim();
    }
    let options: Vec<String> = parts.iter().map(|part| capitalised(part)).collect();
    if options.iter().any(|one| one.is_empty() || one.split_whitespace().count() > 8) {
        return Vec::new();
    }
    options
}

/// What a sentence says when the list before it is the choice: "Which do you want?", "Pick one."
const CHOOSING: &[&str] = &["which", "pick", "choose", "prefer", "one of", "option"];

/// The longest a choice runs; a longer list line is a finding, not something to pick.
const CHOICE_WORDS: usize = 8;

/// The lines of the list the sentence at `at` closes, where it says to choose among them: the run of
/// list lines right before its own line, when it is not one of them. Two to six, each short.
fn listed_before(close: &[Closing], at: usize) -> Vec<String> {
    let said = close[at].sentence.to_lowercase();
    if close[at].listed || !CHOOSING.iter().any(|cue| said.contains(cue)) {
        return Vec::new();
    }
    let mut options = Vec::new();
    let mut line = close[at].line;
    while line > 0 {
        line -= 1;
        let parts: Vec<&str> = close.iter().filter(|one| one.line == line && one.listed).map(|one| one.sentence.as_str()).collect();
        if parts.is_empty() {
            break;
        }
        options.push(capitalised(parts.join(" ").trim_end_matches('.')));
    }
    options.reverse();
    if !(2..=6).contains(&options.len()) || options.iter().any(|one| one.split_whitespace().count() > CHOICE_WORDS) {
        return Vec::new();
    }
    options
}

fn capitalised(text: &str) -> String {
    let text = text.trim();
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
