//! The sentence drawn: the opening ghosted after the words that begin it, its list under the caret, the sentence with
//! its blanks as underlined fields, the key hints and the feature list under the blank that has the keys, and the line
//! that says what it starts. State and keys are `story.rs` and `sheet.rs`.

use gpui::{deferred, div, prelude::*, px, AnyElement, App, Context, Hsla, MouseButton, Role, SharedString, Window};

use super::sheet::DispatchSheet;
use super::story::ListRow;
use super::story_kinds::{kinds, Piece};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::titlebar::parts::{glyph, kbd, sans, tip};

/// One row of a list under the caret.
struct PopRow {
    glyph: Option<(&'static str, Hsla)>,
    label: SharedString,
    ink: Option<Hsla>,
    says: SharedString,
}

/// What a press on row `at` of a list does.
type Choose = fn(&mut DispatchSheet, usize, &mut Window, &mut Context<DispatchSheet>);

impl DispatchSheet {
    /// The words typed and the rest of the opening they begin, ghosted after them, with the list of the openings
    /// they begin hanging from the caret. It lies under the request's own text, which draws the same words.
    pub(super) fn ghost(&self, words: &str, begun: &[usize], cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let marked = self.opening % begun.len();
        let kind = &kinds()[begun[marked]];
        let line_h = look.length(token::LEADING_LG);
        let typed = words.trim_start().chars().count();
        let rest: String = kind.opening.chars().skip(typed).collect();
        let rows = begun
            .iter()
            .map(|&at| {
                let kind = &kinds()[at];
                let ink = look.color(Token(kind.hue));
                PopRow { glyph: Some((kind.glyph, ink)), label: kind.opening.into(), ink: Some(ink), says: kind.name.into() }
            })
            .collect();
        let mut hints = vec![("⇥", "take")];
        if begun.len() > 1 {
            hints.insert(0, ("↑↓", "choose"));
        }
        let list = self.pop(look, "Openings", rows, marked, hints, cx, |this, at, window, cx| {
            if let Some(&kind) = story_begun(this, cx).get(at) {
                this.take_opening(kind, window, cx);
            }
        });
        let words: SharedString = if words.is_empty() { " ".into() } else { words.to_string().into() };
        // The request's own text stands a little higher than this copy of it, so the copy is lifted to meet it.
        div()
            .flex()
            .items_start()
            .mt(-look.length(token::SPACE_1))
            .child(div().flex_none().child(words))
            .child(div().relative().flex_none().w_0().h(line_h).child(deferred(div().absolute().top(line_h).left_0().child(list)).with_priority(1)))
            .child(div().flex_none().text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(rest)))
            .into_any_element()
    }

    /// The quiet line at the foot of the empty box that names the six openings, each in its hue.
    pub(super) fn openings_line(&self, cx: &Context<Self>) -> Option<AnyElement> {
        if self.story.is_some() || !self.words(cx).is_empty() {
            return None;
        }
        let look = cx.look();
        let all = kinds();
        let mut line = div().flex().flex_wrap().items_center().gap_x(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS));
        for (at, kind) in all.iter().enumerate() {
            if at > 0 {
                line = line.child(div().text_color(look.color(token::FG_SUBTLE)).child("·"));
            }
            line = line.child(div().text_color(look.color(Token(kind.hue))).child(kind.opening));
        }
        let said = format!("Openings: {}", all.iter().map(|kind| kind.opening).collect::<Vec<_>>().join(", "));
        Some(labelled(div().id("dispatch-openings").absolute().bottom_0().left_0().right_0().px(gpui_component::Size::Medium.input_px()).child(line), Role::Label, said).into_any_element())
    }

    /// The sentence in the request's place: the opening as a tinted lead, then the text and the blanks, wrapping as words do.
    pub(super) fn story_field(&self, window: &Window, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let Some(story) = &self.story else { return div().into_any_element() };
        let kind = story.kind();
        let hue = look.color(Token(kind.hue));
        let has_keys = self.story_key.is_focused(window);
        let line_h = look.length(token::LEADING_LG);
        let catalog = self.features();
        let catalog = &*catalog;
        let mut sentence = div()
            .id("dispatch-sentence")
            .track_focus(&self.story_key)
            .flex()
            .flex_wrap()
            .items_center()
            .content_start()
            .gap_y(look.length(token::SPACE_1))
            .text_size(look.length(token::TEXT_LG))
            .line_height(line_h)
            .child(labelled(
                div()
                    .id("dispatch-opening")
                    .flex_none()
                    .mr(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(hue.opacity(0.16))
                    .text_color(hue)
                    .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(kind.opening),
                Role::Label,
                kind.opening,
            ));
        for (at, piece) in kind.pieces.iter().enumerate() {
            let on = has_keys && story.at == at;
            match piece {
                Piece::Text(text) => sentence = sentence.child(div().flex_none().child(*text)),
                Piece::Choice(first, second) => {
                    let word = story.chosen(first, second);
                    let edge = look.color(if on { token::ACCENT } else { token::BORDER_DEFAULT });
                    let ring = if on { look.color(token::ACCENT).opacity(0.12) } else { gpui::transparent_black() };
                    sentence = sentence.child(labelled(
                        div()
                            .id(("dispatch-blank", at))
                            .relative()
                            .flex_none()
                            .px(look.length(token::SPACE_2))
                            .rounded(look.length(token::RADIUS_MD))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(edge)
                            .bg(ring)
                            .cursor_pointer()
                            .tooltip(tip(format!("{first} or {second}")))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, window, cx| {
                                    window.prevent_default();
                                    cx.stop_propagation();
                                    this.story_choose(at, window, cx);
                                }),
                            )
                            .child(word)
                            .children(on.then(|| self.hint_under(look, &[("space", "switch"), ("⇥", "next"), ("⇧⇥", "back"), ("Esc", "plain words"), ("⌘↵", "start")], cx))),
                        Role::Button,
                        format!("Timing, {word}"),
                    ));
                }
                Piece::Blank(_) | Piece::Feature(_) => {
                    let name = match piece {
                        Piece::Blank(name) => *name,
                        _ => "feature",
                    };
                    let value = &story.values[at];
                    let listed = on && matches!(piece, Piece::Feature(_)) && story.list_open(catalog);
                    let ink = if matches!(piece, Piece::Feature(_)) { look.color(token::ACCENT) } else { look.color(token::FG_DEFAULT) };
                    let edge = look.color(if on {
                        token::ACCENT
                    } else if value.is_empty() {
                        token::BORDER_SUBTLE
                    } else {
                        token::BORDER_DEFAULT
                    });
                    let caret = || div().flex_none().w(px(1.)).h(line_h * 0.8).bg(look.color(token::ACCENT));
                    let shown = if value.is_empty() {
                        div()
                            .relative()
                            .text_color(look.color(token::FG_SUBTLE))
                            .child(name)
                            .children(on.then(|| div().absolute().top(line_h * 0.1).left_0().child(caret())))
                            .into_any_element()
                    } else {
                        div().flex().items_center().text_color(ink).child(SharedString::from(value.clone())).children(on.then(caret)).into_any_element()
                    };
                    let under = if listed {
                        Some(self.feature_list(story.list(catalog).unwrap_or_default(), story.row, look, cx))
                    } else if on {
                        Some(self.hint_under(look, &[("⇥", "next"), ("⇧⇥", "back"), ("Esc", "plain words"), ("⌘↵", "start")], cx))
                    } else {
                        None
                    };
                    sentence = sentence.child(labelled(
                        div()
                            .id(("dispatch-blank", at))
                            .relative()
                            .flex_none()
                            .border_b(look.length(token::BORDER_WIDTH) * 2.)
                            .border_color(edge)
                            .bg(if on { look.color(token::ACCENT).opacity(0.08) } else { gpui::transparent_black() })
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, window, cx| {
                                    window.prevent_default();
                                    cx.stop_propagation();
                                    this.story_go(at, window, cx);
                                }),
                            )
                            .child(shown)
                            .children(under),
                        Role::TextInput,
                        name,
                    ));
                }
            }
        }
        labelled(
            div()
                .id("dispatch-story")
                .relative()
                .flex_1()
                .min_w_0()
                .h(line_h * 5.)
                .px(gpui_component::Size::Medium.input_px())
                .py(gpui_component::Size::Medium.input_py())
                .child(sentence)
                .child(self.story_footer(window, cx)),
            Role::Group,
            "Request sentence",
        )
        .into_any_element()
    }

    /// What the sentence starts, and the tools it asks for, at the foot of its box. The tools list opens from the + under it.
    fn story_footer(&self, window: &Window, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let Some(story) = &self.story else { return div().into_any_element() };
        let says = story.starts(&self.features());
        div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .px(gpui_component::Size::Medium.input_px())
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(labelled(
                div().id("dispatch-starts").text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(says.clone())),
                Role::Status,
                says,
            ))
            .children(self.tool_chips(window, cx))
            .into_any_element()
    }

    /// The keys a blank takes, in a small card under it.
    fn hint_under(&self, look: &Look, keys: &[(&'static str, &'static str)], cx: &Context<Self>) -> AnyElement {
        let _ = cx;
        let mut row = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_1))
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::FG_MUTED));
        for (key, says) in keys {
            row = row.child(div().flex().items_center().gap(look.length(token::SPACE_1)).child(kbd(*key, look)).child(*says));
        }
        deferred(div().absolute().top_full().left_0().mt(look.length(token::SPACE_1)).child(self.card(look, row.into_any_element())).into_any_element())
            .with_priority(1)
            .into_any_element()
    }

    /// The feature list under its blank, with its keys at the foot.
    fn feature_list(&self, rows: Vec<ListRow>, marked: usize, look: &Look, cx: &Context<Self>) -> AnyElement {
        let name = if rows.iter().any(|row| row.project.is_some()) { "Projects" } else { "Features" };
        let rows = rows.into_iter().map(|row| PopRow { glyph: None, label: row.title, ink: None, says: if row.area_row { "Area".into() } else { row.area } }).collect();
        let list = self.pop(look, name, rows, marked, vec![("↑↓", "choose"), ("↵", "pick"), ("⇥", "next")], cx, |this, at, window, cx| this.story_row(at, window, cx));
        deferred(div().absolute().top_full().left_0().mt(look.length(token::SPACE_1)).child(list)).with_priority(1).into_any_element()
    }

    /// A card that floats over what is under it.
    fn card(&self, look: &Look, content: AnyElement) -> AnyElement {
        div()
            .occlude()
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .font_family(sans(look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(content)
            .into_any_element()
    }

    /// A list of rows under the caret or a blank: the row marked, a press that picks one, and the keys it takes at the foot.
    #[allow(clippy::too_many_arguments)]
    fn pop(&self, look: &Look, name: &'static str, rows: Vec<PopRow>, marked: usize, hints: Vec<(&'static str, &'static str)>, cx: &Context<Self>, choose: Choose) -> AnyElement {
        let items = rows.into_iter().enumerate().map(|(at, row)| {
            let said = if row.says.is_empty() { row.label.clone() } else { format!("{}, {}", row.label, row.says).into() };
            labelled(
                div()
                    .id(("dispatch-pop-row", at))
                    .h(look.length(token::H_MENU_ITEM))
                    .px(look.length(token::SPACE_3))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_SM))
                    .bg(if at == marked { look.color(token::BG_HOVER) } else { gpui::transparent_black() })
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                            choose(this, at, window, cx);
                        }),
                    )
                    .children(row.glyph.map(|(name, ink)| glyph(name, look.length(token::SPACE_3), ink)))
                    .child(div().flex_none().text_color(row.ink.unwrap_or_else(|| look.color(token::FG_DEFAULT))).child(row.label))
                    .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(row.says)),
                Role::ListBoxOption,
                said,
            )
        });
        let list = labelled(
            div()
                .id(SharedString::from(format!("dispatch-pop-{name}")))
                .min_w(look.length(token::PALETTE_WIDTH) * 0.5)
                .max_h(look.length(token::H_MENU_ITEM) * 6.)
                .overflow_y_scroll()
                .track_scroll(&self.list_scroll)
                .flex()
                .flex_col()
                .py(look.length(token::SPACE_1))
                .children(items),
            Role::ListBox,
            name,
        );
        let mut keys = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_1))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::FG_MUTED));
        for (key, says) in hints {
            keys = keys.child(div().flex().items_center().gap(look.length(token::SPACE_1)).child(kbd(key, look)).child(says));
        }
        self.card(look, div().flex().flex_col().child(list).child(keys).into_any_element())
    }
}

/// The kinds the sheet's words begin, for a press on a row of their list.
fn story_begun(sheet: &DispatchSheet, cx: &App) -> Vec<usize> {
    sheet.openings(cx)
}
