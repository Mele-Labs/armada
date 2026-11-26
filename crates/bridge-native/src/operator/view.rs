//! The page drawn, as the approved sketches lay it out: two raised panels side by side on the base ground. The main panel
//! holds the status line with what the Operator does unasked, the card in front, the new-card strip, the talk under the
//! card and the box at the foot, all one width. The right panel holds Next up and the feed Since you last looked. Roomier
//! than the sketch: controls a step taller, corners rounder, as the native screens are.

use gpui::{canvas, div, prelude::*, px, AnyElement, Context, Div, FontWeight, Hsla, MouseButton, Pixels, Role, SharedString, Stateful, Window};
use ipc::MainCiState;

use super::face::{self, Kind};
use super::page::OperatorPageView;
use super::status::Status;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::titlebar::parts::{glyph, kbd, sans, tip};

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
    /// A press that undoes a failure, such as Restart on a stopped Operator: filled in the failed hue.
    Danger,
    Plain,
    Bare,
}

pub(crate) fn press(id: impl Into<SharedString>, label: SharedString, icon: Option<&'static str>, key: Option<SharedString>, weight: Weight, look: &Look) -> Stateful<Div> {
    let id: SharedString = id.into();
    let (ground, hover, ink, edge) = match weight {
        Weight::Pick => (look.color(token::OPERATOR), look.color(token::OPERATOR), look.color(token::FG_INVERSE), look.color(token::OPERATOR)),
        Weight::JobPick => (look.color(token::ACCENT), look.color(token::ACCENT_HOVER), look.color(token::TEXT_ON_ACCENT), look.color(token::ACCENT)),
        Weight::Danger => {
            let danger = look.color(token::STATUS_COMPLETED_FAILED);
            (danger, danger.opacity(0.85), look.color(token::FG_INVERSE), danger)
        }
        Weight::Plain => (look.color(token::BG_RAISED), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), look.color(token::BORDER_STRONG)),
        Weight::Bare => (gpui::transparent_black(), look.color(token::BG_HOVER), look.color(token::FG_MUTED), gpui::transparent_black()),
    };
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    let cap = key.map(|key| {
        let cap = kbd(key, look);
        if matches!(weight, Weight::Pick | Weight::JobPick | Weight::Danger) { cap.bg(ink.opacity(0.16)).border_color(ink.opacity(0.5)).text_color(ink) } else { cap }
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
        .font_weight(FontWeight(look.number(if matches!(weight, Weight::Pick | Weight::JobPick | Weight::Danger) { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
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
/// so the page's own Enter (Yes) is not said as well. Any view's, as the card it presses on is shared.
pub(crate) fn pressed<V: 'static>(element: Stateful<Div>, cx: &Context<V>, act: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static) -> Stateful<Div> {
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
pub(super) fn bone(width: f32, height: Pixels, look: &Look) -> Div {
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
        // A host that draws the chrome gets the Cockpit's header row instead, with no Mac or slot pills beside it.
        let embedded = self.embedded;
        let mains: Vec<AnyElement> = if embedded { Vec::new() } else { mains.collect() };
        let slots = slots_read.filter(|_| !embedded).map(|slots| {
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
        // The head is as wide as the main panel: where it is too narrow, the controls at its right wrap to a line of their own.
        // Embedded, the line is one row and the status words give way before the controls do.
        let lead = if embedded {
            self.embed_lead(look)
        } else {
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
                ))
        };
        div()
            .flex_none()
            .flex()
            .when(!embedded, |head| head.flex_wrap())
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .child(lead)
            .children(mains)
            .children(slots)
            .child(
                div()
                    .flex_none()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .children(acts_refused)
                    .children(acts)
                    .children(switch_off),
            )
    }

    /// The Cockpit's header row: the state's dot, the name, the state chip and the status line, as the approved page draws it.
    fn embed_lead(&self, look: &Look) -> Div {
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let status = self.status();
        let (word, said, hue, ground) = status.chip();
        let line = status.line(now);
        let dot = div().id("operator-state-dot").flex_none().w(look.length(token::SPACE_2)).h(look.length(token::SPACE_2)).rounded_full().bg(look.color(hue)).tooltip(tip(said));
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .child(dot)
            .child(labelled(
                div().id("operator-title").flex_none().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Operator"),
                Role::Heading,
                "Operator",
            ))
            .child(labelled(chip(word, look.color(hue), look.color(ground), look).id("operator-state").tooltip(tip(said)), Role::Status, word))
            .child(labelled(
                div().id("operator-line").flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(line.clone()),
                Role::Status,
                line,
            ))
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

    /// The panels' bones while the page is read: the main panel under its head, and the right panel.
    fn skeleton(&self, look: &Look) -> [AnyElement; 2] {
        let line = look.length(token::LEADING_SM);
        let main = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .p(look.length(token::SPACE_4))
            .child(div().h(look.length(token::SPACE_12) * 4.).rounded(round(look)).bg(look.color(token::BG_HOVER)))
            .children((0..3).map(|_| bone(1., look.length(token::H_CONTROL_LG) + look.length(token::SPACE_2), look)));
        let side = div().flex().flex_col().gap(look.length(token::SPACE_3)).p(look.length(token::SPACE_4)).child(bone(0.6, line, look)).children((0..4).map(|_| bone(1., look.length(token::H_CONTROL_LG) + look.length(token::SPACE_1), look)));
        [main.into_any_element(), side.into_any_element()]
    }
}

/// The right panel, for Next up and the feed: the width of the old digest.
pub(super) fn side(look: &Look) -> Div {
    crate::reviews::panel(look).flex_none().w(side_width(look)).h_full()
}

fn side_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 7.5
}

/// The width the card and the talk are read at, as the approved page draws the Operator's column.
pub(super) fn readable(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 15.
}

/// The narrowest pane that keeps the side column beside the main one: the main panel keeps room for a card's options, the
/// side column its own width and the gap and padding between them. Narrower, the side column folds away.
pub(super) fn side_floor(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 10. + side_width(look) + look.length(token::GAP_PANEL) + look.length(token::SPACE_3) * 2.
}

impl OperatorPageView {
    /// How wide the page's own pane is: as it was drawn last, else the window's width until it is drawn once.
    pub(super) fn pane_width(&self, window: &Window) -> Pixels {
        self.drawn.borrow().get("page").map_or_else(|| window.viewport_size().width, |bounds| bounds.size.width)
    }

    /// How wide the Ready to try card has until it is drawn once: the pane's own width less the panels and padding around it,
    /// never the window's, and no wider than the readable column where embedded.
    pub(super) fn ready_room(&self, window: &Window, look: &Look) -> f32 {
        let around = if self.embedded { px(0.) } else { look.length(token::SPACE_3) * 2. };
        let side = if self.side_fits(window, look) { side_width(look) + look.length(token::GAP_PANEL) } else { px(0.) };
        let insets = around + look.length(token::SPACE_4) * 2. + look.length(token::BORDER_WIDTH) * 2.;
        let room = (self.pane_width(window) - side - insets).max(px(0.));
        f32::from(if self.embedded { room.min(readable(look)) } else { room })
    }

    /// Whether the pane has room for the side column beside the main panel.
    pub(super) fn side_fits(&self, window: &Window, look: &Look) -> bool {
        self.pane_width(window) >= side_floor(look)
    }

    /// Measures the pane the page is drawn in, and draws again where its width crossed the side column's floor since the last
    /// frame, so a pane narrower than the window's first guess folds it at once.
    fn pane_watch(&self, look: &Look, page: gpui::EntityId) -> gpui::Canvas<()> {
        let (drawn, was, floor) = (self.drawn.clone(), self.side_drawn.clone(), side_floor(look));
        canvas(
            move |bounds, window, cx| {
                drawn.borrow_mut().insert("page", bounds);
                if (bounds.size.width >= floor) != was.get() {
                    window.refresh();
                    cx.notify(page);
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }
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
        let ready = self.ready_card(window, cx);
        *self.ready_drawn.borrow_mut() = ready;
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
            .relative()
            .size_full()
            .flex()
            .gap(look.length(token::GAP_PANEL))
            // Embedded, the host draws the ground and the padding around the pane.
            .when(!self.embedded, |root| root.p(look.length(token::SPACE_3)).bg(look.color(token::BG_BASE)))
            .text_color(look.color(token::FG_DEFAULT))
            .font_family(sans(look))
            .text_size(look.length(token::TEXT_SM));
        // The status line heads the main panel, over everything the card and the talk draw.
        let side_on = self.side_fits(window, look);
        self.side_drawn.set(side_on);
        let root = root.child(self.pane_watch(look, cx.entity_id()));
        let head = div().flex_none().border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(self.head(look, cx));
        let main = crate::reviews::panel(look).flex_1().min_w_0().h_full().flex().flex_col().child(head);
        let Some(page) = self.page.as_ref() else {
            let [bones, side_bones] = self.skeleton(look);
            return root.child(main.child(bones)).when(side_on, |root| root.child(side(look).child(side_bones))).into_any_element();
        };
        let feed = self.feed(page, look, cx);
        let working = self.working_group(page, look, cx);
        let next_up = self.next_up(look, cx);
        root.child(main.child(self.main_column(window, look, cx)))
            .when(side_on, |root| {
                root.child(side(look).child(div().id("operator-side").size_full().overflow_y_scroll().p(look.length(token::SPACE_4)).child(div().flex().flex_col().gap(look.length(token::SPACE_4)).children(next_up).children(working).child(feed))))
            })
            .children(self.later_menu.as_ref().map(|(menu, _)| menu.clone()))
            .children(self.acts_menu.as_ref().map(|(menu, _)| menu.clone()))
            .into_any_element()
    }
}

impl OperatorPageView {
    /// The Ready to try in front, drawn from the Job view at the card's width: as drawn last, else as the window has room for it.
    /// None until the Job view shows its Job.
    fn ready_card(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let job = self.ready_in_front()?;
        let look = cx.look().clone();
        let item = self.open_item()?.item.id.clone();
        let wide = self.drawn.borrow().get("ready").map(|drawn| f32::from(drawn.size.width)).unwrap_or_else(|| self.ready_room(window, &look));
        crate::job_view::ready_on_page(job.as_str(), &item, wide.max(0.), cx)
    }

    /// Under the status line: a Mac asking to pair, the card in front or its folded line, the new-card strip, the talk, and
    /// the box at the foot. A stopped Operator's red card stands in front of any queued card, and the talk and the box dim
    /// under the scrim until it is back.
    fn main_column(&self, window: &Window, look: &Look, cx: &Context<Self>) -> Div {
        let status = self.status();
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let folded = self.is_folded();
        let mut front: Vec<AnyElement> = self.pairing_cards(look, cx);
        let card = if matches!(status, Status::Off) || self.stopped_in_front() {
            self.status_card(look, cx)
        } else if let Some(idea) = self.front_idea(cx) {
            // A kept idea brought from Next up stands in front in the Operator's card's place.
            if folded { self.folded(&idea.id, super::parked::words_of(&idea).into(), None, look, cx) } else { self.idea_card(&idea, window, look, cx) }
        } else if let Some(turn) = self.front_working() {
            // A card from the feed's working entries stands there too, to be read: it is picked, so it takes no press.
            if folded { self.folded(&turn.id, face::card_title(turn), face::ask_line(turn), look, cx) } else { self.working_card(turn, look, cx) }
        } else if let Some(turn) = self.open_item() {
            if folded { self.folded(&turn.id, face::card_title(turn), face::ask_line(turn), look, cx) } else { self.card(turn, false, window, look, cx) }
        } else if folded {
            // Nothing is in front: the Operator's state stands in the card's place.
            let last = self.page.as_ref().and_then(|page| page.feed.first()).map(|entry| super::status::last_act(entry, now).into());
            self.folded("status", status.title(now).into(), last, look, cx)
        } else {
            self.status_card(look, cx)
        };
        front.push(card);
        // Too narrow for the side column, Next up stands under the card, so no queued card is out of sight.
        if !self.side_fits(window, look) {
            front.extend(self.next_up(look, cx));
        }
        // A card in front keeps its whole height, so nothing in it is scrolled away: the talk gives up its room and scrolls.
        let whole = !folded;
        let dim = matches!(status, Status::Stopped { .. });
        // The talk and the box under the front: the talk gives up its room to the card.
        let talk = div()
            .relative()
            .flex_grow(1.)
            .flex_shrink(1.)
            // Its parts hold their own least room: the box keeps its whole height.
            .min_h_0()
            .flex()
            .flex_col()
            .child(self.talk(look, cx))
            .child(div().flex_none().child(self.composer.clone()))
            .children(dim.then(|| div().absolute().inset_0().bg(look.color(token::SCRIM))));
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .pt(look.length(token::SPACE_4))
            // Embedded, the card, the talk and the box stand in one column at a readable width, centred in the pane.
            .when(self.embedded, |column| column.w_full().max_w(readable(look) + look.length(token::SPACE_4) * 2.).mx_auto())
            .child(
                div()
                    .id("operator-main")
                    .track_scroll(&self.main_scroll)
                    .min_h_0()
                    // A card holds its whole height; a folded line and what stands under it give up their room before the talk.
                    .flex_shrink(if whole { 0. } else { 100. })
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    // Each part held at its own height.
                    .children(front.into_iter().map(|part| div().flex_none().child(part))),
            )
            .children(self.strip(look, cx))
            .child(talk)
    }
}
