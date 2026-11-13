//! The plan rings in the end region, the card they open and the step-down notification
//! (`TitleBar.tsx` `PlanUsageFigures`, `PlanUsageLoading`, `LimitSection`; `downshift.tsx`).

use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, px, Anchor, AnimationExt as _, AnyElement, Bounds, Context, FontWeight, Hsla,
    PathBuilder, Pixels, Point, Role, SharedString,
};

use super::face::{offer_of, toast_of, Band, Limit, Remedy, Usage};
use super::parts::{glyph, kbd, mono, no_drag, small, tip, Btn, Weight};
use super::TitleBar;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The glyph is 22 square: centre 11, the outer ring at radius 9 and the inner 4 inside it, 2.5 wide.
const SIDE: f32 = 22.;
const OUTER: f32 = 9.;
const STEP: f32 = 4.;
const STROKE: f32 = 2.5;

fn tone_of(band: Band, look: &Look) -> Hsla {
    match band {
        Band::Normal => look.color(token::FG_MUTED),
        Band::Warning => look.color(token::STATUS_AWAITING_REVIEW),
        Band::Danger => look.color(token::STATUS_COMPLETED_FAILED),
    }
}

fn on_circle(centre: Point<Pixels>, radius: f32, turn: f32) -> Point<Pixels> {
    point(centre.x + px(radius * turn.sin()), centre.y - px(radius * turn.cos()))
}

/// A whole ring.
fn ring(centre: Point<Pixels>, radius: f32) -> Option<gpui::Path<Pixels>> {
    let mut path = PathBuilder::stroke(px(STROKE));
    path.move_to(point(centre.x + px(radius), centre.y));
    path.arc_to(point(px(radius), px(radius)), px(0.), false, true, point(centre.x - px(radius), centre.y));
    path.arc_to(point(px(radius), px(radius)), px(0.), false, true, point(centre.x + px(radius), centre.y));
    path.close();
    path.build().ok()
}

/// An arc starting `from` of a turn clockwise from twelve o'clock, running `run` of a turn.
fn arc(centre: Point<Pixels>, radius: f32, from: f32, run: f32) -> Option<gpui::Path<Pixels>> {
    if run <= 0. {
        return None;
    }
    if run >= 0.999 {
        return ring(centre, radius);
    }
    let tau = std::f32::consts::TAU;
    let mut path = PathBuilder::stroke(px(STROKE));
    path.move_to(on_circle(centre, radius, from * tau));
    path.arc_to(point(px(radius), px(radius)), px(0.), run > 0.5, true, on_circle(centre, radius, (from + run) * tau));
    path.build().ok()
}

/// What one ring paints: its fill and how far it runs, from where.
struct Ring {
    ink: Hsla,
    from: f32,
    run: f32,
}

fn rings(track: Hsla, rings: Vec<Ring>) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds: Bounds<Pixels>, _, window, _| {
            let centre = bounds.center();
            for (at, one) in rings.iter().enumerate() {
                let radius = OUTER - STEP * at as f32;
                if let Some(path) = ring(centre, radius) {
                    window.paint_path(path, track);
                }
                if let Some(path) = arc(centre, radius, one.from, one.run) {
                    window.paint_path(path, one.ink);
                }
            }
        },
    )
    .size_full()
}

impl TitleBar {
    /// The rings, the loading glyph, or nothing.
    pub(super) fn meter(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let side = px(SIDE);
        let track = look.color(token::BG_HOVER);
        let ring_ink = Self::focus_ring(look);
        match &self.face.usage {
            Usage::Hidden => None,
            Usage::Loading => {
                let ink = look.color(token::FG_MUTED);
                let period = look.duration(token::DURATION_PULSE).mul_f32(1.25);
                Some(
                    labelled(
                        no_drag(div()
                            .id("plan-usage-loading")
                            .flex_none()
                            .size(side)
                            .tooltip(tip("Analyzing Usage"))
                            .child(div().size(side).with_animation("plan-usage-sweep", gpui::Animation::new(period).repeat(), move |holder, at| {
                                // An arc of a quarter on each track: the outer clockwise, the inner reversed.
                                holder.child(rings(track, vec![Ring { ink, from: at, run: 0.25 }, Ring { ink, from: 1. - at, run: 0.25 }]))
                            }))),
                        Role::Image,
                        "Analyzing Usage",
                    )
                    .into_any_element(),
                )
            }
            Usage::Limits(limits) => {
                let name: SharedString = Usage::meter_name(limits).into();
                let drawn: Vec<Ring> = limits.iter().map(|one| Ring { ink: tone_of(one.band, look), from: 0., run: one.arc }).collect();
                let bell = self.face.downshift.as_ref().and_then(|(state, until)| offer_of(state, until).attention);
                let at = std::rc::Rc::clone(&self.at.meter);
                Some(
                    labelled(
                        no_drag(div()
                            .id("plan-usage")
                            .relative()
                            .flex_none()
                            .h(look.length(token::H_CONTROL_LG))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_1))
                            .px(look.length(token::SPACE_2))
                            .rounded(look.length(token::RADIUS_LG))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(gpui::transparent_black())
                            .cursor_pointer()
                            .track_focus(&self.meter_focus)
                            .focus_visible(move |style| style.border_color(ring_ink))
                            .hover(|style| style.bg(look.color(token::BG_HOVER)))
                            // Esc puts a card the keys opened away, and the keys stay on the meter.
                            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                                if event.keystroke.key == "escape" && this.popup == Some(super::Popup::Usage { pinned: false }) {
                                    this.meter_quiet = true;
                                    this.meter_by_keys = false;
                                    this.shut(window, cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .child(canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                            .child(div().flex_none().size(side).child(rings(track, drawn)))
                            .children(bell.map(|said| {
                                labelled(
                                    div()
                                        .id("plan-usage-attention")
                                        .flex_none()
                                        .tooltip(tip(said.clone()))
                                        .child(glyph("bell-ring", px(12.), look.color(token::STATUS_AWAITING_REVIEW))),
                                    Role::Image,
                                    said,
                                )
                            }))
                            .on_hover(cx.listener(|this, over: &bool, window, cx| this.meter_hovered(*over, window, cx)))
                            .on_click(cx.listener(|this, _, window, cx| this.meter_pressed(window, cx)))),
                        Role::Button,
                        name,
                    )
                    .into_any_element(),
                )
            }
        }
    }

    /// The plan card: a head, a title, a section for each limit, and the keys that work in it.
    pub(super) fn usage_card(&self, limits: &[Limit], look: &Look, cx: &Context<Self>) -> AnyElement {
        let weekly_offer = self.face.downshift.as_ref().map(|(state, until)| offer_of(state, until));
        let sections: Vec<AnyElement> = limits
            .iter()
            .enumerate()
            .map(|(at, one)| {
                let offer = (one.name == "Weekly limit").then(|| weekly_offer.clone()).flatten();
                self.limit_section(look, at, one, offer, cx)
            })
            .collect();
        labelled(
            div()
                .id("plan-card")
                .track_focus(&self.popup_focus)
                .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| match event.keystroke.key.as_str() {
                    "escape" => {
                        this.escape(window, cx);
                        cx.stop_propagation();
                    }
                    "enter" => {
                        // Keep open: the card stays and is pinned.
                        this.popup = Some(super::Popup::Usage { pinned: true });
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                }))
                .on_hover(cx.listener(|this, over: &bool, _, cx| {
                    this.over_card = *over;
                    cx.notify();
                }))
                .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                    if this.menu.is_none() && this.dialog.is_none() {
                        this.shut(window, cx);
                    }
                }))
                .occlude()
                .child(
                    super::parts::card(look)
                        .w(look.length(token::W_TOOLTIP))
                        .gap(look.length(token::SPACE_3))
                        .p(look.length(token::SPACE_4))
                        .border_t(look.length(token::BORDER_WIDTH) * 2.)
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .child(
                            small(look)
                                .flex()
                                .items_center()
                                .gap(look.length(token::SPACE_2))
                                .text_color(look.color(token::FG_MUTED))
                                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                .child(glyph("activity", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                                .child("Plan"),
                        )
                        .child(
                            div()
                                .text_size(look.length(token::TEXT_SM))
                                .line_height(look.length(token::LEADING_SM))
                                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                .child("Plan limits"),
                        )
                        .children(sections)
                        .child(
                            small(look)
                                .flex()
                                .flex_wrap()
                                .gap_x(look.length(token::SPACE_3))
                                .gap_y(look.length(token::SPACE_1))
                                .pt(look.length(token::SPACE_3))
                                .border_t(look.length(token::BORDER_WIDTH))
                                .border_color(look.color(token::BORDER_SUBTLE))
                                .text_color(look.color(token::FG_MUTED))
                                .child(div().flex().items_center().gap(look.length(token::SPACE_1)).child(kbd("Enter", look)).child("Keep open"))
                                .child(div().flex().items_center().gap(look.length(token::SPACE_1)).child(kbd("Esc", look)).child("Close")),
                        ),
                ),
            Role::Region,
            "Plan limits",
        )
        .into_any_element()
    }

    /// One limit: its name and percent on a line, when it resets, any offer's fact and its acts.
    fn limit_section(&self, look: &Look, at: usize, one: &Limit, offer: Option<super::face::Offer>, cx: &Context<Self>) -> AnyElement {
        let tone = tone_of(one.band, look);
        let mut facts: Vec<(&'static str, SharedString, bool)> = Vec::new();
        if let Some(resets) = &one.resets_in {
            facts.push(("Resets in", resets.clone(), true));
        }
        if let Some(offer) = &offer {
            facts.push(("Default model", offer.default_model.clone(), false));
        }
        let acts: Vec<AnyElement> = offer
            .iter()
            .flat_map(|offer| offer.acts.iter().enumerate())
            .map(|(index, act)| {
                let remedy = act.remedy;
                let weight = if act.primary { Weight::Primary } else { Weight::Secondary };
                labelled(
                    Btn::new(("offer-act", index), act.label.clone(), weight)
                        .draw(look, cx)
                        .on_click(cx.listener(move |this, _, window, cx| this.remedy_pressed(remedy, window, cx))),
                    Role::Button,
                    act.label.clone(),
                )
                .into_any_element()
            })
            .collect();
        labelled(
            div()
                .id(("limit", at))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .when(at > 0, |section| {
                    section.pt(look.length(token::SPACE_3)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE))
                })
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .justify_between()
                        .gap(look.length(token::SPACE_3))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .child(one.name)
                        .child(div().font_family(mono(look)).text_color(tone).child(format!("{}%", one.percent))),
                )
                .children((!facts.is_empty()).then(|| {
                    div().flex().flex_col().gap(look.length(token::BORDER_WIDTH) * 2.).children(facts.into_iter().map(|(term, value, is_mono)| {
                        small(look)
                            .flex()
                            .gap(look.length(token::SPACE_2))
                            .child(div().text_color(look.color(token::FG_SUBTLE)).child(term))
                            .child(div().text_color(look.color(token::FG_DEFAULT)).when(is_mono, |value| value.font_family(mono(look))).child(value))
                    }))
                }))
                .children((!acts.is_empty()).then(|| {
                    div().flex().flex_wrap().gap(look.length(token::SPACE_2)).mt(look.length(token::SPACE_1)).children(acts)
                })),
            Role::Region,
            one.name,
        )
        .into_any_element()
    }

    /// The step-down notification at the window's bottom right: one sentence and one act.
    pub(super) fn toast(&self, look: &Look, viewport: gpui::Size<Pixels>, cx: &Context<Self>) -> Option<AnyElement> {
        let (state, _) = self.face.downshift.as_ref()?;
        let toast = toast_of(state)?;
        let remedy = toast.remedy;
        let chooses = toast.sentence_chooses;
        let inset = look.length(token::SPACE_6);
        let sentence = div()
            .id("toast-text")
            .flex_1()
            .min_w_0()
            .child(toast.text.clone())
            .when(chooses, |text| {
                text.cursor_pointer()
                    .hover(|style| style.underline())
                    .on_click(cx.listener(|this, _, window, cx| this.remedy_pressed(Remedy::Choose, window, cx)))
            });
        Some(
            deferred(
                anchored().position(point(viewport.width - inset, viewport.height - inset)).anchor(Anchor::BottomRight).child(labelled(
                    div().id("toast").child(
                        super::parts::card(look)
                            .max_w(look.length(token::W_TOAST))
                            .flex_row()
                            .items_center()
                            .gap(look.length(token::SPACE_4))
                            .p(look.length(token::SPACE_4))
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .child(sentence)
                            .child(labelled(
                                div()
                                    .id("toast-action")
                                    .flex_none()
                                    .cursor_pointer()
                                    .focusable()
                                    .tab_stop(true)
                                    .text_size(look.length(token::TEXT_XS))
                                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                                    .text_color(look.color(token::ACCENT))
                                    .hover(|style| style.text_color(look.color(token::ACCENT_HOVER)))
                                    .child(toast.action.clone())
                                    .on_click(cx.listener(move |this, _, window, cx| this.remedy_pressed(remedy, window, cx))),
                                Role::Button,
                                toast.action.clone(),
                            )),
                    ),
                    Role::Status,
                    toast.text.clone(),
                )),
            )
            .with_priority(3)
            .into_any_element(),
        )
    }
}
