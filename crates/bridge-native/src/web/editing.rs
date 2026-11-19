//! Edit on a page. **The page he saves is the revision's own HTML with only its words changed**: `editing.js` makes each
//! block of words editable in the web view and reads back what every text node says, and [`spliced`] writes those words
//! into the revision as the Job wrote it (`get_artifact_page_source`), so its markup, scripts and sketches are kept byte for
//! byte and `save_artifact_edit` finds the same elements in the same order. The page in the web view is the sealed one,
//! with the kit's buttons and votes drawn in, so its own markup is never what is sent.
//!
//! A text node is matched to the stretch of the source it was parsed from by order, and each match is checked against the
//! words the node held before he typed: a page whose words cannot be matched is refused here, never saved crooked.

use serde::{Deserialize, Serialize};

const SOURCE: &str = include_str!("editing.js");

/// The elements whose words take an Edit, as Fleet's `page_words.rs` names them.
const WORDS: &[&str] = &["p", "h1", "h2", "h3", "h4", "h5", "h6", "li", "td", "th", "dt", "dd", "blockquote", "figcaption", "caption"];
/// Elements whose words are never his to edit: what Fleet refuses an edit inside, and what the kit draws over.
const SKIP: &[&str] = &["script", "style", "textarea", "title", "noscript", "template", "svg", "armada-sketch", "armada-action", "armada-vote"];
/// Elements whose content is text up to their end tag.
const RAW: &[&str] = &["script", "style", "textarea", "title", "noscript"];
/// Elements with no end tag.
const VOID: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];
/// What closes an open `p`, as the browser's parser closes it.
const CLOSES_P: &[&str] = &[
    "address", "article", "aside", "blockquote", "details", "div", "dl", "fieldset", "figcaption", "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr", "li", "main",
    "nav", "ol", "p", "pre", "section", "table", "ul", "dd", "dt",
];

/// The colours the page marks with, as CSS: a changed block's edge and fill, and the faint line under one he can type in.
#[derive(Serialize)]
pub struct Marks {
    pub edge: String,
    pub fill: String,
    pub line: String,
}

/// What the page's text nodes said when Edit began, and say now, in document order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Texts {
    pub was: Vec<String>,
    pub now: Vec<String>,
}

impl Texts {
    /// Whether any words changed, white space aside.
    pub fn changed(&self) -> bool {
        self.was.iter().zip(&self.now).any(|(was, now)| squeezed(was) != squeezed(now))
    }
}

/// What an Edit left half done keeps: the words of each text node, and the revision they were typed on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kept {
    pub revision: u32,
    pub texts: Vec<String>,
}

fn ask(call: &str) -> String {
    format!("{SOURCE}\n{call}")
}

/// The script that turns Edit on, with `kept` typed back in where it has one word for each text node.
pub fn start(marks: &Marks, kept: Option<&[String]>) -> String {
    let marks = ipc::encode(marks).unwrap_or_else(|_| "{}".into());
    let kept = kept.and_then(|texts| ipc::encode(&texts).ok()).unwrap_or_else(|| "null".into());
    ask(&format!("ArmadaEdit.start({}, {})", marks.replace('<', "\\u003c"), kept.replace('<', "\\u003c")))
}

/// The script that reads back what each text node says.
pub fn read() -> String {
    ask("ArmadaEdit.read()")
}

/// How many text nodes Edit turned on for, from the answer to [`start`]: `None` where the page did not answer.
pub fn started(answer: Option<&str>) -> Option<usize> {
    #[derive(Deserialize)]
    struct Started {
        texts: usize,
    }
    ipc::decode::<Started>("page edit", answer?.as_bytes()).ok().map(|started| started.texts)
}

/// The texts an answer to [`read`] holds, where it holds one word for each node.
pub fn texts_of(answer: Option<&str>) -> Option<Texts> {
    let texts: Texts = ipc::decode::<Option<Texts>>("page words", answer?.as_bytes()).ok()??;
    (texts.was.len() == texts.now.len()).then_some(texts)
}

/// The page `source` with each text node's words as he left them. Refused, in words for him, where the source does not
/// hold the text nodes the page did.
pub fn spliced(source: &str, texts: &Texts) -> Result<String, String> {
    let stretches = stretches(source);
    let unmatched = || "This page's words could not be matched to its markup, so the Edit was not saved".to_string();
    if stretches.len() != texts.was.len() || texts.now.len() != texts.was.len() {
        return Err(unmatched());
    }
    for (range, was) in stretches.iter().zip(&texts.was) {
        if let Some(read) = unescaped(&source[range.clone()]) {
            if squeezed(&read) != squeezed(was) {
                return Err(unmatched());
            }
        }
    }
    let mut out = String::with_capacity(source.len());
    let mut from = 0;
    for ((range, was), now) in stretches.iter().zip(&texts.was).zip(&texts.now) {
        if was == now {
            continue;
        }
        out.push_str(&source[from..range.start]);
        out.push_str(&escaped(now, was.contains('\u{a0}')));
        from = range.end;
    }
    out.push_str(&source[from..]);
    Ok(out)
}

/// The words of each text node the browser makes of `source` inside a block of words, in order: what [`read`] answers
/// for a page drawn from it. A stretch with an entity not read here is given as written.
pub fn words_of(source: &str) -> Vec<String> {
    stretches(source).into_iter().map(|range| unescaped(&source[range.clone()]).unwrap_or_else(|| source[range].to_string())).collect()
}

/// The stretches of `source` the browser makes a text node of inside a block of words, in order, empty ones left out.
fn stretches(source: &str) -> Vec<std::ops::Range<usize>> {
    let lower = source.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut open: Vec<String> = Vec::new();
    let mut found = Vec::new();
    let (mut at, mut text_from) = (0, 0);
    fn take(from: usize, to: usize, open: &[String], found: &mut Vec<std::ops::Range<usize>>) {
        let words = open.iter().any(|name| WORDS.contains(&name.as_str()));
        let skipped = open.iter().any(|name| SKIP.contains(&name.as_str()));
        if to > from && words && !skipped {
            found.push(from..to);
        }
    }
    while at < bytes.len() {
        if bytes[at] != b'<' {
            at += 1;
            continue;
        }
        let next = bytes.get(at + 1).copied().unwrap_or_default();
        let end = if lower[at..].starts_with("<!--") {
            lower[at..].find("-->").map_or(lower.len(), |close| at + close + 3)
        } else if next == b'!' || next == b'?' || (next == b'/' && bytes.get(at + 2).is_some_and(u8::is_ascii_alphabetic)) || next.is_ascii_alphabetic() {
            tag_end(&lower, at + 1)
        } else {
            // A bare `<` is text.
            at += 1;
            continue;
        };
        take(text_from, at, &open, &mut found);
        let tag = &lower[at..end];
        (at, text_from) = (end, end);
        if let Some(name) = tag.strip_prefix("</").map(name_of) {
            if let Some(place) = open.iter().rposition(|one| *one == name) {
                open.truncate(place);
            }
        } else if next.is_ascii_alphabetic() {
            let name = name_of(&tag[1..]);
            implied_ends(&mut open, &name);
            if RAW.contains(&name.as_str()) {
                // Its content is one text, up to its end tag, and never words.
                at = lower[end..].find(&format!("</{name}")).map_or(lower.len(), |close| end + close);
                text_from = at;
            } else if !VOID.contains(&name.as_str()) {
                // The parser drops a newline straight after `<pre>`.
                if name == "pre" && bytes.get(end) == Some(&b'\n') {
                    text_from = end + 1;
                }
                open.push(name);
            }
        }
    }
    take(text_from, source.len(), &open, &mut found);
    found
}

/// The elements an opening `name` ends, as the browser's parser ends them.
fn implied_ends(open: &mut Vec<String>, name: &str) {
    let ends = |open: &mut Vec<String>, closed: &[&str], within: &[&str]| {
        if let Some(place) = open.iter().rposition(|one| closed.contains(&one.as_str()) || within.contains(&one.as_str())) {
            if closed.contains(&open[place].as_str()) {
                open.truncate(place);
            }
        }
    };
    if CLOSES_P.contains(&name) {
        ends(open, &["p"], &["button", "table", "td", "th", "li", "div", "section", "blockquote"]);
    }
    match name {
        "li" => ends(open, &["li"], &["ul", "ol"]),
        "dt" | "dd" => ends(open, &["dt", "dd"], &["dl"]),
        "td" | "th" => ends(open, &["td", "th"], &["tr", "table"]),
        "tr" => ends(open, &["tr", "td", "th"], &["table", "tbody", "thead", "tfoot"]),
        _ => {}
    }
}

fn tag_end(lower: &str, from: usize) -> usize {
    let mut quote = None;
    for (at, byte) in lower.bytes().enumerate().skip(from) {
        match (quote, byte) {
            (None, b'"' | b'\'') => quote = Some(byte),
            (Some(open), _) if open == byte => quote = None,
            (None, b'>') => return at + 1,
            _ => {}
        }
    }
    lower.len()
}

fn name_of(after: &str) -> String {
    after.chars().take_while(|c| !c.is_ascii_whitespace() && !matches!(c, '/' | '>')).collect()
}

fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Text as the browser reads it, or `None` where it holds a named entity not read here, which then goes unchecked.
fn unescaped(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest[1..].find(';').map(|end| end + 1).filter(|end| *end <= 32) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let name = &rest[1..end];
        let read = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            _ => match name.strip_prefix('#') {
                Some(number) => match number.strip_prefix(['x', 'X']) {
                    Some(hex) => u32::from_str_radix(hex, 16).ok(),
                    None => number.parse().ok(),
                }
                .and_then(char::from_u32),
                None if name.chars().all(|c| c.is_ascii_alphanumeric()) => return None,
                None => {
                    out.push('&');
                    rest = &rest[1..];
                    continue;
                }
            },
        };
        let Some(read) = read else { return None };
        out.push(read);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Some(out.replace("\r\n", "\n").replace('\r', "\n"))
}

/// His words as HTML text. The no-break spaces a typed space can become are plain spaces again, unless the words he edited
/// held one of their own.
fn escaped(text: &str, keep_no_break: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\u{a0}' if keep_no_break => out.push_str("&nbsp;"),
            '\u{a0}' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = include_str!("../../fixtures/page-actions-source.html");

    fn texts_of_page(source: &str) -> Vec<String> {
        stretches(source).into_iter().map(|range| unescaped(&source[range]).unwrap_or_default()).collect()
    }

    #[test]
    fn the_words_of_text_headings_and_cells_are_read_in_order_and_what_the_kit_draws_is_not() {
        let words: Vec<String> = texts_of_page(PAGE).iter().map(|one| squeezed(one)).collect();
        assert_eq!(words.first().map(String::as_str), Some("Stale branch design"));
        assert!(words.iter().any(|one| one.starts_with("A branch is stale")));
        assert!(words.contains(&"web/dark-mode".to_string()) && words.contains(&"Behind main".to_string()));
        assert!(!words.iter().any(|one| one.contains("amber chip") || one.contains("Amber chip")), "a vote's takes are the kit's: {words:?}");
    }

    #[test]
    fn his_words_go_into_the_markup_and_nothing_else_moves() {
        let was = texts_of_page(PAGE);
        let mut now = was.clone();
        let at = was.iter().position(|one| one.starts_with("A branch is stale")).expect("the first paragraph");
        now[at] = now[at].replace("Four branches", "Five branches & <one> more");
        let page = spliced(PAGE, &Texts { was: was.clone(), now }).expect("his words fit the page");
        assert!(page.contains("Five branches &amp; &lt;one&gt; more are stale now."), "{page}");
        assert_eq!(page.replace("Five branches &amp; &lt;one&gt; more", "Four branches"), PAGE, "only his words changed");
        assert_eq!(spliced(PAGE, &Texts { now: was.clone(), was }).as_deref(), Ok(PAGE), "no change is the page as it was");
    }

    #[test]
    fn a_page_whose_words_do_not_match_its_markup_is_refused_and_never_saved_crooked() {
        let mut was = texts_of_page(PAGE);
        let now = was.clone();
        was.pop();
        assert!(spliced(PAGE, &Texts { was: was.clone(), now: now.clone() }).is_err(), "one text node too few");
        let mut moved = texts_of_page(PAGE);
        moved.swap(1, 2);
        assert!(spliced(PAGE, &Texts { was: moved.clone(), now: moved }).is_err(), "the words the page held are not the source's");
    }

    #[test]
    fn inline_marks_comments_and_entities_split_the_words_as_the_browser_does() {
        let source = "<ul><li>One <b>bold</b>&nbsp;word<!-- c -->after<li>Two &rarr; x</ul><p>a<div>outside</div><pre>\ncode</pre>";
        let found = texts_of_page(source);
        assert_eq!(found, ["One ", "bold", "\u{a0}word", "after", "", "a"], "an unknown entity goes unchecked; a p ends at a div");
        let mut now = found.clone();
        now[2] = "\u{a0}words\u{a0}typed".into();
        let page = spliced(source, &Texts { was: found, now }).expect("fits");
        assert!(page.contains("<b>bold</b>&nbsp;words&nbsp;typed<!-- c -->"), "{page}");
    }

    #[test]
    fn a_typed_space_the_browser_made_unbreakable_is_a_space_again() {
        let source = "<p>Two words</p>";
        let page = spliced(source, &Texts { was: vec!["Two words".into()], now: vec!["Two\u{a0}new words".into()] }).expect("fits");
        assert_eq!(page, "<p>Two new words</p>");
    }

    #[test]
    fn the_calls_carry_their_values_as_json_so_no_word_ends_the_script() {
        let marks = Marks { edge: "#f0a".into(), fill: "rgba(1,2,3,0.1)".into(), line: "#abc".into() };
        let kept = vec!["say \"hi\"</script>".to_string()];
        let script = start(&marks, Some(&kept));
        assert!(script.ends_with(r##"ArmadaEdit.start({"edge":"#f0a","fill":"rgba(1,2,3,0.1)","line":"#abc"}, ["say \"hi\"\u003c/script>"])"##), "{script}");
        assert!(start(&marks, None).ends_with(", null)"));
        assert_eq!(started(Some(r#"{"texts":12}"#)), Some(12));
        assert_eq!(started(None), None);
        assert_eq!(texts_of(Some(r#"{"was":["a"],"now":["b"]}"#)), Some(Texts { was: vec!["a".into()], now: vec!["b".into()] }));
        assert_eq!(texts_of(Some("null")), None);
        assert_eq!(texts_of(Some(r#"{"was":["a"],"now":[]}"#)), None);
        assert!(!Texts { was: vec!["a b".into()], now: vec!["a  b".into()] }.changed(), "white space is not an edit");
    }
}
