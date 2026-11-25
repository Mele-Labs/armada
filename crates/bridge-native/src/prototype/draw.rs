//! The scene as gpui elements: each part of the markup is drawn from `crate::parts`, so a
//! prototype looks as the app does, and each `data-part` is a labelled element, so VoiceOver, a
//! note and a walk name it.
//!
//! **One walk does two jobs.** Drawing a scene and checking one are the same pass over the same
//! tree, so a name the draw would mark can never be one the check missed. A check draws every
//! `<state>`, binds each list's first item and discards the elements; it keeps the trouble.

use gpui::{div, prelude::*, AnyElement, Div, ElementId, Hsla, Role, SharedString, Stateful, WeakEntity};
use serde_json::Value;

use super::fixtures::{Fixtures, Gap, Read, Scope};
use super::markup::{Kid, Node, Scene};
use super::style::{self, Align, Justify, Size, Style, Trouble};
use super::PrototypeView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::parts::{self, button::Kind, text::TextStyle, Tone};

/// What a pass reads: the scene, its data, and the state in front.
pub struct Walk<'a> {
    look: &'a Look,
    scene: &'a Scene,
    fixtures: &'a Fixtures,
    state: &'a str,
    /// A check draws every state; a draw only the one in front.
    check: bool,
    /// Who a press is told: the view the scene is drawn in.
    view: Option<WeakEntity<PrototypeView>>,
    locals: Vec<(String, &'a Value)>,
    /// The lists being walked, innermost last: which bindings of a list's items found a field on some item.
    lists: Vec<Walked>,
    /// Whether the part being drawn sits in a column: a small part then keeps its own width, as it does in HTML's inline flow.
    in_col: bool,
    pub trouble: Trouble,
}

/// A list being walked: the name its items take, the fixture it reads, and each binding on an item with whether any item had it.
struct Walked {
    local: String,
    source: String,
    seen: Vec<(String, bool)>,
}

/// The scene, drawn in `state`, and the trouble it had.
pub fn draw<'a>(look: &'a Look, scene: &'a Scene, fixtures: &'a Fixtures, state: &'a str, view: WeakEntity<PrototypeView>) -> (AnyElement, Trouble) {
    let mut walk = Walk { look, scene, fixtures, state, check: false, view: Some(view), locals: Vec::new(), lists: Vec::new(), in_col: false, trouble: Trouble::default() };
    let drawn = walk.node(&scene.root, "n", 0).unwrap_or_else(|| div().into_any_element());
    (drawn, walk.trouble)
}

/// The trouble the scene would have drawn, in any state.
pub fn check(look: &Look, scene: &Scene, fixtures: &Fixtures) -> Trouble {
    let mut walk = Walk { look, scene, fixtures, state: "", check: true, view: None, locals: Vec::new(), lists: Vec::new(), in_col: false, trouble: Trouble::default() };
    let _ = walk.node(&scene.root, "n", 0);
    walk.trouble
}

/// A registry glyph the icon set has.
pub fn has_glyph(name: &str) -> bool {
    use gpui::AssetSource as _;
    crate::assets::Assets.load(&crate::assets::icon_path(name)).ok().flatten().is_some()
}

impl<'a> Walk<'a> {
    fn scope(&self) -> Scope<'_> {
        Scope { fixtures: self.fixtures, locals: &self.locals }
    }

    /// The value a path names, held by the fixtures and so outliving the pass.
    fn value_of(&self, whole: &str) -> Option<&'a Value> {
        let (name, rest) = whole.split_once('.').unwrap_or((whole, ""));
        let root: &'a Value = self.locals.iter().rev().find(|(local, _)| local == name).map(|(_, item)| *item).or_else(|| self.fixtures.value(name))?;
        super::fixtures::value_at(root, rest)
    }

    /// `text` with its bindings read. A binding that names nothing draws nothing: on one item of a list
    /// that is only that item's data, and it is trouble only where no item of the list has the field.
    fn bind(&mut self, text: &str) -> String {
        let mut reads = Vec::new();
        let bound = self.scope().bind(text, &mut reads);
        for read in reads {
            self.read(read);
        }
        bound
    }

    fn read(&mut self, read: Read) {
        let in_list = read.local.as_ref().and_then(|local| self.lists.iter().rposition(|walked| &walked.local == local));
        match (read.gap, in_list) {
            (None, Some(at)) => self.note_seen(at, read.whole, true),
            (Some(Gap::Nothing), Some(at)) => self.note_seen(at, read.whole, false),
            (Some(Gap::Nothing), None) => self.trouble.push(format!("`{{{}}}` matches no field in {}", read.whole, read.fixture)),
            (Some(Gap::Many(why)), _) => self.trouble.push(format!("`{{{}}}` {why}", read.whole)),
            (None, None) => {}
        }
    }

    fn note_seen(&mut self, at: usize, whole: String, found: bool) {
        let seen = &mut self.lists[at].seen;
        match seen.iter_mut().find(|(one, _)| *one == whole) {
            Some((_, hit)) => *hit |= found,
            None => seen.push((whole, found)),
        }
    }

    fn attr(&mut self, node: &Node, name: &str) -> Option<String> {
        node.attr(name).map(|text| self.bind(text))
    }

    fn colour(&mut self, node: &Node, attr: &str) -> Option<Hsla> {
        let word = node.attr(attr)?;
        Some(style::colour(self.look, node, attr, word, &mut self.trouble))
    }

    /// The tone a part draws in: its `tone`, else `default`.
    fn tone(&mut self, node: &Node, default: Tone) -> Hsla {
        self.colour(node, "tone").unwrap_or_else(|| default.ink(self.look))
    }

    fn glyph_attr(&mut self, node: &Node, name: &str) -> Option<String> {
        let word = self.attr(node, name)?;
        if has_glyph(&word) {
            Some(word)
        } else {
            self.trouble.push(format!("<{}> {name}=\"{word}\" is no registry glyph", node.tag));
            Some("circle-question-mark".to_string())
        }
    }

    fn kids(&mut self, node: &Node, path: &str) -> Vec<AnyElement> {
        let was = self.in_col;
        self.in_col = match node.tag.as_str() {
            "scene" | "state" | "pane" | "card" | "list" => true,
            "box" => node.attr("dir") != Some("row"),
            _ => false,
        };
        let drawn = self.kids_in(node, path);
        self.in_col = was;
        drawn
    }

    fn kids_in(&mut self, node: &Node, path: &str) -> Vec<AnyElement> {
        let mut drawn = Vec::new();
        let mut seen: Vec<&str> = Vec::new();
        for (at, kid) in node.kids.iter().enumerate() {
            match kid {
                Kid::Node(child) => {
                    let first = !seen.contains(&child.tag.as_str());
                    seen.push(child.tag.as_str());
                    drawn.extend(self.node(child, &format!("{path}.{at}"), usize::from(!first)));
                }
                Kid::Text(words) => {
                    let said = self.bind(words);
                    if said.is_empty() {
                        continue;
                    }
                    drawn.push(parts::text::text(self.look, ElementId::Name(format!("{path}.{at}").into()), TextStyle::Body, said).into_any_element());
                }
            }
        }
        drawn
    }

    /// One part. `seen` is how many of its tag came before it among its siblings: the first row of a list draws no rule above it.
    fn node(&mut self, node: &Node, path: &str, seen: usize) -> Option<AnyElement> {
        let look = self.look;
        let id = || ElementId::Name(SharedString::from(path.to_string()));
        let style = style::read(look, node, &mut self.trouble);
        let name = self.attr(node, "data-part");
        match node.tag.as_str() {
            "fixture" => None,
            "state" => {
                let shown = self.check || node.attr("id") == Some(self.state);
                if node.attr("id").is_none() {
                    self.trouble.push("<state> has no id".to_string());
                }
                let kids = self.kids(node, path);
                shown.then(|| self.finish(div().id(id()).flex().flex_col().gap(look.length(token::SPACE_3)).children(kids), node, &style, Role::Group, name, String::new()))
            }
            "scene" => {
                let kids = self.kids(node, path);
                Some(self.finish(div().id(id()).flex().flex_col().gap(look.length(token::SPACE_3)).p(look.length(token::SPACE_4)).children(kids), node, &style, Role::Group, name, String::new()))
            }
            "box" => {
                let kids = self.kids(node, path);
                Some(self.finish(div().id(id()).flex().flex_col().children(kids), node, &style, Role::Group, name, String::new()))
            }
            "pane" => {
                let title = self.attr(node, "title").unwrap_or_default();
                let kids = self.kids(node, path);
                let body = div().flex().flex_col().gap(look.length(token::SPACE_3)).p(look.length(token::SPACE_3)).children(kids);
                let pane = parts::pane::pane(look, id(), title.clone()).child(body);
                Some(self.finish(pane, node, &style, Role::Region, name.or(Some(title)).filter(|name| !name.is_empty()), String::new()))
            }
            "card" => {
                let hue = self.colour(node, "tone");
                let title = self.attr(node, "title");
                let kids = self.kids(node, path);
                let card = parts::card::card(look, id(), hue)
                    .children(title.clone().map(|title| parts::card::head(look, ElementId::Name(format!("{path}.head").into()), hue, title)))
                    .child(parts::card::body(look, ElementId::Name(format!("{path}.body").into())).children(kids));
                Some(self.finish(card, node, &style, Role::Group, name.or(title).filter(|name| !name.is_empty()), String::new()))
            }
            "row" => {
                let title = self.attr(node, "title").filter(|title| !title.is_empty());
                let says = self.attr(node, "says").filter(|says| !says.is_empty());
                let glyph = self.glyph_attr(node, "glyph").map(|glyph| (glyph, self.tone(node, Tone::Neutral)));
                let controls = self.kids(node, path);
                let row = parts::row::row(look, id(), seen == 0, glyph.as_ref().map(|(glyph, ink)| (glyph.as_str(), *ink)), title.clone().map(SharedString::from), says.map(SharedString::from), controls);
                Some(self.finish(row, node, &style, Role::Row, name.or(title).filter(|name| !name.is_empty()), String::new()))
            }
            "chip" => {
                let ink = self.tone(node, Tone::Neutral);
                let glyph = self.glyph_attr(node, "glyph");
                let words = self.words(node);
                if words.is_empty() {
                    return None;
                }
                let chip = parts::chip::chip(look, id(), ink, glyph.as_deref(), words.clone());
                Some(self.finish(chip, node, &style, Role::Status, name, words))
            }
            "button" => {
                let kind = match node.attr("kind") {
                    None => Kind::Secondary,
                    Some(word) => Kind::named(word).unwrap_or_else(|| {
                        self.trouble.push(format!("<button> kind=\"{word}\" is not primary, secondary or ghost"));
                        Kind::Secondary
                    }),
                };
                let glyph = self.glyph_attr(node, "glyph");
                let words = self.words(node);
                let button = parts::button::button(look, id(), kind, glyph.as_deref(), words.clone(), node.flag("off"));
                Some(self.finish(button, node, &style, Role::Button, name, words))
            }
            "tabs" => {
                let kids = self.kids(node, path);
                Some(self.finish(parts::tabs::track(look, id()).children(kids), node, &style, Role::TabList, name, String::new()))
            }
            "tab" => {
                let words = self.words(node);
                let picked = node.flag("active") || node.attr("goes").is_some_and(|goes| goes == self.state);
                Some(self.finish(parts::tabs::tab(look, id(), words.clone(), picked), node, &style, Role::Tab, name, words))
            }
            "list" => Some(self.list(node, path, &style, name)),
            "text" => {
                let words = self.words(node);
                if words.is_empty() {
                    return None;
                }
                let kind = match node.attr("style") {
                    None => TextStyle::Body,
                    Some(word) => TextStyle::named(word).unwrap_or_else(|| {
                        self.trouble.push(format!("<text> style=\"{word}\" is not body, heading, title, muted, small or mono"));
                        TextStyle::Body
                    }),
                };
                let tone = self.colour(node, "tone");
                let line = parts::text::text(look, id(), kind, words.clone()).when_some(tone, |line, tone| line.text_color(tone));
                Some(self.finish(line, node, &style, Role::Label, name, words))
            }
            "glyph" => {
                let glyph = self.glyph_attr(node, "name");
                let ink = self.colour(node, "tone").unwrap_or_else(|| look.color(token::FG_DEFAULT));
                let step = match node.attr("size") {
                    None => look.length(token::SPACE_4),
                    Some(word) => look.length_named(&super::style::space_token(word)).unwrap_or_else(|_| {
                        self.trouble.push(format!("<glyph> size=\"{word}\" is no space step"));
                        look.length(token::SPACE_4)
                    }),
                };
                let mark = div().id(id()).flex_none().flex().items_center().justify_center().children(glyph.map(|glyph| parts::glyph(&glyph, step, ink)));
                let label = name.or_else(|| node.attr("name").map(str::to_string));
                Some(self.finish(mark, node, &style, Role::Image, label, String::new()))
            }
            other => {
                self.trouble.push(format!("<{other}> is no part of a scene"));
                let marker = look.unknown_marker();
                let boxed = div()
                    .id(id())
                    .flex_none()
                    .px(look.length(token::SPACE_2))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(marker)
                    .text_color(marker)
                    .text_size(look.length(token::TEXT_XS))
                    .child(format!("<{other}>"));
                Some(boxed.into_any_element())
            }
        }
    }

    /// A list: its children one under another, or one set of children for each item of an array.
    fn list(&mut self, node: &Node, path: &str, style: &Style, name: Option<String>) -> AnyElement {
        let look = self.look;
        let mut entries: Vec<AnyElement> = Vec::new();
        let each = node.attr("each").map(str::to_string);
        match each {
            None => {
                let kids = self.kids(node, path);
                entries.extend(kids.into_iter().enumerate().map(|(at, kid)| parts::list::entry(look, at == 0, kid).into_any_element()));
            }
            Some(whole) => {
                let local = node.attr("as").unwrap_or("item").to_string();
                let limit = node.attr("limit").and_then(|limit| limit.parse::<usize>().ok());
                let items = self.value_of(&whole);
                match items {
                    Some(Value::Array(items)) => {
                        let source = whole.split('.').next().unwrap_or_default().to_string();
                        self.lists.push(Walked { local: local.clone(), source, seen: Vec::new() });
                        for (at, item) in items.iter().take(limit.unwrap_or(items.len())).enumerate() {
                            self.locals.push((local.clone(), item));
                            let drawn = self.kids(node, &format!("{path}#{at}"));
                            self.locals.pop();
                            let group = div().id(ElementId::Name(format!("{path}#{at}").into())).flex().flex_col().children(drawn);
                            entries.push(parts::list::entry(look, at == 0, group.into_any_element()).into_any_element());
                        }
                        if let Some(walked) = self.lists.pop() {
                            for (whole, _) in walked.seen.into_iter().filter(|(_, hit)| !hit) {
                                self.trouble.push(format!("`{{{whole}}}` matches no field in {}", walked.source));
                            }
                        }
                    }
                    _ => self.trouble.push(format!("<list> each=\"{whole}\" names no array")),
                }
            }
        }
        self.finish(parts::list::list(look, ElementId::Name(SharedString::from(path.to_string()))).children(entries), node, style, Role::List, name, String::new())
    }

    /// The words directly in a part, bound.
    fn words(&mut self, node: &Node) -> String {
        let words = node.words();
        self.bind(&words)
    }

    /// The box attributes, the press `goes` makes and the label, put on a drawn part.
    fn finish(&mut self, drawn: Stateful<Div>, node: &Node, style: &Style, role: Role, name: Option<String>, words: String) -> AnyElement {
        let look = self.look;
        let mut drawn = apply(drawn, style, look);
        if self.in_col && style.align.is_none() && matches!(node.tag.as_str(), "chip" | "button" | "tabs" | "glyph") {
            drawn = drawn.self_start();
        }
        let goes = node.attr("goes").map(str::to_string);
        if let Some(target) = &goes {
            if !self.scene.states.iter().any(|state| state == target) {
                self.trouble.push(format!("<{}> goes=\"{target}\" names no state", node.tag));
            }
            if let Some(view) = self.view.clone() {
                let (target, key_target) = (target.clone(), target.clone());
                let key_view = view.clone();
                drawn = drawn
                    .focusable()
                    .tab_stop(true)
                    .cursor_pointer()
                    .on_click(move |_, window, cx| go(&view, &target, window, cx))
                    .on_key_down(move |event, window, cx| {
                        let key = event.keystroke.key.as_str();
                        if (key == "enter" || key == "space") && !event.keystroke.modifiers.modified() {
                            go(&key_view, &key_target, window, cx);
                            cx.stop_propagation();
                        }
                    });
            }
        }
        let label = name.or_else(|| goes.is_some().then(|| words.clone()).filter(|words| !words.is_empty()));
        match label {
            Some(label) => labelled(drawn, role, label).into_any_element(),
            None => drawn.into_any_element(),
        }
    }
}

/// A press on a part that goes to `state`. The part may be gone once the state is drawn, so the keys go back to the scene's own root.
fn go(view: &WeakEntity<PrototypeView>, state: &str, window: &mut gpui::Window, cx: &mut gpui::App) {
    if let Ok(keys) = view.update(cx, |this, cx| {
        this.go(state, cx);
        this.focus.clone()
    }) {
        window.focus(&keys, cx);
    }
}

/// The box attributes a part carries, on its element.
fn apply(mut drawn: Stateful<Div>, style: &Style, look: &Look) -> Stateful<Div> {
    match style.row {
        Some(true) => drawn = drawn.flex().flex_row(),
        Some(false) => drawn = drawn.flex().flex_col(),
        None => {}
    }
    if let Some((y, x)) = style.pad {
        drawn = drawn.py(y).px(x);
    }
    if let Some(gap) = style.gap {
        drawn = drawn.gap(gap);
    }
    if let Some(radius) = style.radius {
        drawn = drawn.rounded(radius);
    }
    if let Some(fill) = style.fill {
        drawn = drawn.bg(fill);
    }
    if let Some(edge) = style.edge {
        drawn = drawn.border(look.length(token::BORDER_WIDTH)).border_color(edge);
    }
    drawn = match style.align {
        Some(Align::Start) => drawn.items_start(),
        Some(Align::Center) => drawn.items_center(),
        Some(Align::End) => drawn.items_end(),
        Some(Align::Stretch) => drawn.items_stretch(),
        None => drawn,
    };
    drawn = match style.justify {
        Some(Justify::Start) => drawn.justify_start(),
        Some(Justify::Center) => drawn.justify_center(),
        Some(Justify::End) => drawn.justify_end(),
        Some(Justify::Between) => drawn.justify_between(),
        None => drawn,
    };
    if style.grow {
        drawn = drawn.flex_1().min_w_0();
    }
    drawn = match style.w {
        Some(Size::Points(points)) => drawn.w(points),
        Some(Size::Fraction(fraction)) => drawn.w(gpui::relative(fraction)),
        None => drawn,
    };
    match style.h {
        Some(Size::Points(points)) => drawn.h(points),
        Some(Size::Fraction(fraction)) => drawn.h(gpui::relative(fraction)),
        None => drawn,
    }
}
