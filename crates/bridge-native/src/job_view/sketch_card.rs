//! The sketch card in a Job's thread, in the Band look the owner approved on 14 Nov 2026: a header
//! band tinted by where the sketch stands, the drawing in a sunken well, and the acts in a footer.
//!
//! What the card does is what it did: a press on the band or the drawing opens the sketch, the
//! pointer or the keys on it play its steps, and an answer goes to Fleet through `settle`. Only
//! where it draws and where its acts sit changed. The pinned sketch and the review sheet keep their own.

use gpui::{div, prelude::*, px, AnyElement, Context, FontWeight, Hsla, Rgba, Role, SharedString, Stateful, Window};
use ipc::{AnswerSketch, ArtifactLink, ArtifactLinkKind, ArtifactRef, DismissWaiting, SessionId, SketchHow, ThreadSketch};

use super::acts::{kbd, Answer, Mode};
use super::cards::{drawn_against, quote, settled_of, shown_scene, Said};
use super::rows::{mono, tip};
use super::{JobView, Page};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::scene::{Clock, Fit, Lens};

/// Where a sketch stands, which colours its band and words its chip.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Standing {
    Asking,
    /// The waiting plan names it, so the plan's Approve answers it.
    WithPlan,
    Approved,
    SentBack,
    Reading,
    Withdrawn,
}

impl Standing {
    fn of(card: &ThreadSketch, said: Option<Said>, with_plan: bool) -> Self {
        match said {
            Some(Said::Approved) => Standing::Approved,
            Some(Said::SentBack) => Standing::SentBack,
            None if card.withdrawn => Standing::Withdrawn,
            None if with_plan && card.asked && !card.replaced => Standing::WithPlan,
            None if card.asked && !card.replaced => Standing::Asking,
            None => Standing::Reading,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Standing::Asking => "Asking",
            Standing::WithPlan => "With the plan",
            Standing::Approved => "Approved",
            Standing::SentBack => "Sent back",
            Standing::Reading => "Reading",
            Standing::Withdrawn => "Withdrawn",
        }
    }

    fn ink(self) -> Token {
        match self {
            Standing::Asking | Standing::WithPlan => token::STATUS_AWAITING_REVIEW,
            Standing::Approved => token::STATUS_COMPLETED_SUCCESS,
            Standing::SentBack => token::STATUS_REJECTED,
            Standing::Reading => token::STATUS_NOT_STARTED,
            Standing::Withdrawn => token::STATUS_KILLED,
        }
    }

    fn tip(self) -> &'static str {
        match self {
            Standing::Asking => "Waiting on your answer",
            Standing::WithPlan => "The plan's Approve answers it",
            Standing::Approved => "You approved it",
            Standing::SentBack => "You sent it back with changes",
            Standing::Reading => "Shown to you, with no answer asked",
            Standing::Withdrawn => "The Job took it back",
        }
    }
}

/// `color-mix(in srgb, <ink> pct, --bg-raised)`.
fn mixed(look: &Look, ink: Token, pct: f32) -> Hsla {
    let (base, ink) = (Rgba::from(look.color(token::BG_RAISED)), Rgba::from(look.color(ink)));
    let at = |from: f32, to: f32| from + (to - from) * pct;
    Rgba { r: at(base.r, ink.r), g: at(base.g, ink.g), b: at(base.b, ink.b), a: 1. }.into()
}

/// What the card's acts need to reach Fleet, kept by the closures that press them.
#[derive(Clone)]
struct Acts {
    ix: usize,
    key: String,
    sketch: String,
    drawn: Option<String>,
    asking: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Approve,
    Secondary,
    Primary,
    Ghost,
}

/// A key cap on a button that has a fill of its own.
fn cap_on(look: &Look, text: &str, ink: Hsla) -> gpui::Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_KBD))
        .px(look.length(token::SPACE_1))
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(ink.opacity(0.5))
        .text_color(ink)
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .child(SharedString::from(text.to_string()))
}

/// One of the footer's buttons: 28 px tall, its key cap at the right. The caller adds `on_click`.
fn act(look: &Look, id: String, said: &'static str, kind: Kind, cap: &str, on: bool) -> Stateful<gpui::Div> {
    let base = div()
        .id(SharedString::from(id))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .gap(look.length(token::SPACE_1) + look.length(token::BORDER_WIDTH) * 2.)
        .h(look.length(token::H_CONTROL))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(gpui::transparent_black())
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)));
    let (styled, key_cap) = match kind {
        Kind::Approve => {
            let ink = look.color(token::FG_INVERSE);
            (
                base.bg(look.color(token::STATUS_COMPLETED_SUCCESS)).border_color(look.color(token::STATUS_COMPLETED_SUCCESS)).text_color(ink).hover(|style| style.opacity(0.9)),
                cap_on(look, cap, ink),
            )
        }
        Kind::Primary => {
            let ink = look.color(token::TEXT_ON_ACCENT);
            (
                base.bg(look.color(token::ACCENT)).border_color(look.color(token::ACCENT)).text_color(ink).hover(|style| style.bg(look.color(token::ACCENT_HOVER))),
                cap_on(look, cap, ink),
            )
        }
        Kind::Secondary => (
            base.bg(look.color(token::BG_OVERLAY)).border_color(look.color(token::BORDER_STRONG)).text_color(look.color(token::FG_DEFAULT)).hover(|style| style.bg(look.color(token::BG_HOVER))),
            kbd(look, cap),
        ),
        Kind::Ghost => (
            base.text_color(look.color(token::FG_MUTED)).hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT))),
            kbd(look, cap),
        ),
    };
    styled.when(on, |button| button.cursor_pointer()).when(!on, |button| button.opacity(0.4)).child(said).child(key_cap)
}

/// The icon of one For link.
fn link_icon(kind: ArtifactLinkKind) -> &'static str {
    match kind {
        ArtifactLinkKind::PullRequest => "git-pull-request",
        ArtifactLinkKind::PlanStep | ArtifactLinkKind::Task => "circle-dot",
        ArtifactLinkKind::Link => "external-link",
    }
}

fn link_chip(look: &Look, link: &ArtifactLink) -> AnyElement {
    let ink = look.color(token::ACCENT);
    labelled(
        div()
            .id(SharedString::from(format!("for-link:{}", link.target)))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_SM))
            .bg(ink.opacity(0.12))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .text_color(ink)
            .child(gpui::svg().path(icon_path(link_icon(link.kind))).flex_none().size(look.length(token::TEXT_2XS)).text_color(ink))
            .child(SharedString::from(link.label.clone())),
        Role::Label,
        link.label.clone(),
    )
    .into_any_element()
}

impl JobView {
    /// The Job's sketch put to the person, or shown (B1.2, B1.3).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn sketch(&self, ix: usize, card: &ThreadSketch, page: &Page, look: &Look, _now: chrono::DateTime<chrono::Local>, cx: &Context<Self>) -> AnyElement {
        let key = card.id.clone();
        let optimistic = page.cards.answers.get(&key);
        let settled = settled_of(card.settled.as_ref(), optimistic);
        // One the waiting plan names is answered by the plan's Approve, so its card asks nothing of its own.
        let standing = Standing::of(card, settled.as_ref().map(|(said, _)| *said), self.sketch_waits_with_plan(&card.sketch, cx));
        let revisions = card.revisions.len();
        let picked = page.cards.revision.get(&key).copied().filter(|at| *at < revisions);
        let shown_earlier = picked.is_some_and(|at| at + 1 < revisions);
        let asking = standing == Standing::Asking && !shown_earlier;
        let acts = Acts { ix, key: key.clone(), sketch: card.sketch.clone(), drawn: self.drawn_of(&card.sketch, cx), asking };
        let (scene_key, scene) = shown_scene(card, picked);
        let pointed = page.cards.pointed.as_deref() == Some(key.as_str()) || page.cards.focused.as_deref() == Some(key.as_str());
        let playing = page.cards.playing.as_ref().filter(|(at, _)| *at == key).map(|(_, since)| *since);
        let plays = scene.as_ref().is_some_and(|scene| crate::scene::plays(scene));
        let drawing = scene.map(|scene| {
            let height = self.drawing_height(&scene, look);
            let drawn = drawn_against(card, picked, &scene_key, scene, true, Fit::Preview, Clock::both(playing), Lens::default());
            self.well(height, card.replaced, drawn, look)
        });
        let band = self.band(ix, card, standing, picked, shown_earlier, look, cx);
        let press = self.sketch_press(&acts, None, &card.title, band, drawing, plays, page, cx);
        let words = settled.as_ref().and_then(|(said, words)| (*said == Said::SentBack).then_some(words.as_deref()).flatten());
        // A narrow thread lets the card reach a little past the words, as the old frame did (`--spacious-bleed`), so Approve and Ask for changes fit one row.
        let bleed = self.narrow(look);
        let body = div()
            .flex()
            .flex_col()
            .min_w_0()
            .when(bleed, |body| body.mx(-(look.length(token::SPACE_4) + look.length(token::SPACE_1))))
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG) * 1.5)
            .bg(look.color(token::BG_RAISED))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(if pointed { token::ACCENT } else { token::BORDER_DEFAULT }))
            .shadow(look.shadows(token::SHADOW_CARD))
            .child(press)
            .children(words.map(|words| div().px(look.length(token::SPACE_4)).pb(look.length(token::SPACE_3)).child(quote(look, words))))
            .child(self.sketch_footer(&acts, page, look, cx));
        labelled(
            body.id(SharedString::from(format!("sketch:{key}"))).when(card.replaced, |frame| frame.border_dashed()),
            Role::Article,
            format!("Sketch: {}", card.title),
        )
        .into_any_element()
    }

    /// A sketch a plan names, drawn inside the plan: its card's band and drawing, with no footer, since the plan's
    /// Approve answers it. A press opens it and the pointer on it plays its steps, as on its card.
    pub(super) fn plan_sketch(&self, key: &str, card: &ThreadSketch, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let settled = settled_of(card.settled.as_ref(), page.cards.answers.get(&card.id));
        let standing = Standing::of(card, settled.as_ref().map(|(said, _)| *said), self.sketch_waits_with_plan(&card.sketch, cx));
        let acts = Acts { ix: 0, key: key.to_string(), sketch: card.sketch.clone(), drawn: None, asking: false };
        let (scene_key, scene) = shown_scene(card, None);
        let pointed = page.cards.pointed.as_deref() == Some(key) || page.cards.focused.as_deref() == Some(key);
        let playing = page.cards.playing.as_ref().filter(|(at, _)| at == key).map(|(_, since)| *since);
        let plays = scene.as_ref().is_some_and(|scene| crate::scene::plays(scene));
        let drawing = scene.map(|scene| {
            let height = self.drawing_height(&scene, look);
            let drawn = drawn_against(card, None, &scene_key, scene, true, Fit::Preview, Clock::both(playing), Lens::default());
            self.well(height, false, drawn, look)
        });
        let band = self.band(0, card, standing, None, false, look, cx);
        let press = self.sketch_press(&acts, Some(&card.id), &card.title, band, drawing, plays, page, cx);
        div()
            .flex()
            .flex_col()
            .min_w_0()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG) * 1.5)
            .bg(look.color(token::BG_RAISED))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(if pointed { token::ACCENT } else { token::BORDER_DEFAULT }))
            .child(press)
            .into_any_element()
    }

    /// The header: the kind's mark, the title, the revision and where it stands; under them what it is for.
    #[allow(clippy::too_many_arguments)]
    fn band(&self, ix: usize, card: &ThreadSketch, standing: Standing, picked: Option<usize>, shown_earlier: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let ink = standing.ink();
        let revisions = card.revisions.len();
        let kind = look.color(token::KIND_STEP);
        let tile = div()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(look.length(token::SPACE_6) + look.length(token::BORDER_WIDTH) * 2.)
            .rounded(look.length(token::RADIUS_MD))
            .bg(kind.opacity(0.18))
            .child(gpui::svg().path(icon_path("layers")).flex_none().size(look.length(token::TEXT_BASE)).text_color(kind));
        let title = div()
            .flex_grow(1.)
            .min_w_0()
            .text_size(look.length(token::TEXT_BASE))
            .line_height(look.length(token::LEADING_BASE))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_DEFAULT))
            .child(SharedString::from(card.title.clone()));
        let chip = labelled(
            div()
                .id(SharedString::from(format!("state:{}", card.id)))
                .flex_none()
                .px(look.length(token::SPACE_2))
                .rounded_full()
                .bg(look.color(ink).opacity(0.16))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(ink))
                .tooltip(tip(standing.tip()))
                .child(standing.word()),
            Role::Status,
            standing.word(),
        );
        let top = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .child(tile)
            .child(title)
            .when(revisions >= 2, |row| row.child(self.revision_button(ix, &card.id, &card.title, revisions, picked, true, look, cx)))
            .child(chip);
        let against = card.against.as_deref().filter(|_| !shown_earlier);
        let line = card.for_line.as_ref();
        let under = line.is_some() || card.replaced || against.is_some();
        let muted = |text: String| div().flex_none().text_color(look.color(token::FG_SUBTLE)).child(text);
        let under = under.then(|| {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .children(line.map(|line| {
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .max_w_full()
                        .child(
                            div()
                                .flex()
                                .gap(look.length(token::SPACE_1))
                                .min_w_0()
                                .max_w_full()
                                .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child("For:"))
                                .children(line.text.clone().map(|text| div().flex_1().min_w_0().child(SharedString::from(text)))),
                        )
                        .children(line.links.iter().map(|link| link_chip(look, link)))
                }))
                .when(card.replaced, |row| row.child(muted("Replaced".to_string())))
                .children(against.map(|against| muted(format!("Against {against}"))))
        });
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1) + look.length(token::BORDER_WIDTH) * 2.)
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .bg(mixed(look, ink, 0.18))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(ink).opacity(0.4))
            .child(top)
            .children(under)
            .into_any_element()
    }

    /// How tall the drawing stands where the card is as wide as the thread: the scene's own extent at the
    /// scale the card fits it to, which is never above one.
    fn drawing_height(&self, scene: &ipc::Scene, look: &Look) -> gpui::Pixels {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        let mut reach = |x: f64, y: f64| {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        };
        for node in &scene.nodes {
            reach(node.x, node.y);
            reach(node.x + node.w.unwrap_or(180.), node.y + node.h.unwrap_or(80.));
        }
        for point in scene.strokes.iter().flat_map(|stroke| stroke.points.iter()) {
            reach(point.x, point.y);
        }
        let floor = look.length(token::SPACE_12) * 2.;
        if x1 <= x0 || y1 <= y0 {
            return floor;
        }
        // The card's edge to the drawing: the frame's inset and the well's padding, each side, and the well's edge.
        let around = look.length(token::SPACE_4) * 4. + look.length(token::BORDER_WIDTH) * 2.;
        let bleed = if self.narrow(look) { (look.length(token::SPACE_4) + look.length(token::SPACE_1)) * 2. } else { px(0.) };
        let across = f32::from(self.pane.unwrap_or(px(720.)) - look.length(token::SPACE_4) * 2. + bleed - around).max(1.);
        let scale = (across / (x1 - x0) as f32).min(1.);
        // The fit keeps four in a hundred clear on each side.
        px((y1 - y0) as f32 * scale * 1.08).max(floor)
    }

    /// The drawing's well: sunken, its edge a hairline, cut at 240 with a fade at its foot where the drawing runs on.
    fn well(&self, height: gpui::Pixels, replaced: bool, drawn: AnyElement, look: &Look) -> AnyElement {
        let most = look.length(token::SPACE_12) * 5.;
        let pad = look.length(token::SPACE_3);
        let cut = height + pad * 2. + look.length(token::BORDER_WIDTH) * 2. > most;
        let sunken = look.color(token::BG_SUNKEN);
        div()
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .child(
                div()
                    .relative()
                    .overflow_hidden()
                    .max_h(most)
                    .rounded(look.length(token::RADIUS_LG))
                    .bg(sunken)
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .px(look.length(token::SPACE_4))
                    .py(pad)
                    .child(div().w_full().h(height).when(replaced, |inner| inner.opacity(0.5)).child(drawn))
                    .when(cut, |well| {
                        well.child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .bottom_0()
                                .h(look.length(token::SPACE_12))
                                .bg(gpui::linear_gradient(180., gpui::linear_color_stop(sunken.opacity(0.), 0.), gpui::linear_color_stop(sunken, 1.))),
                        )
                    }),
            )
            .into_any_element()
    }

    /// The band and the drawing as one press, `Open sketch …`, which plays the steps while the pointer or the keys
    /// are on it, as Electron's `SketchRow` does. While it has focus its keys answer: A approves, C asks for changes,
    /// D dismisses, Enter opens. Drawn elsewhere than its card, `inside` names the card it opens, and Enter alone answers.
    #[allow(clippy::too_many_arguments)]
    fn sketch_press(&self, acts: &Acts, inside: Option<&str>, title: &str, band: AnyElement, drawing: Option<AnyElement>, plays: bool, page: &Page, cx: &Context<Self>) -> AnyElement {
        let key = acts.key.as_str();
        let spot = page.keys.spot(&format!("sketch-press:{key}"), cx);
        let opens = inside.unwrap_or(key).to_string();
        let (hover_key, open_key, enter_key, alone) = (key.to_string(), opens.clone(), opens, inside.is_none());
        let was = page.cards.focused.as_deref() == Some(key);
        let (focus, me, focus_key) = (spot.focus.clone(), cx.entity().downgrade(), key.to_string());
        let acts = acts.clone();
        labelled(
            div()
                .id(SharedString::from(format!("sketch-press:{key}")))
                .relative()
                .flex()
                .flex_col()
                .cursor_pointer()
                .track_focus(&spot.focus)
                .on_hover(cx.listener(move |this, over: &bool, _, cx| this.sketch_pointed(&hover_key, Some(*over), None, plays, cx)))
                .on_click(cx.listener(move |this, _, window, cx| this.read_sketch(&open_key, &open_key, window, cx)))
                .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.modifiers.modified() {
                        return;
                    }
                    match event.keystroke.key.as_str() {
                        "enter" | "space" => this.read_sketch(&enter_key, &enter_key, window, cx),
                        _ if !alone => return,
                        "a" if acts.asking => this.approve_sketch(&acts, window, cx),
                        "c" if acts.asking => this.ask_changes(&acts, window, cx),
                        "d" if acts.asking => this.dismiss_sketch(&acts, window, cx),
                        "d" => this.archive_sketch(&acts, window, cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                .child(band)
                .children(drawing)
                .child(
                    gpui::canvas(
                        move |_, window, cx| {
                            let now = focus.is_focused(window);
                            if now != was {
                                let (me, focus_key) = (me.clone(), focus_key.clone());
                                cx.defer(move |cx| {
                                    let _ = me.update(cx, |this, cx| this.sketch_pointed(&focus_key, None, Some(now), plays, cx));
                                });
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                ),
            Role::Button,
            format!("Open sketch {title}"),
        )
        .into_any_element()
    }

    /// The footer: Approve and Ask for changes while it asks, then Dismiss, or Archive where it does not, and Open.
    /// Ask for changes opens a note box in its place.
    fn sketch_footer(&self, acts: &Acts, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = acts.key.as_str();
        let noting = acts.asking && page.cards.modes.get(key) == Some(&Mode::Note);
        if (!acts.asking || !page.cards.modes.contains_key(key)) && self.holds_kept(key) {
            page.cards.kept_seen.borrow_mut().push((key.to_string(), acts.ix, acts.asking));
        }
        let mut footer = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE));
        if let Some(why) = page.cards.said.get(key) {
            footer = footer.child(super::acts::refusal(look, key, why.clone()));
        }
        if noting {
            return footer.child(self.note_box(acts, page, look, cx)).into_any_element();
        }
        let tail = |name: &'static str, cap: &'static str, tip_said: &'static str, on_press: Box<dyn Fn(&mut JobView, &mut Window, &mut Context<JobView>)>, id: &str, cx: &Context<Self>| {
            let id = format!("{id}:{key}");
            labelled(
                act(look, id, name, Kind::Ghost, cap, true).tooltip(tip(tip_said)).on_click(cx.listener(move |this, _, window, cx| on_press(this, window, cx))),
                Role::Button,
                name,
            )
        };
        let (a1, a2, a3, a4) = (acts.clone(), acts.clone(), acts.clone(), acts.clone());
        // Approve and Ask for changes stay together on the first row; Dismiss (or Archive) and Open wrap as one group, right-aligned.
        let group = || div().flex().flex_none().items_center().gap(look.length(token::SPACE_2));
        let row = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2));
        let row = if acts.asking {
            row.child(
                group()
                    .child(labelled(
                        act(look, format!("approve:{key}"), "Approve", Kind::Approve, "A", true)
                            .tooltip(tip("Approve the sketch"))
                            .on_click(cx.listener(move |this, _, window, cx| this.approve_sketch(&a1, window, cx))),
                        Role::Button,
                        "Approve",
                    ))
                    .child(labelled(
                        act(look, format!("ask:{key}"), "Ask for changes", Kind::Secondary, "C", true)
                            .tooltip(tip("Send it back with a note"))
                            .on_click(cx.listener(move |this, _, window, cx| this.ask_changes(&a2, window, cx))),
                        Role::Button,
                        "Ask for changes",
                    )),
            )
        } else {
            row
        };
        let put_away = if acts.asking {
            tail("Dismiss", "D", "Put it away with no answer", Box::new(move |this, window, cx| this.dismiss_sketch(&a3, window, cx)), "dismiss", cx)
        } else {
            tail("Archive", "D", "Fold it away from your list", Box::new(move |this, window, cx| this.archive_sketch(&a3, window, cx)), "archive", cx)
        };
        let open = tail("Open", "\u{21b5}", "Read it on its own", Box::new(move |this, window, cx| this.read_sketch(&a4.key, &a4.key, window, cx)), "open", cx);
        let row = row.child(group().ml_auto().child(put_away).child(open));
        footer
            .child(super::acts::gated(row.id(SharedString::from(format!("answers:{key}"))), self.off_shown_to("/sessions/sketch/answer", cx)))
            .into_any_element()
    }

    /// What should change: the note, with Cancel and Send under it.
    fn note_box(&self, acts: &Acts, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = acts.key.as_str();
        let note_blank = self.typed_in(key, Mode::Note, cx).trim().is_empty();
        let (send, cancel, press, esc) = (acts.clone(), acts.clone(), acts.clone(), acts.clone());
        // `⌘Enter` in the box sends it back, as Send does.
        page.keys.commit(
            key,
            Some(std::rc::Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| {
                let note = this.typed_in(&press.key, Mode::Note, cx).trim().to_string();
                if !note.is_empty() {
                    this.send_back_sketch(&press, note, window, cx);
                }
            })),
        );
        let send_key = key.to_string();
        let reveal = page.cards.reveal.get(key).cloned().map(|flag| {
            let (me, ix) = (cx.entity().downgrade(), acts.ix);
            gpui::canvas(
                move |_, _, cx| {
                    if flag.replace(false) {
                        let me = me.clone();
                        cx.defer(move |cx| {
                            let _ = me.update(cx, |this, cx| this.reveal_sketch(ix, cx));
                        });
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        });
        div()
            .relative()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .children(reveal)
            .capture_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                    cx.stop_propagation();
                    this.shut_sketch_note(&esc, window, cx);
                }
            }))
            .child(
                div()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_MUTED))
                    .child("What should change?"),
            )
            .children(self.box_of(key, Mode::Note, "What should change?", look, cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(look.length(token::SPACE_2))
                    .child(labelled(
                        act(look, format!("cancel:{key}"), "Cancel", Kind::Ghost, "Esc", true)
                            .tooltip(tip("Keep the sketch as it is"))
                            .on_click(cx.listener(move |this, _, window, cx| this.shut_sketch_note(&cancel, window, cx))),
                        Role::Button,
                        "Cancel",
                    ))
                    .child(labelled(
                        act(look, format!("send-back:{send_key}"), "Send", Kind::Primary, "\u{2318}\u{21b5}", !note_blank).when(!note_blank, |button| {
                            button.tooltip(tip("Send it back with this note")).on_click(cx.listener(move |this, _, window, cx| {
                                let note = this.typed_in(&send.key, Mode::Note, cx).trim().to_string();
                                this.send_back_sketch(&send, note, window, cx);
                            }))
                        }),
                        Role::Button,
                        "Send",
                    )),
            )
            .into_any_element()
    }

    fn narrow(&self, look: &Look) -> bool {
        self.pane.is_some_and(|width| width < look.length(token::SPACE_12) * 10.)
    }

    fn sketch_session(&self) -> SessionId {
        SessionId::carried(self.shown.clone().unwrap_or_default().to_string())
    }

    fn answer_sketch_with(&mut self, acts: &Acts, how: SketchHow, words: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let body = ipc::encode(&AnswerSketch { session_id: self.sketch_session(), sketch: acts.sketch.clone(), drawn: acts.drawn.clone().unwrap_or_default(), how, words: words.clone() }).map_err(|why| format!("{why:?}"));
        let answer = match words {
            Some(words) => Answer::SentBack(words),
            None => Answer::Approved,
        };
        self.settle(acts.ix, acts.key.clone(), answer, "/sessions/sketch/answer", body, window, cx);
    }

    fn approve_sketch(&mut self, acts: &Acts, window: &mut Window, cx: &mut Context<Self>) {
        self.answer_sketch_with(acts, SketchHow::Approved, None, window, cx);
    }

    fn send_back_sketch(&mut self, acts: &Acts, note: String, window: &mut Window, cx: &mut Context<Self>) {
        self.answer_sketch_with(acts, SketchHow::SentBack, Some(note), window, cx);
    }

    fn ask_changes(&mut self, acts: &Acts, window: &mut Window, cx: &mut Context<Self>) {
        self.open_box(acts.ix, &acts.key, Mode::Note, "", window, cx);
        // `open_box` reveals the card at the height it had; the note makes it taller, so it is revealed again once drawn.
        if let Some(cards) = self.cards_mut() {
            cards.reveal.insert(acts.key.clone(), std::rc::Rc::new(std::cell::Cell::new(true)));
        }
    }

    /// The card is whole in the thread again, now that it has its new height.
    fn reveal_sketch(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get(id)) {
            page.list.scroll_to_reveal_item(page.entry(ix));
        }
        cx.notify();
    }

    /// The note shuts and the keys go back to the card.
    fn shut_sketch_note(&mut self, acts: &Acts, window: &mut Window, cx: &mut Context<Self>) {
        self.close_box(acts.ix, &acts.key, cx);
        let spot = self.shown.as_ref().and_then(|id| self.pages.get(id)).and_then(|page| page.keys.spot_of(&format!("sketch-press:{}", acts.key)));
        if let Some(spot) = spot {
            window.focus(&spot.focus, cx);
        }
    }

    /// Dismiss puts the item away with no verdict, the act the card's Withdraw was.
    fn dismiss_sketch(&mut self, acts: &Acts, window: &mut Window, cx: &mut Context<Self>) {
        let body = ipc::encode(&DismissWaiting { session_id: self.sketch_session(), item_id: acts.sketch.clone(), reason: None, misread: false }).map_err(|why| format!("{why:?}"));
        self.settle(acts.ix, acts.key.clone(), Answer::Withdrawn, "/sessions/waiting/dismiss", body, window, cx);
    }

    /// Archive folds the sketch away in his Artifacts tab with no verdict (`archive_artifact`).
    fn archive_sketch(&mut self, acts: &Acts, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let body = ipc::encode(&ArtifactRef { artifact: acts.sketch.clone(), session_id: Some(self.sketch_session()) }).map_err(|why| format!("{why:?}"));
        self.post_card(job, acts.ix, acts.key.clone(), "/sessions/artifacts/archive", body, window, cx);
    }
}
