//! The box draws the markdown it holds, as Electron's does (`message-dom.ts`): a `> ` line as a quote,
//! a `- ` or `* ` line as a list item, and a web address, a repository path or a markdown link as a
//! link. Each is an atomic token over the words themselves, so what is kept and sent is the text it
//! always was.

use std::ops::Range;

use gpui_component::input::InlineToken;

/// The first part of every format token's id; a chip's id starts with its kind instead.
const FORMAT: &str = "fmt";
const SEP: char = '\u{1f}';

/// What a format token draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Format {
    Quote,
    Item,
    /// A web address, or the address a markdown link points at.
    Web(String),
    File(String),
}

pub(super) fn format_of(id: &str) -> Option<Format> {
    let mut parts = id.split(SEP);
    if parts.next()? != FORMAT {
        return None;
    }
    Some(match (parts.next()?, parts.next()) {
        ("quote", _) => Format::Quote,
        ("item", _) => Format::Item,
        ("web", Some(href)) => Format::Web(href.to_string()),
        ("file", Some(path)) => Format::File(path.to_string()),
        _ => return None,
    })
}

fn token(kind: &str, href: &str, text: &str, label: &str) -> InlineToken {
    InlineToken::new(format!("{FORMAT}{SEP}{kind}{SEP}{href}"), text.to_string()).with_label(label.to_string())
}

/// The line's marker, `> ` for a quote and `- ` or `* ` for an item, as a byte length.
fn marker(line: &str) -> Option<(&'static str, usize)> {
    if line.starts_with("> ") {
        Some(("quote", 2))
    } else if line.starts_with("- ") || line.starts_with("* ") {
        Some(("item", 2))
    } else {
        None
    }
}

/// The lines of `text` with the byte offset each starts at.
fn lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut at = 0;
    text.split('\n').map(move |line| {
        let start = at;
        at += line.len() + 1;
        (start, line)
    })
}

const KNOWN: &[&str] = &["rs", "ts", "tsx", "js", "jsx", "mjs", "json", "toml", "md", "mdx", "css", "html", "yml", "yaml", "sh", "py", "go", "lock", "txt", "sql", "svg"];

fn is_web(core: &str) -> bool {
    let rest = core.strip_prefix("https://").or_else(|| core.strip_prefix("http://"));
    rest.and_then(|rest| rest.chars().next()).is_some_and(|first| !matches!(first, '/' | '<' | '>')) && !core.contains(['<', '>'])
}

/// A repository path as typed: `crates/fleet/src/servers.rs`, `servers.rs:60`, `@src/` (`isPath`).
fn is_path(core: &str) -> bool {
    let at = core.starts_with('@');
    let body = if at { &core[1..] } else { core };
    let body = match body.find(':') {
        Some(colon) if body[colon + 1..].chars().all(|c| c.is_ascii_digit() || c == ':' || c == '-') && body.len() > colon + 1 => &body[..colon],
        Some(_) => return false,
        None => body,
    };
    let (path, dir) = match body.strip_suffix('/') {
        Some(path) => (path, true),
        None => (body, false),
    };
    let word = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'));
    if path.is_empty() || !path.split('/').all(word) || path.chars().all(|c| c == '.') {
        return false;
    }
    if dir {
        return at;
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    let extension = name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).filter(|ext| {
        (1..=8).contains(&ext.len()) && ext.starts_with(|c: char| c.is_ascii_alphabetic()) && ext.chars().all(|c| c.is_ascii_alphanumeric())
    });
    let known = extension.as_deref().is_some_and(|ext| KNOWN.contains(&ext));
    let folder = path.contains('/');
    if at {
        folder || known
    } else if folder {
        extension.is_some()
    } else {
        known
    }
}

/// The part of a word that is the address: not the bracket or quote before it, nor the stop after (`coreOf`).
fn core_of(word: &str) -> Range<usize> {
    let bytes = word.as_bytes();
    let (mut from, mut to) = (0, bytes.len());
    while from < to && b"(\"'`<[".contains(&bytes[from]) {
        from += 1;
    }
    while to > from && b".,;:!?\"'`>]".contains(&bytes[to - 1]) {
        to -= 1;
    }
    while to > from && bytes[to - 1] == b')' && !word[from..to].contains('(') {
        to -= 1;
    }
    from..to
}

/// `[words](https://…)` at `at`, as its byte length, words and address.
fn markdown_link(text: &str) -> Option<(usize, &str, &str)> {
    let close = text.find(']')?;
    let words = &text[1..close];
    if words.is_empty() || words.contains(['\n', '[']) {
        return None;
    }
    let rest = text[close + 1..].strip_prefix('(')?;
    let end = rest.find(|c: char| c == ')' || c.is_whitespace())?;
    let href = &rest[..end];
    (rest[end..].starts_with(')') && is_web(href)).then_some((close + 1 + 1 + end + 1, words, href))
}

/// The format tokens `text` should carry, clear of `taken` (the chips and tokens already there). A word
/// is drawn as a link only once something follows it, so the one being typed stays words (`followed`).
pub(super) fn marks_of(text: &str, taken: &[Range<usize>]) -> Vec<(Range<usize>, InlineToken)> {
    let free = |range: &Range<usize>| !taken.iter().any(|held| held.start < range.end && range.start < held.end);
    let mut marks: Vec<(Range<usize>, InlineToken)> = Vec::new();
    for (start, line) in lines(text) {
        let mut from = 0;
        if let Some((kind, len)) = marker(line) {
            let range = start..start + len;
            if free(&range) {
                marks.push((range, token(kind, "", &line[..len], if kind == "quote" { "Quote" } else { "Item" })));
            }
            from = len;
        }
        // Markdown links first, so the address inside one is not drawn twice.
        let mut held: Vec<Range<usize>> = Vec::new();
        let mut at = from;
        while let Some(open) = line[at..].find('[') {
            let open = at + open;
            match markdown_link(&line[open..]) {
                Some((len, words, href)) => {
                    let range = start + open..start + open + len;
                    if free(&range) {
                        marks.push((range.clone(), token("web", href, &line[open..open + len], words)));
                    }
                    held.push(range);
                    at = open + len;
                }
                None => at = open + 1,
            }
        }
        let mut offset = from;
        for word in line[from..].split(' ') {
            let word_at = offset;
            offset += word.len() + 1;
            // The last word of the box is still being typed.
            if word.is_empty() || start + word_at + word.len() >= text.len() {
                continue;
            }
            let core = core_of(word);
            let address = &word[core.clone()];
            let kind = if is_web(address) {
                "web"
            } else if is_path(address) {
                "file"
            } else {
                continue;
            };
            let range = start + word_at + core.start..start + word_at + core.end;
            if free(&range) && !held.iter().any(|link| link.start < range.end && range.start < link.end) {
                marks.push((range, token(kind, address, address, address)));
            }
        }
    }
    marks
}

/// What Enter does in a quote or a list, or `None` where it is the box's own: a plain Enter sends and a
/// Shift+Enter puts in a new line (`enterInBlock`). In a list either starts the next item, and on an empty
/// item ends the list. In a quote, Shift+Enter starts another quote line; Enter at the end of its last
/// line leaves it for a plain line, and on an empty one goes back to plain.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Enter {
    /// These words go in at the caret.
    Insert(&'static str),
    /// The line's marker goes, and the line is a plain one.
    Leave(Range<usize>),
}

pub(super) fn enter_at(text: &str, caret: usize, shift: bool) -> Option<Enter> {
    let (start, line) = lines(text).take_while(|(start, _)| *start <= caret).last()?;
    let (kind, len) = marker(line)?;
    let end = start + line.len();
    let empty = line[len..].trim().is_empty();
    if kind == "item" {
        return Some(if empty { Enter::Leave(start..start + len) } else { Enter::Insert("\n- ") });
    }
    if shift {
        return Some(Enter::Insert("\n> "));
    }
    let last = !text[end..].strip_prefix('\n').is_some_and(|next| next.starts_with("> "));
    if caret != end || !last {
        return None;
    }
    Some(if empty { Enter::Leave(start..start + len) } else { Enter::Insert("\n") })
}

/// The message as it is sent: a blank line where a quote or a list ends in a line of words (`sentText`).
/// The box draws that line apart from the block; markdown would read it as the block's own lazy
/// continuation, and the thread would draw what the box did not.
pub(crate) fn sent_text(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut sent = Vec::with_capacity(lines.len());
    for (at, line) in lines.iter().enumerate() {
        sent.push(*line);
        if let Some(next) = lines.get(at + 1) {
            if marker(line).is_some() && !next.is_empty() && marker(next).is_none() {
                sent.push("");
            }
        }
    }
    sent.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(text: &str) -> Vec<(String, Format)> {
        marks_of(text, &[]).into_iter().map(|(range, token)| (text[range].to_string(), format_of(token.id()).expect("a format"))).collect()
    }

    #[test]
    fn quotes_items_and_links_are_drawn_as_typed() {
        let text = "> held words\n- one\n* two\nsee https://example.com/a, and [the doc](https://example.com/d) in crates/fleet/src/lib.rs now";
        assert_eq!(
            drawn(text),
            vec![
                ("> ".into(), Format::Quote),
                ("- ".into(), Format::Item),
                ("* ".into(), Format::Item),
                ("[the doc](https://example.com/d)".into(), Format::Web("https://example.com/d".into())),
                ("https://example.com/a".into(), Format::Web("https://example.com/a".into())),
                ("crates/fleet/src/lib.rs".into(), Format::File("crates/fleet/src/lib.rs".into())),
            ]
        );
    }

    #[test]
    fn the_word_being_typed_and_plain_words_stay_words() {
        assert!(drawn("open https://example.com").is_empty());
        assert!(drawn("e.g. v1.2 and -flag or >quote").is_empty());
    }

    #[test]
    fn a_line_of_words_after_a_list_or_quote_is_sent_apart_from_it() {
        assert_eq!(sent_text("- one\n- two\nsee this"), "- one\n- two\n\nsee this");
        assert_eq!(sent_text("> held\nplain"), "> held\n\nplain");
        assert_eq!(sent_text("- one\n\nplain\n1. Take B"), "- one\n\nplain\n1. Take B");
    }

    #[test]
    fn a_chip_keeps_its_place() {
        let text = "- @Fix the chip";
        assert!(marks_of(text, &[0..2]).is_empty());
    }

    #[test]
    fn enter_goes_on_in_a_list_and_leaves_an_empty_item() {
        assert_eq!(enter_at("- one", 5, false), Some(Enter::Insert("\n- ")));
        assert_eq!(enter_at("- one\n- ", 8, false), Some(Enter::Leave(6..8)));
        assert_eq!(enter_at("> held", 6, false), Some(Enter::Insert("\n")));
        assert_eq!(enter_at("> held\n> ", 9, false), Some(Enter::Leave(7..9)));
        assert_eq!(enter_at("> held\n> more", 6, false), None);
        assert_eq!(enter_at("plain words", 11, false), None);
    }

    #[test]
    fn shift_enter_goes_on_in_a_quote_and_is_the_box_own_elsewhere() {
        assert_eq!(enter_at("> held", 6, true), Some(Enter::Insert("\n> ")));
        assert_eq!(enter_at("- one", 5, true), Some(Enter::Insert("\n- ")));
        assert_eq!(enter_at("plain words", 11, true), None);
    }
}
