//! Whether a reply asks the person for something, read from how it ends.
//! `docs/spikes/reply-asks-classifier.md` has the rules measured beside the language model of `asks_model`.
//!
//! **Pure, and leaning toward yes.** A missed ask leaves the person unaware that a Session waits on him;
//! a wrong call costs one dismiss. Only the close is read, since that is where an agent puts what it needs.

/// The sentence of `reply` that asks the person to answer, decide or do something, if one does.
pub(crate) fn asking_line(reply: &str) -> Option<String> {
    let blocks = prose_blocks(reply);
    let from = closing_from(&blocks);
    // A command fenced for the person is run by him, so the line that hands it over asks. Those are
    // looked for further back, since the reply goes on to say what happens after he has run it.
    let handed = blocks[blocks.len().saturating_sub(4)..].iter().flatten().rev().find(|line| line.command.is_some());
    let found = blocks[from..]
        .iter()
        .flatten()
        .rev()
        .find_map(|line| sentences(unmarked(line.text)).into_iter().rev().find(|sentence| asks(sentence)));
    found.or_else(|| handed.and_then(|line| sentences(unmarked(line.text)).pop()))
        .map(|sentence| sentence.replace("**", ""))
}

/// The sentences of the close the rules read, in order: what the language model reads of a reply.
pub(crate) fn closing_sentences(reply: &str) -> Vec<String> {
    let blocks = prose_blocks(reply);
    blocks[closing_from(&blocks)..]
        .iter()
        .flatten()
        .flat_map(|line| sentences(unmarked(line.text)))
        .map(|sentence| sentence.replace("**", ""))
        .collect()
}

/// What the fine-tuned model reads of a reply: the close's sentences, as one text.
pub(crate) fn closing_text(reply: &str) -> String {
    closing_sentences(reply).join(" ")
}

/// One sentence of the close, with what the sentence readers of `extracting` need around it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Closing {
    pub sentence: String,
    /// Which line of the close it is on; two sentences of one line share it.
    pub line: usize,
    /// The line is an item of a list.
    pub listed: bool,
    /// The shell command fenced for the person right after the line, if one is.
    pub command: Option<String>,
}

/// The sentences of the close with headings left out, since a heading introduces what follows and
/// asks nothing itself: a `#` line, a line that is wholly bold, and a short line ending in `:` or `?`
/// that stands alone before the next paragraph. A line a command follows is kept whatever its shape,
/// since it hands the command over.
pub(crate) fn closing(reply: &str) -> Vec<Closing> {
    let blocks = prose_blocks(reply);
    let from = closing_from(&blocks);
    let last_block = blocks.len().saturating_sub(1);
    let mut out = Vec::new();
    let mut at = 0;
    for (block_at, block) in blocks.iter().enumerate().skip(from) {
        for line in block {
            let text = line.text;
            if heading(line, block.len() == 1 && block_at < last_block) {
                at += 1;
                continue;
            }
            out.extend(sentences(unmarked(text)).into_iter().map(|sentence| Closing {
                sentence: sentence.replace("**", ""),
                line: at,
                listed: marker_len(text) > 0,
                command: line.command.clone(),
            }));
            at += 1;
        }
    }
    out
}

/// Whether a line introduces what follows and asks nothing itself: a `#` line, a line that is wholly
/// bold, or (`alone` before the next paragraph) a short line ending in `:` or `?`. A line a command
/// follows is kept whatever its shape, since it hands the command over.
fn heading(line: &Line, alone: bool) -> bool {
    let text = line.text;
    let bare = text.trim_end_matches([':', '?', ' ']);
    let bold = bare.len() > 4 && (bare.starts_with("**") && bare.ends_with("**") || bare.starts_with("__") && bare.ends_with("__")) && !bare[2..bare.len() - 2].contains("**");
    let short = alone && text.ends_with([':', '?']) && text.split_whitespace().count() < 6;
    // `## Next` is a heading; `#2568 is held` names a pull request.
    let marked = text.starts_with('#') && text.trim_start_matches('#').starts_with(' ');
    line.command.is_none() && (marked || bold || short)
}

/// The sentences `sentence` could be worded as, all the agent's own and the sentence first: the rest of
/// the paragraph it sits in, or of the list item, or, where it is a heading, the paragraph it heads.
/// Only the close is looked in, so a sentence that is not there has itself alone.
pub(crate) fn candidates_for(reply: &str, sentence: &str) -> Vec<String> {
    let blocks = prose_blocks(reply);
    let from = closing_from(&blocks);
    let last_block = blocks.len().saturating_sub(1);
    let said = |line: &Line| -> Vec<String> { sentences(unmarked(line.text)).into_iter().map(|one| one.replace("**", "")).collect() };
    let mut out = vec![sentence.to_string()];
    for (block_at, block) in blocks.iter().enumerate().skip(from) {
        let Some(line) = block.iter().find(|line| said(line).iter().any(|one| one == sentence)) else {
            continue;
        };
        let of: Vec<&Line> = if is_list(block) {
            vec![line]
        } else if heading(line, block.len() == 1 && block_at < last_block) {
            blocks.get(block_at + 1).map(|next| next.iter().collect()).unwrap_or_default()
        } else {
            block.iter().collect()
        };
        for one in of.into_iter().flat_map(|line| said(line)) {
            if !out.contains(&one) {
                out.push(one);
            }
        }
        break;
    }
    out
}

/// One line of a paragraph, and the shell command fenced for the person right after it, if one is.
struct Line<'a> {
    text: &'a str,
    command: Option<String>,
}

/// How many of the last paragraphs are the close: two, since an agent often asks and then states where
/// the work stands. A list is read with the lines that introduce it.
fn closing_from(blocks: &[Vec<Line>]) -> usize {
    let mut from = blocks.len().saturating_sub(2);
    while from > 0 && is_list(&blocks[from]) {
        from -= 1;
    }
    from
}

/// The paragraphs of the reply that are words to the person: fenced code, table rows and rules are left
/// out, and so is any paragraph that is only a link or a path, since an agent ends on those after what it asks.
fn prose_blocks(reply: &str) -> Vec<Vec<Line<'_>>> {
    let mut blocks: Vec<Vec<Line>> = vec![Vec::new()];
    let (mut fenced, mut handing) = (false, false);
    for line in reply.lines() {
        let line = line.trim();
        if let Some(tag) = line.strip_prefix("```") {
            handing = !fenced && matches!(tag.trim(), "bash" | "sh" | "shell" | "zsh");
            if handing {
                if let Some(last) = blocks.iter_mut().rev().find_map(|block| block.last_mut()) {
                    last.command = Some(String::new());
                }
            }
            fenced = !fenced;
            continue;
        }
        let rule = line.len() >= 3 && line.chars().all(|c| matches!(c, '-' | '*' | '_' | '='));
        if fenced {
            // The command's lines, for the line that hands it over.
            if handing {
                if let Some(Line { command: Some(command), .. }) = blocks.iter_mut().rev().find_map(|block| block.last_mut()) {
                    if !command.is_empty() {
                        command.push('\n');
                    }
                    command.push_str(line);
                }
            }
            continue;
        }
        if line.starts_with('|') {
            continue;
        }
        if line.is_empty() || rule {
            if !blocks.last().is_some_and(Vec::is_empty) {
                blocks.push(Vec::new());
            }
            continue;
        }
        let text = line.trim_start_matches('>').trim_start();
        blocks.last_mut().expect("always one").push(Line { text, command: None });
    }
    blocks.retain(|block| words(block) >= 3);
    blocks
}

/// Words made only of letters, outside links and backticks: a paragraph with fewer than three is a
/// path, a link or a label, not something said.
fn words(block: &[Line]) -> usize {
    block
        .iter()
        .flat_map(|line| line.text.split_whitespace())
        .filter(|word| !word.contains("://") && !word.contains('`') && !word.contains('/'))
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|word| word.len() >= 2 && word.chars().all(char::is_alphabetic))
        .count()
}

fn is_list(block: &[Line]) -> bool {
    block.iter().filter(|line| marker_len(line.text) > 0).count() * 2 > block.len()
}

/// The length of a list marker at the start of a line: `- `, `* `, `1. `, `a) ` or none.
fn marker_len(line: &str) -> usize {
    let bytes = line.as_bytes();
    let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    match (digits, bytes.get(digits), bytes.get(digits + 1), bytes.get(digits + 2)) {
        (0, Some(b'-' | b'*'), Some(b' '), _) => 2,
        (1.., Some(b'.' | b')'), Some(b' '), _) => digits + 2,
        (0, Some(letter), Some(b')'), Some(b' ')) if letter.is_ascii_alphabetic() => 3,
        _ => 0,
    }
}

fn unmarked(line: &str) -> &str {
    line[marker_len(line)..].trim_start()
}

/// The sentences of one line, split where `.`, `?` or `!` is followed by a space and not inside backticks.
fn sentences(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut start, mut ticks) = (0, false);
    for (at, c) in line.char_indices() {
        if c == '`' {
            ticks = !ticks;
        }
        if ticks || !matches!(c, '.' | '?' | '!') {
            continue;
        }
        let after = line[at + 1..].trim_start_matches(['*', '_', '"', '\'', ')', '”', '’']);
        if !(after.is_empty() || after.starts_with(char::is_whitespace)) {
            continue;
        }
        if c == '.' && abbreviated(&line[start..at]) {
            continue;
        }
        let end = line.len() - after.len();
        out.push(line[start..end].trim());
        start = end;
    }
    if start < line.len() {
        out.push(line[start..].trim());
    }
    out.retain(|sentence| !sentence.is_empty());
    out
}

fn abbreviated(chunk: &str) -> bool {
    let word = chunk.rsplit(char::is_whitespace).next().unwrap_or("").to_ascii_lowercase();
    matches!(word.as_str(), "e.g" | "i.e" | "vs" | "etc" | "cf")
}

/// What an agent says when it needs the person. Each is a phrase and not a pattern, so a miss is read
/// and added in one line. A phrase that ends in a letter must end a word.
const ASKING: &[&str] = &[
    "want me to",
    "would you like",
    "do you want",
    "should i",
    "shall i",
    "should we",
    "shall we",
    "which one",
    "which do you",
    "which would you",
    "let me know which",
    "let me know what",
    "let me know whether",
    "tell me which",
    "tell me what",
    "tell me whether",
    "tell me to",
    "then tell me",
    "say which",
    "say whether",
    "say the word",
    "say what to change",
    "your call",
    "it's yours",
    "up to you",
    "you decide",
    "you should decide",
    "until you choose",
    "until you decide",
    "pick one",
    "choose one",
    "need you to",
    "i need to know",
    "needs your",
    "need your",
    "can you",
    "could you",
    "would you",
    "please",
    "reply with",
    "once you approve",
    "once you pick",
    "when you're happy",
    "wait for your",
    "pressing approve",
    "on your behalf",
    "your part",
    "is yours",
    "until you run",
    "when you approve",
    "once you've looked",
    "for you to run",
    "step for you",
    "yours to",
    "your ok",
    "once you've tried",
    "once you've approved",
    "for you to decide",
    "once you've walked",
    "your go",
    "yourself",
    "approve it",
    "approve whichever",
    "leave notes",
    "leave a note",
    "when you're ready",
    "whenever you're ready",
    "if you disagree",
    "to look:",
    "what to press",
];

/// What a clause starts with when it tells the person to act: "Press Allow once", "Next: merge #12", "once it lands, run `x`".
const COMMANDS: &[&str] = &[
    "press ", "approve ", "answer ", "pick ", "choose ", "reply ", "say ", "walk ", "paste ", "use the ", "merge ", "run `", "run /", "run the ", "run this", "run these",
    "after you approve", "ok it",
];

/// Said at the start of a sentence, or as a heading, what is owed is asked; said in the middle of one it
/// is a status ("the item is still waiting for you").
const HEADINGS: &[&str] = &["waiting on you", "still waiting on you", "still waiting for you", "waiting for your", "needs you"];

/// Said where nothing is owed, which the phrases above would still match.
const NOTHING_OWED: &[&str] = &[
    "nothing is waiting",
    "nothing needs you",
    "nothing is owed",
    "nothing is left for you",
    "nothing for you",
    "nothing to do",
    "no action",
    "you don't need to",
    "you do not need to",
    "not waiting on you",
    "no longer waiting on you",
    "may need you",
    "might need you",
];

/// The plan is a card in the thread with its own Approve, so a line pointing at it asks for nothing more.
const PLAN_CARD: &[&str] = &["approve the plan", "review the plan"];

/// An offer left open, which asks for nothing until he takes it up.
const CONDITIONAL: &[&str] = &["if you want", "if you'd like", "if you would like", "if you'd rather", "if you prefer"];

fn asks(sentence: &str) -> bool {
    let said = sentence.replace('*', "").replace('’', "'").to_lowercase();
    if NOTHING_OWED.iter().any(|phrase| said.contains(phrase)) {
        return false;
    }
    let quoted = said.starts_with(['"', '“']);
    if said.trim_end_matches(['"', '\'', ')', '_', ' ']).ends_with('?') {
        return !quoted;
    }
    if CONDITIONAL.iter().chain(PLAN_CARD).any(|phrase| said.contains(phrase)) {
        return false;
    }
    if HEADINGS.iter().any(|phrase| said.starts_with(phrase) || said.contains(&format!("{phrase}:"))) {
        return true;
    }
    told(&said) || said.starts_with("i can ") && said.contains(", or ") || ASKING.iter().any(|phrase| says(&said, phrase))
}

/// Whether the sentence is an instruction to the person: it starts with one, after a label such as
/// "Next:", or follows a lead like "once it lands,". A label alone ("Merge conflicts:") is not one.
fn told(said: &str) -> bool {
    let body = said.split_once(':').filter(|(label, _)| label.len() <= 24).map_or(said, |(_, rest)| rest);
    let commands = |clause: &str| COMMANDS.iter().any(|start| clause.trim_start().starts_with(start));
    let lead = ["once ", "when ", "after ", "then "].iter().any(|word| body.trim_start().starts_with(word));
    commands(body) || lead && body.split_once(',').is_some_and(|(_, rest)| commands(rest))
}

/// Whether `phrase` is in `said` as whole words.
fn says(said: &str, phrase: &str) -> bool {
    said.match_indices(phrase).any(|(at, _)| {
        let before = said[..at].chars().next_back();
        let after = said[at + phrase.len()..].chars().next();
        before.map_or(true, |c| !c.is_alphanumeric())
            && (!phrase.ends_with(char::is_alphanumeric) || after.map_or(true, |c| !c.is_alphanumeric()))
    })
}
