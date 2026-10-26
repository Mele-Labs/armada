//! What Fleet reads out of a Job's document, a page of HTML the Job designed: its title, and the
//! sketches it embeds. **A scan and not a parse**: the page is the Job's own and may be any HTML, and
//! Fleet needs three things from it. Bridge reads the same page with a real parser
//! (`session-view/document-page.ts`), and `sketches_in` follows the rules it reads a sketch by.

/// One `<armada-sketch>` of a page, in the order the page holds them.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PageSketch {
    /// The `title` attribute, where the page gave one.
    pub title: Option<String>,
    /// The scene as text, to be checked and not read here.
    pub scene: String,
}

/// The page's `<title>`, else its first `<h1>`, else the file's stem.
pub(crate) fn title_of(html: &str, path: &str) -> String {
    let lower = html.to_ascii_lowercase();
    ["title", "h1"]
        .iter()
        .find_map(|name| {
            let (_, start) = open_tag(&lower, name, 0)?;
            let end = lower[start..].find(&format!("</{name}")).map_or(lower.len(), |at| start + at);
            let words = squeezed(&unescaped(&without_tags(&html[start..end])));
            (!words.is_empty()).then_some(words)
        })
        .unwrap_or_else(|| stem_of(path))
}

fn stem_of(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem).to_string()
}

/// Every sketch the page embeds. A sketch's scene is its `<script type="application/json">` first, else
/// its `scene` attribute, as Bridge reads it; a sketch with neither has an empty scene.
pub(crate) fn sketches_in(html: &str) -> Vec<PageSketch> {
    let lower = html.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some((tag, content)) = open_tag(&lower, "armada-sketch", from) {
        let end = lower[content..].find("</armada-sketch").map_or(lower.len(), |at| content + at);
        let title = attribute(&html[tag..content], "title").filter(|title| !title.trim().is_empty());
        let scene = script_in(html, &lower, content, end)
            .or_else(|| attribute(&html[tag..content], "scene"))
            .unwrap_or_default();
        found.push(PageSketch { title, scene });
        from = end;
    }
    found
}

/// Where the next `<name ...>` opens at or after `from`, as `(start of the tag, after its '>')`.
/// `lower` is the page in lower case, so the offsets are the page's own.
fn open_tag(lower: &str, name: &str, from: usize) -> Option<(usize, usize)> {
    let needle = format!("<{name}");
    let mut at = from;
    loop {
        let start = at + lower.get(at..)?.find(&needle)?;
        let after = start + needle.len();
        match lower.as_bytes().get(after) {
            Some(b'>') | Some(b'/') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') => {
                return Some((start, tag_end(lower, after)));
            }
            _ => at = after,
        }
    }
}

/// The offset after the '>' that closes a tag opened before `from`, skipping any '>' inside a quoted value.
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

/// The text of the first `application/json` script between `from` and `end`.
fn script_in(html: &str, lower: &str, from: usize, end: usize) -> Option<String> {
    let mut at = from;
    while let Some((tag, content)) = open_tag(&lower[..end], "script", at) {
        let close = lower[content..end].find("</script").map_or(end, |found| content + found);
        if attribute(&html[tag..content], "type").is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json")) {
            return Some(html[content..close].to_string());
        }
        at = close;
    }
    None
}

/// The value of attribute `name` in the text of an opening tag, with its entities read.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(found) = lower[from..].find(name) {
        let start = from + found;
        let after = start + name.len();
        from = after;
        let separated = start > 0 && lower.as_bytes()[start - 1].is_ascii_whitespace();
        let rest = tag[after..].trim_start();
        let Some(rest) = rest.strip_prefix('=').filter(|_| separated) else { continue };
        let rest = rest.trim_start();
        let value = match rest.chars().next()? {
            quote @ ('"' | '\'') => {
                let inner = &rest[1..];
                &inner[..inner.find(quote)?]
            }
            _ => rest.split(|c: char| c.is_ascii_whitespace() || c == '>').next().unwrap_or_default(),
        };
        return Some(unescaped(value));
    }
    None
}

fn without_tags(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut inside = false;
    for c in html.chars() {
        match c {
            '<' => inside = true,
            '>' if inside => inside = false,
            _ if !inside => text.push(c),
            _ => {}
        }
    }
    text
}

fn unescaped(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
