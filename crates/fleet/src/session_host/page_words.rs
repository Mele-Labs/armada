//! His Edit of a page: **the words changed and nothing else**. The page he saved and the revision he edited
//! are read into the same elements, with the same attributes, in the same order, and text changed only inside
//! a text, heading, list or table-cell element. A scan as `document_page` scans, with no parser.
//! `docs/concepts/artifact.md`, *His Edit*.

use std::collections::BTreeMap;

use super::document_page::{squeezed, tag_end, unescaped};

/// The elements whose words take an Edit, and every element inside one.
const WORDS: &[&str] = &["p", "h1", "h2", "h3", "h4", "h5", "h6", "li", "td", "th", "dt", "dd", "blockquote", "figcaption", "caption"];
/// Elements with no end tag.
const VOID: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];
/// Elements whose content is text up to their end tag, never elements.
const RAW: &[&str] = &["script", "style", "textarea", "title"];
/// Elements whose text is never words, inside a words element or not.
const NEVER_WORDS: &[&str] = &["script", "style", "textarea", "title", "armada-sketch", "svg"];
/// The most characters of an element a refusal quotes.
const MOST_NAMED: usize = 80;

/// One block whose words he changed.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Changed {
    /// The `h2` it comes under, as he left it, else `Top`.
    pub section: String,
    pub old: String,
    pub new: String,
}

/// A tag, a comment or a doctype, compared whole between the two.
#[derive(Debug, PartialEq, Eq)]
enum Mark {
    Open { name: String, attributes: Vec<(String, String)> },
    Close(String),
    Other(String),
}

/// The text before a mark, and where it stands.
struct Text {
    raw: String,
    /// The mark that opened the innermost words element around it.
    block: Option<usize>,
    /// The mark that opened the innermost element around it.
    inside: Option<usize>,
}

/// A page as its marks and the texts between them: `texts[i]` comes before `marks[i]`, and the last after them all.
struct Scanned {
    marks: Vec<(Mark, String)>,
    texts: Vec<Text>,
}

/// The blocks whose words he changed, in page order, or why it is not an Edit of words, naming the first element
/// that differs. `revision` is the number `base` is, for the refusal.
pub(super) fn words_changed(base: &str, his: &str, revision: u32) -> Result<Vec<Changed>, String> {
    let (was, now) = (scanned(base), scanned(his));
    let named = |raw: &str| raw.chars().take(MOST_NAMED).collect::<String>();
    for at in 0..was.marks.len().max(now.marks.len()) {
        let differs = match (was.marks.get(at), now.marks.get(at)) {
            (Some(one), Some(other)) if one.0 == other.0 => continue,
            (Some(one), Some(other)) => format!("`{}` where revision {revision} has `{}`", named(&other.1), named(&one.1)),
            (None, Some(other)) => format!("`{}`, which revision {revision} does not have", named(&other.1)),
            (Some(one), None) => format!("no `{}`, where revision {revision} has one", named(&one.1)),
            (None, None) => continue,
        };
        return Err(format!("an Edit changes words only, and his page has {differs}"));
    }
    let mut blocks: BTreeMap<usize, (String, String)> = BTreeMap::new();
    for (one, other) in was.texts.iter().zip(&now.texts) {
        match one.block {
            Some(block) => {
                let words = blocks.entry(block).or_default();
                words.0.push_str(&one.raw);
                words.1.push_str(&other.raw);
            }
            None if squeezed(&one.raw) != squeezed(&other.raw) => {
                let place = one.inside.map_or_else(|| String::from("the page outside any element"), |at| format!("`{}`", named(&was.marks[at].1)));
                return Err(format!(
                    "an Edit changes words only in text, headings, lists and table cells, and his page changes the words in {place}"
                ));
            }
            None => {}
        }
    }
    let section = |block: usize| {
        let heading = (0..block).rev().find(|at| matches!(&now.marks[*at].0, Mark::Open { name, .. } if name == "h2"))?;
        let words: String = now.texts.iter().filter(|text| text.block == Some(heading)).map(|text| text.raw.as_str()).collect();
        Some(squeezed(&unescaped(&words))).filter(|words| !words.is_empty())
    };
    Ok(blocks
        .into_iter()
        .map(|(block, (old, new))| (block, squeezed(&unescaped(&old)), squeezed(&unescaped(&new))))
        .filter(|(_, old, new)| old != new)
        .map(|(block, old, new)| Changed { section: section(block).unwrap_or_else(|| String::from("Top")), old, new })
        .collect())
}

fn scanned(html: &str) -> Scanned {
    let lower = html.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut scanned = Scanned { marks: Vec::new(), texts: Vec::new() };
    let mut open: Vec<(String, usize)> = Vec::new();
    let (mut at, mut text_from) = (0, 0);
    while at < bytes.len() {
        if bytes[at] != b'<' {
            at += 1;
            continue;
        }
        let next = bytes.get(at + 1).copied().unwrap_or_default();
        let (mark, end) = if lower[at..].starts_with("<!--") {
            let end = lower[at..].find("-->").map_or(lower.len(), |found| at + found + 3);
            (Mark::Other(html[at..end].to_string()), end)
        } else if next == b'!' || next == b'?' {
            let end = tag_end(&lower, at + 2);
            (Mark::Other(html[at..end].to_string()), end)
        } else if next == b'/' && bytes.get(at + 2).is_some_and(u8::is_ascii_alphabetic) {
            let end = tag_end(&lower, at + 2);
            (Mark::Close(name_of(&lower[at + 2..end])), end)
        } else if next.is_ascii_alphabetic() {
            let end = tag_end(&lower, at + 1);
            let inner = html[at + 1..end].trim_end_matches('>').trim_end().trim_end_matches('/');
            let name = name_of(&inner.to_ascii_lowercase());
            (Mark::Open { attributes: attributes_of(&inner[name.len().min(inner.len())..]), name }, end)
        } else {
            // A bare `<` is text.
            at += 1;
            continue;
        };
        scanned.texts.push(text_of(&html[text_from..at], &open));
        let index = scanned.marks.len();
        let raw = html[at..end].to_string();
        (at, text_from) = (end, end);
        match &mark {
            Mark::Open { name, .. } if RAW.contains(&name.as_str()) => {
                open.push((name.clone(), index));
                // Its content is one text, up to its end tag.
                at = lower[end..].find(&format!("</{name}")).map_or(lower.len(), |found| end + found);
            }
            Mark::Open { name, .. } if !VOID.contains(&name.as_str()) && !raw.ends_with("/>") => open.push((name.clone(), index)),
            Mark::Close(name) => {
                if let Some(from) = open.iter().rposition(|(one, _)| one == name) {
                    open.truncate(from);
                }
            }
            _ => {}
        }
        scanned.marks.push((mark, raw));
    }
    scanned.texts.push(text_of(&html[text_from..], &open));
    scanned
}

/// `raw` as a text inside the elements `open` holds, innermost last.
fn text_of(raw: &str, open: &[(String, usize)]) -> Text {
    let never = open.iter().any(|(name, _)| NEVER_WORDS.contains(&name.as_str()));
    let block = open.iter().rev().find(|(name, _)| WORDS.contains(&name.as_str())).map(|(_, at)| *at).filter(|_| !never);
    Text { raw: raw.to_string(), block, inside: open.last().map(|(_, at)| *at) }
}

/// A tag's name, from the text after its `<` or `</`.
fn name_of(lower: &str) -> String {
    lower.chars().take_while(|c| !c.is_ascii_whitespace() && !matches!(c, '/' | '>')).collect()
}

/// A tag's attributes, from the text after its name, by name with their entities read.
fn attributes_of(tag: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut rest = tag;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == '/');
        if rest.is_empty() {
            break;
        }
        let name: String = rest.chars().take_while(|c| !c.is_ascii_whitespace() && !matches!(c, '=' | '/' | '>')).collect();
        rest = rest[name.len()..].trim_start();
        let mut value = String::new();
        if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();
            let (said, left) = match after.chars().next() {
                Some(quote @ ('"' | '\'')) => after[1..].split_once(quote).unwrap_or((&after[1..], "")),
                _ => after.split_at(after.find(|c: char| c.is_ascii_whitespace()).unwrap_or(after.len())),
            };
            value = unescaped(said);
            rest = left;
        }
        if name.is_empty() {
            // Nothing a name is made of: step past the one character, so the scan ends.
            rest = rest.get(1..).unwrap_or_default();
            continue;
        }
        found.push((name.to_ascii_lowercase(), value));
    }
    found.sort();
    found
}
