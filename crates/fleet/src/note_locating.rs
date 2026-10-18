//! The lines of code a pinned note's spot was drawn from, found by the words on screen.
//! `send_session_note`.
//!
//! **The text on screen is the anchor, whatever the language**: a label or a template is in the
//! source as written, and only the data in it is not. So the words are split at anything that
//! looks like data into runs, and a line scores by the runs it holds. Over the words come three
//! things the page knows: the element's tag drawn on the line, the component around the line, and
//! a stamp the build put on the element, which is believed only where the text bears it out.

use ipc::{Candidate, NotePlace};

/// The most candidates a note carries.
const MOST: usize = 5;
/// A run shorter than this, once its punctuation is trimmed, matches too much to say anything.
const SHORTEST_RUN: usize = 4;
/// How far past a stamp's line its words may be: an element's text is often a few lines below
/// the tag that opens it.
const STAMP_REACH: usize = 15;
/// The most of a line a candidate quotes.
const QUOTED: usize = 160;

/// What [`located`] made of a stamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StampRead {
    /// The page stamped nothing.
    Absent,
    /// The stamp's line held the screen's words, and it ranks first.
    Checked,
    /// The stamp did not hold up, and why.
    Dropped(String),
}

/// The candidates, best first, and what became of the stamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located {
    pub candidates: Vec<Candidate>,
    pub stamp: StampRead,
}

/// Where on the page a run of words was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Said {
    Label,
    Text,
    Nearby,
}

impl Said {
    fn named(self) -> &'static str {
        match self {
            Said::Label => "the label's",
            Said::Text => "the text's",
            Said::Nearby => "a name around it,",
        }
    }

    /// The element's own words locate it; the names around it only place it.
    fn own(self) -> bool {
        self != Said::Nearby
    }
}

/// A run of the screen's words with no data in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub words: String,
    pub said: Said,
}

/// Every run in `place`'s label, text and nearby names, each once, the element's own first.
pub fn runs_of(place: &NotePlace) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let sources = place
        .element
        .label
        .iter()
        .map(|label| (label.as_str(), Said::Label))
        .chain(std::iter::once((place.element.text.as_str(), Said::Text)))
        .chain(place.nearby.iter().map(|name| (name.as_str(), Said::Nearby)));
    for (text, said) in sources {
        for words in runs_in(text) {
            if !runs.iter().any(|one| one.words.eq_ignore_ascii_case(&words)) {
                runs.push(Run { words, said });
            }
        }
    }
    runs
}

/// `text` cut at every word holding a digit: a count, a percentage, a date, a time or a size.
fn runs_in(text: &str) -> Vec<String> {
    let mut runs = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut flush = |current: &mut Vec<&str>| {
        let joined = current.join(" ");
        let trimmed = joined.trim_matches(|c: char| !c.is_alphanumeric());
        if trimmed.chars().count() >= SHORTEST_RUN {
            runs.push(trimmed.to_string());
        }
        current.clear();
    };
    for word in text.split_whitespace() {
        if word.chars().any(|c| c.is_ascii_digit()) {
            flush(&mut current);
        } else {
            current.push(word);
        }
    }
    flush(&mut current);
    runs
}

/// Whether a path is one a page is not drawn from: dependencies, build output, lockfiles, tests and
/// fixtures. Searched only where nothing else matched.
pub fn set_aside(path: &str) -> bool {
    const DIRECTORIES: &[&str] = &[
        "node_modules", "dist", "build", "target", "out", ".next", "coverage",
        "tests", "test", "__tests__", "fixtures", "__fixtures__", "testdata",
    ];
    let mut parts = path.split('/');
    let name = parts.next_back().unwrap_or("");
    let lockfile = name.ends_with(".lock")
        || name.ends_with("-lock.json")
        || name.ends_with("-lock.yaml")
        || name.ends_with(".lockb");
    let test = name.contains(".test.") || name.contains(".spec.") || name.ends_with("_test.rs") || name.ends_with("_test.go");
    lockfile || test || parts.any(|part| DIRECTORIES.contains(&part))
}

/// One line that held a run, scored.
struct Scored {
    file: String,
    line: usize,
    score: usize,
    why: String,
    text: String,
}

/// The lines among `files`, each a repository-relative path and its text, that `place` was drawn
/// from. **The stamp's file is among `files` wherever it exists**, so a stamp whose file is not
/// there names a file that is not.
pub fn located(place: &NotePlace, files: &[(String, String)]) -> Located {
    let runs = runs_of(place);
    let wanted: Vec<&str> = place.component.iter().chain(place.owners.iter()).map(String::as_str).collect();
    let score_all = |aside: bool| {
        files
            .iter()
            .filter(|(path, _)| set_aside(path) == aside)
            .flat_map(|(path, text)| scored_in(path, text, &runs, place, &wanted))
            .collect::<Vec<Scored>>()
    };
    let mut scored = score_all(false);
    if scored.is_empty() {
        scored = score_all(true);
    }
    scored.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.file.cmp(&b.file)).then(a.line.cmp(&b.line)));

    let (stamp, stamped) = match place.stamp.as_deref() {
        None => (StampRead::Absent, None),
        Some(stamp) => match stamp_checked(stamp, files, &runs, &wanted) {
            Ok(first) => (StampRead::Checked, Some(first)),
            Err(why) => (StampRead::Dropped(why), None),
        },
    };
    let mut candidates: Vec<Candidate> = Vec::new();
    if let Some(first) = stamped {
        scored.retain(|one| !(one.file == first.file && one.line == first.line));
        candidates.push(candidate(first));
    }
    candidates.extend(scored.into_iter().map(candidate));
    candidates.truncate(MOST);
    Located { candidates, stamp }
}

fn candidate(one: Scored) -> Candidate {
    Candidate {
        file: one.file,
        line: u32::try_from(one.line).unwrap_or(u32::MAX),
        why: one.why,
        text: one.text,
    }
}

/// The page's stamp as the first candidate, or why it does not hold up.
fn stamp_checked(stamp: &str, files: &[(String, String)], runs: &[Run], wanted: &[&str]) -> Result<Scored, String> {
    let (path, line) = stamp
        .rsplit_once(':')
        .and_then(|(path, line)| Some((path.trim_start_matches("./"), line.parse::<usize>().ok()?)))
        .filter(|(_, line)| *line > 0)
        .ok_or_else(|| format!("the page's stamp {stamp} is not a path and a line"))?;
    let (file, text) = files
        .iter()
        .find(|(file, _)| file == path)
        .ok_or_else(|| format!("the page's stamp {stamp} names a file that is not in the checkout"))?;
    let lines: Vec<&str> = text.lines().collect();
    if line > lines.len() {
        return Err(format!("the page's stamp {stamp} is past the file's {} lines", lines.len()));
    }
    let window = &lines[line - 1..lines.len().min(line + STAMP_REACH)];
    let (offset, run) = window
        .iter()
        .enumerate()
        .find_map(|(at, said)| runs.iter().find(|run| holds(said, &run.words).is_some()).map(|run| (at, run)))
        .ok_or_else(|| {
            format!("the page's stamp {stamp} was dropped: nothing on its line or the {STAMP_REACH} after it says what the screen says")
        })?;
    let place = if offset == 0 { "this line".to_string() } else { format!("line {}", line + offset) };
    let mut why = format!("the page's stamp, checked: {} words \"{}\" are on {place}", run.said.named(), run.words);
    if let Some(name) = enclosing(&lines, line - 1, wanted) {
        why.push_str(&format!(", inside {name}"));
    }
    Ok(Scored { file: file.clone(), line, score: usize::MAX, why, text: quoted(lines[line - 1]) })
}

/// Each line of one file that holds a run, with its score and why.
fn scored_in(path: &str, text: &str, runs: &[Run], place: &NotePlace, wanted: &[&str]) -> Vec<Scored> {
    let lines: Vec<&str> = text.lines().collect();
    let tag = place.element.tag.to_ascii_lowercase();
    let mut found = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let mut score = 0;
        let mut best: Option<(usize, &Run, bool)> = None;
        for run in runs {
            let Some(whole) = holds(line, &run.words) else { continue };
            let weight = run.words.chars().count() * if run.said.own() { 2 } else { 1 } * if whole { 2 } else { 1 };
            score += weight;
            if best.is_none_or(|(most, _, _)| weight > most) {
                best = Some((weight, run, whole));
            }
        }
        let Some((_, run, whole)) = best else { continue };
        let mut why = format!(
            "{} words \"{}\" are on this line{}",
            run.said.named(),
            run.words,
            if whole { ", as written" } else { "" }
        );
        if draws(line, &tag) {
            score += 8;
            why.push_str(&format!(", where the <{tag}> is drawn"));
        }
        if let Some(name) = enclosing(&lines, at, wanted) {
            score += if place.component.as_deref() == Some(name) { 20 } else { 5 };
            why.push_str(&format!(", inside {name}"));
        }
        found.push(Scored { file: path.to_string(), line: at + 1, score, why, text: quoted(line) });
    }
    found
}

/// Whether `line` holds `words`, ignoring case, and if so whether it holds them whole somewhere: no
/// letter joined to either end, as a label or a template piece is, rather than inside a longer
/// phrase.
fn holds(line: &str, words: &str) -> Option<bool> {
    let lower = line.to_lowercase();
    let wanted = words.to_lowercase();
    let mut whole = None;
    let mut from = 0;
    while let Some(at) = lower[from..].find(&wanted) {
        let start = from + at;
        let end = start + wanted.len();
        let before = lower[..start].trim_end().chars().next_back();
        let after = lower[end..].trim_start().chars().next();
        let joined = before.is_some_and(char::is_alphanumeric) || after.is_some_and(char::is_alphanumeric);
        whole = Some(whole.unwrap_or(false) || !joined);
        from = end;
    }
    whole
}

/// Whether `line` opens an element named `tag`, as markup does.
fn draws(line: &str, tag: &str) -> bool {
    if tag.is_empty() {
        return false;
    }
    let opening = format!("<{tag}");
    let lower = line.to_ascii_lowercase();
    lower.match_indices(&opening).any(|(at, _)| {
        lower[at + opening.len()..].chars().next().is_none_or(|c| c.is_whitespace() || c == '>' || c == '/')
    })
}

/// The component or owner in `wanted` whose definition encloses line `at`, read upward. **A
/// definition at the left margin that is not wanted ends the search**: the line is inside it.
fn enclosing<'a>(lines: &[&str], at: usize, wanted: &[&'a str]) -> Option<&'a str> {
    for line in lines[..=at].iter().rev() {
        let Some(name) = defined(line) else { continue };
        if let Some(found) = wanted.iter().find(|one| **one == name) {
            return Some(found);
        }
        if !line.starts_with(char::is_whitespace) {
            return None;
        }
    }
    None
}

/// The name a line defines, in the languages a page is drawn in: `function X`, `const X =`,
/// `class X`, `def X`, `fn X`, `func X`.
fn defined(line: &str) -> Option<&str> {
    const OPENERS: &[&str] = &["function", "const", "let", "var", "class", "def", "fn", "func"];
    let mut words = line.split(|c: char| c.is_whitespace() || matches!(c, '(' | '<' | ':' | '=' | '{'));
    while let Some(word) = words.next() {
        if OPENERS.contains(&word) {
            let name = words.find(|one| !one.is_empty())?.trim_start_matches('*');
            return (!name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$')).then_some(name);
        }
    }
    None
}

fn quoted(line: &str) -> String {
    let trimmed = line.trim();
    match trimmed.char_indices().nth(QUOTED) {
        Some((cut, _)) => trimmed[..cut].to_string(),
        None => trimmed.to_string(),
    }
}
