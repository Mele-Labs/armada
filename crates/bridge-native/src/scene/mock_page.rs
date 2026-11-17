//! The page a `mock` node is drawn from: the sealed HTML a web view with JavaScript off loads and snapshots. It is
//! Classic's `MockView` built as text, since there is no script to adopt sheets or add fonts after the load: the
//! kept sheets, then the node's own `css`, as `<style>` blocks, each font as an `@font-face` carrying its bytes, then
//! the node's markup cleaned by `mockNodes` (`mock-frame.ts`). A policy sealing the page rides in it as `export-svg.ts`
//! sets one, widened for fonts only.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use gpui::AssetSource;
use html5ever::serialize::{serialize, SerializeOpts, TraversalScope};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{parse_document, parse_fragment, Attribute, LocalName, Namespace, ParseOpts, QualName};
use ipc::{KeptFont, KeptStyles};
use markup5ever_rcdom::{Handle, Node, NodeData, RcDom, SerializableHandle};

use crate::assets::{icon_path, Assets};

/// `MOCK_POLICY` of `export-svg.ts`, with the `data:` fonts the page carries allowed and nothing else.
const POLICY: &str = "default-src 'none'; style-src 'unsafe-inline'; font-src data:";
/// Dropped from the markup with everything inside them: nothing that runs, loads or reaches outside the page is drawn.
const DROPPED: [&str; 11] = ["script", "style", "link", "iframe", "frame", "object", "embed", "base", "meta", "noscript", "template"];
/// Attributes that carry an address; a fragment alone is kept for an SVG's `use`.
const ADDRESSES: [&str; 9] = ["src", "srcset", "href", "xlink:href", "action", "formaction", "poster", "background", "data"];
/// The size of a glyph with no `data-size`.
const GLYPH: u32 = 16;
const HTML_NS: &str = "http://www.w3.org/1999/xhtml";

/// Under the kept sheets: the app's sans, so markup no sheet gives a face is not drawn in the web view's serif.
const BASE: &str = "html{font-family:var(--font-sans,system-ui,sans-serif);}";
/// Over everything: a mock is a still, so an animation stands at its end and a transition has none. An entry animation
/// whose first frame is clear (`armada-dialog-in`) otherwise leaves the snapshot blank.
const STILL: &str = "*,*::before,*::after{animation-duration:0s!important;animation-delay:0s!important;animation-iteration-count:1!important;transition:none!important;}";

/// The page for a `mock`: its `html` and own `css`, the style set Fleet kept for it (`None` where it has none: no
/// sheets, no fonts, no theme) and the theme the stylesheets switch on, as Classic sets `data-theme` on the root.
pub fn page(html: &str, css: Option<&str>, kept: Option<&KeptStyles>, dark: bool) -> String {
    let mut head = format!(r#"<meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="{POLICY}">"#);
    if let Some(kept) = kept {
        let faces: String = kept.fonts.iter().filter_map(font_face).collect();
        if !faces.is_empty() {
            let _ = write!(head, "<style>{faces}</style>");
        }
        for sheet in &kept.sheets {
            let _ = write!(head, "<style>{}</style>", style_text(&sheet.css));
        }
    }
    if let Some(css) = css {
        let _ = write!(head, "<style>{}</style>", style_text(css));
    }
    let theme = match kept {
        Some(_) => format!(r#" data-theme="{}""#, if dark { "dark" } else { "light" }),
        None => String::new(),
    };
    format!("<!doctype html><html{theme}><head>{head}</head><body>{}</body></html>", sanitise(html))
}

/// The mock as a sheet in a page that holds it among its own words, a sketch on a document page: the kept sheets, its
/// own css and the still, scoped to it so no rule reaches the page around it. The page carries the tokens and the face;
/// a font, which no `@scope` can hold, is left out.
pub(crate) fn sheet(html: &str, css: Option<&str>, kept: Option<&KeptStyles>) -> String {
    let mut rules = String::new();
    let sheets = kept.map(|kept| kept.sheets.iter().map(|sheet| sheet.css.as_str()).collect()).unwrap_or_else(Vec::new);
    // A sheet whose braces do not pair would close the scope and reach the page, so it is left out.
    for css in sheets.into_iter().chain(css).filter(|css| closed(css)) {
        rules.push_str(&style_text(css));
        rules.push('\n');
    }
    rules.push_str(STILL);
    format!(r#"<div xmlns="{HTML_NS}" class="sk-mock"><style>@scope {{ {rules} }}</style>{}</div>"#, sanitise(html))
}

/// Whether every block a sheet opens it closes, and none it did not open, past its comments and strings.
fn closed(css: &str) -> bool {
    let (mut depth, mut chars) = (0u32, css.chars().peekable());
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut star = false;
                loop {
                    match chars.next() {
                        None => return false,
                        Some('/') if star => break,
                        Some(one) => star = one == '*',
                    }
                }
            }
            '"' | '\'' => loop {
                match chars.next() {
                    None | Some('\n') => return false,
                    Some('\\') => {
                        chars.next();
                    }
                    Some(end) if end == c => break,
                    Some(_) => {}
                }
            },
            '{' => depth += 1,
            '}' => match depth.checked_sub(1) {
                Some(less) => depth = less,
                None => return false,
            },
            _ => {}
        }
    }
    depth == 0
}

/// A sheet's text safe inside a `<style>`: a `</style` in it would end the block, as `export-svg.ts` guards.
fn style_text(css: &str) -> Cow<'_, str> {
    if !css.to_ascii_lowercase().contains("</style") {
        return Cow::Borrowed(css);
    }
    let lower = css.to_ascii_lowercase();
    let mut out = String::with_capacity(css.len() + 4);
    let mut from = 0;
    while let Some(at) = lower[from..].find("</style") {
        out.push_str(&css[from..from + at]);
        out.push_str("<\\/style");
        from += at + "</style".len();
    }
    out.push_str(&css[from..]);
    Cow::Owned(out)
}

/// A kept font as the `@font-face` that carries its bytes. One whose bytes are not base64 is left out.
fn font_face(font: &KeptFont) -> Option<String> {
    use base64::Engine as _;
    let plain = |text: &str| !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || " .%-".contains(c));
    if font.bytes.is_empty() || !font.bytes.chars().all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c)) {
        return None;
    }
    let head = base64::engine::general_purpose::STANDARD.decode(&font.bytes[..font.bytes.len().min(8) / 4 * 4]).unwrap_or_default();
    let (mime, format) = match head.get(..4)? {
        b"wOF2" => ("font/woff2", "woff2"),
        b"wOFF" => ("font/woff", "woff"),
        b"OTTO" => ("font/otf", "opentype"),
        [0, 1, 0, 0] | b"true" => ("font/ttf", "truetype"),
        _ => return None,
    };
    let family: String = font.family.chars().filter(|c| !matches!(c, '"' | '\\' | '<' | '>' | '\n' | '\r')).collect();
    let weight = if plain(&font.weight) { font.weight.as_str() } else { "400" };
    let style = font.style.as_deref().filter(|style| plain(style)).map(|style| format!("font-style:{style};")).unwrap_or_default();
    Some(format!(r#"@font-face{{font-family:"{family}";font-weight:{weight};{style}src:url(data:{mime};base64,{}) format("{format}");}}"#, font.bytes))
}

/// The HTML cleaned as `mockNodes` cleans it: parsed inert, with no script, handler, address or inline style left, and
/// `<i data-icon>` swapped for its glyph. What comes back is what goes between `<body>` and `</body>`.
pub fn sanitise(html: &str) -> String {
    let opts = ParseOpts { tree_builder: TreeBuilderOpts { scripting_enabled: false, ..Default::default() }, ..Default::default() };
    let dom = parse_document(RcDom::default(), opts).one(format!("<!doctype html><body>{html}").as_str());
    let Some(body) = first_element(&dom.document, "body") else { return String::new() };
    clean(&body);
    let mut out = Vec::new();
    let opts = SerializeOpts { scripting_enabled: false, traversal_scope: TraversalScope::ChildrenOnly(None), create_missing_parent: false };
    if serialize(&mut out, &SerializableHandle::from(body), opts).is_err() {
        return String::new();
    }
    String::from_utf8(out).unwrap_or_default()
}

/// A part of a mock's markup a note may name, in the order the markup holds them, with no layout.
#[derive(Clone, Debug, PartialEq)]
pub struct Outlined {
    pub key: String,
    pub label: String,
    pub depth: u32,
}

/// The markup's parts as the page reports them (`PARTS` in `web/render.rs`): the nearest element carrying `data-part`, else the
/// nearest with words, once each. A walk has no web view to measure them, so it stands them in a column.
pub fn outline(html: &str) -> Vec<Outlined> {
    let opts = ParseOpts { tree_builder: TreeBuilderOpts { scripting_enabled: false, ..Default::default() }, ..Default::default() };
    let dom = parse_document(RcDom::default(), opts).one(format!("<!doctype html><body>{html}").as_str());
    let Some(body) = first_element(&dom.document, "body") else { return Vec::new() };
    clean(&body);
    let mut found: Vec<Outlined> = Vec::new();
    let mut named: Vec<(String, Handle)> = Vec::new();
    collect_named(&body, &mut named);
    visit(&body, &mut vec![body.clone()], &named, &mut found);
    found
}

fn words_of(at: &Handle) -> String {
    let mut text = String::new();
    fn add(at: &Handle, text: &mut String) {
        match &at.data {
            NodeData::Text { contents } => text.push_str(&contents.borrow()),
            _ => at.children.borrow().iter().for_each(|child| add(child, text)),
        }
    }
    add(at, &mut text);
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn data_part(at: &Handle) -> Option<String> {
    match &at.data {
        NodeData::Element { attrs, .. } => value_of(&attrs.borrow(), "data-part"),
        _ => None,
    }
}

fn collect_named(at: &Handle, into: &mut Vec<(String, Handle)>) {
    for child in at.children.borrow().iter() {
        if let Some(name) = data_part(child) {
            into.push((name, child.clone()));
        }
        collect_named(child, into);
    }
}

fn has_own_words(at: &Handle) -> bool {
    at.children.borrow().iter().any(|child| matches!(&child.data, NodeData::Text { contents } if !contents.borrow().trim().is_empty()))
}

/// `partKey`: a `data-part` (and its place among the same name where it repeats), else the path of `nth-child`s from the body.
/// `chain` is the body and then each element down to the part.
fn key_of(chain: &[Handle], named: &[(String, Handle)]) -> String {
    let Some(part) = chain.last() else { return String::new() };
    if let Some(name) = data_part(part) {
        let same: Vec<_> = named.iter().filter(|(other, _)| *other == name).collect();
        return match same.iter().position(|(_, one)| std::rc::Rc::ptr_eq(one, part)) {
            Some(at) if same.len() > 1 => format!("{name}[{}]", at + 1),
            _ => name,
        };
    }
    let mut path = vec!["body".to_string()];
    for pair in chain.windows(2) {
        let (parent, one) = (&pair[0], &pair[1]);
        let NodeData::Element { name, .. } = &one.data else { continue };
        let nth = parent.children.borrow().iter().filter(|child| matches!(child.data, NodeData::Element { .. })).position(|child| std::rc::Rc::ptr_eq(child, one)).map_or(1, |index| index + 1);
        path.push(format!("{}:nth-child({nth})", name.local));
    }
    path.join(" > ")
}

fn visit(at: &Handle, trail: &mut Vec<Handle>, named: &[(String, Handle)], found: &mut Vec<Outlined>) {
    for child in at.children.borrow().iter() {
        if !matches!(child.data, NodeData::Element { .. }) {
            continue;
        }
        trail.push(child.clone());
        if data_part(child).is_some() || has_own_words(child) {
            // The nearest ancestor (or itself) naming a part, else the nearest with words.
            let part = trail.iter().skip(1).rposition(|one| data_part(one).is_some()).or_else(|| trail.iter().skip(1).rposition(|one| !words_of(one).is_empty()));
            if let Some(at) = part.map(|at| at + 1) {
                let key = key_of(&trail[..=at], named);
                if !found.iter().any(|one| one.key == key) {
                    let label = data_part(&trail[at]).unwrap_or_else(|| {
                        let said = words_of(&trail[at]);
                        if said.chars().count() > 40 {
                            format!("{}\u{2026}", said.chars().take(40).collect::<String>().trim_end())
                        } else {
                            said
                        }
                    });
                    found.push(Outlined { key, label, depth: at as u32 });
                }
            }
        }
        visit(child, trail, named, found);
        trail.pop();
    }
}

/// The first element called `local` at or below `at`'s children, one level of `html` down.
fn first_element(at: &Handle, local: &str) -> Option<Handle> {
    for child in at.children.borrow().iter() {
        if let NodeData::Element { name, .. } = &child.data {
            if &*name.local == local {
                return Some(child.clone());
            }
            if &*name.local == "html" {
                return first_element(child, local);
            }
        }
    }
    None
}

fn value_of(attrs: &[Attribute], local: &str) -> Option<String> {
    attrs.iter().find(|a| &*a.name.local == local).map(|a| a.value.to_string())
}

fn set_attr(attrs: &mut Vec<Attribute>, local: &str, value: &str) {
    let value = StrTendril::from(value);
    match attrs.iter_mut().find(|a| &*a.name.local == local) {
        Some(one) => one.value = value,
        None => attrs.push(Attribute { name: QualName::new(None, Namespace::from(""), LocalName::from(local)), value }),
    }
}

/// `at`'s children with what `mockNodes` removes gone, and the glyph standing where an icon was.
fn clean(at: &Handle) {
    let children = at.children.take();
    let mut kept = Vec::with_capacity(children.len());
    for child in children {
        match &child.data {
            NodeData::Comment { .. } | NodeData::Doctype { .. } | NodeData::ProcessingInstruction { .. } => continue,
            NodeData::Element { name, attrs, .. } => {
                if DROPPED.contains(&&*name.local) {
                    continue;
                }
                attrs.borrow_mut().retain(|a| {
                    let local = a.name.local.to_string().to_ascii_lowercase();
                    let qualified = a.name.prefix.as_ref().map_or_else(|| local.clone(), |prefix| format!("{prefix}:{local}"));
                    let address = ADDRESSES.contains(&&*qualified) || ADDRESSES.contains(&&*local);
                    !(local.starts_with("on") || local == "style" || (address && !a.value.trim().starts_with('#')))
                });
                if &*name.local == "i" && &*name.ns == HTML_NS {
                    if let Some(drawn) = glyph(&attrs.borrow()) {
                        drawn.parent.set(Some(std::rc::Rc::downgrade(at)));
                        kept.push(drawn);
                        continue;
                    }
                }
                clean(&child);
            }
            NodeData::Document | NodeData::Text { .. } => {}
        }
        kept.push(child);
    }
    *at.children.borrow_mut() = kept;
}

/// The lucide glyph a `<i data-icon data-size>` names, as `mock-frame.ts` draws it: its own attributes, but those two, laid over the glyph's.
fn glyph(attrs: &[Attribute]) -> Option<Handle> {
    let name = value_of(attrs, "data-icon")?;
    if !ipc::MOCK_GLYPHS.contains(&name.as_str()) {
        return None;
    }
    let size = value_of(attrs, "data-size").and_then(|size| size.trim().parse::<u32>().ok()).unwrap_or(GLYPH).to_string();
    // The lucide the asset set ships renamed two of the glyphs the registry names.
    let file = match name.as_str() {
        "circle-help" => "circle-question-mark",
        "filter" => "funnel",
        other => other,
    };
    let bytes = Assets.load(&icon_path(file)).ok().flatten()?;
    let dom = parse_fragment(RcDom::default(), ParseOpts::default(), QualName::new(None, Namespace::from(HTML_NS), LocalName::from("div")), Vec::new()).one(String::from_utf8_lossy(&bytes).as_ref());
    let svg = first_element(&dom.document, "svg")?;
    // `RcDom` empties every node it drops, even one still held; the glyph leaves its tree first.
    for parent in dom.document.children.borrow().iter() {
        parent.children.borrow_mut().retain(|one| !std::rc::Rc::ptr_eq(one, &svg));
    }
    if let NodeData::Element { attrs: own, .. } = &svg.data {
        let mut own = own.borrow_mut();
        set_attr(&mut own, "width", &size);
        set_attr(&mut own, "height", &size);
        set_attr(&mut own, "class", &format!("lucide lucide-{name}"));
        set_attr(&mut own, "aria-hidden", "true");
        for a in attrs.iter().filter(|a| !matches!(&*a.name.local, "data-icon" | "data-size")) {
            set_attr(&mut own, &a.name.local, &a.value);
        }
    }
    Some(svg)
}

/// What Fleet answered for a style set, by hash, kept for the run: a hash is content addressed, so an answer never changes.
#[derive(Default)]
pub struct KeptCache {
    held: Mutex<HashMap<String, Arc<KeptStyles>>>,
}

/// A hash as Fleet holds it: 32 lower-case hex characters.
fn is_hash(hash: &str) -> bool {
    hash.len() == 32 && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl KeptCache {
    /// The style set already read for `hash`.
    pub fn held(&self, hash: &str) -> Option<Arc<KeptStyles>> {
        self.held.lock().ok()?.get(hash).cloned()
    }

    /// The style set `hash` names, asked of Fleet once (`GET /sessions/kept_styles?hash=`) and kept. **Blocks**: run it
    /// on the background executor. With no `port` the mock Fleet answers. A refusal or a failed read is not kept, so a
    /// later frame asks again.
    pub fn read(&self, port: Option<u16>, hash: &str) -> Result<Arc<KeptStyles>, String> {
        if !is_hash(hash) {
            return Err(format!("{hash:?} is not a style set hash"));
        }
        if let Some(had) = self.held(hash) {
            return Ok(had);
        }
        let read = match port {
            None => crate::mock::kept_styles(hash),
            Some(port) => {
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                let (status, bytes) = runtime.block_on(crate::transport::get(port, &format!("/sessions/kept_styles?hash={hash}")))?;
                if !(200..300).contains(&status) {
                    return Err(format!("Fleet answered {status}: {}", String::from_utf8_lossy(&bytes)));
                }
                ipc::decode::<KeptStyles>("KeptStyles", &bytes).map_err(|why| why.to_string())?
            }
        };
        let read = Arc::new(read);
        if let Ok(mut held) = self.held.lock() {
            held.insert(hash.to_string(), read.clone());
        }
        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::KeptSheet;

    fn sheet(css: &str) -> KeptSheet {
        KeptSheet { path: "a.css".into(), css: css.into(), edited: false }
    }

    #[test]
    fn a_part_is_the_nearest_data_part_else_the_nearest_words() {
        let parts = outline(r#"<div><span>2 behind</span><b data-part="Count">9</b></div><p>Pull <i>requests</i></p>"#);
        let said: Vec<_> = parts.iter().map(|one| (one.key.as_str(), one.label.as_str())).collect();
        assert_eq!(said, [
                ("body > div:nth-child(1) > span:nth-child(1)", "2 behind"),
                ("Count", "Count"),
                ("body > p:nth-child(2)", "Pull requests"),
                ("body > p:nth-child(2) > i:nth-child(1)", "requests")
            ]);
    }

    #[test]
    fn a_name_that_repeats_is_told_apart_by_its_place() {
        let parts = outline(r#"<u data-part="Row">a</u><u data-part="Row">b</u>"#);
        assert_eq!(parts.iter().map(|one| one.key.as_str()).collect::<Vec<_>>(), ["Row[1]", "Row[2]"]);
    }

    #[test]
    fn a_script_and_every_element_that_loads_or_runs_is_dropped_with_what_is_inside() {
        // A void element has no inside; the others take their content with them.
        let void = ["link", "meta", "base", "embed", "frame"];
        for tag in DROPPED {
            let out = sanitise(&format!("<p>kept</p><{tag}>{}", if void.contains(&tag) { String::new() } else { format!("inner</{tag}>") }));
            assert!(!out.contains(&format!("<{tag}")) && !out.contains("inner"), "{tag}: {out}");
            assert!(out.contains("<p>kept</p>"), "{tag}: {out}");
        }
    }

    #[test]
    fn a_handler_is_dropped_and_the_rest_of_the_element_stays() {
        assert_eq!(sanitise(r#"<button class="b" onclick="alert(1)" ONMOUSEOVER="x()">Go</button>"#), r#"<button class="b">Go</button>"#);
    }

    #[test]
    fn an_inline_style_is_dropped() {
        assert_eq!(sanitise(r#"<div style="color:red" id="a">x</div>"#), r#"<div id="a">x</div>"#);
    }

    #[test]
    fn an_address_is_dropped_and_a_fragment_is_kept() {
        let out = sanitise(r##"<a href="https://x.test/">a</a><img src="https://x.test/a.png" srcset="b.png 2x" alt="i"><a href=" #top">b</a><form action="/go"><button formaction="/x">s</button></form><video poster="p.png" background="b.png"></video><svg><use xlink:href="#sym"/><use xlink:href="https://x.test/s.svg#a"/></svg>"##);
        for gone in ["x.test", "srcset", "/go", "formaction", "poster", "background=", "b.png"] {
            assert!(!out.contains(gone), "{gone}: {out}");
        }
        assert!(out.contains(r##"href=" #top""##) && out.contains(r##"xlink:href="#sym""##), "{out}");
        assert!(out.contains(r#"alt="i""#), "{out}");
    }

    #[test]
    fn a_comment_and_a_document_wrapper_leave_only_the_body() {
        assert_eq!(sanitise("<!-- c --><html><head><title>t</title></head><body><p>x</p></body></html>").matches("<p>x</p>").count(), 1);
        assert!(!sanitise("<!-- c --><p>x</p>").contains("<!--"));
    }

    #[test]
    fn an_icon_becomes_the_lucide_svg_at_its_size_and_keeps_its_own_class() {
        let out = sanitise(r#"<i data-icon="bot" data-size="20" class="mine" data-part="who"></i>"#);
        assert!(out.starts_with("<svg") && !out.contains("<i"), "{out}");
        assert!(out.contains(r#"width="20""#) && out.contains(r#"height="20""#), "{out}");
        assert!(out.contains(r#"class="mine""#) && out.contains(r#"data-part="who""#), "{out}");
        assert!(out.contains(r#"aria-hidden="true""#) && out.contains("<path") && !out.contains("data-icon"), "{out}");
    }

    #[test]
    fn an_icon_with_no_size_is_sixteen_and_is_named_lucide() {
        let out = sanitise(r#"<i data-icon="circle-check"></i>"#);
        assert!(out.contains(r#"width="16""#) && out.contains("lucide lucide-circle-check"), "{out}");
    }

    #[test]
    fn an_icon_the_registry_does_not_name_stays_as_written() {
        assert_eq!(sanitise(r#"<i data-icon="no-such">x</i>"#), r#"<i data-icon="no-such">x</i>"#);
    }

    #[test]
    fn every_glyph_a_mock_may_name_is_drawn() {
        for name in ipc::MOCK_GLYPHS {
            let out = sanitise(&format!(r#"<i data-icon="{name}"></i>"#));
            assert!(out.starts_with("<svg"), "{name}: {out}");
        }
    }

    #[test]
    fn the_page_carries_the_policy_and_no_script_can_reach_it() {
        let out = page(r#"<script>x()</script><p onclick="y()">a</p>"#, None, None, false);
        assert!(out.contains(&format!(r#"<meta http-equiv="Content-Security-Policy" content="{POLICY}">"#)), "{out}");
        assert!(out.contains("<body><p>a</p></body>") && !out.contains("script>x"), "{out}");
    }

    #[test]
    fn the_sheets_come_in_order_and_the_nodes_own_css_last() {
        let kept = KeptStyles { sheets: vec![sheet(".one{}"), sheet(".two{}")], fonts: Vec::new() };
        let out = page("<p>a</p>", Some(".own{}"), Some(&kept), false);
        let at = |needle: &str| out.find(needle).unwrap_or_else(|| panic!("{needle} in {out}"));
        assert!(at("<style>.one{}</style>") < at("<style>.two{}</style>") && at("<style>.two{}</style>") < at("<style>.own{}</style>") && at("<style>.own{}</style>") < at("<body>"));
    }

    #[test]
    fn a_style_end_tag_in_a_sheet_cannot_end_its_block() {
        let kept = KeptStyles { sheets: vec![sheet("a{} </STYLE><b onload=x>")], fonts: Vec::new() };
        let out = page("", None, Some(&kept), false);
        assert_eq!(out.matches("</style>").count(), 1, "{out}");
    }

    #[test]
    fn the_theme_is_set_on_the_root_when_a_style_set_is_kept() {
        let kept = KeptStyles { sheets: vec![sheet("a{}")], fonts: Vec::new() };
        assert!(page("", None, Some(&kept), true).contains(r#"<html data-theme="dark">"#));
        assert!(page("", None, Some(&kept), false).contains(r#"<html data-theme="light">"#));
        assert!(page("", None, None, true).contains("<html><head>"));
    }

    #[test]
    fn a_font_is_a_face_with_its_bytes_in_a_data_address() {
        use base64::Engine as _;
        let woff2 = base64::engine::general_purpose::STANDARD.encode(b"wOF2 some bytes");
        let font = |bytes: &str, family: &str| KeptFont { family: family.into(), weight: "100 900".into(), style: Some("italic".into()), bytes: bytes.into() };
        let kept = KeptStyles { sheets: vec![sheet("a{}")], fonts: vec![font(&woff2, r#"In"ter"#), font("not base64!", "Bad"), font(&base64::engine::general_purpose::STANDARD.encode(b"junk"), "Junk")] };
        let out = page("", None, Some(&kept), false);
        assert!(out.contains(&format!(r#"@font-face{{font-family:"Inter";font-weight:100 900;font-style:italic;src:url(data:font/woff2;base64,{woff2}) format("woff2");}}"#)), "{out}");
        assert!(!out.contains("Bad") && !out.contains("Junk"), "{out}");
        assert!(out.find("@font-face") < out.find("<style>a{}"), "{out}");
    }

    #[test]
    fn a_style_set_is_read_once_and_a_bad_hash_is_refused() {
        let cache = KeptCache::default();
        let hash = "0123456789abcdef0123456789abcdef";
        assert!(cache.held(hash).is_none());
        let first = cache.read(None, hash).expect("the mock answers");
        assert!(!first.sheets.is_empty());
        assert!(Arc::ptr_eq(&first, &cache.held(hash).expect("kept")));
        assert!(Arc::ptr_eq(&first, &cache.read(None, hash).expect("kept")));
        for bad in ["", "ABCDEF0123456789abcdef0123456789", "0123456789abcdef", "../../etc/passwd/0123456789abcdef"] {
            assert!(cache.read(None, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_mock_style_set_draws_a_chip() {
        let kept = KeptCache::default().read(None, "0123456789abcdef0123456789abcdef").expect("the mock answers");
        let out = page(r#"<span class="armada-chip">2 behind</span>"#, None, Some(&kept), true);
        assert!(out.contains(".armada-chip") && out.contains("2 behind"));
    }

    #[test]
    fn a_sheet_that_would_close_the_mocks_scope_is_left_out() {
        let kept = KeptStyles { sheets: vec![sheet(".one{}"), sheet("} body{display:none}"), sheet(r#"a{content:"}"}/* } */"#)], fonts: Vec::new() };
        let out = super::sheet("<p>a</p>", Some(".own{"), Some(&kept));
        assert!(!out.contains("display:none") && !out.contains(".own{"), "{out}");
        assert!(out.contains(".one{}") && out.contains(r#"a{content:"}"}"#) && out.contains(STILL), "{out}");
    }
}
