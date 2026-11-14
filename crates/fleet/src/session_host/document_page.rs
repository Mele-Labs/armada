//! What Fleet reads out of a Job's document, a page of HTML the Job designed: its title, the actions and votes it declares, and the
//! sketches it embeds. **A scan and not a parse**: the page is the Job's own and may be any HTML, and
//! Fleet needs a few things from it. Bridge reads the same page with a real parser
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

/// Whether the page marks itself as asking his approval: `<meta name="armada-review" content="approve">`.
/// A page without it is for reading.
pub(crate) fn asks_approval(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    let mut from = 0;
    while let Some((tag, end)) = open_tag(&lower, "meta", from) {
        let said = |name: &str| attribute(&html[tag..end], name).map(|value| value.trim().to_ascii_lowercase());
        if said("name").as_deref() == Some("armada-review") && said("content").as_deref() == Some("approve") {
            return true;
        }
        from = end;
    }
    false
}

/// What the page says it is for: `<meta name="armada-for" content="...">`, and each
/// `<link rel="armada-for" href="..." title="...">` in the order the page holds them. Nothing is checked here.
pub(crate) fn for_of(html: &str) -> (Option<String>, Vec<ipc::ForLink>) {
    let lower = html.to_ascii_lowercase();
    let mut text = None;
    let mut from = 0;
    while let Some((tag, end)) = open_tag(&lower, "meta", from) {
        if text.is_none() && attribute(&html[tag..end], "name").is_some_and(|name| name.trim().eq_ignore_ascii_case("armada-for")) {
            text = attribute(&html[tag..end], "content").map(|said| squeezed(&said));
        }
        from = end;
    }
    let mut links = Vec::new();
    from = 0;
    while let Some((tag, end)) = open_tag(&lower, "link", from) {
        let said = |name: &str| attribute(&html[tag..end], name);
        if said("rel").is_some_and(|rel| rel.split_whitespace().any(|one| one.eq_ignore_ascii_case("armada-for"))) {
            links.push(ipc::ForLink { label: said("title").map(|title| squeezed(&title)).unwrap_or_default(), url: said("href").unwrap_or_default() });
        }
        from = end;
    }
    (text, links)
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

/// One `<armada-action>` of a page, in the order the page holds them, its attributes trimmed and nothing checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageAction {
    pub id: String,
    pub act: String,
    pub label: String,
    pub item: String,
    pub brief: String,
    pub repository: String,
    pub ask: String,
    pub option: String,
    /// The text of the `<tr>`, else the `<li>`, it sits in, cut to [`MOST_ROW_CHARS`] characters. Empty where it
    /// sits in neither.
    pub row: String,
}

/// The most characters of the row an action sits in that Fleet reads.
pub(crate) const MOST_ROW_CHARS: usize = 300;

/// Every action the page declares.
pub(crate) fn actions_in(html: &str) -> Vec<PageAction> {
    let lower = html.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some((tag, end)) = open_tag(&lower, "armada-action", from) {
        let said = |name: &str| attribute(&html[tag..end], name).map(|value| squeezed(&value)).unwrap_or_default();
        let row = ["tr", "li"].iter().find_map(|name| row_around(html, &lower, name, tag)).unwrap_or_default();
        found.push(PageAction {
            id: said("id"),
            act: said("act"),
            label: said("label"),
            item: said("item"),
            brief: attribute(&html[tag..end], "brief").map(|value| value.trim().to_string()).unwrap_or_default(),
            repository: said("repository"),
            ask: said("ask"),
            option: said("option"),
            row: row.chars().take(MOST_ROW_CHARS).collect(),
        });
        from = end;
    }
    found
}

/// One `<armada-vote>` of a page, in the order the page holds them, its attributes trimmed and nothing checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageVote {
    pub id: String,
    pub form: String,
    pub question: String,
    /// The named parts a `score` vote marks, in the order its `parts` lists them.
    pub parts: Vec<String>,
    pub takes: Vec<PageTake>,
}

/// One `<armada-take>` of a vote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageTake {
    pub id: String,
    pub title: String,
}

/// Every vote the page declares, each with the takes between its tag and its close. A vote is not read inside
/// another.
pub(crate) fn votes_in(html: &str) -> Vec<PageVote> {
    let lower = html.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some((tag, end)) = open_tag(&lower, "armada-vote", from) {
        let said = |name: &str| attribute(&html[tag..end], name).map(|value| squeezed(&value)).unwrap_or_default();
        let close = lower[end..].find("</armada-vote").map_or(lower.len(), |at| end + at);
        let mut takes = Vec::new();
        let mut at = end;
        while let Some((take, after)) = open_tag(&lower[..close], "armada-take", at) {
            let take_said = |name: &str| attribute(&html[take..after], name).map(|value| squeezed(&value)).unwrap_or_default();
            takes.push(PageTake { id: take_said("id"), title: take_said("title") });
            at = after;
        }
        let parts = said("parts").split(',').map(str::trim).filter(|part| !part.is_empty()).map(str::to_string).collect();
        found.push(PageVote { id: said("id"), form: said("form"), question: said("question"), parts, takes });
        from = close;
    }
    found
}

/// The words of the `<name>` element open around offset `at`, where one is: the last that opens before it and
/// does not close before it.
fn row_around(html: &str, lower: &str, name: &str, at: usize) -> Option<String> {
    let (mut open, mut from) = (None, 0);
    while let Some((start, after)) = open_tag(&lower[..at], name, from) {
        open = Some(start);
        from = after;
    }
    let start = open?;
    if lower[start..at].contains(&format!("</{name}")) {
        return None;
    }
    let end = lower[at..].find(&format!("</{name}")).map_or(lower.len(), |found| at + found);
    let words = squeezed(&unescaped(&without_tags(&html[start..end])));
    (!words.is_empty()).then_some(words)
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
pub(super) fn tag_end(lower: &str, from: usize) -> usize {
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

pub(super) fn unescaped(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

pub(super) fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
