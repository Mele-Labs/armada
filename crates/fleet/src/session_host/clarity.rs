//! Whether an agent's waiting item is clear to the person reading it. `docs/spikes/item-clarity.md`
//! has the rules and the head measured.
//!
//! **A clear item holds a verb aimed at him and the thing it names**, reads alone, and is not a status
//! report. A fenced command handed over is the verb and the thing. "Bigger than it looked?" is none of
//! these: it ends in `?`, and was shown to him as his question. Pure, so the rules are tested without a
//! model; `item_clear` reads the same items beside them.

/// Whether this reader is in force: `waiting_for` refuses an unclear item, and a reply's derived item
/// takes its clearest sentence. **It is**: the rules and the head are measured and the owner picked
/// it on 22 Oct 2026.
pub(crate) const IN_FORCE: bool = true;

/// The score for "unclear", one minus the head's probability of clear, at and above which the head calls
/// an item unclear: the most recall at which the dev half's five folds by session kept precision at 0.90
/// beside the rules (a head alone needs 0.72). `docs/spikes/item-clarity.md` has what it cost held out.
pub(crate) const UNCLEAR_AT: f32 = 0.64;

/// Why an item cannot be read alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unclear {
    /// It points at something else he would have to find: "see above", "three options".
    Pointer,
    /// It reports where things stand and asks nothing of him.
    Status,
    /// It holds no verb aimed at him, or none with a thing it names.
    NoAsk,
}

impl Unclear {
    /// What the agent reads for why the item was refused.
    pub(crate) fn why(self) -> &'static str {
        match self {
            Unclear::Pointer => "it points at something else he would have to find, and an item is read alone",
            Unclear::Status => "it reports where things stand and asks nothing of him",
            Unclear::NoAsk => "it holds no verb aimed at him with the thing it names",
        }
    }
}

/// Words that point at what is said elsewhere, matched whole once lowercased.
const POINTING: &[&str] = &[
    "elsewhere",
    "as before",
    "as discussed",
    "as mentioned",
    "my last message",
    "my previous message",
    "my earlier message",
    "my message",
    "my reply",
    "the thread",
];

/// What a count of choices is a count of, when the choices are not listed: "three options".
const CHOICES: &[&str] = &["choices", "options", "questions", "decisions", "ways", "approaches", "takes", "directions"];

const NUMBERS: &[&str] = &["two", "three", "four", "five", "six", "several", "few"];

/// What he is asked to do, as a clause's first word.
const VERBS: &[&str] = &[
    "accept", "add", "allow", "answer", "approve", "ask", "bind", "bring", "build", "cancel", "check", "choose", "clean", "clear", "click", "close", "confirm", "copy", "create", "cut", "decide", "delete", "describe", "disable", "document", "draw", "drop", "enable", "explain", "file", "fix", "fold", "free", "give", "grant", "hide", "hold", "install", "keep", "kill", "land", "let", "log", "look", "make", "merge", "mock", "move", "name", "open", "pause", "paste", "pick", "press", "pull", "push", "quit", "read", "rebase", "rebuild", "refresh", "register", "reject", "release", "reload", "remove", "rename", "reply", "rerun", "restart", "resume", "retry", "revert", "review", "reverse", "run", "save", "say", "select", "send", "set", "share", "show", "sign", "sketch", "skip", "split", "start", "stop", "switch", "tell", "test", "track", "try", "tune", "turn", "update", "use", "verify", "wait", "walk", "watch", "wire", "write",
    // A command line handed over: the program is the verb.
    "armada", "cargo", "cd", "gh", "git", "npm", "scripts",
];

/// What opens a question or a choice to make, as a clause's first word.
const ASKING: &[&str] = &[
    "which", "what", "where", "who", "whose", "how", "why", "when", "should", "shall", "do", "does", "did", "can", "could", "would", "will", "may", "might", "must", "is", "are", "am", "has", "have", "want", "need", "ok", "okay", "any",
];

/// Words that lead a clause without being its verb: "Also try", "Optional: git prune".
const LEADS: &[&str] = &["also", "then", "now", "please", "first", "next", "optional", "maybe", "and", "or", "but", "so", "after"];

/// Words that name nothing: a clause of only these has no thing in it.
const STOP: &[&str] = &[
    "a", "an", "the", "it", "its", "this", "that", "these", "those", "them", "they", "he", "she", "we", "you", "i", "me", "my", "your", "our", "their", "here", "there", "now", "again", "too", "please", "once", "first", "then", "also", "just", "so", "to", "of", "on", "in", "at", "for", "with", "from", "by", "as", "is", "are", "be", "been", "was", "were", "am", "and", "or", "but", "if", "when", "while", "than", "not", "no", "yes", "either", "both", "one", "any", "all", "some", "next", "back", "out", "up", "off", "over", "only", "still", "already", "other", "another", "same", "each", "do", "does", "did",
];

/// Words of a report of where things stand.
const STATUS: &[&str] = &[
    "waiting", "waits", "queued", "queue", "merging", "merged", "pushed", "pass", "passes", "passed", "done", "finished", "complete", "completed", "ready", "running", "fixed", "landed", "green", "red",
];

/// Why `text` is unclear, else `None`. `runs` says a command to run goes with it: the verb and the thing.
pub(crate) fn unclear(text: &str, runs: bool) -> Option<Unclear> {
    let words = words_of(text);
    let padded = format!(" {} ", words.join(" "));
    if POINTING.iter().any(|phrase| padded.contains(&format!(" {phrase} "))) || points_up_or_down(&words) || counts_choices(text, &words) {
        return Some(Unclear::Pointer);
    }
    if runs || asks_him(text) {
        return None;
    }
    if words.iter().any(|word| STATUS.contains(&word.as_str())) {
        return Some(Unclear::Status);
    }
    Some(Unclear::NoAsk)
}

/// The lowercase words of a text, punctuation and markdown gone.
fn words_of(text: &str) -> Vec<String> {
    text.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect()
}

/// What "above" or "below" follows when it points at what was said: "see above", "the options below".
const POINTED_AT: &[&str] = &["see", "as", "the", "of", "listed", "described", "mentioned", "named", "discussed", "outlined", "command", "commands", "list", "ones", "one", "items", "steps", "plan", "summary", "message", "reply", "question", "questions", "choices", "options", "decisions", "ways", "approaches", "takes", "directions"];

/// "Above" or "below" that ends the item or follows one of [`POINTED_AT`]; "the header above the band" is not.
fn points_up_or_down(words: &[String]) -> bool {
    words.iter().enumerate().any(|(at, word)| {
        matches!(word.as_str(), "above" | "below") && (at + 1 == words.len() || at > 0 && POINTED_AT.contains(&words[at - 1].as_str()))
    })
}

/// "Three options" or "3 design choices" with nothing named after them: no colon or bracket follows.
fn counts_choices(text: &str, words: &[String]) -> bool {
    let count = |word: &str| (word.chars().all(|c| c.is_ascii_digit()) && word != "1") || NUMBERS.contains(&word);
    let lower = text.to_lowercase();
    words.iter().enumerate().any(|(at, word)| {
        count(word)
            && words[at + 1..words.len().min(at + 4)].iter().any(|next| CHOICES.contains(&next.as_str()))
            && !lower[lower.find(word.as_str()).unwrap_or(0)..].contains([':', '('])
    })
}

/// Whether a clause of the text holds a verb for him with a thing, or a question or a choice with one.
///
/// A verb has its thing in its own clause, else after its colon ("Walk: the sidebar"), else in the
/// clauses before it ("Five gaps the agents found: file them?"). A question word leads a question
/// whether or not a `?` ends it, after a colon only with one. A line ending in `?` that offers "X, or Y" is a choice.
fn asks_him(text: &str) -> bool {
    let said = text.replace(['*', '`'], "");
    let question = said.trim().trim_end_matches(['"', '\'', ')']).ends_with('?');
    let clauses = clauses_of(&said);
    let all = |clauses: &[String]| clauses.iter().map(|clause| clause.split_whitespace().filter(|w| names(w)).count()).sum::<usize>();
    if question && said.contains(" or ") && all(&clauses) >= 4 {
        return true;
    }
    clauses.iter().enumerate().any(|(at, clause)| {
        let mut words = clause.split_whitespace().peekable();
        while words.next_if(|w| LEADS.contains(&w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase().as_str())).is_some() {}
        let rest: Vec<&str> = words.collect();
        let Some(lead) = rest.first().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()) else {
            return false;
        };
        let here = rest.iter().skip(1).filter(|w| names(w)).count();
        if VERBS.contains(&lead.as_str()) {
            return here >= 1 || all(&clauses[at + 1..clauses.len().min(at + 2)]) >= 1 || all(&clauses[..at]) >= 2;
        }
        (at == 0 || question) && ASKING.contains(&lead.as_str()) && all(&clauses) >= if question { 2 } else { 3 }
    })
}

/// Whether a word of a clause names something: it is not one of the words that name nothing.
fn names(word: &str) -> bool {
    let word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '#').to_lowercase();
    !word.is_empty() && !STOP.contains(&word.as_str())
}

/// The clauses of a text: split at punctuation, brackets, and the joins of a list of things to do.
fn clauses_of(text: &str) -> Vec<String> {
    let mut spaced = text.replace('\n', ". ");
    for join in [" then ", " and ", " or ", " so ", " — "] {
        spaced = spaced.replace(join, ". ");
    }
    spaced
        .split(|c: char| matches!(c, ':' | ';' | ',' | '.' | '?' | '!' | '(' | ')'))
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .map(str::to_string)
        .collect()
}

/// The first of `items`, each `(id, text, runs)`, that is unclear, with why; `None` while `in_force` is
/// false. The id is what the agent called it, so the refusal can name it.
pub(crate) fn first_unclear<'a>(items: impl IntoIterator<Item = (&'a str, &'a str, bool)>, in_force: bool) -> Option<(&'a str, Unclear)> {
    if !in_force {
        return None;
    }
    items.into_iter().find_map(|(id, text, runs)| unclear(text, runs).map(|why| (id, why)))
}

/// What the agent reads when its list is refused for an item the owner could not act on.
pub(crate) fn unclear_item_refusal(id: &str, why: Unclear) -> String {
    format!(
        "Refused: item \"{id}\" is not clear to the owner: {}. His list shows an item as you word it, so say \
         what he is to do and the thing it names in one line that reads alone, then call waiting_for again \
         with the item reworded.",
        why.why()
    )
}
