//! A forge markdown body as Reviews draws it: what the forge hides is cleaned out first, then
//! `TextView` draws the rest in the Job thread's prose style, and every link opens in the browser.

use gpui::{prelude::*, relative, AnyElement, ElementId, HighlightStyle, SharedString};
use gpui_component::text::{TextView, TextViewStyle};

use crate::look::{token, Look};

/// `source` drawn as markdown after `clean`. `id` keeps its selection and scroll apart from the
/// other bodies on the page.
pub(crate) fn body(id: impl Into<SharedString>, source: &str, look: &Look) -> AnyElement {
    let base = look.length(token::TEXT_SM);
    let style = TextViewStyle::default()
        .heading_font_size(|level, base| match level {
            1 => base * 1.5,
            2 => base * 1.3,
            3 => base * 1.15,
            _ => base,
        })
        .inline_code(HighlightStyle { background_color: Some(look.color(token::BG_SUNKEN)), ..Default::default() });
    let mut style = style;
    style.heading_base_font_size = base;
    TextView::markdown(ElementId::Name(id.into()), clean(source))
        .w_full()
        .text_size(base)
        .line_height(relative(1.6))
        .text_color(look.color(token::FG_DEFAULT))
        .blockquote_style(
            gpui::StyleRefinement::default()
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .border_l(look.length(token::BORDER_WIDTH) * 2.)
                .border_color(look.color(token::ACCENT))
                .rounded_r(look.length(token::RADIUS_SM))
                .bg(look.color(token::ACCENT_FAINT))
                .text_color(look.color(token::FG_DEFAULT)),
        )
        .style(style)
        .selectable(true)
        // No surface navigates: every address opens in the default browser.
        .on_link_click(|address, _, _, cx| cx.open_url(address))
        .into_any_element()
}

/// What the forge shows of `source`: HTML comments gone, a `<details>` folded to its summary, a
/// linked badge a plain link, layout tags dropped with their text kept, `&nbsp;` a space. Code
/// fences and code spans stay as written.
pub(super) fn clean(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut at = 0;
    while at < source.len() {
        let rest = &source[at..];
        let line_start = at == 0 || source.as_bytes()[at - 1] == b'\n';
        if let Some(end) = line_start.then(|| fence_end(rest)).flatten() {
            out.push_str(&rest[..end]);
            at += end;
        } else if rest.starts_with('`') {
            let end = span_end(rest);
            out.push_str(&rest[..end]);
            at += end;
        } else if rest.starts_with("<!--") {
            at += rest.find("-->").map_or(rest.len(), |end| end + 3);
        } else if rest.starts_with("&nbsp;") {
            out.push(' ');
            at += 6;
        } else if let Some(used) = rest.starts_with('<').then(|| tag(rest, &mut out)).flatten() {
            at += used;
        } else {
            let one = rest.chars().next().map_or(1, char::len_utf8);
            out.push_str(&rest[..one]);
            at += one;
        }
    }
    out
}

/// How much of `rest` a fenced block takes, where a fence opens its first line.
fn fence_end(rest: &str) -> Option<usize> {
    let line = rest.split_inclusive('\n').next()?;
    let open = line.trim_start_matches(' ');
    if line.len() - open.len() > 3 {
        return None;
    }
    let mark = open.chars().next().filter(|mark| *mark == '`' || *mark == '~')?;
    let width = open.chars().take_while(|one| *one == mark).count();
    if width < 3 {
        return None;
    }
    let mut used = line.len();
    for next in rest[used..].split_inclusive('\n') {
        used += next.len();
        let shut = next.trim_start_matches(' ').trim_end();
        if shut.len() >= width && shut.chars().all(|one| one == mark) {
            break;
        }
    }
    Some(used)
}

/// How much of `rest` a code span takes: through the next backtick run of the same length, or just the opening run.
fn span_end(rest: &str) -> usize {
    let width = rest.chars().take_while(|one| *one == '`').count();
    let mut at = width;
    while let Some(found) = rest[at..].find('`') {
        let start = at + found;
        let run = rest[start..].chars().take_while(|one| *one == '`').count();
        if run == width {
            return start + run;
        }
        at = start + run;
    }
    width
}

/// An HTML tag read at the start of `rest`.
struct Tag<'a> {
    name: String,
    closing: bool,
    attrs: &'a str,
    end: usize,
}

fn read_tag(rest: &str) -> Option<Tag<'_>> {
    let inner = rest.strip_prefix('<')?;
    let closing = inner.starts_with('/');
    let named = inner.trim_start_matches('/');
    let length = named.chars().take_while(char::is_ascii_alphanumeric).count();
    if length == 0 || !named.starts_with(|first: char| first.is_ascii_alphabetic()) {
        return None;
    }
    if !named[length..].starts_with(|next: char| next.is_whitespace() || next == '>' || next == '/') {
        return None;
    }
    let mut quote: Option<char> = None;
    let mut end = None;
    for (at, one) in rest.char_indices().skip(1) {
        match (quote, one) {
            (Some(open), one) if one == open => quote = None,
            (None, '"' | '\'') => quote = Some(one),
            (None, '>') => {
                end = Some(at + 1);
                break;
            }
            _ => {}
        }
    }
    let end = end?;
    let head = rest.len() - named.len();
    Some(Tag { name: named[..length].to_ascii_lowercase(), closing, attrs: &rest[head + length..end - 1], end })
}

/// The value of attribute `key` in a tag's attributes, quoted or bare.
fn attr(attrs: &str, key: &str) -> Option<String> {
    let lower = attrs.to_ascii_lowercase();
    let mut from = 0;
    while let Some(found) = lower[from..].find(key) {
        let start = from + found;
        from = start + key.len();
        let before_ok = start == 0 || lower.as_bytes()[start - 1].is_ascii_whitespace();
        let after = attrs[from..].trim_start();
        if !before_ok || !after.starts_with('=') {
            continue;
        }
        let value = after[1..].trim_start();
        return Some(match value.chars().next() {
            Some(open @ ('"' | '\'')) => value[1..].split(open).next().unwrap_or("").to_string(),
            _ => value.split(|one: char| one.is_whitespace() || one == '>').next().unwrap_or("").to_string(),
        });
    }
    None
}

/// The text of `html` with every tag taken out and an image's alt kept.
fn plain(html: &str) -> String {
    let mut out = String::new();
    let mut at = 0;
    while at < html.len() {
        let rest = &html[at..];
        if let Some(tag) = rest.starts_with('<').then(|| read_tag(rest)).flatten() {
            if tag.name == "img" {
                out.push_str(&attr(tag.attrs, "alt").unwrap_or_default());
            }
            at += tag.end;
        } else {
            let one = rest.chars().next().map_or(1, char::len_utf8);
            out.push_str(&rest[..one]);
            at += one;
        }
    }
    out.replace("&nbsp;", " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Where the first closing `</name>` in `rest` ends, or the end of `rest` where none closes.
fn closes(rest: &str, name: &str) -> (usize, usize) {
    let lower = rest.to_ascii_lowercase();
    let needle = format!("</{name}");
    match lower.find(&needle) {
        Some(start) => (start, lower[start..].find('>').map_or(rest.len(), |gt| start + gt + 1)),
        None => (rest.len(), rest.len()),
    }
}

/// Handle the tag at the start of `rest`, writing what it becomes to `out`. How much of `rest` it used, or None for a tag left as written.
fn tag(rest: &str, out: &mut String) -> Option<usize> {
    let tag = read_tag(rest)?;
    match (tag.name.as_str(), tag.closing) {
        ("details", false) => {
            let (inner_end, used) = closes(rest, "details");
            let inner = &rest[tag.end..inner_end];
            let lower = inner.to_ascii_lowercase();
            let summary = lower.find("<summary").map(|start| {
                let from = inner[start..].find('>').map_or(inner.len(), |gt| start + gt + 1);
                let (stop, _) = closes(&inner[from..], "summary");
                plain(&inner[from..from + stop])
            });
            // Its contents stay hidden: TextView cannot fold, so the summary is all that shows.
            out.push_str(&format!("\n\n**\u{203a} {}**\n\n", summary.unwrap_or_else(|| "Details".into())));
            Some(used)
        }
        ("a", false) => {
            let (inner_end, used) = closes(rest, "a");
            let inner = &rest[tag.end..inner_end];
            let Some(address) = attr(tag.attrs, "href") else { return Some(tag.end) };
            let label = plain(inner);
            let label = if label.is_empty() { address.clone() } else { label };
            out.push_str(&format!("[{label}]({address})"));
            Some(used)
        }
        ("img", false) => {
            out.push_str(&attr(tag.attrs, "alt").unwrap_or_default());
            Some(tag.end)
        }
        ("br", _) => {
            out.push('\n');
            Some(tag.end)
        }
        ("p" | "div", true) => {
            out.push('\n');
            Some(tag.end)
        }
        ("p" | "div" | "picture" | "source" | "summary" | "a", _) => Some(tag.end),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn a_comment_is_dropped_and_an_unclosed_one_runs_to_the_end() {
        assert_eq!(clean("a <!-- hidden --> b"), "a  b");
        assert_eq!(clean("a\n<!-- never closed\nstill hidden"), "a\n");
    }

    #[test]
    fn details_folds_to_its_summary_alone() {
        let said = clean("before\n<details><summary>Show logs</summary>\nsecret line\n</details>\nafter");
        assert!(said.contains("**\u{203a} Show logs**"), "{said}");
        assert!(!said.contains("secret line"), "{said}");
        assert!(said.contains("before") && said.contains("after"), "{said}");
    }

    #[test]
    fn a_linked_badge_becomes_a_plain_link() {
        let said = clean(r#"<div><a href="https://example.com/fix"><picture><source srcset="x"><img alt="Fix in editor" src="y"></picture></a></div>"#);
        assert_eq!(said.trim(), "[Fix in editor](https://example.com/fix)");
    }

    #[test]
    fn layout_tags_go_and_their_text_stays() {
        let said = clean("<p>one<br>two</p><div>three</div>");
        assert!(!said.contains('<'), "{said}");
        assert!(said.contains("one") && said.contains("two") && said.contains("three"), "{said}");
    }

    #[test]
    fn nbsp_is_a_space() {
        assert_eq!(clean("a&nbsp;b"), "a b");
    }

    #[test]
    fn code_fences_and_spans_are_left_alone() {
        let source = "```html\n<!-- keep --><div>&nbsp;</div>\n```\nand `<!-- span -->` here <!-- gone -->";
        assert_eq!(clean(source), "```html\n<!-- keep --><div>&nbsp;</div>\n```\nand `<!-- span -->` here ");
    }

    #[test]
    fn plain_markdown_and_autolinks_pass_through() {
        let source = "### heading\n**bold** <https://example.com> and a < b";
        assert_eq!(clean(source), source);
    }
}
