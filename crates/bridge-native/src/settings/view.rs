//! Settings drawn: the overlay's frame, the sidebar, and each entry's page. A schema setting is drawn by its
//! kind, as Classic's `SettingField` draws it (`packages/components/src/compositions/SettingField/`).

use std::collections::BTreeMap;

use gpui::{
    anchored, canvas, deferred, div, prelude::*, px, svg, AnyElement, App, Bounds, Context, FocusHandle, FontWeight, Hsla, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Role, SharedString, Stateful, Div, Window,
};
use gpui_component::input::{Input, Textarea};
use ipc::{Setting, SettingKind, SettingValue};

use super::entries::{self, Body, Entry, Group, GROUPS};
use super::overlay::{held_text, Picking, SettingsOverlay, Typed};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// Whether each repository's Jobs start without the connectors on his account, by Manifest id, `true` for off.
pub const CONNECTORS_OFF: &str = "tokens.connectorsOff";
/// The folders every Job may reach, and may not, past its slots (`REACH_FOLDERS_KEY`).
pub const REACH_FOLDERS: &str = "features.reachFolders";
/// A range this narrow is dragged, with its figure still typed; a wider one, such as a port, is only typed.
const DRAGGED_SPAN: i64 = 1000;
/// A mark at each step, until they would crowd.
const MARKED_STEPS: i64 = 32;
/// Why nothing can be sent.
const NOT_LIVE: &str = "Fleet is not connected, so nothing can be changed.";

pub fn slides(one: &Setting) -> bool {
    match one.kind {
        SettingKind::Integer { min, max, .. } | SettingKind::Seconds { min, max } => max - min <= DRAGGED_SPAN,
        _ => false,
    }
}

/// `15 minutes`, `1 hour`, `90 seconds`: whole units only, anything else in seconds (`inWords`).
pub fn in_words(seconds: i64) -> String {
    let plural = |n: i64, unit: &str| format!("{n} {unit}{}", if n == 1 { "" } else { "s" });
    if seconds >= 3600 && seconds % 3600 == 0 {
        plural(seconds / 3600, "hour")
    } else if seconds >= 60 && seconds % 60 == 0 {
        plural(seconds / 60, "minute")
    } else {
        plural(seconds, "second")
    }
}

/// `2`, `15%`, `10 GiB`, `$10`.
fn figure(value: i64, unit: Option<&str>) -> String {
    match unit {
        None | Some("") => value.to_string(),
        Some("%") => format!("{value}%"),
        Some("$") => format!("${value}"),
        Some(unit) => format!("{value} {unit}"),
    }
}

/// The shipped value as a person reads it, or `None` for a kind that draws no line (`controlOf`).
fn shipped(one: &Setting) -> Option<String> {
    let number = |value: &SettingValue, fallback: i64| if let SettingValue::Integer(n) = value { *n } else { fallback };
    match &one.kind {
        SettingKind::Integer { min, unit, .. } => Some(figure(number(&one.default, *min), unit.as_deref())),
        SettingKind::Seconds { min, .. } => Some(in_words(number(&one.default, *min))),
        SettingKind::Boolean => Some(if one.default == SettingValue::Bool(true) { "on".into() } else { "off".into() }),
        _ => None,
    }
}

/// The folders a Job always reaches and never reaches, as settings.json holds them (`reachFoldersOf`).
pub fn reach_folders(value: Option<&SettingValue>) -> (Vec<String>, Vec<String>) {
    let list = |name: &str| match value {
        Some(SettingValue::Map(map)) => match map.get(name) {
            Some(SettingValue::List(items)) => items.iter().filter_map(|item| if let SettingValue::Text(text) = item { Some(text.clone()) } else { None }).collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    (list("allow"), list("block"))
}

/// Each repository's connectors switch as set, by Manifest id. A value that is not a map of true and false reads
/// as nothing set, so each repository follows its armada.yml.
pub fn connectors_set(value: Option<&SettingValue>) -> BTreeMap<String, bool> {
    match value {
        Some(SettingValue::Map(map)) => map.iter().filter_map(|(id, off)| if let SettingValue::Bool(off) = off { Some((id.clone(), *off)) } else { None }).collect(),
        _ => BTreeMap::new(),
    }
}

/// The stops a pane's own controls add after the schema's.
pub fn pane_stops(this: &SettingsOverlay, shown: &Entry, _cx: &App) -> Vec<FocusHandle> {
    let prefix = match shown.body {
        Body::Features => "reach:",
        Body::Repository => "connectors:",
        _ => return Vec::new(),
    };
    this.panes.iter().filter(|(key, _)| key.starts_with(prefix)).map(|(_, handle)| handle.clone()).collect()
}

pub fn tip(said: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView {
    let said: SharedString = said.into();
    move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx)
}

pub fn mono(look: &Look) -> SharedString {
    look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default()
}

/// The key, read in mono at the right of its row (`Readout`).
fn readout(look: &Look, text: &str) -> Div {
    div()
        .flex_none()
        .px(look.length(token::SPACE_2) - look.length(token::BORDER_WIDTH) * 2.)
        .rounded(look.length(token::RADIUS_SM))
        .bg(look.color(token::BG_SUNKEN))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .text_color(look.color(token::FG_MUTED))
        .child(SharedString::from(text.to_string()))
}

/// The focus colour, from the theme's focus edge, as every native ring takes it.
pub fn ring_colour(look: &Look) -> Hsla {
    look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour)
}

/// The ring a focused control draws, in the focus colour.
pub fn ringed(look: &Look, element: Stateful<Div>, focus: &FocusHandle, window: &Window) -> Stateful<Div> {
    if focus.is_focused(window) {
        element.border(look.length(token::FOCUS_RING_WIDTH)).border_color(ring_colour(look))
    } else {
        element
    }
}

/// A quiet button: its words, and a hover fill (`Button variant="ghost" size="sm"`).
pub fn ghost(look: &Look, id: impl Into<gpui::ElementId>, said: &str, off: bool) -> Stateful<Div> {
    div()
        .id(id.into())
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(gpui::transparent_black())
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(if off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) })
        .when(!off, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
        .child(SharedString::from(said.to_string()))
}

/// A switch as Electron's `Switch`: a recessed track with the knob, and ON or OFF in the half it leaves.
pub fn switch_track(look: &Look, on: bool, off: bool) -> Div {
    let edge = look.length(token::BORDER_WIDTH);
    let tall = look.length(token::H_BADGE) + look.length(token::SPACE_2);
    let pad = edge * 3.;
    let knob = tall - pad * 2.;
    let air = look.length(token::SPACE_2);
    let lit = on && !off;
    let word = div()
        .flex_none()
        .w(look.length(token::SPACE_4) + look.length(token::SPACE_1))
        .flex()
        .justify_center()
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_SWITCH))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(if lit { look.color(token::FG_INVERSE) } else { look.color(token::FG_MUTED) })
        .child(if on { "ON" } else { "OFF" });
    let disc = div()
        .flex_none()
        .size(knob)
        .rounded_full()
        .border(edge * 2.)
        .border_color(if lit { look.color(token::ACCENT).opacity(0.6) } else { look.color(token::BORDER_STRONG) })
        .bg(if on { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) });
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(air)
        .h(tall)
        .pl(pad - edge)
        .pr(pad - edge)
        .border(edge)
        .border_color(if off { look.color(token::BORDER_SUBTLE) } else if lit { look.color(token::ACCENT) } else { look.color(token::BORDER_STRONG) })
        .rounded(tall / 2.)
        .bg(if lit { look.color(token::ACCENT) } else { look.color(token::BG_SUNKEN) })
        .map(|track| if on { track.pl(air).child(word).child(disc) } else { track.pr(air).child(disc).child(word) })
}

/// A card, as Classic's `Card` with a header: the title, what sits at the header's right, then the body.
pub fn card(look: &Look, id: impl Into<gpui::ElementId>, title: &str, right: Option<AnyElement>, body: Vec<AnyElement>) -> Stateful<Div> {
    div()
        .id(id.into())
        .flex_none()
        .flex()
        .flex_col()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .bg(look.color(token::SURFACE_CARD))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::PAD_CARD))
                .pt(look.length(token::PAD_CARD))
                .pb(look.length(token::SPACE_4))
                .child(
                    div()
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(SharedString::from(title.to_string())),
                )
                .children(right),
        )
        .child(div().flex().flex_col().gap(look.length(token::SPACE_6)).px(look.length(token::PAD_CARD)).pb(look.length(token::PAD_CARD)).children(body))
}

/// A grey bar where words will be, while Fleet has not answered.
pub fn bone(look: &Look, wide: Pixels, tall: Pixels) -> Div {
    div().flex_none().w(wide).h(tall).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::BG_HOVER))
}

pub fn small(look: &Look, said: impl Into<SharedString>, ink: Hsla) -> Div {
    div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(ink).child(said.into())
}

/// Where a labelled element was drawn last frame, for a list opened under it.
pub fn bounds_of(window: &Window, cx: &App, role: Role, name: &str) -> Option<Bounds<Pixels>> {
    cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, name)).map(|drawn| drawn.bounds)
}

impl SettingsOverlay {
    // ---- the sidebar --------------------------------------------------------------------------

    fn render_side(&self, visible: &[&'static Entry], shown: Option<&'static Entry>, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let side_keys = self.side.is_focused(window);
        let mut groups: Vec<AnyElement> = Vec::new();
        for group in GROUPS {
            let rows: Vec<&&Entry> = visible.iter().filter(|one| one.group == group).collect();
            if rows.is_empty() {
                continue;
            }
            let label = match (group, &self.scope) {
                (Group::Project, Some(scope)) => scope.name.clone(),
                _ => group.label().to_string(),
            };
            // The new look's band (`depth.css`, `.armada-system-side__head`), which Classic always runs: the foreground at
            // 8% under mono words. The words are muted, so only the entries read as pickable.
            let caption = div()
                .flex()
                .items_center()
                .min_h(look.length(token::H_CONTROL_SM))
                .pl(look.length(token::SPACE_3))
                .pr(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::FG_DEFAULT).opacity(0.08))
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_MUTED))
                .child(SharedString::from(label.clone()));
            let mut list = div().flex().flex_col().gap(look.length(token::SPACE_1)).child(caption);
            for one in rows {
                let on = shown.is_some_and(|shown| shown.id == one.id);
                let id = one.id;
                let row = div()
                    .id(SharedString::from(format!("settings-entry-{id}")))
                    .flex()
                    .items_center()
                    .min_h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .cursor_pointer()
                    .map(|row| {
                        if on {
                            row.bg(look.color(token::SURFACE_ROW_SELECTED)).text_color(look.color(token::FG_DEFAULT)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        } else {
                            row.text_color(look.color(token::FG_MUTED)).hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER)))
                        }
                    })
                    .when(on && side_keys, |row| row.border(look.length(token::FOCUS_RING_WIDTH)).border_color(ring_colour(look)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.pick(id, cx);
                        window.focus(&this.side, cx);
                    }))
                    .child(one.label);
                list = list.child(labelled(row, Role::Button, one.label));
            }
            groups.push(labelled(list.id(SharedString::from(format!("settings-group-{}", group.label()))), Role::Group, label).into_any_element());
        }
        let search = labelled(
            div()
                .id("settings-search")
                .flex_none()
                .mx(look.length(token::SPACE_2))
                .key_context(crate::keymap::TEXT_BOX)
                .text_size(look.length(token::TEXT_SM))
                .child(Input::new(&self.query)),
            Role::SearchInput,
            "Search Settings",
        );
        let head = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::PAD_CARD))
            .pt(look.length(token::PAD_CARD))
            .child(labelled(
                div()
                    .id("settings-title")
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child("Settings"),
                Role::Heading,
                "Settings",
            ));
        labelled(
            div()
                .id("settings-side")
                .track_focus(&self.side)
                .flex_none()
                .w(look.length(token::W_SETTINGS_INDEX))
                .h_full()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::SURFACE_CARD))
                .child(head)
                .child(search)
                .child(
                    div()
                        .id("settings-side-scroll")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .track_scroll(&self.side_scroll)
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_3))
                        .px(look.length(token::SPACE_1))
                        .pb(look.length(token::SPACE_3))
                        .children(groups),
                ),
            Role::Navigation,
            "Settings",
        )
        .into_any_element()
    }

    // ---- a schema setting ---------------------------------------------------------------------

    fn render_field(&self, one: &Setting, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let key = one.key.clone();
        let Some(stops) = self.stops.get(&key) else { return div().into_any_element() };
        let saving = self.saving.as_deref() == Some(key.as_str());
        let off = !self.live || self.saving.as_ref().is_some_and(|other| other != &key) || saving;
        let modified = one.saved.is_some();
        let refused = self.refused.get(&key).cloned();
        let title: SharedString = one.title.clone().into();
        let keyed = readout(look, &one.key);
        let reset_said = if matches!(one.kind, SettingKind::Prompt) { "Reset to shipped" } else { "Reset" };
        let marks = modified.then(|| {
            let chip = div()
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_SM))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::SETTING_MODIFIED))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .text_color(look.color(token::SETTING_MODIFIED))
                .child("Modified");
            let reset_key = key.clone();
            let reset = ringed(look, ghost(look, SharedString::from(format!("reset-{key}")), reset_said, off), &stops.reset, window)
                .track_focus(&stops.reset)
                .when(!off, |button| {
                    button
                        .on_click(cx.listener(move |this, _, window, cx| this.save(reset_key.clone(), None, window, cx)))
                        .on_key_down(cx.listener({
                            let key = key.clone();
                            move |this, event: &KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.save(key.clone(), None, window, cx);
                                }
                            }
                        }))
                });
            div().flex().items_center().gap(look.length(token::SPACE_2)).child(labelled(chip.id(SharedString::from(format!("modified-{key}"))), Role::Status, "Modified")).child(labelled(reset, Role::Button, format!("{reset_said}: {}", one.title)))
        });
        let mut notes: Vec<AnyElement> = Vec::new();
        if one.pending_restart {
            notes.push(labelled(small(look, "Restart Fleet to apply.", look.color(token::FG_DEFAULT)).id(SharedString::from(format!("restart-{key}"))), Role::Status, "Restart Fleet to apply.").into_any_element());
        }
        if let Some(env) = &one.overridden_by_env {
            let said = format!("Overridden by ${env} where Fleet runs, so the value here is not the one in force.");
            notes.push(labelled(small(look, said.clone(), look.color(token::FG_DEFAULT)).id(SharedString::from(format!("env-{key}"))), Role::Status, said).into_any_element());
        }
        let refused_line = |look: &Look| refused.clone().map(|said| labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id(SharedString::from(format!("refused-{}", one.key))), Role::Alert, said));
        let ships = shipped(one);

        if let SettingKind::Boolean = one.kind {
            let on = one.value == SettingValue::Bool(true);
            let flip_key = key.clone();
            let control = ringed(
                look,
                div()
                    .id(SharedString::from(format!("switch-{key}")))
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .track_focus(&stops.control)
                    .when(!off, |row| row.cursor_pointer())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .min_w_0()
                            .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(title.clone()))
                            .when(!one.description.is_empty(), |words| words.child(small(look, one.description.clone(), look.color(token::FG_MUTED)))),
                    )
                    .child(switch_track(look, on, off))
                    .when(!off, |row| {
                        row.on_click(cx.listener(move |this, _, window, cx| this.save(flip_key.clone(), Some(SettingValue::Bool(!on)), window, cx))).on_key_down(cx.listener({
                            let key = key.clone();
                            move |this, event: &KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.save(key.clone(), Some(SettingValue::Bool(!on)), window, cx);
                                }
                            }
                        }))
                    }),
                &stops.control,
                window,
            );
            return div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(labelled(control, Role::Switch, one.title.clone()))
                .child(div().flex().items_center().justify_between().gap(look.length(token::SPACE_2)).min_h(look.length(token::SPACE_6)).child(div().children(marks)).child(keyed))
                .children(ships.map(|ships| small(look, format!("Ships {ships}."), look.color(token::FG_MUTED))))
                .children(notes)
                .children(refused_line(look))
                .into_any_element();
        }

        let typed_field: AnyElement = match (&one.kind, self.typed.get(&key)) {
            (SettingKind::Choice { options }, _) => {
                let key = key.clone();
                let now = held_text(&one.value);
                let name = one.title.clone();
                let options = options.clone();
                let trigger = ringed(
                    look,
                    div()
                        .id(SharedString::from(format!("choice-{key}")))
                        .track_focus(&stops.control)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(look.length(token::SPACE_2))
                        .h(look.length(token::H_CONTROL))
                        .px(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .bg(look.color(token::BG_SUNKEN))
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(if off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) })
                        .when(!off, |trigger| trigger.cursor_pointer())
                        .when(one.description.is_empty(), |trigger| trigger.tooltip(tip(one.tip.clone().unwrap_or_else(|| one.title.clone()))))
                        .child(SharedString::from(now.clone()))
                        .child(svg().path(icon_path("chevron-down")).flex_none().size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED))),
                    &stops.control,
                    window,
                )
                .when(!off, |trigger| {
                    let (press_key, press_name, press_options, press_now) = (key.clone(), name.clone(), options.clone(), now.clone());
                    trigger
                        .on_click(cx.listener(move |this, _, window, cx| {
                            let at = bounds_of(window, cx, Role::ComboBox, &press_name).map(|bounds| bounds.bottom_left()).unwrap_or_default();
                            let options = press_options.iter().map(|word| (SharedString::from(word.clone()), *word == press_now)).collect();
                            this.open_choice(Picking::Setting(press_key.clone()), press_name.clone().into(), options, at, false, window, cx);
                        }))
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space" | "down") {
                                let at = bounds_of(window, cx, Role::ComboBox, &name).map(|bounds| bounds.bottom_left()).unwrap_or_default();
                                let options = options.iter().map(|word| (SharedString::from(word.clone()), *word == now)).collect();
                                this.open_choice(Picking::Setting(key.clone()), name.clone().into(), options, at, true, window, cx);
                                cx.stop_propagation();
                            }
                        }))
                });
                labelled(trigger, Role::ComboBox, one.title.clone()).into_any_element()
            }
            (_, Some(Typed::Lines(state))) => labelled(
                div()
                    .id(SharedString::from(format!("lines-{key}")))
                    .key_context(crate::keymap::TEXT_BOX)
                    .w_full()
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_SM))
                    .child(Textarea::new(state).disabled(off)),
                Role::TextInput,
                one.title.clone(),
            )
            .into_any_element(),
            (_, Some(Typed::Line(state))) => {
                let unit = match &one.kind {
                    SettingKind::Seconds { .. } => Some("s".to_string()),
                    SettingKind::Integer { unit, .. } => unit.clone(),
                    _ => None,
                };
                let field = div()
                    .id(SharedString::from(format!("line-{key}")))
                    .key_context(crate::keymap::TEXT_BOX)
                    .flex_1()
                    .min_w_0()
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_SM))
                    .child(Input::new(state).disabled(off).when_some(unit, |input, unit| input.suffix(small(look, unit, look.color(token::FG_MUTED))))
                        .when(self.dirty(&key, cx) || saving, |input| input.suffix(self.save_button(one, look, window, cx))));
                labelled(field, Role::TextInput, one.title.clone()).into_any_element()
            }
            _ => div().into_any_element(),
        };
        let typed_text = self.typed.get(&key).map(|typed| typed.text(cx)).unwrap_or_default();
        let in_range = match one.kind {
            SettingKind::Integer { min, max, .. } | SettingKind::Seconds { min, max } => typed_text.trim().parse::<i64>().is_ok_and(|n| (min..=max).contains(&n)),
            _ => true,
        };
        let dirty = self.dirty(&key, cx);
        let range_said = match one.kind {
            SettingKind::Integer { min, max, .. } | SettingKind::Seconds { min, max } if dirty && !in_range => Some(format!("Between {min} and {max}.")),
            _ => None,
        };
        let row: AnyElement = if slides(one) {
            div().flex().items_center().gap(look.length(token::SPACE_4)).child(self.render_slider(one, &typed_text, in_range, off, look, window, cx)).child(div().flex_none().w(px(144.)).child(typed_field)).into_any_element()
        } else if matches!(one.kind, SettingKind::Prompt | SettingKind::TextList) {
            div()
                .flex()
                .flex_col()
                .items_end()
                .gap(look.length(token::SPACE_2))
                .child(typed_field)
                .when(dirty || saving, |row| row.child(self.save_button(one, look, window, cx)))
                .into_any_element()
        } else {
            typed_field
        };
        let seconds_said = match one.kind {
            SettingKind::Seconds { .. } if in_range => typed_text.trim().parse::<i64>().ok().map(in_words),
            _ => None,
        };
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(look.length(token::SPACE_3))
                    .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(title))
                    .child(keyed),
            )
            .children(marks)
            .when(!one.description.is_empty(), |field| field.child(small(look, one.description.clone(), look.color(token::FG_MUTED))))
            .child(row)
            .children(range_said.map(|said| labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id(SharedString::from(format!("range-{key}"))), Role::Alert, said)))
            .children(seconds_said.map(|said| small(look, said, look.color(token::FG_MUTED))))
            .children(ships.map(|ships| small(look, format!("Ships at {ships}."), look.color(token::FG_MUTED))))
            .children(notes)
            .children(refused_line(look))
            .into_any_element()
    }

    fn save_button(&self, one: &Setting, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let key = one.key.clone();
        let Some(stops) = self.stops.get(&key) else { return div().into_any_element() };
        let saving = self.saving.as_deref() == Some(key.as_str());
        let text = self.typed.get(&key).map(|typed| typed.text(cx)).unwrap_or_default();
        let takes = super::overlay::typed_value(&one.kind, &text).is_some();
        let off = !self.live || self.saving.is_some() || !takes;
        let said = if saving { "Saving…" } else { "Save" };
        let button = ringed(look, ghost(look, SharedString::from(format!("save-{key}")), said, off), &stops.save, window).track_focus(&stops.save).when(!off, |button| {
            let press = key.clone();
            button.on_click(cx.listener(move |this, _, window, cx| this.save_typed(&press, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.save_typed(&key, window, cx);
                }
            }))
        });
        labelled(button, Role::Button, format!("Save {}", one.title)).into_any_element()
    }

    /// A figure's slider: the track takes the slack, a mark at each step while they do not crowd. It moves what
    /// is typed; Save sends it.
    fn render_slider(&self, one: &Setting, typed: &str, in_range: bool, off: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let (SettingKind::Integer { min, max, .. } | SettingKind::Seconds { min, max }) = one.kind else { return div().into_any_element() };
        let Some(stops) = self.stops.get(&one.key) else { return div().into_any_element() };
        let held = if let SettingValue::Integer(n) = one.value { n } else { min };
        let at = if in_range { typed.trim().parse::<i64>().unwrap_or(held) } else { held };
        let fill = if max > min { (at - min) as f32 / (max - min) as f32 } else { 0. };
        let steps = max - min + 1;
        let tracks = self.tracks.clone();
        let key = one.key.clone();
        let rail = look.length(token::SPACE_1) + look.length(token::BORDER_WIDTH);
        let knob = look.length(token::SPACE_4);
        let ink = if off { look.color(token::FG_SUBTLE) } else { look.color(token::ACCENT) };
        let slide_to = move |bounds: Option<Bounds<Pixels>>, x: Pixels| -> Option<i64> {
            let bounds = bounds?;
            let share = ((x - bounds.origin.x) / bounds.size.width).clamp(0., 1.);
            Some(min + ((max - min) as f32 * share).round() as i64)
        };
        let marks = (steps <= MARKED_STEPS).then(|| {
            div().absolute().left_0().right_0().top(knob / 2. + rail).flex().justify_between().children((0..steps).map(|step| {
                div().w(look.length(token::BORDER_WIDTH)).h(look.length(token::SPACE_1)).bg(if min + step <= at { ink } else { look.color(token::BORDER_DEFAULT) })
            }))
        });
        let element = div()
            .id(SharedString::from(format!("slider-{key}")))
            .track_focus(&stops.slider)
            .relative()
            .flex_1()
            .min_w_0()
            .h(knob + look.length(token::SPACE_3))
            .when(!off, |slider| slider.cursor_pointer())
            .child(canvas({
                let tracks = tracks.clone();
                let key = key.clone();
                move |bounds, _, _| {
                    tracks.borrow_mut().insert(key.clone(), bounds);
                }
            }, |_, _, _, _| {}).absolute().left_0().top_0().size_full())
            .child(div().absolute().left_0().right_0().top(knob / 2. - rail / 2.).h(rail).rounded(rail).bg(look.color(token::BG_SUNKEN)))
            .child(div().absolute().left_0().top(knob / 2. - rail / 2.).h(rail).rounded(rail).w(gpui::relative(fill)).bg(ink))
            .children(marks)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left(gpui::relative(fill))
                    .ml(-knob / 2.)
                    .size(knob)
                    .rounded_full()
                    .border(look.length(token::BORDER_WIDTH) * 2.)
                    .border_color(ink)
                    .bg(look.color(token::BG_BASE))
                    .when(stops.slider.is_focused(window), |knob| knob.border_color(ring_colour(look))),
            )
            .when(!off, |slider| {
                let (down_key, move_key, down_tracks, move_tracks) = (key.clone(), key.clone(), tracks.clone(), tracks.clone());
                slider
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                        let bounds = down_tracks.borrow().get(&down_key).copied();
                        if let Some(to) = slide_to(bounds, event.position.x) {
                            this.slide(&down_key, to, window, cx);
                        }
                        if let Some(stops) = this.stops.get(&down_key) {
                            window.focus(&stops.slider, cx);
                        }
                    }))
                    .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, window, cx| {
                        if event.pressed_button != Some(MouseButton::Left) {
                            return;
                        }
                        let bounds = move_tracks.borrow().get(&move_key).copied();
                        if let Some(to) = slide_to(bounds, event.position.x) {
                            this.slide(&move_key, to, window, cx);
                        }
                    }))
            });
        labelled(element, Role::Slider, format!("{} slider", one.title)).into_any_element()
    }

    // ---- a page -------------------------------------------------------------------------------

    /// settings.json's path and its open press, drawn in the strip of a page's first schema card.
    fn file_strip(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let path = self.list.as_ref().map_or_else(|| "settings.json".to_string(), |list| list.path.clone());
        let off = self.list.is_none();
        let press = ringed(
            look,
            div()
                .id("settings-open-file")
                .track_focus(&self.file_stop)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .when(!off, |press| press.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
                .tooltip(tip("Open settings.json"))
                .child(svg().path(icon_path("file-text")).size(look.length(token::SPACE_4)).text_color(if off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_MUTED) }))
                .when(!off, |press| {
                    press.on_click(cx.listener(|this, _, _, cx| this.open_file(cx))).on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.open_file(cx);
                        }
                    }))
                }),
            &self.file_stop,
            window,
        );
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .child(div().min_w_0().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(path)))
            .child(labelled(press, Role::Button, "Open settings.json"))
            .into_any_element()
    }

    /// What sits at the top of the first card: Fleet's refusal of the file, or why it did not open.
    fn file_note(&self, look: &Look) -> Vec<AnyElement> {
        let mut notes = Vec::new();
        if let Some(refused) = self.list.as_ref().and_then(|list| list.refused.as_ref()) {
            let said = format!("{} The last settings that read cleanly stay in force until the file is fixed.", refused.reason);
            notes.push(
                labelled(
                    div()
                        .id("settings-file-refused")
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .p(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_MD))
                        .bg(look.color(token::NOTICE_CAUTION_BG))
                        .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::NOTICE_CAUTION)).child("Fleet refused settings.json"))
                        .children(refused.key.clone().map(|key| small(look, key, look.color(token::FG_DEFAULT)).font_family(mono(look))))
                        .child(small(look, said, look.color(token::FG_DEFAULT))),
                    Role::Alert,
                    "Fleet refused settings.json",
                )
                .into_any_element(),
            );
        }
        if let Some(said) = &self.file_said {
            notes.push(labelled(small(look, said.clone(), look.color(token::FG_MUTED)).id("settings-file-said"), Role::Status, said.clone()).into_any_element());
        }
        notes
    }

    /// One settings.json section as a card, or bones in its place while Fleet has not answered.
    fn schema_card(&self, id: &str, first: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let label = entries::SECTIONS.iter().find(|one| one.id == id).map_or(id, |one| one.label);
        let strip = first.then(|| self.file_strip(look, window, cx));
        let mut body: Vec<AnyElement> = Vec::new();
        if first {
            body.extend(self.file_note(look));
        }
        match &self.list {
            None => {
                for n in 0..3 {
                    body.push(
                        labelled(
                            div()
                                .id(SharedString::from(format!("settings-bone-{n}")))
                                .flex()
                                .flex_col()
                                .gap(look.length(token::SPACE_2))
                                .child(div().flex().justify_between().child(bone(look, px(160.), look.length(token::LEADING_SM))).child(bone(look, px(120.), look.length(token::LEADING_XS))))
                                .child(bone(look, px(360.), look.length(token::LEADING_XS)))
                                .child(bone(look, px(480.), look.length(token::H_CONTROL))),
                            Role::Image,
                            "Loading",
                        )
                        .into_any_element(),
                    );
                }
            }
            Some(list) => {
                if !self.live {
                    body.push(labelled(small(look, NOT_LIVE, look.color(token::FG_MUTED)).id("settings-not-live"), Role::Status, NOT_LIVE).into_any_element());
                }
                for (n, one) in entries::settings_in(list, id).into_iter().enumerate() {
                    let field = self.render_field(one, look, window, cx);
                    body.push(
                        div()
                            .flex()
                            .flex_col()
                            .when(n > 0, |row| row.pt(look.length(token::SPACE_4)))
                            .child(field)
                            .into_any_element(),
                    );
                }
            }
        }
        labelled(card(look, SharedString::from(format!("settings-card-{id}")), label, strip, body), Role::Region, label).into_any_element()
    }

    /// The folders a Job's card always allows or blocks, each with Remove (`ReachFoldersSettings`).
    fn reach_card(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let one = self.setting(REACH_FOLDERS)?;
        let (allow, block) = reach_folders(Some(&one.value));
        let off = !self.live || self.saving.is_some();
        let mut body: Vec<AnyElement> = Vec::new();
        let rows: Vec<(String, bool)> = block.into_iter().map(|folder| (folder, false)).chain(allow.into_iter().map(|folder| (folder, true))).collect();
        if rows.is_empty() {
            let said = "No folders. Always allow or Always block on a job's card adds one.";
            body.push(small(look, said, look.color(token::FG_MUTED)).into_any_element());
        }
        let mut list = div().flex().flex_col().gap(look.length(token::SPACE_2));
        for (folder, allowed) in rows {
            let said = if allowed { "Always allowed" } else { "Always blocked" };
            let glyph = if allowed { "folder" } else { "ban" };
            let handle = self.panes.get(&format!("reach:{folder}")).cloned();
            let press_folder = folder.clone();
            let remove = ghost(look, SharedString::from(format!("remove-{folder}")), "Remove", off)
                .border_color(look.color(token::BORDER_DEFAULT))
                .when_some(handle.clone(), |button, handle| ringed(look, button.track_focus(&handle), &handle, window))
                .when(!off, |button| {
                    let key_folder = folder.clone();
                    button.on_click(cx.listener(move |this, _, window, cx| this.remove_folder(&press_folder, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.remove_folder(&key_folder, window, cx);
                        }
                    }))
                });
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .child(labelled(div().id(SharedString::from(format!("mark-{folder}"))).tooltip(tip(said)).child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED))), Role::Image, said))
                    .child(div().flex_1().min_w_0().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(folder.clone())))
                    .child(labelled(remove, Role::Button, format!("Remove {folder}"))),
            );
        }
        body.push(labelled(list.id("reach-folders"), Role::List, "Folders jobs reach").into_any_element());
        if let Some(said) = self.refused.get(REACH_FOLDERS) {
            body.push(labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id("reach-refused"), Role::Alert, said.clone()).into_any_element());
        }
        Some(labelled(card(look, "settings-card-reach", "Folders jobs reach", None, body), Role::Region, "Folders jobs reach").into_any_element())
    }

    /// One switch per repository for the connectors on his account (`RepositorySwitches`).
    fn connectors_card(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let one = self.setting(CONNECTORS_OFF)?;
        let set = connectors_set(Some(&one.value));
        let repositories = self.wire.repositories(cx).unwrap_or_default();
        let off = !self.live || self.saving.is_some();
        let help = "Off, Jobs in that repository start without the connectors on your account, such as Notion, Drive and Slack. Servers in the repository's own .mcp.json are kept, and sessions started outside Armada are unaffected. A switch you have not set follows the repository's armada.yml.";
        let mut body: Vec<AnyElement> = vec![small(look, help, look.color(token::FG_MUTED)).into_any_element()];
        for (id, name, yml_off) in repositories {
            let on = !set.get(&id).copied().unwrap_or(yml_off);
            let handle = self.panes.get(&format!("connectors:{id}")).cloned();
            let said = format!("Connectors in {name}");
            let (press_id, key_id) = (id.clone(), id.clone());
            let row = div()
                .id(SharedString::from(format!("connectors-{id}")))
                .flex()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(name.clone())))
                .child(switch_track(look, on, off))
                .when_some(handle, |row, handle| ringed(look, row.track_focus(&handle), &handle, window))
                .when(!off, |row| {
                    row.cursor_pointer()
                        .on_click(cx.listener(move |this, _, window, cx| this.switch_connectors(&press_id, yml_off, !on, window, cx)))
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.switch_connectors(&key_id, yml_off, !on, window, cx);
                            }
                        }))
                });
            body.push(labelled(row, Role::Switch, said).into_any_element());
        }
        if let Some(said) = self.refused.get(CONNECTORS_OFF) {
            body.push(labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id("connectors-refused"), Role::Alert, said.clone()).into_any_element());
        }
        Some(labelled(card(look, "settings-card-connectors", "Connectors", None, body), Role::Region, "Connectors").into_any_element())
    }

    fn render_page(&self, shown: Option<&'static Entry>, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        // Models is two columns side by side, each scrolling itself, as Classic's Kit pages are.
        if shown.is_some_and(|one| one.body == Body::Models) {
            return self.render_models(look, window, cx);
        }
        if shown.is_some_and(|one| one.body == Body::Workflows) {
            return self.render_workflows(look, window, cx);
        }
        let mut cards: Vec<AnyElement> = Vec::new();
        match shown.map(|one| one.body) {
            Some(Body::Schema(ids)) => {
                for (n, id) in ids.iter().enumerate() {
                    cards.push(self.schema_card(id, n == 0, look, window, cx));
                }
            }
            Some(Body::Features) => {
                cards.push(self.schema_card("features", true, look, window, cx));
                cards.extend(self.reach_card(look, window, cx));
            }
            Some(Body::Repository) => cards.extend(self.connectors_card(look, window, cx)),
            Some(Body::ProjectValues) => cards.extend(super::project::render(self, look, window, cx)),
            Some(Body::Keyboard) => cards.push(self.render_keyboard(look, window, cx)),
            Some(Body::Theme) => {
                cards.push(self.render_theme(look, window, cx));
                cards.extend(self.render_mods(super::mods::Page::Theme, look, window, cx));
            }
            Some(Body::Models | Body::Workflows) => {}
            Some(body @ Body::Mods(_)) => cards.extend(super::mods::page_of(body).and_then(|page| self.render_mods(page, look, window, cx))),
            None => {}
        }
        div()
            .id("settings-page")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::GAP_PANEL))
            .children(cards)
            .into_any_element()
    }

    pub fn render_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if !self.open {
            return div().into_any_element();
        }
        // A mod Fleet lists after Settings opened gets its keys before it is drawn, and a Fleet mod's file is read.
        self.mod_keys(cx);
        self.read_mods(cx);
        if self.shown(cx).is_some_and(|one| one.body == Body::Models) {
            self.read_models(cx);
            self.model_keys(cx);
        }
        if self.shown(cx).is_some_and(|one| one.body == Body::Workflows) {
            self.workflow_keys(cx);
        }
        let look = cx.look();
        let room = crate::room::of(window, cx);
        let visible = self.visible(cx);
        let shown = self.shown(cx);
        let side = self.render_side(&visible, shown, look, window, cx);
        let page = self.render_page(shown, look, window, cx);
        let popup = self.popup.as_ref().map(|(menu, _)| menu.clone());
        let boxed = labelled(
            div()
                .id("settings-box")
                .occlude()
                .w(room.size.width * 0.9)
                .h(room.size.height * 0.9)
                .flex()
                .gap(look.length(token::GAP_PANEL))
                .p(look.length(token::GAP_PANEL))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_HIGHLIGHT))
                .bg(look.color(token::BG_BASE))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(side)
                .child(page)
                .children(self.mods.panel.as_ref().map(|opened| {
                    div()
                        .flex_none()
                        .w(room.size.width * 0.9 * 0.36)
                        .h_full()
                        .rounded(look.length(token::RADIUS_LG))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .overflow_hidden()
                        .child(opened.view.clone())
                })),
            Role::Dialog,
            "Settings",
        );
        deferred(
            anchored().position(room.origin).anchor(gpui::Anchor::TopLeft).child(
                div()
                    .id("settings-layer")
                    .track_focus(&self.focus)
                    .key_context("settings")
                    .w(room.size.width)
                    .h(room.size.height)
                    .flex()
                    .justify_center()
                    .items_center()
                    .bg(look.color(token::SCRIM))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close(window, cx)))
                    .child(boxed)
                    .children(popup),
            ),
        )
        .with_priority(2)
        .into_any_element()
    }
}
