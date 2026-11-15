//! How much a page makes him read before what it asks. **A page that asks, and buries the ask, is not put to
//! him**: Fleet keeps the revision and tells the Job what to move, as it tells a vote that will not count.
//! The measure is words of body text, outside headings, between the top of the body and the first block that
//! holds the ask: an `<armada-vote>`, an `<armada-action>` or the first `<h2>`. A scan and not a parse, as
//! `document_page` is. `docs/concepts/document.md`, *Asking*.

use super::document_page::{open_tag, tag_end, unescaped, without_tags};

/// The most words of body text a page that asks may put before the block that holds the ask.
pub(crate) const MOST_WORDS_BEFORE_THE_ASK: usize = 120;

/// Headings, and what is not text at all: they are not read as the words before the ask.
const NOT_BODY_TEXT: [&str; 11] = ["head", "script", "style", "template", "h1", "h2", "h3", "h4", "h5", "h6", "title"];

/// What is wrong with a page that asks, as one sentence for its Job, or `None` where its ask is up front: some
/// opening words before the first block that holds the ask, and no more than [`MOST_WORDS_BEFORE_THE_ASK`].
/// Called only for a page that asks. Nothing is read of a page that does not.
pub(crate) fn buried_ask(html: &str, path: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let from = open_tag(&lower, "body", 0).map_or(0, |(_, after)| after);
    let (at, holder) = ["armada-vote", "armada-action", "h2"]
        .into_iter()
        .filter_map(|name| open_tag(&lower, name, from).map(|(start, _)| (start, name)))
        .min()
        .map_or((html.len(), "end"), |(start, name)| (start, name));
    let words = words_in(&html[from..at]);
    let holds = match holder {
        "end" => "the end of the page".to_string(),
        name => format!("the first <{name}>"),
    };
    let move_it = "Open with one sentence on what he decides, then the vote, the buttons or the Approve, and fold detail below them";
    if words == 0 {
        return Some(format!(
            "In the page {path}, no sentence comes before {holds}. {move_it}. This revision is not put to him until it opens with one."
        ));
    }
    (words > MOST_WORDS_BEFORE_THE_ASK).then(|| {
        format!(
            "In the page {path}, {words} words come before {holds}, and he reads {MOST_WORDS_BEFORE_THE_ASK} at most before what he decides. {move_it}. This revision is not put to him until it does."
        )
    })
}

/// The words of body text in `html`: its headings, its head, its scripts, its styles and its comments left out.
fn words_in(html: &str) -> usize {
    let mut text = html.to_string();
    for name in NOT_BODY_TEXT {
        text = without_element(&text, name);
    }
    unescaped(&without_tags(&without_comments(&text))).split_whitespace().count()
}

/// `html` with each `<name ...>` through its closing tag taken out.
fn without_element(html: &str, name: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let (mut kept, mut from) = (String::with_capacity(html.len()), 0);
    while let Some((start, after)) = open_tag(&lower, name, from) {
        kept.push_str(&html[from..start]);
        from = match lower[after..].find(&format!("</{name}")) {
            Some(close) => tag_end(&lower, after + close),
            None => lower.len(),
        };
    }
    kept.push_str(html.get(from..).unwrap_or_default());
    kept
}

/// `html` with each `<!-- ... -->` taken out.
fn without_comments(html: &str) -> String {
    let (mut kept, mut rest) = (String::with_capacity(html.len()), html);
    while let Some(start) = rest.find("<!--") {
        kept.push_str(&rest[..start]);
        rest = rest[start..].find("-->").map_or("", |end| &rest[start + end + 3..]);
    }
    kept.push_str(rest);
    kept
}
