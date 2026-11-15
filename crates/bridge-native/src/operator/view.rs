//! The page drawn, as the approved sketch "Operator page: one card, Discuss in the composer" lays it out: the
//! head with what the Operator does unasked, the one card in front with the box directly under it, and
//! Next up and the feed Since you last looked in the column at the right. Roomier than the sketch: controls a step taller,
//! corners rounder, as the native screens are.

use gpui::{canvas, div, prelude::*, AnyElement, Context, Div, FontWeight, Hsla, MouseButton, Pixels, Role, SharedString, Stateful, Window};
use ipc::MainCiState;

use super::face::{self, Kind};
use super::page::OperatorPageView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::titlebar::parts::{glyph, kbd, sans, spinner, tip};

/// The card's corners: rounder than a control's, as the native screens draw cards.
pub(super) fn round(look: &Look) -> Pixels {
    look.length(token::RADIUS_LG) * 1.5
}

pub(crate) fn chip(word: impl Into<SharedString>, hue: Hsla, ground: Hsla, look: &Look) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded_full()
        .bg(ground)
        .text_color(hue)
        .text_size(look.length(token::TEXT_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .child(word.into())
}

pub(crate) fn kind_chip(kind: Kind, look: &Look) -> Div {
    chip(kind.word, look.color(kind.hue), look.color(kind.ground), look)
}

/// A press on the page: the pick filled in the Operator's hue, the rest outlined, Later and Ask bare. `JobPick` is a
/// Job's own pick on one of its questions, filled in the accent that rings a Job's pick elsewhere.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Weight {
    Pick,
    JobPick,
    Plain,
    Bare,
}

pub(crate) fn press(id: impl Into<SharedString>, label: SharedString, icon: Option<&'static str>, key: Option<SharedString>, weight: Weight, look: &Look) -> Stateful<Div> {
    let id: SharedString = id.into();
    let (ground, hover, ink, edge) = match weight {
        Weight::Pick => (look.color(token::OPERATOR), look.color(token::OPERATOR), look.color(token::FG_INVERSE), look.color(token::OPERATOR)),
        Weight::JobPick => (look.color(token::ACCENT), look.color(token::ACCENT_HOVER), look.color(token::TEXT_ON_ACCENT), look.color(token::ACCENT)),
        Weight::Plain => (look.color(token::BG_RAISED), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), look.color(token::BORDER_STRONG)),
        Weight::Bare => (gpui::transparent_black(), look.color(token::BG_HOVER), look.color(token::FG_MUTED), gpui::transparent_black()),
    };
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    let cap = key.map(|key| {
        let cap = kbd(key, look);
        if matches!(weight, Weight::Pick | Weight::JobPick) { cap.bg(ink.opacity(0.16)).border_color(ink.opacity(0.5)).text_color(ink) } else { cap }
    });
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .h(look.length(token::H_CONTROL_LG))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(edge)
        .bg(ground)
        .text_color(ink)
        .text_size(look.length(token::TEXT_SM))
        .font_weight(FontWeight(look.number(if matches!(weight, Weight::Pick | Weight::JobPick) { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .focusable()
        .tab_stop(true)
        .focus_visible(move |style| style.border_color(ring))
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .children(icon.map(|icon| glyph(icon, look.length(token::SPACE_4), ink)))
        .child(label)
        .children(cap)
}

/// What a press does on a click, and on Enter or Space while it has the keys: the key goes no further,
/// so the page's own Enter (Yes) is not said as well.
pub(super) fn pressed(
    element: Stateful<Div>,
    cx: &Context<OperatorPageView>,
    act: impl Fn(&mut OperatorPageView, &mut Window, &mut Context<OperatorPageView>) + 'static,
) -> Stateful<Div> {
    let act = std::rc::Rc::new(act);
    let by_key = act.clone();
    element.on_click(cx.listener(move |this, _, window, cx| act(this, window, cx))).on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
        let key = &event.keystroke;
        if matches!(key.key.as_str(), "enter" | "space") && !key.modifiers.modified() {
            cx.stop_propagation();
            by_key(this, window, cx);
        }
    }))
}

/// A press drawn as words, such as a link or Show all: a tab stop whose ring sits on the words alone.
fn worded(element: Stateful<Div>, look: &Look) -> Stateful<Div> {
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    element
        .focusable()
        .tab_stop(true)
        .px(look.length(token::SPACE_1))
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(gpui::transparent_black())
        .focus_visible(move |style| style.border_color(ring))
        .cursor_pointer()
}

/// A grey shape where words will be, while the page is read.
fn bone(width: f32, height: Pixels, look: &Look) -> Div {
    div().h(height).w(gpui::relative(width)).rounded(look.length(token::RADIUS_LG)).bg(look.color(token::BG_HOVER))
}
/// A press that opens a menu or a list keeps where it was drawn, by name, so the menu opens under it.
pub(super) fn measured(drawn: &super::page::Bounded, name: &'static str) -> gpui::Canvas<()> {
    let drawn = drawn.clone();
    canvas(move |bounds, _, _| { drawn.borrow_mut().insert(name, bounds); }, |_, _, _, _| {}).absolute().size_full()
}

impl OperatorPageView {
    // -- the head ----------------------------------------------------------------------------------

    fn head(&self, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        // The first stop drawn is where Tab goes from the page itself.
        let first = std::cell::Cell::new(Some(self.first_stop.clone()));
        let pill = |id: &'static str| {
            div()
                .id(id)
                .when_some(first.take(), |pill, keys| pill.track_focus(&keys))
                // Tab reaches each pill, and its tooltip says what it reads.
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .rounded_full()
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .text_color(look.color(token::FG_MUTED))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        };
        let meter = |share: f32, fill: Token| {
            div()
                .w(look.length(token::SPACE_8) + look.length(token::SPACE_1))
                .h(look.length(token::SPACE_1) * 1.5)
                .rounded_full()
                .bg(look.color(token::BG_SUNKEN))
                .child(div().h_full().w(gpui::relative(share.clamp(0., 1.))).rounded_full().bg(look.color(fill)))
        };
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let (mains, slots_read) = self.page.as_ref().map_or((&[][..], None), |page| (&page.mains[..], page.slots.as_ref()));
        let mains = mains.iter().enumerate().map(|(at, main)| {
            let since = main.since.as_ref().and_then(|since| minute(since.as_str(), now));
            let (said, mark) = face::main_said(main, since.as_deref());
            let red = main.state == Some(MainCiState::Red);
            let pill = pill(if at == 0 { "pill-main-0" } else if at == 1 { "pill-main-1" } else { "pill-main" })
                .when(red, |pill| {
                    pill.border_color(look.color(token::STATUS_COMPLETED_FAILED))
                        .bg(look.color(token::STATUS_COMPLETED_FAILED_BG))
                        .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                })
                .tooltip(tip(said.clone()))
                .children(mark.map(|(mark, hue)| glyph(mark, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4., look.color(if red { hue } else { token::FG_MUTED }))))
                .child(format!("{} main", main.repository));
            labelled(pill, Role::Status, said).into_any_element()
        });
        let mains: Vec<AnyElement> = mains.collect();
        let slots = slots_read.map(|slots| {
            let (held, all) = (slots.used, slots.total);
            let said = format!("Slots {held} of {all} held");
            labelled(
                pill("pill-slots").tooltip(tip(said.clone())).child(format!("Slots {held} of {all}")).child(meter(held as f32 / all.max(1) as f32, token::FG_MUTED)),
                Role::Status,
                said,
            )
        });
        let on = self.page.as_ref().is_some_and(|page| page.on);
        // What the Operator may do unasked: the project's name, then its value, or the name alone where he has not said. On All
        // it is each project's own, so the chip says so.
        let acts = on.then(|| {
            let (said, tip_said, name): (String, SharedString, String) = if self.acts_on_all(cx) {
                ("Unasked by project".into(), "What the Operator does unasked, set for each project".into(), "What the Operator does unasked, by project".into())
            } else {
                let project = self.acts_project(cx)?;
                let value = project.acts.and_then(|acts| super::page::ACTS.iter().find(|(_, one, ..)| *one == acts));
                let said = match value {
                    Some((_, _, label, _)) => format!("{}: {label}", project.name),
                    None => project.name.clone(),
                };
                let tip_said = value.map_or("Not set. The Operator asks once, and keeps your answer", |(_, _, _, means)| *means);
                (said.clone(), tip_said.into(), format!("What the Operator does unasked, {said}"))
            };
            let chip = press("acts", said.clone().into(), None, None, Weight::Plain, look)
                .when_some(first.take(), |chip, keys| chip.track_focus(&keys))
                .relative()
                .aria_expanded(self.acts_menu.is_some())
                .tooltip(tip(tip_said))
                .child(glyph("chevron-down", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                .child(measured(&self.drawn, "acts"));
            Some(labelled(pressed(chip, cx, |this, window, cx| this.open_acts(window, cx)), Role::Button, name))
        });
        let acts = acts.flatten();
        let acts_refused = self.acts_refused.clone().map(|why| {
            labelled(div().id("acts-refused").text_size(look.length(token::TEXT_XS)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(why.clone()), Role::Alert, why)
        });
        // Where the page reads on, the one press that reads it off. The last place in the head, so Tab reaches the rest first.
        let switch_off = on.then(|| {
            let refused = self.switch_off_refused.clone().map(|why| {
                labelled(
                    div().id("switch-off-refused").text_size(look.length(token::TEXT_XS)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(format!("Not switched off: {why}")),
                    Role::Alert,
                    format!("Not switched off: {why}"),
                )
            });
            let control = labelled(
                pressed(
                    press("operator-switch-off", "Switch off".into(), Some("power"), None, Weight::Bare, look).tooltip(tip("Nothing is handed to the Operator and it makes no calls")),
                    cx,
                    |this, _, cx| this.switch_off(cx),
                ),
                Role::Button,
                "Switch the Operator off",
            );
            div().flex_none().flex().items_center().gap(look.length(token::SPACE_2)).children(refused).child(control)
        });
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_3))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(glyph("bot", look.length(token::SPACE_4) + look.length(token::SPACE_1), look.color(token::OPERATOR)))
                    .child(labelled(
                        div().id("operator-title").text_size(look.length(token::TEXT_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Operator"),
                        Role::Heading,
                        "Operator",
                    )),
            )
            .children(mains)
            .children(slots)
            .children(acts_refused)
            .children(acts)
            .children(switch_off)
    }

    /// A time as the page reads it: a press flips every time on the page between a clock and an age.
    pub(super) fn stamp(&self, at: &str, lead: Option<&'static str>, (size, ink, weight): (Token, Token, Token), look: &Look, cx: &Context<Self>) -> AnyElement {
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let (on_clock, ago) = (minute(at, now).map(|clock| lead.map_or(clock.clone(), |lead| format!("{lead} {clock}"))), crate::job_view::age(at, now));
        let (shown, other) = if self.aged { (ago, on_clock) } else { (on_clock, ago) };
        let Some(shown) = shown else { return gpui::Empty.into_any_element() };
        let reading = shown.trim_start_matches(lead.unwrap_or_default()).trim_start().to_string();
        let stamp = worded(div().id(SharedString::from(format!("stamp:{at}"))), look)
            .flex_none()
            // A hairline inside the ring, so the gap before it and the ring together are one word space.
            .px(look.length(token::BORDER_WIDTH))
            // Its sentence's face said again: without it the time has drawn in the system's face.
            .font_family(sans(look))
            .font_weight(FontWeight(look.number(weight)))
            .text_size(look.length(size))
            .text_color(look.color(ink))
            .hover(|style| style.text_color(cx.look().color(token::FG_DEFAULT)))
            .tooltip(tip(other.unwrap_or_default()))
            .child(shown);
        let pressable = labelled(
            pressed(stamp, cx, |this, _, cx| {
                cx.stop_propagation();
                this.flip_stamps(cx);
            }),
            Role::Button,
            format!("Time {reading}"),
        );
        // The ring is drawn inside this: the gap before the time is the wrapper's, so it never reaches the word.
        div().flex_none().pl(look.length(token::SPACE_1) / 2.).child(pressable).into_any_element()
    }

    // -- skeletons ---------------------------------------------------------------------------------

    fn skeleton(&self, look: &Look) -> impl IntoElement {
        let line = look.length(token::LEADING_SM);
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_4))
                    .px(look.length(token::SPACE_6))
                    .py(look.length(token::SPACE_6))
                    .child(bone(0.2, line, look))
                    .child(div().h(look.length(token::SPACE_12) * 4.).rounded(round(look)).bg(look.color(token::BG_HOVER)))
                    .children((0..3).map(|_| bone(1., look.length(token::H_CONTROL_LG) + look.length(token::SPACE_2), look))),
            )
            .child(side(look).child(div().flex().flex_col().gap(look.length(token::SPACE_3)).child(bone(0.6, line, look)).children((0..4).map(|_| bone(1., look.length(token::H_CONTROL_LG) + look.length(token::SPACE_1), look)))))
    }

    /// Where the page says the Operator is off: what that means, and the switch Electron's Permissions tab has.
    fn off(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let switch = if self.switching {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_color(look.color(token::FG_MUTED))
                .child(spinner("switch-spin", look.length(token::SPACE_4), look.color(token::OPERATOR), cx))
                .child(if self.switching_off { "Switching off" } else { "Switching on" })
                .into_any_element()
        } else {
            labelled(
                // The press goes as the page reads on: the keys go to the page, where Tab starts at the pills.
                pressed(press("operator-switch", "Switch on".into(), Some("power"), None, Weight::Pick, look).tooltip(tip("Queue what needs you here")), cx, |this, window, cx| {
                    this.switch_on(cx);
                    window.focus(&this.focus, cx);
                }),
                Role::Button,
                "Switch the Operator on",
            )
            .into_any_element()
        };
        let refused = self.switch_refused.clone().map(|why| {
            labelled(
                div().id("switch-refused").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(format!("Not switched on: {why}")),
                Role::Alert,
                format!("Not switched on: {why}"),
            )
        });
        labelled(
            div()
                .id("operator-off")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_4))
                .rounded(round(look))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::OPERATOR_EDGE))
                .bg(look.color(token::OPERATOR_FAINT))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .child(glyph("bot", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("The Operator is off"))
                                .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Every Job asks you directly.")),
                        )
                        .child(switch),
                )
                .children(refused),
            Role::Region,
            "The Operator is off",
        )
        .into_any_element()
    }
}

/// The column at the right, for Next up and the feed: lower than the page, with the width of the old digest.
pub(super) fn side(look: &Look) -> Div {
    div()
        .flex_none()
        .w(look.length(token::SPACE_12) * 7.5)
        .h_full()
        .p(look.length(token::SPACE_4))
        .border_l(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_SUNKEN))
}

/// A time as the page reads it, in his clock to the minute: `12:47`, with the day when it is not today (`Nov 10 16:00`).
pub(super) fn minute(at: &str, now: chrono::DateTime<chrono::Local>) -> Option<String> {
    let then = chrono::DateTime::parse_from_rfc3339(at).ok()?.with_timezone(&chrono::Local);
    let held = crate::settings::time::held();
    Some(crate::settings::time::stamp_of(then, ipc::time_format::TimeFormat { seconds: false, ..held }, now))
}

impl Render for OperatorPageView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.reveal_pairing(window, cx);
        let look = cx.look();
        let root = div()
            .id("operator-page")
            .key_context("call")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::on_act))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                // From the page itself, Tab starts at its first pill, not wherever the window's order wraps to.
                let key = &event.keystroke;
                if key.key == "tab" && !key.modifiers.modified() && this.focus.is_focused(window) {
                    window.focus(&this.first_stop, cx);
                    cx.stop_propagation();
                } else if crate::keymap::traverse(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .text_color(look.color(token::FG_DEFAULT))
            .font_family(sans(look))
            .text_size(look.length(token::TEXT_SM));
        let top = div().flex_none().border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(self.head(look, cx));
        let Some(page) = self.page.as_ref() else {
            return root.child(top).child(self.skeleton(look)).into_any_element();
        };
        let off = (!page.on).then(|| self.off(look, cx));
        let talking = self.talk.is_some();
        let main = if talking { self.talk_column(window, look, cx) } else { self.card_column(off, window, look, cx) };
        let feed = self.feed(page, look, cx);
        let next_up = self.next_up(look, cx);
        root.relative()
            .child(top)
            .child(div().flex_1().min_h_0().flex().child(main).child(side(look).id("operator-side").overflow_y_scroll().child(div().flex().flex_col().gap(look.length(token::SPACE_4)).children(next_up).child(feed))))
            .children(self.later_menu.as_ref().map(|(menu, _)| menu.clone()))
            .children(self.acts_menu.as_ref().map(|(menu, _)| menu.clone()))
            .into_any_element()
    }
}

impl OperatorPageView {
    /// The main column with no talk: a Mac asking to pair, the card in front, and the box directly under it.
    fn card_column(&self, off: Option<AnyElement>, window: &Window, look: &Look, cx: &Context<Self>) -> gpui::Stateful<Div> {
        let mut parts: Vec<AnyElement> = Vec::new();
        // Another Mac waiting for his Confirm stands first: it lapses in five minutes and no Job holds it.
        parts.extend(self.pairing_cards(look, cx));
        parts.extend(off);
        // A kept idea brought from Next up stands in front in the Operator's card's place.
        // A card from the feed's working entries stands there too, to be read: it is picked, so it takes no press.
        match (self.front_idea(cx), self.front_working()) {
            (Some(idea), _) => parts.push(self.idea_card(&idea, window, look, cx)),
            (None, Some(turn)) => parts.push(self.working_card(turn, look, cx)),
            (None, None) => parts.extend(self.open_item().map(|turn| self.card(turn, false, window, look, cx))),
        }
        parts.push(self.composer.clone().into_any_element());
        div()
            .id("operator-main")
            .track_scroll(&self.main_scroll)
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .px(look.length(token::SPACE_6))
            .pt(look.length(token::SPACE_4) + look.length(token::SPACE_1))
            // Room under the last row, so it never sits on the window's edge.
            .pb(look.length(token::SPACE_12))
            .gap(look.length(token::SPACE_3))
            // Each part a child of the scroll, held at its own height, so a tall card scrolls instead of squeezing the box.
            .children(parts.into_iter().map(|part| div().flex_none().child(part)))
    }
}
