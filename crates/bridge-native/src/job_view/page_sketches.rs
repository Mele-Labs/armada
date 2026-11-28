//! The sketches a page holds, drawn before the page reaches its web view.
//!
//! Fleet seals a page without touching an `<armada-sketch>`: the element keeps its `<script type="application/json">`
//! scene (or its `scene` attribute) as the Job wrote it, and the kit's script and stylesheet draw nothing for it, since
//! `<armada-sketch>` is the kit's one element and Bridge draws it (`document-kit.ts`). Native draws it here: each
//! sketch's scene is read as the thread reads one, drawn as the page `Save as web page` writes (`export::inline`, with
//! every step lit), and put in the element in place of the scene. The sealed page's policy allows inline styles and no
//! scripts, and the drawing needs neither of its own. A sketch inside an `<armada-take>` is the same element, so a vote's
//! takes carry their drawings side by side as the kit lays the takes out.

use std::hash::{Hash as _, Hasher as _};

use gpui::{App, SharedString, Window};

use crate::scene::export;

/// How an `<armada-sketch>` sits in the page: a framed block with its title above, the drawing scaled to the width it is given.
const STYLE: &str = "<style>armada-sketch{display:block;margin:var(--space-4) 0;border:var(--edge-active) solid var(--border-subtle);border-radius:var(--radius-lg);overflow:hidden;background:var(--bg-sunken)}armada-sketch[title]::before{content:attr(title);display:block;padding:var(--space-2) var(--space-3);font-size:var(--text-xs);color:var(--fg-muted)}armada-sketch svg{display:block;inline-size:100%;block-size:auto}armada-take armada-sketch{margin:var(--space-3) 0 0}</style>";

/// One element the scan found: where its content runs, and what it said.
pub(super) struct Found<'a> {
    /// Through the end of the opening tag.
    pub(super) content: usize,
    /// Where the closing tag starts, or the page's end where none closes it.
    pub(super) close: usize,
    pub(super) tag: &'a str,
    pub(super) inner: &'a str,
}

/// The attribute `name` of an opening tag, entities read; `None` where the tag does not carry it.
pub(super) fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(found) = lower[from..].find(name) {
        let at = from + found;
        from = at + name.len();
        let before_ok = at > 0 && lower.as_bytes()[at - 1].is_ascii_whitespace();
        let rest = tag[from..].trim_start();
        if !before_ok || !rest.starts_with('=') {
            continue;
        }
        let rest = rest[1..].trim_start();
        let quote = rest.chars().next().filter(|c| matches!(c, '"' | '\''))?;
        let value = &rest[1..];
        let end = value.find(quote)?;
        return Some(unescaped(&value[..end]));
    }
    None
}

fn unescaped(text: &str) -> String {
    text.replace("&quot;", "\"").replace("&#34;", "\"").replace("&#39;", "'").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// The end of the opening tag that starts at `from`: after its `>`, not counting one inside a quoted value.
pub(super) fn tag_end(html: &str, from: usize) -> usize {
    let mut quote: Option<char> = None;
    for (at, c) in html[from..].char_indices() {
        match (quote, c) {
            (Some(open), c) if c == open => quote = None,
            (None, '"' | '\'') => quote = Some(c),
            (None, '>') => return from + at + 1,
            _ => {}
        }
    }
    html.len()
}

/// Each `<name>` element of the page, in order.
pub(super) fn scan<'a>(html: &'a str, name: &str) -> Vec<Found<'a>> {
    let lower = html.to_ascii_lowercase();
    let (open, closing) = (format!("<{name}"), format!("</{name}"));
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = lower[from..].find(&open) {
        let start = from + at;
        let after = start + open.len();
        // `<armada-sketchy>` is not it.
        if !lower.as_bytes().get(after).is_some_and(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/')) {
            from = after;
            continue;
        }
        let content = tag_end(html, after);
        let close = lower[content..].find(&closing).map_or(html.len(), |at| content + at);
        found.push(Found { content, close, tag: &html[start..content], inner: &html[content..close] });
        from = close;
    }
    found
}

/// A sketch's scene as text: its first `<script type="application/json">`, else its `scene` attribute, as Fleet reads it.
fn scene_of(found: &Found<'_>) -> Option<String> {
    json_of(found).or_else(|| attribute(found.tag, "scene"))
}

/// The text of an element's first `<script type="application/json">`.
pub(super) fn json_of(found: &Found<'_>) -> Option<String> {
    let lower = found.inner.to_ascii_lowercase();
    let mut from = 0;
    while let Some(at) = lower[from..].find("<script") {
        let start = from + at;
        let open_end = tag_end(found.inner, start);
        let tag = &found.inner[start..open_end];
        if attribute(tag, "type").is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json")) {
            let end = lower[open_end..].find("</script").map_or(found.inner.len(), |close| open_end + close);
            return Some(found.inner[open_end..end].to_string());
        }
        from = open_end;
    }
    None
}

/// `html` with each sketch's content replaced by what `draw` makes of it: its place among the page's sketches, its title and
/// its scene text. A sketch `draw` answers `None` for is left as the Job wrote it. Pure, so it is read without a window.
pub(super) fn drawn_with(html: &str, mut draw: impl FnMut(usize, &str, &str) -> Option<String>) -> String {
    let mut out = String::with_capacity(html.len());
    let mut at = 0;
    for (index, found) in scan(html, "armada-sketch").into_iter().enumerate() {
        let title = attribute(found.tag, "title").unwrap_or_default();
        let Some(drawing) = scene_of(&found).and_then(|scene| draw(index, title.trim(), &scene)) else { continue };
        out.push_str(&html[at..found.content]);
        out.push_str(&drawing);
        at = found.close;
    }
    out.push_str(&html[at..]);
    // The look the elements are drawn in goes in the page's head, which the sealed page always has: the policy leads it.
    match out.to_ascii_lowercase().find("</head") {
        Some(head) if at > 0 => {
            out.insert_str(head, STYLE);
            out
        }
        _ => out,
    }
}

/// The page with its sketches drawn. `artifact` and each scene's own words name the drawing, so a page kept again with another
/// scene in the same place is laid out again.
pub(super) fn drawn(html: &str, artifact: &str, window: &mut Window, cx: &mut App) -> String {
    if !html.to_ascii_lowercase().contains("<armada-sketch") {
        return html.to_string();
    }
    drawn_with(html, |_, title, scene| {
        let raw: ipc::RawScene = serde_json::from_str(scene).ok()?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        scene.hash(&mut hasher);
        let key = format!("page:{artifact}:{:x}", hasher.finish());
        let read = crate::scene::read(&key, &raw)?;
        Some(export::inline(&SharedString::from(key), &read, title, window, cx))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"A"}],"edges":[]}"#;

    fn page(sketches: &str) -> String {
        format!("<html><head><meta http-equiv=\"x\"></head><body><h1>T</h1>{sketches}<p>After.</p></body></html>")
    }

    #[test]
    fn a_sketch_holds_its_drawing_in_place_of_its_scene_and_the_page_around_it_is_kept() {
        let html = page(&format!(r#"<armada-sketch title="Flow"><script type="application/json">{SCENE}</script></armada-sketch>"#));
        let out = drawn_with(&html, |at, title, scene| {
            assert_eq!((at, title, scene), (0, "Flow", SCENE));
            Some("<svg>drawn</svg>".into())
        });
        assert!(out.contains(r#"<armada-sketch title="Flow"><svg>drawn</svg></armada-sketch><p>After.</p>"#), "{out}");
        assert!(!out.contains("application/json"), "the scene is gone from the element");
        assert!(out.contains("<style>armada-sketch{") && out.find("<style>") < out.find("</head>"), "its look is in the head");
    }

    #[test]
    fn a_scene_in_an_attribute_is_read_with_its_entities() {
        let html = page(r#"<armada-sketch title='One' scene="{&quot;nodes&quot;:[],&quot;edges&quot;:[]}"></armada-sketch>"#);
        let mut seen = String::new();
        drawn_with(&html, |_, _, scene| {
            seen = scene.to_string();
            None
        });
        assert_eq!(seen, r#"{"nodes":[],"edges":[]}"#);
    }

    #[test]
    fn a_sketch_in_a_take_is_drawn_like_any_other_and_each_keeps_its_place() {
        let one = format!(r#"<armada-sketch title="Chip"><script type="application/json">{SCENE}</script></armada-sketch>"#);
        let html = page(&format!(
            r#"{one}<armada-vote id="mark" form="pick"><armada-take id="chip" title="Chip"><p>Words.</p>{one}</armada-take><armada-take id="fade" title="Fade">{one}</armada-take></armada-vote>"#
        ));
        let mut places = Vec::new();
        let out = drawn_with(&html, |at, _, _| {
            places.push(at);
            Some(format!("<svg>{at}</svg>"))
        });
        assert_eq!(places, [0, 1, 2]);
        assert!(out.contains(r#"<armada-take id="chip" title="Chip"><p>Words.</p><armada-sketch title="Chip"><svg>1</svg></armada-sketch></armada-take>"#), "{out}");
        assert!(out.contains(r#"<armada-take id="fade" title="Fade"><armada-sketch title="Chip"><svg>2</svg></armada-sketch></armada-take>"#), "{out}");
    }

    #[test]
    fn a_sketch_that_cannot_be_drawn_and_a_page_with_none_are_left_as_written() {
        let html = page(r#"<armada-sketch title="Bad"><script type="application/json">{"nodes":1}</script></armada-sketch><armada-sketchy>x</armada-sketchy>"#);
        assert_eq!(drawn_with(&html, |_, _, _| None), html, "no drawing, no change and no style");
        let plain = page("<p>No sketch.</p>");
        assert_eq!(drawn_with(&plain, |_, _, _| unreachable!()), plain);
    }

    #[test]
    fn a_quoted_greater_than_does_not_end_the_opening_tag() {
        let html = page(r#"<armada-sketch title="a > b"><script type="application/json">{}</script></armada-sketch>"#);
        let out = drawn_with(&html, |_, title, _| {
            assert_eq!(title, "a > b");
            Some("<svg/>".into())
        });
        assert!(out.contains(r#"<armada-sketch title="a > b"><svg/></armada-sketch>"#), "{out}");
    }
}
