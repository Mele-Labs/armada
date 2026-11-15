//! The split rows every Settings page but Limits is drawn in (the approved take "Split rows", 13 Nov 2026): a head
//! band in the page's hue, then cards of rows. Each row is a setting, its title and what it does at the left and its
//! control, with its own words, at the right, a hairline between rows. A value changed from what ships has an accent
//! edge, a Changed chip and a Reset beside its control; a setting that needs a restart says so in an amber chip.

use gpui::{div, prelude::*, px, svg, AnyElement, App, Context, FontWeight, Hsla, KeyDownEvent, Pixels, Role, SharedString, Stateful, Div, Window};
use ipc::{Setting, SettingKind, SettingValue};

use super::entries::Entry;
use super::overlay::SettingsOverlay;
use super::view::{bone, ringed, tip};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// How wide a dropdown is drawn in a row.
pub const SELECT_W: f32 = 220.;
/// How wide a typed figure's field is drawn, with its unit written out inside.
pub const FIELD_W: f32 = 184.;
/// The most options a choice draws as segments; more is a dropdown.
pub const SEGMENTS_MAX: usize = 3;
/// A page narrower than this stacks each row's words over its control.
const STACK_BELOW: f32 = 600.;
/// How many letters of a description fit a line before it wraps.
const SAYS_LETTERS: f32 = 52.;

/// What a page is for, under its name in the band.
pub fn purpose_of(entry: &Entry) -> &'static str {
    match entry.id {
        "kit:prompts" => "What agents are told before a step's own brief.",
        "kit:harness" => "The agent program Fleet runs, and where it is found.",
        "fleet:limits" => "How much of this Mac Fleet takes",
        "fleet:timeouts" => "How long Fleet waits before it gives up.",
        "fleet:retention" => "How long Fleet keeps what it records.",
        "fleet:plan" => "When Fleet steps down on the weekly plan.",
        "fleet:token-use" => "What Jobs spend on tokens.",
        "fleet:machine" => "This Mac's name.",
        "fleet:account" => "The sign-in app Armada uses.",
        "fleet:features" => "Switches for what Armada does, and the folders Jobs reach.",
        "bridge:spacing" => "How much room Bridge leaves around things.",
        "bridge:time" => "How every stamp Bridge draws is written.",
        "bridge:editor" => "Where code opens.",
        "bridge:terminal" => "Which terminal opens.",
        "bridge:links" => "Where pull requests open.",
        _ => "",
    }
}

/// How the rows of a page are spread: words and control side by side about 5 to 7, or stacked where the page is narrow.
#[derive(Clone, Copy)]
pub struct Spread {
    pub stacked: bool,
    pub left: Pixels,
    pub right: Pixels,
}

/// What a row holds, apart from its look.
pub struct Row {
    pub id: SharedString,
    pub title: SharedString,
    /// The title's tooltip: the key, where there is one.
    pub tip: Option<SharedString>,
    /// After the title: Changed, Restart Fleet to apply.
    pub chips: Vec<AnyElement>,
    /// The description, then any note, under the title.
    pub says: Vec<AnyElement>,
    pub changed: bool,
    /// A registry glyph before the title, and the ink it is drawn in.
    pub glyph: Option<(&'static str, Hsla)>,
    pub control: AnyElement,
}

/// The ink of a segment.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// Not the word in force.
    No,
    /// The word in force, set here.
    Set,
    /// The word in force, but set somewhere below.
    Muted,
}

fn press_keys(event: &KeyDownEvent) -> bool {
    matches!(event.keystroke.key.as_str(), "enter" | "space")
}

/// A chip after a title: its words in a state's ink on that state's wash.
pub fn chip(look: &Look, id: impl Into<gpui::ElementId>, said: &str, ink: Hsla, wash: Hsla) -> Stateful<Div> {
    div()
        .id(id.into())
        .flex_none()
        .px(look.length(token::SPACE_2))
        .rounded_full()
        .bg(wash)
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(ink)
        .child(SharedString::from(said.to_string()))
}

/// The accent chip on a value that is not what ships.
pub fn changed_chip(look: &Look, key: &str) -> AnyElement {
    labelled(chip(look, SharedString::from(format!("changed-{key}")), "Changed", look.color(token::ACCENT_HOVER), look.color(token::ACCENT_MUTED)), Role::Status, "Changed").into_any_element()
}

/// The amber chip on a setting Fleet applies after it restarts.
pub fn restart_chip(look: &Look, key: &str) -> AnyElement {
    let said = "Restart Fleet to apply";
    labelled(chip(look, SharedString::from(format!("restart-{key}")), said, look.color(token::STATUS_AWAITING_REVIEW), look.color(token::STATUS_AWAITING_REVIEW_BG)), Role::Status, said).into_any_element()
}

/// A page's cards of rows, the hairlines between them, and an edge that clips a changed row's mark.
pub fn rows_card(look: &Look, id: impl Into<gpui::ElementId>, rows: Vec<AnyElement>) -> Stateful<Div> {
    div()
        .id(id.into())
        .flex_none()
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_RAISED))
        .children(rows)
}

/// A description's line: muted, with room between lines, no wider than a comfortable measure.
pub fn says(look: &Look, said: impl Into<SharedString>) -> Div {
    div()
        .max_w(look.length(token::TEXT_XS) * (SAYS_LETTERS * 0.55))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_SM))
        .text_color(look.color(token::FG_MUTED))
        .child(said.into())
}

/// One setting as a row: the words at the left, the control at the right.
pub fn split_row(look: &Look, spread: Spread, row: Row, first: bool) -> Stateful<Div> {
    let edge = look.length(token::BORDER_WIDTH);
    let title = div()
        .id(SharedString::from(format!("row-title-{}", row.id)))
        .min_w_0()
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(look.color(token::FG_DEFAULT))
        .when_some(row.tip, |title, said| title.tooltip(tip(said)))
        .child(row.title);
    let words = div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1) + edge * 2.)
        .min_w_0()
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x(look.length(token::SPACE_2))
                .gap_y(look.length(token::SPACE_1))
                .children(row.glyph.map(|(glyph, ink)| svg().flex_none().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(ink)))
                .child(title)
                .children(row.chips),
        )
        .children(row.says);
    let (left, right) = if spread.stacked {
        (div().min_w_0().w_full().child(words), div().min_w_0().w_full().child(row.control))
    } else {
        (div().flex_none().min_w_0().w(spread.left).child(words), div().flex_none().min_w_0().w(spread.right).child(row.control))
    };
    div()
        .id(row.id)
        .relative()
        .flex()
        .when(spread.stacked, |line| line.flex_col())
        .gap(look.length(token::SPACE_8))
        .when(spread.stacked, |line| line.gap(look.length(token::SPACE_3)))
        .px(look.length(token::SPACE_6))
        .py(look.length(token::SPACE_4) + edge * 2.)
        .when(!first, |line| line.border_t(edge).border_color(look.color(token::BORDER_SUBTLE)))
        .when(row.changed, |line| line.child(div().absolute().left_0().top_0().bottom_0().w(edge * 3.).bg(look.color(token::ACCENT))))
        .child(left)
        .child(right)
}

/// The sunken track the segments sit in.
pub fn segment_track(look: &Look, id: impl Into<gpui::ElementId>) -> Stateful<Div> {
    div()
        .id(id.into())
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1) / 2.)
        .p(look.length(token::SPACE_1) / 2. + look.length(token::BORDER_WIDTH))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_SUNKEN))
}

/// One segment: its words, raised in an accent wash and ring where it is the word set, raised plain where the word
/// in force is set somewhere below.
pub fn segment(look: &Look, id: impl Into<gpui::ElementId>, said: SharedString, pick: Pick, off: bool) -> Stateful<Div> {
    let edge = look.length(token::BORDER_WIDTH);
    div()
        .id(id.into())
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL) - look.length(token::SPACE_1) - edge * 2.)
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .border(edge)
        .border_color(gpui::transparent_black())
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .map(|segment| match pick {
            Pick::Set => segment.bg(look.color(token::ACCENT_MUTED)).border_color(look.color(token::ACCENT)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)),
            Pick::Muted => segment.bg(look.color(token::BG_OVERLAY)).border_color(look.color(token::BORDER_DEFAULT)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_MUTED)),
            Pick::No => segment.text_color(if off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_MUTED) }),
        })
        .when(!off && pick == Pick::No, |segment| segment.cursor_pointer().hover(|style| style.text_color(look.color(token::FG_DEFAULT))))
        .child(said)
}

/// A press that puts a changed value back: "Reset to Never", in a quiet ink.
pub fn reset_press(look: &Look, id: impl Into<gpui::ElementId>, said: &str, off: bool) -> Stateful<Div> {
    super::view::ghost(look, id, said, off)
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(if off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_MUTED) })
}

/// What ships, in words: `Never`, `30 seconds`, `on`. A prompt, which is a page of words, is "shipped".
pub fn default_words(one: &Setting) -> String {
    let number = |value: &SettingValue| if let SettingValue::Integer(n) = value { Some(*n) } else { None };
    match &one.kind {
        SettingKind::Boolean => if one.default == SettingValue::Bool(true) { "on".into() } else { "off".into() },
        SettingKind::Seconds { .. } => number(&one.default).map_or_else(|| "default".into(), super::view::in_words),
        SettingKind::Integer { unit, .. } => number(&one.default).map_or_else(|| "default".into(), |n| match unit.as_deref().map(unit_in_words) {
            None | Some("") => n.to_string(),
            Some("%") => format!("{n}%"),
            Some(unit) => format!("{n} {unit}"),
        }),
        SettingKind::Prompt | SettingKind::TextList => "shipped".into(),
        _ => {
            let held = super::overlay::held_text(&one.default);
            if held.is_empty() { "empty".into() } else { held }
        }
    }
}

/// A unit as it is written inside a field: `%` for percent, nothing for a port.
pub fn unit_in_words(unit: &str) -> &str {
    match unit {
        "ms" => "milliseconds",
        "s" => "seconds",
        "min" => "minutes",
        "percent" => "%",
        "port" => "",
        other => other,
    }
}

impl SettingsOverlay {
    /// How wide the page is drawn: the overlay's box, less its padding, the sidebar, and a mod's panel where one is open.
    pub(super) fn page_width(&self, look: &Look, window: &Window, cx: &App) -> Pixels {
        let room = crate::room::of(window, cx);
        let boxed = room.size.width * 0.9;
        let gap = look.length(token::GAP_PANEL);
        let panel = if self.mods.panel.is_some() { boxed * 0.36 + gap } else { px(0.) };
        boxed - gap * 2. - look.length(token::W_SETTINGS_INDEX) - gap - panel - look.length(token::BORDER_WIDTH) * 2.
    }

    pub(super) fn spread(&self, look: &Look, window: &Window, cx: &App) -> Spread {
        let page = self.page_width(look, window, cx);
        let inside = page - look.length(token::BORDER_WIDTH) * 2. - look.length(token::SPACE_6) * 2.;
        let gap = look.length(token::SPACE_8);
        if f32::from(page) < STACK_BELOW {
            return Spread { stacked: true, left: inside, right: inside };
        }
        let across = inside - gap;
        Spread { stacked: false, left: across * (5. / 12.), right: across * (7. / 12.) }
    }

    /// The head: the page's glyph and name, what it is for, and where its values are kept with the press that opens the
    /// file. Limits draws it flush across its card's top; every other page draws it as a card of its own (`boxed`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn page_band(&self, entry: &Entry, purpose: SharedString, ids: &str, boxed: bool, in_file: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(entry.hue);
        let edge = look.length(token::BORDER_WIDTH);
        let off = self.list.is_none();
        let path = self.list.as_ref().map_or_else(|| "settings.json".to_string(), |list| list.path.clone());
        let open = ringed(
            look,
            div()
                .id("settings-open-file")
                .track_focus(&self.file_stop)
                .flex_none()
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(if off { look.color(token::FG_SUBTLE) } else { look.color(token::ACCENT) })
                .when(!off, |press| press.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
                .child("Open")
                .when(!off, |press| {
                    press.on_click(cx.listener(|this, _, _, cx| this.open_file(cx))).on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        if press_keys(event) {
                            this.open_file(cx);
                        }
                    }))
                }),
            &self.file_stop,
            window,
        );
        let glyph = div()
            .flex_none()
            .size(look.length(token::SPACE_8) + edge * 2.)
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.length(token::RADIUS_LG))
            .bg(hue.opacity(0.28))
            .child(svg().path(icon_path(entry.icon)).size(look.length(token::SPACE_4) + edge).text_color(hue));
        let name = div()
            .flex()
            .flex_col()
            .min_w_0()
            .child(labelled(
                div().id(SharedString::from(format!("{ids}-title"))).text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(entry.label),
                Role::Heading,
                entry.label,
            ))
            .children((!purpose.is_empty()).then(|| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(purpose.clone())));
        let kept = in_file.then(|| {
            div()
                .flex_none()
                .ml_auto()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(
                    div()
                        .id(SharedString::from(format!("{ids}-kept")))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_SUBTLE))
                        .tooltip(tip(SharedString::from(path)))
                        .child("Kept in this Mac's settings.json"),
                )
                .child(labelled(open, Role::Button, "Open settings.json"))
        });
        div()
            .id(SharedString::from(format!("{ids}-band")))
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(look.length(token::SPACE_3) + edge * 2.)
            .gap_y(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4))
            .when(boxed, |band| band.rounded(look.length(token::RADIUS_LG)).border(edge).border_color(hue.opacity(0.35)).bg(look.color(token::BG_RAISED).blend(hue.opacity(0.1))))
            .when(!boxed, |band| band.border_b(edge).border_color(hue.opacity(0.35)).bg(look.color(token::SURFACE_CARD).blend(hue.opacity(0.12))))
            .child(glyph)
            .child(name)
            .children(kept)
            .into_any_element()
    }

    /// A page of rows: its band, what Fleet said about the file, then each card, scrolling as one.
    pub(super) fn rows_page(&self, entry: &Entry, purpose: SharedString, in_file: bool, notes: Vec<AnyElement>, cards: Vec<AnyElement>, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let band = self.page_band(entry, purpose, "page", true, in_file, look, window, cx);
        let scroll = labelled(
            div()
                .id("settings-page-scroll")
                .size_full()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_4))
                .child(band)
                .children(notes)
                .children(cards),
            Role::Region,
            entry.label,
        );
        div().id("settings-page").flex_1().min_w_0().h_full().child(scroll).into_any_element()
    }

    /// A card of bones in the place of rows, while Fleet has not answered.
    pub(super) fn row_bones(&self, look: &Look, spread: Spread) -> AnyElement {
        let rows = (0..3).map(|n| {
            let says = vec![bone(look, px(160.), look.length(token::LEADING_SM)).into_any_element(), bone(look, px(280.), look.length(token::LEADING_XS)).into_any_element()];
            let control = bone(look, px(220.), look.length(token::H_CONTROL)).into_any_element();
            let row = Row { id: SharedString::from(format!("settings-bone-row-{n}")), title: SharedString::default(), tip: None, chips: Vec::new(), says, changed: false, glyph: None, control };
            labelled(div().id(SharedString::from(format!("settings-bone-{n}"))).child(split_row(look, spread, row, n == 0)), Role::Image, "Loading").into_any_element()
        });
        rows_card(look, "settings-bones", rows.collect()).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_are_written_out_in_full_inside_a_field() {
        assert_eq!(unit_in_words("ms"), "milliseconds");
        assert_eq!(unit_in_words("s"), "seconds");
        assert_eq!(unit_in_words("min"), "minutes");
        assert_eq!(unit_in_words("percent"), "%");
        assert_eq!(unit_in_words("port"), "");
        for whole in ["GiB", "KiB", "days", "minutes", "thousand tokens", "Drones"] {
            assert_eq!(unit_in_words(whole), whole);
        }
    }
}
