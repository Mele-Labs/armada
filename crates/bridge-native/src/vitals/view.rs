//! The Vitals page drawn from what `model.rs` made: the head band with the gate's chip and the span switch, then the tiles two
//! across. A tile is a card: its header band in its state's hue with the reading, its graph and keys, and a foot of the settings
//! that limit it in fixed columns. The foot's controls are Settings › Limits' own (`settings/limits.rs`). Every value is a token.

use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, Div, FontWeight, Hsla, IntoElement, KeyDownEvent, Pixels, Render, Role, SharedString, Window};
use ipc::{SettingApplies, SettingValue};

use super::model::{Control, Face, Graph, Mark, Span, Tile, WINDOWS};
use super::VitalsScreen;
use crate::generated::actions::ActionScope;
use crate::graph::plot::{GraphPlot, Inks};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::reviews::{card_inner_radius, panel};
use crate::settings::rows::{default_words, segment, segment_track, Pick};
use crate::settings::view::{bone, ghost, refusal, ring_colour, ringed, tip};
use crate::titlebar::parts::{glyph, mono, sans};

/// The width a foot's control column takes, so the controls line up down every foot.
fn control_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 3.5
}

/// The width of a foot's last column: what ships, and Reset where the value is changed.
fn ships_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 3.
}

/// The change log's columns: who, when, and the change.
fn who_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 2.
}

fn when_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 1.75
}

fn moved_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 2.25
}

fn undo_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 1.25
}

fn press_keys(event: &KeyDownEvent) -> bool {
    matches!(event.keystroke.key.as_str(), "enter" | "space")
}

/// A key's mark in a legend: a short line in the series' ink, or a dashed one for a limit.
fn key_mark(look: &Look, ink: Hsla, dashed: bool) -> Div {
    let tall = look.length(token::BORDER_WIDTH) * 2.;
    if dashed {
        let dash = || div().flex_none().w(look.length(token::SPACE_1)).h(tall).bg(ink);
        div().flex_none().flex().gap(look.length(token::SPACE_1) / 2.).child(dash()).child(dash()).child(dash())
    } else {
        div().flex_none().w(look.length(token::SPACE_4)).h(tall).rounded(tall).bg(ink)
    }
}

/// A time in this Mac's clock: `04:12`, with the day where it is not the reading's last.
fn when(at: i64, last: i64) -> String {
    use chrono::TimeZone;
    let local = |ms: i64| chrono::Local.timestamp_millis_opt(ms).single();
    match (local(at), local(last)) {
        (Some(at), Some(last)) if at.date_naive() == last.date_naive() => at.format("%H:%M").to_string(),
        (Some(at), _) => at.format("%-d %b %H:%M").to_string(),
        _ => String::new(),
    }
}

/// A value as the log writes it, with its unit.
fn said_value(value: &SettingValue, kind: &ipc::SettingKind) -> String {
    let n = match value {
        SettingValue::Integer(n) => *n,
        other => return crate::settings::overlay::held_text(other),
    };
    match kind {
        ipc::SettingKind::Seconds { .. } => format!("{n} s"),
        ipc::SettingKind::Integer { unit: Some(unit), .. } => {
            let unit = crate::settings::limits::unit_shown(unit);
            if unit.is_empty() || unit.chars().next().is_some_and(char::is_uppercase) {
                n.to_string()
            } else if unit == "%" {
                format!("{n}%")
            } else {
                format!("{n} {unit}")
            }
        }
        _ => n.to_string(),
    }
}

impl VitalsScreen {
    /// The band over the tiles: the page's glyph and name, what the gate does now, and the span switch.
    fn render_head(&self, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let title = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .child(glyph("laptop", look.length(token::SPACE_4) + look.length(token::SPACE_1), look.color(token::FG_MUTED)))
            .child(labelled(div().id("vitals-title").text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Vitals"), Role::Heading, "Vitals"));
        let chip = match &self.reading {
            None => bone(look, look.length(token::SPACE_12) * 3., look.length(token::LEADING_XS)).into_any_element(),
            Some(_) => {
                let (said, glyph_name, hue, wash) = match &self.short {
                    Some(short) => (short.said(), "pause", token::STATUS_COMPLETED_FAILED, token::STATUS_COMPLETED_FAILED_BG),
                    None => ("Clear".to_string(), "check", token::STATUS_COMPLETED_SUCCESS, token::STATUS_COMPLETED_SUCCESS_BG),
                };
                let ink = look.color(hue);
                labelled(
                    div()
                        .id("vitals-gate")
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_1) + look.length(token::BORDER_WIDTH))
                        .px(look.length(token::SPACE_2) + look.length(token::BORDER_WIDTH))
                        .py(look.length(token::SPACE_1) / 2.)
                        .rounded_full()
                        .bg(look.color(wash))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(ink)
                        .tooltip(tip("What the gate does with the next Check"))
                        .child(glyph(glyph_name, look.length(token::SPACE_3), ink))
                        .child(SharedString::from(said.clone())),
                    Role::Status,
                    said,
                )
                .into_any_element()
            }
        };
        let mut track = segment_track(look, "vitals-span");
        for span in Span::ALL {
            let on = span == self.span;
            let one = segment(look, SharedString::from(format!("vitals-span-{}", span.label())), span.label().into(), if on { Pick::Set } else { Pick::No }, false)
                .aria_toggled(if on { gpui::Toggled::True } else { gpui::Toggled::False })
                .tooltip(tip(span.said()))
                .on_click(cx.listener(move |this, _, _, cx| this.pick(span, cx)));
            track = track.child(labelled(one, Role::RadioButton, span.said()));
        }
        let band = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_4))
            .min_h(look.length(token::H_ROW_UNIT) + look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .bg(look.color(token::SURFACE_WELL))
            .child(title)
            .child(chip)
            .child(div().flex_1())
            .child(labelled(track, Role::RadioGroup, "Span"));
        panel(look).id("vitals-band").flex_none().child(band).into_any_element()
    }

    /// A tile's header band: its glyph in its state's hue, its name, and the reading at the right, big, with the sub-reading under it.
    fn render_header(&self, face: &Face, look: &Look) -> Div {
        let tile = face.tile;
        let (hue, wash) = match face.state {
            Some(state) => (look.color(state.hue()), look.color(state.wash())),
            None => (look.color(token::FG_MUTED), look.color(token::SURFACE_WELL)),
        };
        let said = face.state.map_or_else(|| tile.title().to_string(), |state| format!("{}: {}", tile.title(), state.said()));
        let mark = labelled(
            div()
                .id(SharedString::from(format!("vitals-state-{}", tile.id())))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::SPACE_6) + look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .bg(hue.opacity(0.18))
                .tooltip(tip(face.state.map_or("No limit reads it", |state| state.said())))
                .child(glyph(tile.glyph(), look.length(token::SPACE_4), hue)),
            Role::Image,
            said,
        );
        let readings = (!face.big.is_empty()).then(|| {
            div()
                .flex_none()
                .flex()
                .flex_col()
                .items_end()
                .child(labelled(
                    div()
                        .id(SharedString::from(format!("vitals-big-{}", tile.id())))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_XL))
                        .line_height(look.length(token::LEADING_XL))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(SharedString::from(face.big.clone())),
                    Role::Label,
                    format!("{} now: {}", tile.title(), face.big),
                ))
                .child(div().max_w(look.length(token::SPACE_12) * 6.).truncate().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(face.sub.clone())))
        });
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_h(look.length(token::SPACE_12) + look.length(token::SPACE_4))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .rounded_t(card_inner_radius(look))
            .bg(wash)
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(mark)
            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(tile.title()))
            .children(readings)
    }

    /// One graph and its keys. A limit only typed is drawn in the accent, the line and its key both.
    fn render_graph(&self, graph: &Graph, look: &Look) -> Div {
        let mut inks = Inks::of(look);
        inks.rule = if graph.unsaved { look.color(token::ACCENT) } else { look.color(token::FG_DEFAULT) };
        let plot = GraphPlot::new(SharedString::from(format!("vitals-plot-{}", graph.id)), Rc::clone(&graph.chart), inks.clone());
        let drawing = labelled(div().id(SharedString::from(format!("vitals-chart-{}", graph.id))).w_full().h(look.length(token::SPACE_12) * graph.tall).child(plot), Role::Image, format!("Chart: {}", graph.name));
        let keys = graph.legend.iter().map(|mark| match mark {
            Mark::Series { name, slot, note } => div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(key_mark(look, inks.of_series(*slot), false))
                .child(SharedString::from(name.clone()))
                .children(note.map(|said| div().px(look.length(token::SPACE_1) + look.length(token::BORDER_WIDTH)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::BG_OVERLAY)).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_DEFAULT)).child(said)))
                .into_any_element(),
            Mark::Rule { name, unsaved } => {
                let ink = if *unsaved { look.color(token::ACCENT) } else { look.color(token::FG_DEFAULT) };
                div().flex().items_center().gap(look.length(token::SPACE_2)).child(key_mark(look, ink, true)).child(div().when(*unsaved, |said| said.text_color(ink)).child(SharedString::from(if *unsaved { format!("{name}, not saved") } else { name.clone() }))).into_any_element()
            }
        });
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .child(drawing)
            .child(div().flex().flex_wrap().items_center().gap_x(look.length(token::SPACE_4)).gap_y(look.length(token::SPACE_1)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).children(keys))
    }

    /// The controls of every setting a foot draws, made by the Settings overlay that holds them, by key.
    fn controls(&self, window: &Window, cx: &mut gpui::Context<Self>) -> BTreeMap<String, (AnyElement, Option<AnyElement>)> {
        let look = cx.look().clone();
        let keys: Vec<&'static str> = Tile::ALL.iter().flat_map(|tile| tile.keys().iter().copied()).filter(|key| super::model::control_of(key) != Control::Windows).collect();
        self.limits.update(cx, |limits, cx| {
            let mut out = BTreeMap::new();
            for key in keys {
                let Some(one) = limits.setting(key).cloned() else { continue };
                let saving = limits.saving.as_deref() == Some(key);
                let off = limits.saving.is_some();
                let lit = crate::settings::limits::changed(&one);
                let control = match super::model::control_of(key) {
                    Control::Stepper => limits.limits_stepper(&one, lit, off, &look, window, cx),
                    _ => limits.number_field(&one, lit, off, saving, control_width(&look), false, &look, window, cx),
                };
                let reset = (one.saved.is_some() && one.saved.as_ref() != Some(&one.default) || lit).then(|| limits.limits_reset(&one, off, &look, window, cx));
                out.insert(key.to_string(), (control, reset));
            }
            out
        })
    }

    /// The load gate's windows: one segment a window, the stop the keys reach by Tab; the side arrows and + and − move it.
    fn render_windows(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let key = "limits.loadWindow";
        let store = self.limits.read(cx);
        let (Some(one), Some(stops)) = (store.setting(key), store.stops.get(key)) else { return div().into_any_element() };
        let now = match crate::settings::limits::current(one) {
            SettingValue::Integer(n) => *n,
            _ => 1,
        };
        let off = store.saving.is_some();
        let mut track = segment_track(look, "vitals-windows").track_focus(&stops.control);
        for minutes in WINDOWS {
            let on = minutes == now;
            let said = format!("{minutes} min");
            let one = segment(look, SharedString::from(format!("vitals-window-{minutes}")), said.clone().into(), if on { Pick::Set } else { Pick::No }, off)
                .aria_toggled(if on { gpui::Toggled::True } else { gpui::Toggled::False })
                .when(!off && !on, |one| one.on_click(cx.listener(move |this, _, window, cx| this.save(key, Some(SettingValue::Integer(minutes)), window, cx))));
            track = track.child(labelled(one, Role::RadioButton, said));
        }
        labelled(ringed(look, track, &stops.control, window), Role::RadioGroup, one.title.clone()).into_any_element()
    }

    /// A foot's row: the setting's name, its control, and what ships with Reset beside it where it is changed. Notes go under it.
    fn render_setting(&self, key: &str, controls: &mut BTreeMap<String, (AnyElement, Option<AnyElement>)>, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> Option<AnyElement> {
        let store = self.limits.read(cx);
        let one = store.setting(key)?.clone();
        let refused = store.refused.get(key).cloned();
        let typed = store.dirty(key, cx);
        let (control, reset) = match super::model::control_of(key) {
            Control::Windows => (self.render_windows(look, window, cx), None),
            _ => controls.remove(key)?,
        };
        let name = div()
            .id(SharedString::from(format!("vitals-setting-{key}")))
            .flex_1()
            .min_w_0()
            .truncate()
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_DEFAULT))
            .tooltip(tip(one.description.clone()))
            .child(SharedString::from(one.title.clone()));
        let ships = div()
            .flex_none()
            .w(ships_width(look))
            .flex()
            .items_center()
            .justify_end()
            .gap(look.length(token::SPACE_2))
            .child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(format!("Ships {}", default_words(&one)))))
            .children(reset);
        let now = crate::settings::limits::current(&one).clone();
        let off_note = match (key, &now) {
            ("limits.loadLimit", SettingValue::Integer(0)) => Some("Off: no Check waits on load"),
            ("limits.memoryPressureLevel", SettingValue::Integer(0)) => Some("Off: never short for memory"),
            ("limits.checkStartSpacingSeconds", SettingValue::Integer(0)) => Some("Off: Checks start together"),
            _ => None,
        };
        let mut notes: Vec<AnyElement> = Vec::new();
        if let Some(said) = off_note {
            notes.push(labelled(div().id(SharedString::from(format!("vitals-off-{key}"))).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_MUTED)).child(said), Role::Status, said).into_any_element());
        }
        if typed {
            let said = "Not saved: Enter saves it";
            notes.push(labelled(div().id(SharedString::from(format!("vitals-typed-{key}"))).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::ACCENT)).child(said), Role::Status, said).into_any_element());
        }
        if one.applies == SettingApplies::AtRestart && one.pending_restart {
            let said = "Restart Fleet to apply";
            notes.push(labelled(div().id(SharedString::from(format!("vitals-restart-{key}"))).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::STATUS_AWAITING_REVIEW)).child(said), Role::Status, said).into_any_element());
        }
        if let Some(said) = refused {
            notes.push(labelled(refusal(look, said.clone()).id(SharedString::from(format!("vitals-refused-{key}"))), Role::Alert, said).into_any_element());
        }
        let line = div().flex().items_center().gap(look.length(token::SPACE_3)).min_h(look.length(token::H_CONTROL)).child(name).child(div().flex_none().w(control_width(look)).flex().child(control)).child(ships);
        Some(labelled(div().id(SharedString::from(format!("vitals-row-{key}"))).flex().flex_col().gap(look.length(token::SPACE_1)).child(line).children(notes), Role::Row, one.title.clone()).into_any_element())
    }

    /// A tile's foot: the settings that limit its reading. None where nothing does.
    fn render_foot(&self, tile: Tile, controls: &mut BTreeMap<String, (AnyElement, Option<AnyElement>)>, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> Option<Div> {
        if tile.keys().is_empty() {
            return None;
        }
        let rows: Vec<AnyElement> = tile.keys().iter().filter_map(|key| self.render_setting(key, controls, look, window, cx)).collect();
        Some(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_3))
                .when(tile != Tile::Unread, |foot| foot.mt_auto())
                .rounded_b(card_inner_radius(look))
                .bg(look.color(token::BG_OVERLAY))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .children(rows),
        )
    }

    /// The change log: who changed which limit, when, from what to what, and Undo, in fixed columns.
    fn render_changes(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> Div {
        let last = self.reading.as_ref().and_then(|reading| reading.samples.last()).map_or(0, |one| one.at);
        let store = self.limits.read(cx);
        let head = |name: &'static str, cell: Div| labelled(cell.id(SharedString::from(format!("vitals-change-column-{name}"))).text_color(look.color(token::FG_SUBTLE)).child(name), Role::ColumnHeader, name);
        let heads = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_ROW_HEADER))
            .px(look.length(token::SPACE_4))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .child(head("Who", div().flex_none().w(who_width(look))))
            .child(head("Limit", div().flex_1().min_w_0()))
            .child(head("When", div().flex_none().w(when_width(look))))
            .child(head("Change", div().flex_none().w(moved_width(look))))
            .child(div().flex_none().w(undo_width(look)));
        let rows: Vec<AnyElement> = self
            .changes
            .iter()
            .filter_map(|change| {
                let one = store.setting(&change.key)?;
                let stop = self.undo_stops.get(&change.id)?;
                let moved = format!("{} → {}", said_value(&change.from, &one.kind), said_value(&change.to, &one.kind));
                let id = change.id;
                let undo = ringed(look, ghost(look, SharedString::from(format!("vitals-undo-{id}")), "Undo", false), stop, window).track_focus(stop).on_click(cx.listener(move |this, _, window, cx| this.undo(Some(id), window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if press_keys(event) {
                        this.undo(Some(id), window, cx);
                        cx.stop_propagation();
                    }
                }));
                let undo = labelled(undo.tooltip(tip(format!("Set {} back to {}", one.title, said_value(&change.from, &one.kind)))), Role::Button, format!("Undo {}", one.title));
                let job = change.who != super::model::YOU;
                let line = div()
                    .id(SharedString::from(format!("vitals-change-{id}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .min_h(look.length(token::H_ROW_UNIT))
                    .px(look.length(token::SPACE_4))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .child(div().flex_none().w(who_width(look)).truncate().text_color(look.color(if job { token::FG_DEFAULT } else { token::FG_MUTED })).child(SharedString::from(change.who.clone())))
                    .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(one.title.clone())))
                    .child(div().flex_none().w(when_width(look)).font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(when(change.at, last))))
                    .child(div().flex_none().w(moved_width(look)).truncate().font_family(mono(look)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(moved.clone())))
                    .child(div().flex_none().w(undo_width(look)).flex().justify_end().child(undo));
                Some(labelled(line, Role::Row, format!("{}, {}, {}", change.who, one.title, moved)).into_any_element())
            })
            .collect();
        div().flex().flex_col().pb(look.length(token::SPACE_2)).child(heads).children(rows)
    }

    /// The bones a tile draws while the reading has not come: the header's, a graph's block, and the foot's rows.
    fn render_bones(&self, tile: Tile, look: &Look) -> Vec<AnyElement> {
        let line = look.length(token::LEADING_XS) - px(4.);
        let header = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_h(look.length(token::SPACE_12) + look.length(token::SPACE_4))
            .px(look.length(token::SPACE_4))
            .rounded_t(card_inner_radius(look))
            .bg(look.color(token::SURFACE_WELL))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(bone(look, look.length(token::SPACE_6) + look.length(token::SPACE_1), look.length(token::SPACE_6) + look.length(token::SPACE_1)))
            .child(div().flex_1().child(bone(look, look.length(token::SPACE_12) * 2., line)))
            .child(div().flex().flex_col().items_end().gap(look.length(token::SPACE_1)).child(bone(look, look.length(token::SPACE_12) * 1.5, look.length(token::LEADING_XL) - px(6.))).child(bone(look, look.length(token::SPACE_12) * 2.5, line)));
        let body = div().p(look.length(token::SPACE_4)).child(bone(look, px(0.), look.length(token::SPACE_12) * 3.5).w_full());
        let foot = (!tile.keys().is_empty()).then(|| {
            div()
                .mt_auto()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_3))
                .bg(look.color(token::BG_OVERLAY))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .children(tile.keys().iter().map(|_| div().flex().items_center().gap(look.length(token::SPACE_3)).h(look.length(token::H_CONTROL)).child(div().flex_1().child(bone(look, look.length(token::SPACE_12) * 2.5, line))).child(bone(look, control_width(look), look.length(token::H_CONTROL) - px(4.))).child(div().w(ships_width(look)).flex().justify_end().child(bone(look, look.length(token::SPACE_12) * 1.5, line)))))
        });
        let mut parts = vec![header.into_any_element()];
        if !matches!(tile, Tile::Unread) {
            parts.push(body.into_any_element());
        }
        parts.extend(foot.map(IntoElement::into_any_element));
        parts
    }

    fn render_tile(&self, at: usize, controls: &mut BTreeMap<String, (AnyElement, Option<AnyElement>)>, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let tile = Tile::ALL[at];
        let stop = &self.tiles[at];
        let edge = look.length(token::BORDER_WIDTH);
        let parts: Vec<AnyElement> = match self.faces.get(at).filter(|_| self.reading.is_some()) {
            None => vec![labelled(div().id(SharedString::from(format!("vitals-loading-{}", tile.id()))).flex_1().flex().flex_col().children(self.render_bones(tile, look)), Role::Image, "Loading").into_any_element()],
            Some(face) => {
                let mut parts = vec![self.render_header(face, look).into_any_element()];
                match tile {
                    Tile::Changes => parts.push(self.render_changes(look, window, cx).into_any_element()),
                    Tile::Unread => {}
                    _ => parts.push(div().flex().flex_col().gap(look.length(token::SPACE_4)).p(look.length(token::SPACE_4)).children(face.graphs.iter().map(|graph| self.render_graph(graph, look))).into_any_element()),
                }
                parts.extend(self.render_foot(tile, controls, look, window, cx).map(IntoElement::into_any_element));
                parts
            }
        };
        let focused = stop.is_focused(window);
        let card = div()
            .id(SharedString::from(format!("vitals-tile-{}", tile.id())))
            .track_focus(stop)
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .rounded(look.length(token::RADIUS_LG))
            .border(edge)
            .border_color(if focused { ring_colour(look) } else { look.color(token::BORDER_DEFAULT) })
            .when(focused, |card| card.border(look.length(token::FOCUS_RING_WIDTH)))
            .bg(look.color(token::BG_RAISED))
            .shadow(look.shadows(token::SHADOW_CARD))
            .on_click(cx.listener(move |this, _, window, cx| {
                if !this.tiles[at].contains_focused(window, cx) {
                    this.focus_tile(at, window, cx);
                }
            }))
            .children(parts);
        labelled(card, Role::Group, tile.title()).into_any_element()
    }
}

impl Render for VitalsScreen {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let mut controls = if self.reading.is_some() { self.controls(window, cx) } else { BTreeMap::new() };
        let gap = look.length(token::GAP_PANEL);
        let mut rows: Vec<AnyElement> = Vec::new();
        for pair in (0..Tile::ALL.len()).collect::<Vec<_>>().chunks(2) {
            let tiles: Vec<AnyElement> = pair.iter().map(|at| self.render_tile(*at, &mut controls, &look, window, cx)).collect();
            rows.push(div().flex().gap(gap).children(tiles).into_any_element());
        }
        let tiles = div().id("vitals-scroll").flex_1().min_h_0().flex().flex_col().gap(gap).overflow_y_scroll().track_scroll(&self.scroll).children(rows);
        labelled(
            div()
                .id("vitals")
                .key_context(ActionScope::Vitals.key_context().unwrap_or_default())
                .track_focus(&self.within)
                .on_key_down(cx.listener(Self::on_key))
                .on_action(cx.listener(Self::on_act))
                .size_full()
                .flex()
                .flex_col()
                .gap(gap)
                .p(look.length(token::SPACE_3))
                .bg(look.color(token::BG_BASE))
                .font_family(sans(&look))
                .text_color(look.color(token::FG_DEFAULT))
                .child(self.render_head(&look, cx))
                .child(tiles),
            Role::Region,
            "Vitals",
        )
    }
}
