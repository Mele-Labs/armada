//! Settings › Limits: "tiles fill the page", the owner's take B. A head band in the page's hue, then each group's
//! small head over a grid of tiles, one setting to a tile: its title, what it does, and its control at the foot.
//! A count is stepped, a size or a figure typed with its unit inside, a flag switched. A value changed from what
//! ships is drawn in the accent, with what ships and a Reset beside it.

use gpui::{div, prelude::*, px, svg, AnyElement, App, Context, FocusHandle, Focusable, FontWeight, Hsla, KeyDownEvent, Role, SharedString, Window};
use gpui_component::input::Input;
use ipc::{Setting, SettingKind, SettingValue, SettingsList};

use super::entries::{self, Entry};
use super::overlay::{SettingsOverlay, Typed};
use super::view::{bone, refusal, ringed, small, switch_track, tip, NOT_LIVE};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// A group of tiles: its head, the glyph on each tile, and the keys it holds in the order they are drawn.
pub struct Group {
    pub name: &'static str,
    pub glyph: &'static str,
    pub keys: &'static [&'static str],
}

pub const GROUPS: &[Group] = &[
    Group { name: "At once", glyph: "layers", keys: &["limits.dronesAtOnce", "limits.checksAtOnce", "limits.watchesAtOnce", "limits.watchesPerJob"] },
    Group {
        name: "Memory and disk",
        glyph: "hard-drive",
        keys: &["limits.memorySparePercent", "limits.diskFloorGib", "limits.diskReclaimFloorGib", "limits.slotBuildTrimAfterDays", "limits.slotBuildCeilingGib"],
    },
    Group {
        name: "Agents",
        glyph: "bot",
        keys: &["limits.standingRulesKib", "limits.operatorPlanPause", "limits.contextWatch", "limits.contextWatchKtokens", "limits.disputeReadMinutes"],
    },
    Group { name: "Ports", glyph: "network", keys: &["limits.portRangeBase", "limits.portBlockGranule"] },
];

/// The counts, stepped by one a press.
const COUNTS: &[&str] = &["limits.dronesAtOnce", "limits.checksAtOnce", "limits.watchesAtOnce", "limits.watchesPerJob", "limits.portBlockGranule"];

/// The narrowest a tile is drawn: a page too narrow for another column of this keeps fewer, never squeezed ones.
const TILE_MIN: f32 = 240.;
/// How many tiles stand in a row at most.
const COLUMNS: usize = 3;
/// How wide a typed figure's field is drawn.
const FIELD_W: f32 = 148.;

/// A setting this page draws: any in the Limits section but a value Bridge owns. The Operator's pause is drawn
/// here, though the generic form leaves the Operator's settings to their own page.
pub fn holds(one: &Setting) -> bool {
    entries::section_id(&one.section) == "limits" && !matches!(one.kind, SettingKind::Json)
}

/// The groups the list fills, each with its settings. A Limits key the map does not name goes at the end of the
/// last group, so nothing Fleet sends is dropped.
pub fn grouped(list: &SettingsList) -> Vec<(&'static Group, Vec<&Setting>)> {
    let held: Vec<&Setting> = list.settings.iter().filter(|one| holds(one)).collect();
    let mut groups: Vec<(&'static Group, Vec<&Setting>)> =
        GROUPS.iter().map(|group| (group, group.keys.iter().filter_map(|key| held.iter().copied().find(|one| one.key == *key)).collect())).collect();
    let extra = held.iter().copied().filter(|one| !GROUPS.iter().any(|group| group.keys.contains(&one.key.as_str())));
    if let Some((_, last)) = groups.last_mut() {
        last.extend(extra);
    }
    groups.retain(|(_, rows)| !rows.is_empty());
    groups
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    Stepper,
    Switch,
    Field,
    /// A kind this page has no tile control for: drawn by the generic form, inside the tile.
    Generic,
}

fn control_of(one: &Setting) -> Control {
    match one.kind {
        SettingKind::Boolean => Control::Switch,
        SettingKind::Integer { .. } if COUNTS.contains(&one.key.as_str()) => Control::Stepper,
        SettingKind::Integer { .. } | SettingKind::Seconds { .. } => Control::Field,
        _ => Control::Generic,
    }
}

/// Whether the count is drawn as a stepper, whose presses are its only controls.
pub fn is_stepped(one: &Setting) -> bool {
    control_of(one) == Control::Stepper
}

/// What settings.json holds for it, else what is in force: the value a press steps from.
fn current(one: &Setting) -> &SettingValue {
    one.saved.as_ref().unwrap_or(&one.value)
}

/// Saved, and not what ships.
fn changed(one: &Setting) -> bool {
    one.saved.as_ref().is_some_and(|saved| *saved != one.default)
}

/// A value one press on More or Fewer reaches: a step off `at`, kept inside `min..=max`.
pub fn stepped(at: i64, by: i64, min: i64, max: i64) -> i64 {
    at.saturating_add(by).clamp(min, max)
}

/// A unit as a person reads it beside a figure: `%` for percent, `k tokens`, `min`; a port has none.
pub fn unit_shown(unit: &str) -> &str {
    match unit {
        "percent" => "%",
        "thousand tokens" => "k tokens",
        "minutes" => "min",
        "port" => "",
        other => other,
    }
}

/// How many tiles stand in a row where the grid is `wide` across.
fn columns(wide: f32, gap: f32) -> usize {
    (((wide + gap) / (TILE_MIN + gap)).floor() as usize).clamp(1, COLUMNS)
}

/// What ships, as a word or a figure.
fn shipped_word(one: &Setting) -> String {
    match &one.default {
        SettingValue::Bool(true) => "on".into(),
        SettingValue::Bool(false) => "off".into(),
        SettingValue::Integer(n) => n.to_string(),
        other => super::overlay::held_text(other),
    }
}

/// The stops the page's controls take, in the order they are drawn: Open, then each tile's controls.
pub fn stops(this: &SettingsOverlay, list: &SettingsList, cx: &App) -> Vec<FocusHandle> {
    let mut stops = vec![this.file_stop.clone()];
    for (_, rows) in grouped(list) {
        for one in rows {
            let Some(held) = this.stops.get(&one.key) else { continue };
            match control_of(one) {
                Control::Stepper => {
                    stops.push(held.control.clone());
                    stops.push(held.more.clone());
                }
                Control::Switch => stops.push(held.control.clone()),
                Control::Field | Control::Generic => {
                    match this.typed.get(&one.key) {
                        Some(typed) => stops.push(typed.focus(cx)),
                        None => stops.push(held.control.clone()),
                    }
                    if this.dirty(&one.key, cx) || this.saving.as_deref() == Some(one.key.as_str()) {
                        stops.push(held.save.clone());
                    }
                }
            }
            let reset = if control_of(one) == Control::Generic { one.saved.is_some() } else { changed(one) };
            if reset {
                stops.push(held.reset.clone());
            }
        }
    }
    stops
}

/// The figure's ink, and the edge of its stepper or field: the accent where the value is not what ships.
fn frame_ink(look: &Look, lit: bool) -> (Hsla, Hsla) {
    let edge = look.color(token::BORDER_DEFAULT);
    if lit {
        let accent = look.color(token::ACCENT);
        (accent, edge.blend(accent.opacity(0.6)))
    } else {
        (look.color(token::FG_DEFAULT), edge)
    }
}

fn press_keys(event: &KeyDownEvent) -> bool {
    matches!(event.keystroke.key.as_str(), "enter" | "space")
}

impl SettingsOverlay {
    /// The page: its band, then each group of tiles, or bones while Fleet has not answered.
    pub(super) fn render_limits(&self, entry: &Entry, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(entry.hue);
        let band = self.limits_band(entry, hue, look, window, cx);
        let edge = look.length(token::BORDER_WIDTH);
        let pad = look.length(token::SPACE_6);
        let gap = look.length(token::SPACE_3) + edge * 2.;
        let mut body: Vec<AnyElement> = Vec::new();
        let notes = self.file_note(look);
        if !notes.is_empty() {
            body.push(div().flex().flex_col().gap(look.length(token::SPACE_2)).px(pad).pt(look.length(token::SPACE_4)).children(notes).into_any_element());
        }
        let mut groups = div().flex().flex_col().gap(pad).p(pad);
        match &self.list {
            None => {
                let tile = |n: usize| {
                    labelled(
                        div()
                            .id(SharedString::from(format!("limits-bone-{n}")))
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_3))
                            .p(look.length(token::SPACE_4))
                            .rounded(look.length(token::RADIUS_LG))
                            .border(edge)
                            .border_color(look.color(token::BORDER_DEFAULT))
                            .bg(look.color(token::BG_GLASS))
                            .child(bone(look, px(140.), look.length(token::LEADING_SM)))
                            .child(bone(look, px(200.), look.length(token::LEADING_XS)))
                            .child(bone(look, px(96.), look.length(token::H_CONTROL))),
                        Role::Image,
                        "Loading",
                    )
                    .into_any_element()
                };
                groups = groups.child(div().flex().gap(gap).children((0..COLUMNS).map(tile)));
            }
            Some(list) => {
                if !self.live {
                    body.push(div().px(pad).pt(look.length(token::SPACE_4)).child(labelled(small(look, NOT_LIVE, look.color(token::FG_MUTED)).id("settings-not-live"), Role::Status, NOT_LIVE)).into_any_element());
                }
                let across = self.limits_width(look, window, cx) - pad * 2.;
                let cols = columns(f32::from(across), f32::from(gap));
                for (group, rows) in grouped(list) {
                    groups = groups.child(self.limits_group(group, &rows, cols, hue, look, window, cx));
                }
            }
        }
        body.push(groups.into_any_element());
        let scroll = labelled(
            div().id("settings-page-scroll").size_full().overflow_y_scroll().flex().flex_col().child(band).children(body),
            Role::Region,
            entry.label,
        );
        div()
            .id("settings-page")
            .flex_1()
            .min_w_0()
            .h_full()
            .rounded(look.length(token::RADIUS_LG))
            .border(edge)
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::SURFACE_CARD))
            .overflow_hidden()
            .child(scroll)
            .into_any_element()
    }

    /// How wide the page is drawn: the overlay's box, less its padding, the sidebar, and a mod's panel where one is open.
    fn limits_width(&self, look: &Look, window: &Window, cx: &App) -> gpui::Pixels {
        let room = crate::room::of(window, cx);
        let boxed = room.size.width * 0.9;
        let gap = look.length(token::GAP_PANEL);
        let panel = if self.mods.panel.is_some() { boxed * 0.36 + gap } else { px(0.) };
        boxed - gap * 2. - look.length(token::W_SETTINGS_INDEX) - gap - panel - look.length(token::BORDER_WIDTH) * 2.
    }

    /// The head: the page's glyph and name, and where its values are kept with the press that opens the file.
    fn limits_band(&self, entry: &Entry, hue: Hsla, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
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
                div().id("limits-title").text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(entry.label),
                Role::Heading,
                entry.label,
            ))
            .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child("How much of this Mac Fleet takes"));
        let kept = div()
            .flex_none()
            .ml_auto()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(
                div()
                    .id("limits-kept")
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_SUBTLE))
                    .tooltip(tip(SharedString::from(path)))
                    .child("Kept in this Mac's settings.json"),
            )
            .child(labelled(open, Role::Button, "Open settings.json"));
        div()
            .id("limits-band")
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(look.length(token::SPACE_3) + edge * 2.)
            .gap_y(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4))
            .bg(look.color(token::SURFACE_CARD).blend(hue.opacity(0.12)))
            .border_b(edge)
            .border_color(hue.opacity(0.35))
            .child(glyph)
            .child(name)
            .child(kept)
            .into_any_element()
    }

    /// One group: its head with the hue's bar, then its tiles in rows of `cols`, the last row left short.
    #[allow(clippy::too_many_arguments)]
    fn limits_group(&self, group: &Group, rows: &[&Setting], cols: usize, hue: Hsla, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let edge = look.length(token::BORDER_WIDTH);
        let gap = look.length(token::SPACE_3) + edge * 2.;
        let head = labelled(
            div()
                .id(SharedString::from(format!("limits-group-{}", group.name)))
                .self_start()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(div().flex_none().w(edge * 3.).h(look.length(token::SPACE_3) + edge).rounded(edge * 2.).bg(hue))
                .child(
                    div()
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_MUTED))
                        .child(group.name),
                ),
            Role::Heading,
            group.name,
        );
        let mut grid = div().flex().flex_col().gap(gap);
        for chunk in rows.chunks(cols) {
            let mut line = div().flex().gap(gap);
            for one in chunk {
                line = line.child(self.limits_tile(one, group.glyph, hue, look, window, cx));
            }
            for _ in chunk.len()..cols {
                // The same padding and edge as a tile, so the empty cells take the width a tile's cell would.
                line = line.child(div().flex_1().min_w_0().px(look.length(token::SPACE_4)).border(edge));
            }
            grid = grid.child(line);
        }
        div().flex().flex_col().gap(look.length(token::SPACE_3)).child(head).child(grid).into_any_element()
    }

    /// One setting as a tile.
    fn limits_tile(&self, one: &Setting, glyph: &str, hue: Hsla, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let key = one.key.clone();
        if !self.stops.contains_key(&key) {
            return div().flex_1().min_w_0().into_any_element();
        }
        let edge = look.length(token::BORDER_WIDTH);
        let saving = self.saving.as_deref() == Some(key.as_str());
        let off = !self.live || self.saving.as_ref().is_some_and(|other| other != &key) || saving;
        let lit = changed(one);
        let kind = control_of(one);
        let title: SharedString = one.title.clone().into();
        let accent = look.color(token::ACCENT);

        let top = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2) + edge * 2.)
            .child(
                div()
                    .flex_none()
                    .size(look.length(token::H_BADGE) + edge * 2.)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(hue.opacity(0.22))
                    .child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_3)).text_color(hue)),
            )
            .child(
                div()
                    .id(SharedString::from(format!("limits-name-{key}")))
                    .min_w_0()
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .tooltip(tip(SharedString::from(key.clone())))
                    .child(title.clone()),
            );
        let shown = first_sentence(&one.description);
        let mut said = small(look, shown.to_string(), look.color(token::FG_MUTED)).id(SharedString::from(format!("limits-says-{key}")));
        if shown.len() < one.description.trim_end().len() {
            said = said.tooltip(tip(SharedString::from(one.description.clone())));
        }
        let says = div().flex_1().child(said);

        let typed_text = self.typed.get(&key).map(|typed| typed.text(cx)).unwrap_or_default();
        let dirty = self.dirty(&key, cx);
        let mut notes: Vec<AnyElement> = Vec::new();
        if one.pending_restart {
            notes.push(labelled(small(look, "Restart Fleet to apply.", look.color(token::FG_DEFAULT)).id(SharedString::from(format!("restart-{key}"))), Role::Status, "Restart Fleet to apply.").into_any_element());
        }
        if let Some(env) = &one.overridden_by_env {
            let said = format!("Overridden by ${env} where Fleet runs, so the value here is not the one in force.");
            notes.push(labelled(small(look, said.clone(), look.color(token::FG_DEFAULT)).id(SharedString::from(format!("env-{key}"))), Role::Status, said).into_any_element());
        }
        if let SettingKind::Integer { min, max, .. } | SettingKind::Seconds { min, max } = one.kind {
            let in_range = typed_text.trim().parse::<i64>().is_ok_and(|n| (min..=max).contains(&n));
            if kind == Control::Field && dirty && !in_range {
                let said = format!("Between {min} and {max}.");
                notes.push(labelled(refusal(look, said.clone()).id(SharedString::from(format!("range-{key}"))), Role::Alert, said).into_any_element());
            }
        }
        if let Some(said) = self.refused.get(&key).cloned() {
            notes.push(labelled(refusal(look, said.clone()).id(SharedString::from(format!("refused-{key}"))), Role::Alert, said).into_any_element());
        }

        let beside = lit.then(|| {
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(format!("Default {}", shipped_word(one)))))
                .child(self.limits_reset(one, off, look, window, cx))
        });

        let control: AnyElement = match kind {
            Control::Stepper => self.limits_stepper(one, lit, off, look, window, cx),
            Control::Switch => self.limits_switch(one, off, look, window, cx),
            Control::Field => self.limits_field(one, lit, off, saving, look, window, cx),
            Control::Generic => self.render_field(one, look, window, cx),
        };
        let foot = if kind == Control::Generic {
            control
        } else {
            div().flex().flex_wrap().items_center().justify_between().gap_x(look.length(token::SPACE_3)).gap_y(look.length(token::SPACE_1)).child(control).children(beside).into_any_element()
        };

        labelled(
            div()
                .id(SharedString::from(format!("limits-tile-{key}")))
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2) + edge * 2.)
                .pt(look.length(token::SPACE_3) + edge * 2.)
                .px(look.length(token::SPACE_4))
                .pb(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_LG))
                .border(edge)
                .border_color(if lit { look.color(token::BORDER_DEFAULT).blend(accent.opacity(0.45)) } else { look.color(token::BORDER_DEFAULT) })
                .bg(look.color(token::BG_GLASS))
                .shadow(look.shadows(token::SHADOW_CARD))
                .child(top)
                .child(says)
                .child(foot)
                .children(notes),
            Role::Group,
            one.title.clone(),
        )
        .into_any_element()
    }

    /// Reset: what settings.json holds for it goes, and what ships is in force again.
    fn limits_reset(&self, one: &Setting, off: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let key = one.key.clone();
        let Some(stops) = self.stops.get(&key) else { return div().into_any_element() };
        let said = format!("Reset: {}", one.title);
        let button = ringed(look, super::view::ghost(look, SharedString::from(format!("reset-{key}")), "Reset", off), &stops.reset, window).track_focus(&stops.reset).when(!off, |button| {
            let press = key.clone();
            button.on_click(cx.listener(move |this, _, window, cx| this.save(press.clone(), None, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if press_keys(event) {
                    this.save(key.clone(), None, window, cx);
                }
            }))
        });
        labelled(button, Role::Button, said).into_any_element()
    }

    /// A count: Fewer, the figure, More. Each press saves the figure a step off, kept inside the range.
    fn limits_stepper(&self, one: &Setting, lit: bool, off: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let key = one.key.clone();
        let Some(stops) = self.stops.get(&key) else { return div().into_any_element() };
        let SettingKind::Integer { min, max, .. } = one.kind else { return div().into_any_element() };
        let now = if let SettingValue::Integer(n) = current(one) { *n } else { min };
        let edge = look.length(token::BORDER_WIDTH);
        let (ink, frame) = frame_ink(look, lit);
        let side = |id: &str, glyph: &'static str, said: String, by: i64, handle: &FocusHandle, spent: bool| {
            let to = stepped(now, by, min, max);
            let dead = off || spent;
            let button = ringed(
                look,
                div()
                    .id(SharedString::from(format!("{id}-{key}")))
                    .track_focus(handle)
                    .flex_none()
                    .w(look.length(token::H_CONTROL))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(!dead, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
                    .child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(if dead { look.color(token::FG_SUBTLE) } else { look.color(token::FG_MUTED) })),
                handle,
                window,
            )
            .when(!dead, |button| {
                let (press, tap) = (key.clone(), key.clone());
                button.on_click(cx.listener(move |this, _, window, cx| this.save(press.clone(), Some(SettingValue::Integer(to)), window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if press_keys(event) {
                        this.save(tap.clone(), Some(SettingValue::Integer(to)), window, cx);
                    }
                }))
            });
            labelled(button, Role::Button, said)
        };
        let less = side("less", "minus", format!("Fewer {}", one.title), -1, &stops.control, now <= min);
        let more = side("more", "plus", format!("More {}", one.title), 1, &stops.more, now >= max);
        let value = labelled(
            div()
                .id(SharedString::from(format!("limits-value-{key}")))
                .flex_none()
                .min_w(look.length(token::SPACE_8) + look.length(token::SPACE_2))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .border_l(edge)
                .border_r(edge)
                .border_color(look.color(token::BORDER_SUBTLE))
                .text_size(look.length(token::TEXT_BASE))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(ink)
                .child(SharedString::from(now.to_string())),
            Role::Status,
            format!("{}: {now}", one.title),
        );
        div()
            .flex_none()
            .flex()
            .items_center()
            .h(look.length(token::H_CONTROL))
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_MD))
            .border(edge)
            .border_color(frame)
            .bg(look.color(token::BG_SUNKEN))
            .child(less)
            .child(value)
            .child(more)
            .into_any_element()
    }

    /// A flag's switch.
    fn limits_switch(&self, one: &Setting, off: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let key = one.key.clone();
        let Some(stops) = self.stops.get(&key) else { return div().into_any_element() };
        let on = *current(one) == SettingValue::Bool(true);
        let control = ringed(
            look,
            div()
                .id(SharedString::from(format!("switch-{key}")))
                .track_focus(&stops.control)
                .flex_none()
                .rounded(look.length(token::RADIUS_MD))
                .when(!off, |row| row.cursor_pointer())
                .child(switch_track(look, on, off))
                .when(!off, |row| {
                    let press = key.clone();
                    row.on_click(cx.listener(move |this, _, window, cx| this.save(press.clone(), Some(SettingValue::Bool(!on)), window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if press_keys(event) {
                            this.save(key.clone(), Some(SettingValue::Bool(!on)), window, cx);
                        }
                    }))
                }),
            &stops.control,
            window,
        );
        labelled(control, Role::Switch, one.title.clone()).into_any_element()
    }

    /// A figure typed, with its unit inside at the right and Save inside once it differs from what is in force.
    #[allow(clippy::too_many_arguments)]
    fn limits_field(&self, one: &Setting, lit: bool, off: bool, saving: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let key = one.key.clone();
        let Some(Typed::Line(state)) = self.typed.get(&key) else { return div().into_any_element() };
        let unit = match &one.kind {
            SettingKind::Seconds { .. } => Some("s"),
            SettingKind::Integer { unit, .. } => unit.as_deref().map(unit_shown).filter(|unit| !unit.is_empty()),
            _ => None,
        };
        let focus = state.read(cx).focus_handle(cx);
        let edge = look.length(token::BORDER_WIDTH);
        let (ink, frame) = frame_ink(look, lit);
        let save = (self.dirty(&key, cx) || saving).then(|| self.save_button(one, look, window, cx));
        let suffix = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .children(unit.map(|unit| div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(unit.to_string()))))
            .children(save);
        let frame = div()
            .id(SharedString::from(format!("line-{key}")))
            .key_context(crate::keymap::TEXT_BOX)
            .flex_none()
            .w(px(FIELD_W))
            .h(look.length(token::H_CONTROL))
            .flex()
            .items_center()
            .rounded(look.length(token::RADIUS_MD))
            .border(edge)
            .border_color(frame)
            .bg(look.color(token::BG_SUNKEN))
            .text_size(look.length(token::TEXT_BASE))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(ink)
            .child(Input::new(state).appearance(false).bordered(false).disabled(off).text_color(ink).suffix(suffix));
        labelled(ringed(look, frame, &focus, window), Role::TextInput, one.title.clone()).into_any_element()
    }
}

/// A tile shows the registry's first sentence, as the mock did; the whole
/// description is the tooltip.
fn first_sentence(text: &str) -> &str {
    let text = text.trim_end();
    match text.find(". ") {
        Some(at) => &text[..=at],
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting(key: &str) -> Setting {
        Setting {
            key: key.into(),
            group: "Fleet".into(),
            section: "Limits".into(),
            title: key.into(),
            description: String::new(),
            tip: None,
            kind: SettingKind::Integer { min: 1, max: 8, unit: None },
            default: SettingValue::Integer(2),
            value: SettingValue::Integer(2),
            saved: None,
            applies: ipc::SettingApplies::Live,
            pending_restart: false,
            overridden_by_env: None,
        }
    }

    #[test]
    fn a_step_stays_inside_the_range() {
        assert_eq!(stepped(3, 1, 1, 8), 4);
        assert_eq!(stepped(8, 1, 1, 8), 8);
        assert_eq!(stepped(1, -1, 1, 8), 1);
        assert_eq!(stepped(i64::MAX, 1, 1, 8), 8);
    }

    #[test]
    fn every_key_of_the_map_is_in_one_group_and_the_counts_are_in_the_map() {
        let all: Vec<&str> = GROUPS.iter().flat_map(|group| group.keys.iter().copied()).collect();
        assert_eq!(all.len(), 16);
        let mut once = all.clone();
        once.sort_unstable();
        once.dedup();
        assert_eq!(once.len(), all.len());
        assert!(COUNTS.iter().all(|key| all.contains(key)));
    }

    #[test]
    fn a_limits_key_the_map_does_not_name_goes_at_the_end_of_the_last_group() {
        let list = SettingsList { path: String::new(), refused: None, settings: vec![setting("limits.zebra"), setting("limits.dronesAtOnce"), setting("limits.portBlockGranule")] };
        let groups = grouped(&list);
        let names: Vec<&str> = groups.iter().map(|(group, _)| group.name).collect();
        assert_eq!(names, ["At once", "Ports"]);
        let ports: Vec<&str> = groups[1].1.iter().map(|one| one.key.as_str()).collect();
        assert_eq!(ports, ["limits.portBlockGranule", "limits.zebra"]);
    }

    #[test]
    fn units_read_as_the_page_writes_them() {
        assert_eq!(unit_shown("percent"), "%");
        assert_eq!(unit_shown("thousand tokens"), "k tokens");
        assert_eq!(unit_shown("minutes"), "min");
        assert_eq!(unit_shown("port"), "");
        assert_eq!(unit_shown("GiB"), "GiB");
        assert_eq!(unit_shown("KiB"), "KiB");
        assert_eq!(unit_shown("days"), "days");
    }

    #[test]
    fn narrow_pages_keep_fewer_columns() {
        assert_eq!(columns(974., 14.), 3);
        assert_eq!(columns(600., 14.), 2);
        assert_eq!(columns(300., 14.), 1);
        assert_eq!(columns(100., 14.), 1);
    }

    #[test]
    fn a_value_is_changed_only_where_saved_and_not_what_ships() {
        let mut one = setting("limits.dronesAtOnce");
        assert!(!changed(&one));
        one.saved = Some(SettingValue::Integer(2));
        assert!(!changed(&one));
        one.saved = Some(SettingValue::Integer(3));
        assert!(changed(&one));
    }

    #[test]
    fn a_tile_shows_the_first_sentence_of_the_description() {
        assert_eq!(first_sentence("Disk to start reclaiming at. Fleet frees the oldest builds first."), "Disk to start reclaiming at.");
        assert_eq!(first_sentence("Drones at once."), "Drones at once.");
        assert_eq!(first_sentence("No full stop"), "No full stop");
    }
}
