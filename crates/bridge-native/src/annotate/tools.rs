//! The note card's tools and its pictures (`annotate/Tools.tsx`): show what he wants and not only
//! say it. Size, Colour, Fill, Spacing, Text and Variant each pick from the design tokens, and
//! every pick is a change recorded with what it was, what it is and its token. Pictures pasted or
//! dropped on the card sit under them as the note's examples.
//!
//! **What is not here:** Electron puts each change on the live element so the page shows it. A gpui
//! screen has no per-element style hook to put one on, so a change is recorded and listed and the
//! element is left as drawn. What an element reads before a change is a mock's until Fleet's face
//! carries it.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{div, img, prelude::*, App, Context, ExternalPaths, FontWeight, Image, ImageFormat, Rgba, Role, SharedString, Subscription, Window};
use gpui_component::input::{InputEvent, Textarea, TextareaState};

use crate::capture::Held;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::titlebar::parts::{glyph, icon_button, mono, tip, Btn, Weight};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Size,
    Colour,
    Fill,
    Spacing,
    Text,
    Variant,
}

impl Kind {
    const ALL: [Kind; 6] = [Kind::Size, Kind::Colour, Kind::Fill, Kind::Spacing, Kind::Text, Kind::Variant];

    fn label(self) -> &'static str {
        match self {
            Kind::Size => "Size",
            Kind::Colour => "Colour",
            Kind::Fill => "Fill",
            Kind::Spacing => "Spacing",
            Kind::Text => "Text",
            Kind::Variant => "Variant",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Kind::Size => "a-large-small",
            Kind::Colour => "palette",
            Kind::Fill => "paint-bucket",
            Kind::Spacing => "square-dashed",
            Kind::Text => "text-cursor-input",
            Kind::Variant => "swatch-book",
        }
    }
}

const SIZES: [Token; 7] = [token::TEXT_2XS, token::TEXT_XS, token::TEXT_SM, token::TEXT_BASE, token::TEXT_LG, token::TEXT_XL, token::TEXT_2XL];
const COLOURS: [Token; 11] = [
    token::FG_DEFAULT,
    token::FG_MUTED,
    token::FG_SUBTLE,
    token::FG_INVERSE,
    token::ACCENT,
    token::STATUS_RUNNING,
    token::STATUS_AWAITING_REVIEW,
    token::STATUS_ESCALATED,
    token::STATUS_COMPLETED_SUCCESS,
    token::STATUS_COMPLETED_FAILED,
    token::STATUS_REJECTED,
];
const FILLS: [Token; 11] = [
    token::BG_BASE,
    token::BG_SUNKEN,
    token::BG_RAISED,
    token::BG_OVERLAY,
    token::BG_HOVER,
    token::ACCENT_MUTED,
    token::ACCENT,
    token::STATUS_RUNNING_BG,
    token::STATUS_AWAITING_REVIEW_BG,
    token::STATUS_COMPLETED_SUCCESS_BG,
    token::STATUS_COMPLETED_FAILED_BG,
];
const SPACES: [Token; 7] = [token::SPACE_1, token::SPACE_2, token::SPACE_3, token::SPACE_4, token::SPACE_6, token::SPACE_8, token::SPACE_12];

/// A variant's "before" when the element was drawn with none picked.
const AS_DRAWN: &str = "As drawn";

/// One change a tool recorded.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub kind: Kind,
    pub property: &'static str,
    pub label: &'static str,
    pub before: SharedString,
    pub after: SharedString,
    pub token: Option<&'static str>,
}

/// A picture the note carries to show what he means.
#[derive(Clone)]
pub struct Example {
    pub name: SharedString,
    pub image: Arc<Image>,
}

/// What the card is about, which decides which tools stand.
pub enum Scope {
    /// An element: its own words (for the Text tool) and the stories its component has (for Variant).
    Element { words: Option<SharedString>, stories: Vec<&'static str> },
    /// A box he drew: nothing to restyle, so the tools stand off.
    Region,
    /// The whole window: no tools, only pictures.
    Window,
}

pub struct Tools {
    scope: Scope,
    open: Option<Kind>,
    changes: Vec<Change>,
    examples: Vec<Example>,
    text: Option<(gpui::Entity<TextareaState>, Subscription)>,
}

impl Tools {
    pub fn new(scope: Scope) -> Tools {
        Tools { scope, open: None, changes: Vec::new(), examples: Vec::new(), text: None }
    }

    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    pub fn examples(&self) -> &[Example] {
        &self.examples
    }

    // -- pictures --------------------------------------------------------------------------------

    /// A PNG kept as the note's next example. Anything that is not a PNG is left out, as Electron's
    /// `isPng` does where a picture cannot be drawn again.
    pub fn add_picture(&mut self, png: Vec<u8>, cx: &mut Context<Self>) {
        const MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        if !png.starts_with(&MAGIC) {
            return;
        }
        let name = format!("picture-{}.png", self.examples.len() + 1);
        self.examples.push(Example { name: name.into(), image: Arc::new(Image::from_bytes(ImageFormat::Png, png)) });
        cx.notify();
    }

    /// A picture on the clipboard becomes an example. True when one was there, so the paste is taken.
    pub fn paste(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(item) = cx.read_from_clipboard() else { return false };
        let png = item.entries().iter().find_map(|entry| match entry {
            gpui::ClipboardEntry::Image(image) if image.format == ImageFormat::Png => Some(image.bytes.clone()),
            _ => None,
        });
        match png {
            Some(png) => {
                self.add_picture(png, cx);
                true
            }
            None => false,
        }
    }

    /// Files dropped on the card: each is read off the main thread, and the PNGs are kept.
    pub fn drop_files(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            for path in paths {
                let read = cx.background_executor().spawn(async move { std::fs::read(path).ok() }).await;
                if let Some(png) = read {
                    let _ = this.update(cx, |this, cx| this.add_picture(png, cx));
                }
            }
        })
        .detach();
    }

    /// A picture made from the look's own colours, for a walk that cannot paste one.
    pub fn mock_picture(&mut self, cx: &mut Context<Self>) {
        let png = {
            let look = cx.look();
            let rgba = |token: Token| {
                let colour: Rgba = look.color(token).into();
                image::Rgba([(colour.r * 255.) as u8, (colour.g * 255.) as u8, (colour.b * 255.) as u8, 255])
            };
            let (ground, bar, mark) = (rgba(token::BG_SUNKEN), rgba(token::ACCENT_MUTED), rgba(token::ACCENT));
            let mut picture = image::RgbaImage::from_pixel(96, 64, ground);
            for y in 8..24 {
                for x in 8..88 {
                    picture.put_pixel(x, y, bar);
                }
            }
            for y in 34..48 {
                for x in 8..44 {
                    picture.put_pixel(x, y, mark);
                }
            }
            let mut out = std::io::Cursor::new(Vec::new());
            picture.write_to(&mut out, image::ImageFormat::Png).ok().map(|_| out.into_inner())
        };
        if let Some(png) = png {
            self.add_picture(png, cx);
        }
    }

    fn remove_picture(&mut self, name: &str, cx: &mut Context<Self>) {
        self.examples.retain(|one| one.name != name);
        cx.notify();
    }

    // -- changes ---------------------------------------------------------------------------------

    /// A token picked for `property`: what it read is kept from the first change to it, and the
    /// token already picked, pressed again, undoes that change.
    fn pick(&mut self, kind: Kind, property: &'static str, label: &'static str, picked: Token, cx: &mut Context<Self>) {
        let (before, after) = {
            let look = cx.look();
            (read(look, property), value_of(look, kind, picked))
        };
        if self.changes.iter().any(|one| one.property == property && one.token == Some(picked.0)) {
            self.changes.retain(|one| one.property != property);
        } else {
            let was = self.changes.iter().find(|one| one.property == property).map(|one| one.before.clone());
            self.set(Change { kind, property, label, before: was.unwrap_or_else(|| before.into()), after: after.into(), token: Some(picked.0) });
        }
        cx.notify();
    }

    fn pick_story(&mut self, name: &'static str, cx: &mut Context<Self>) {
        if self.changes.iter().any(|one| one.kind == Kind::Variant && one.after == name) {
            self.changes.retain(|one| one.kind != Kind::Variant);
        } else {
            let was = self.changes.iter().find(|one| one.kind == Kind::Variant).map(|one| one.before.clone());
            self.set(Change { kind: Kind::Variant, property: "variant", label: "Variant", before: was.unwrap_or_else(|| AS_DRAWN.into()), after: name.into(), token: None });
        }
        cx.notify();
    }

    fn typed(&mut self, text: String, cx: &mut Context<Self>) {
        let own = match &self.scope {
            Scope::Element { words, .. } => words.clone().unwrap_or_default(),
            _ => SharedString::default(),
        };
        let before = self.changes.iter().find(|one| one.kind == Kind::Text).map_or(own, |one| one.before.clone());
        if before == text {
            self.changes.retain(|one| one.kind != Kind::Text);
        } else {
            self.set(Change { kind: Kind::Text, property: "text", label: "Text", before, after: text.into(), token: None });
        }
        cx.notify();
    }

    /// `change` in place of the change to the same property, or after them where there is none.
    fn set(&mut self, change: Change) {
        match self.changes.iter_mut().find(|one| one.property == change.property) {
            Some(was) => *was = change,
            None => self.changes.push(change),
        }
    }

    fn undo(&mut self, property: &str, cx: &mut Context<Self>) {
        self.changes.retain(|one| one.property != property);
        cx.notify();
    }

    fn choose(&mut self, kind: Kind, window: &mut Window, cx: &mut Context<Self>) {
        self.open = if self.open == Some(kind) { None } else { Some(kind) };
        if self.open == Some(Kind::Text) && self.text.is_none() {
            let now = self.text_now();
            let state = cx.new(|cx| {
                let mut state = TextareaState::new(window, cx).auto_grow(2, 4);
                state.set_value(now.to_string(), window, cx);
                state
            });
            let watch = cx.subscribe_in(&state, window, |this, state, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let words = state.read(cx).value().to_string();
                    this.typed(words, cx);
                }
            });
            self.text = Some((state, watch));
        }
        cx.notify();
    }

    fn text_now(&self) -> SharedString {
        self.changes
            .iter()
            .find(|one| one.kind == Kind::Text)
            .map(|one| one.after.clone())
            .or_else(|| match &self.scope {
                Scope::Element { words, .. } => words.clone(),
                _ => None,
            })
            .unwrap_or_default()
    }

    fn token_of(&self, property: &str) -> Option<&'static str> {
        self.changes.iter().find(|one| one.property == property).and_then(|one| one.token)
    }
}

/// What an element reads before a change, as a mock's page computes it.
fn read(look: &Look, property: &str) -> String {
    match property {
        "font-size" => length_text(look, token::TEXT_SM),
        "color" => colour_text(look, token::FG_DEFAULT),
        "background-color" => colour_text(look, token::BG_RAISED),
        "padding" => length_text(look, token::SPACE_3),
        "gap" => length_text(look, token::SPACE_2),
        _ => String::new(),
    }
}

fn value_of(look: &Look, kind: Kind, picked: Token) -> String {
    match kind {
        Kind::Colour | Kind::Fill => colour_text(look, picked),
        _ => length_text(look, picked),
    }
}

fn length_text(look: &Look, picked: Token) -> String {
    format!("{}px", f32::from(look.length(picked)))
}

fn colour_text(look: &Look, picked: Token) -> String {
    let colour: Rgba = look.color(picked).into();
    format!("#{:02x}{:02x}{:02x}", (colour.r * 255.).round() as u8, (colour.g * 255.).round() as u8, (colour.b * 255.).round() as u8)
}

/// What a scale's button reads: the token less its family, `--text-lg` as `lg`.
fn step(name: &str) -> &str {
    name.trim_start_matches("--").trim_start_matches("text-").trim_start_matches("space-")
}

impl Render for Tools {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cx: &Context<Self> = cx;
        let look = cx.look();
        let element = matches!(self.scope, Scope::Element { .. });
        let region = matches!(self.scope, Scope::Region);
        let stories: Vec<&'static str> = match &self.scope {
            Scope::Element { stories, .. } => stories.clone(),
            _ => Vec::new(),
        };
        let has_words = match &self.scope {
            Scope::Element { words, .. } => words.is_some() || self.changes.iter().any(|one| one.kind == Kind::Text),
            _ => false,
        };
        let toolbar = (element || region).then(|| {
            let buttons = Kind::ALL.into_iter().filter(|kind| *kind != Kind::Variant || !stories.is_empty()).map(|kind| {
                let off = region || (kind == Kind::Text && !has_words);
                let pressed = self.open == Some(kind);
                let ink = if pressed { look.color(token::TEXT_ON_ACCENT) } else { look.color(token::FG_DEFAULT) };
                let button = keeps(div().id(SharedString::from(format!("tool-{}", kind.label()))))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .pt(look.length(token::SPACE_2))
                    .pb(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_LG))
                    .text_color(ink)
                    .when(pressed, |button| button.bg(look.color(token::ACCENT)))
                    .child(glyph(kind.glyph(), look.length(token::SPACE_4), ink))
                    .child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).child(kind.label()))
                    .when(off, |button| button.opacity(0.5))
                    .when(!off, |button| {
                        button
                            .cursor_pointer()
                            .when(!pressed, |button| button.hover(|style| style.bg(look.color(token::BG_HOVER))))
                            .on_click(cx.listener(move |this, _, window, cx| this.choose(kind, window, cx)))
                    });
                // The label is on the key, so a tooltip is left only to say why one is off.
                let button = if off && element { button.tooltip(tip(format!("{}: no words on it", kind.label()))) } else { button };
                labelled(button, Role::Button, kind.label())
            });
            labelled(keeps(div().id("tools-bar")).flex().gap(look.length(token::SPACE_1)).children(buttons), Role::Toolbar, "Tools")
        });
        let picker = |label: &'static str, sample: bool, tokens: &[Token], current: Option<&'static str>, kind: Kind, property: &'static str, cx: &Context<Self>| {
            let steps = tokens.iter().copied().map(|picked| {
                let on = current == Some(picked.0);
                let said = format!("{} {}", picked.0, value_of(look, kind, picked));
                let ink = if on { look.color(token::TEXT_ON_ACCENT) } else { look.color(token::FG_DEFAULT) };
                let bar = look.length(picked) * 0.5;
                let button = keeps(div().id(SharedString::from(format!("{label}-{}", picked.0))))
                    .h(look.length(token::H_CONTROL_SM))
                    .px(look.length(token::SPACE_2))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(if on { look.color(token::ACCENT) } else { look.color(token::BORDER_DEFAULT) })
                    .bg(if on { look.color(token::ACCENT) } else { look.color(token::BG_SUNKEN) })
                    .text_color(ink)
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.85))
                    .tooltip(tip(said))
                    // A rising bar for a size, a widening one for a space, drawn at half the token.
                    .child(if sample {
                        div().flex_none().w(look.length(token::BORDER_WIDTH) * 2.).h(bar).bg(ink)
                    } else {
                        div().flex_none().h(look.length(token::BORDER_WIDTH) * 2.).w(bar).bg(ink)
                    })
                    .child(step(picked.0).to_string())
                    .on_click(cx.listener(move |this, _, _, cx| this.pick(kind, property, label, picked, cx)));
                labelled(button, Role::Button, picked.0)
            });
            labelled(
                keeps(div().id(SharedString::from(format!("picker-{label}"))))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(label_line(label, look))
                    .child(div().flex().flex_wrap().gap(look.length(token::SPACE_1)).children(steps)),
                Role::Group,
                label,
            )
        };
        let swatches = |label: &'static str, tokens: &[Token], current: Option<&'static str>, kind: Kind, property: &'static str, cx: &Context<Self>| {
            let chips = tokens.iter().copied().map(|picked| {
                let on = current == Some(picked.0);
                let said = format!("{} {}", picked.0, value_of(look, kind, picked));
                let chip = keeps(div().id(SharedString::from(format!("{label}-{}", picked.0))))
                    .size(look.length(token::H_CONTROL_SM))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(if on { look.color(token::ACCENT) } else { look.color(token::BORDER_DEFAULT) })
                    .when(on, |chip| chip.border_2())
                    .bg(look.color(picked))
                    .cursor_pointer()
                    .tooltip(tip(said))
                    .on_click(cx.listener(move |this, _, _, cx| this.pick(kind, property, label, picked, cx)));
                labelled(chip, Role::Button, picked.0)
            });
            labelled(
                keeps(div().id(SharedString::from(format!("picker-{label}"))))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(label_line(label, look))
                    .child(div().flex().flex_wrap().gap(look.length(token::SPACE_1)).children(chips)),
                Role::Group,
                label,
            )
        };
        let tool = if element { self.open } else { None };
        let picked = match tool {
            Some(Kind::Size) => Some(picker("Size", true, &SIZES, self.token_of("font-size"), Kind::Size, "font-size", cx).into_any_element()),
            Some(Kind::Colour) => Some(swatches("Colour", &COLOURS, self.token_of("color"), Kind::Colour, "color", cx).into_any_element()),
            Some(Kind::Fill) => Some(swatches("Fill", &FILLS, self.token_of("background-color"), Kind::Fill, "background-color", cx).into_any_element()),
            Some(Kind::Spacing) => Some(
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .child(picker("Padding", false, &SPACES, self.token_of("padding"), Kind::Spacing, "padding", cx))
                    .child(picker("Gap", false, &SPACES, self.token_of("gap"), Kind::Spacing, "gap", cx))
                    .into_any_element(),
            ),
            Some(Kind::Text) => self.text.as_ref().map(|(state, _)| {
                labelled(div().id("tool-text").child(Textarea::new(state)), Role::TextInput, "Text").into_any_element()
            }),
            Some(Kind::Variant) => {
                let on = self.changes.iter().find(|one| one.kind == Kind::Variant).map(|one| one.after.clone());
                let tiles = stories.iter().copied().map(|name| {
                    let pressed = on.as_deref() == Some(name);
                    let ink = if pressed { look.color(token::TEXT_ON_ACCENT) } else { look.color(token::FG_DEFAULT) };
                    labelled(
                        keeps(div().id(SharedString::from(format!("story-{name}"))))
                            .flex_1()
                            .min_w(look.length(token::SPACE_12) * 2.)
                            .h(look.length(token::H_CONTROL_SM))
                            .px(look.length(token::SPACE_2))
                            .flex()
                            .items_center()
                            .rounded(look.length(token::RADIUS_MD))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(if pressed { look.color(token::ACCENT) } else { look.color(token::BORDER_DEFAULT) })
                            .bg(if pressed { look.color(token::ACCENT) } else { look.color(token::BG_SUNKEN) })
                            .text_color(ink)
                            .text_size(look.length(token::TEXT_2XS))
                            .cursor_pointer()
                            .hover(|style| style.opacity(0.85))
                            .child(name)
                            .on_click(cx.listener(move |this, _, _, cx| this.pick_story(name, cx))),
                        Role::Button,
                        name,
                    )
                });
                Some(
                    labelled(
                        keeps(div().id("picker-Variant"))
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .child(label_line("Variant", look))
                            .child(div().flex().flex_wrap().gap(look.length(token::SPACE_1)).children(tiles)),
                        Role::Group,
                        "Variant",
                    )
                    .into_any_element(),
                )
            }
            None => None,
        };
        let changes = (!self.changes.is_empty()).then(|| {
            let rows = self.changes.iter().map(|change| {
                let property = change.property;
                let name = format!("Undo {}", change.label);
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(div().flex_none().w(look.length(token::SPACE_12)).text_color(look.color(token::FG_MUTED)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(change.label))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_wrap()
                            .child(div().text_color(look.color(token::FG_SUBTLE)).child(shown(change, &change.before)))
                            .child(div().text_color(look.color(token::FG_SUBTLE)).child(" → "))
                            .child(div().text_color(look.color(token::ACCENT)).child(shown(change, &change.after))),
                    )
                    .child(
                        // The token as a filled chip: a change with none (Text) draws no chip, and keeps its column.
                        div().flex_none().w(look.length(token::SPACE_12) * 2.).children(change.token.map(|word| {
                            div()
                                .px(look.length(token::SPACE_1))
                                .rounded(look.length(token::RADIUS_SM))
                                .bg(look.color(token::BG_RAISED))
                                .text_color(look.color(token::FG_MUTED))
                                .truncate()
                                .child(word)
                        })),
                    )
                    .child(labelled(
                        icon_button(SharedString::from(format!("undo-{property}")), "undo-2", look.color(token::FG_MUTED), look)
                            .size(look.length(token::H_CONTROL_SM))
                            .tooltip(tip("Undo"))
                            .on_click(cx.listener(move |this, _, _, cx| this.undo(property, cx))),
                        Role::Button,
                        name,
                    ))
            });
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(labelled(
                    keeps(div().id("changes"))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .px(look.length(token::SPACE_2))
                        .py(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .bg(look.color(token::BG_SUNKEN))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_2XS))
                        .children(rows),
                    Role::List,
                    "Changes",
                ))
                .child(div().flex().justify_end().child(labelled(
                    Btn::new("undo-all", "Undo all", Weight::Secondary).draw(look, cx).on_click(cx.listener(|this, _, _, cx| {
                        this.changes.clear();
                        cx.notify();
                    })),
                    Role::Button,
                    "Undo all",
                )))
        });
        let pictures = (!self.examples.is_empty()).then(|| {
            let items = self.examples.iter().map(|example| {
                let name = example.name.clone();
                let drop = example.name.clone();
                div()
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_1))
                    .child(labelled(
                        div()
                            .id(SharedString::from(format!("picture-{name}")))
                            .w(look.length(token::SPACE_12) + look.length(token::SPACE_6))
                            .h(look.length(token::SPACE_12))
                            .overflow_hidden()
                            .rounded(look.length(token::RADIUS_MD))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(token::ACCENT).opacity(0.55))
                            .bg(look.color(token::BG_SUNKEN))
                            .child(img(example.image.clone()).size_full().object_fit(gpui::ObjectFit::Cover)),
                        Role::Image,
                        name.clone(),
                    ))
                    .child(labelled(
                        icon_button(SharedString::from(format!("remove-{name}")), "x", look.color(token::FG_MUTED), look)
                            .size(look.length(token::H_CONTROL_SM))
                            .tooltip(tip("Remove picture"))
                            .on_click(cx.listener(move |this, _, _, cx| this.remove_picture(&drop, cx))),
                        Role::Button,
                        format!("Remove {name}"),
                    ))
            });
            labelled(keeps(div().id("pictures")).flex().flex_wrap().gap(look.length(token::SPACE_2)).children(items), Role::List, "Pictures")
        });
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .children(toolbar)
            .children(picked)
            .children(changes)
            .children(pictures)
    }
}

/// A press on a control must not move the keys off the note's field: a labelled control takes them,
/// and so does the capture around it.
fn keeps(element: gpui::Stateful<gpui::Div>) -> gpui::Stateful<gpui::Div> {
    element.on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
}

fn label_line(label: &'static str, look: &Look) -> impl IntoElement {
    div()
        .text_color(look.color(token::FG_MUTED))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .child(label)
}

/// A change's side as it is read: words as they are, an empty length as `none`.
fn shown(change: &Change, value: &SharedString) -> SharedString {
    if change.kind == Kind::Text || !value.is_empty() {
        value.clone()
    } else {
        "none".into()
    }
}

/// What a held thing offers the tools: an element its own words and the stories its component has,
/// a box nothing, the whole window only pictures. The stories are a mock's until the story index is
/// Fleet's face.
pub fn scope_of(held: &Held) -> Scope {
    match held {
        Held::Element(drawn) => {
            let words = matches!(drawn.role, Role::Button | Role::Row | Role::Heading | Role::Tab | Role::Link | Role::Status).then(|| drawn.name.clone());
            let stories: &[&str] = match drawn.role {
                Role::Row => &["Default", "Selected"],
                Role::Button => &["Primary", "Secondary", "Ghost"],
                _ => &[],
            };
            Scope::Element { words, stories: stories.to_vec() }
        }
        Held::Region(_) => Scope::Region,
        Held::Window { .. } => Scope::Window,
    }
}

/// Files dropped on a card are PNGs to keep. The owner of a card calls this from its `on_drop`.
pub fn dropped(tools: &gpui::Entity<Tools>, paths: &ExternalPaths, cx: &mut App) {
    let files: Vec<PathBuf> = paths.paths().to_vec();
    tools.update(cx, |tools, cx| tools.drop_files(files, cx));
}
