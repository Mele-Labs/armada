//! A Job's context ring with Compact (`SessionFrame/ContextFigure.tsx`): the window as a ring filled
//! clockwise, its percent beside it in mono, a tick at the autocompact threshold, and by the data it is
//! given one of three forms. **A screen draws; Fleet reads**: the percent and its band are
//! `SessionUsage.context_percent` and `context_band`, and whether Compact is offered and why not is
//! `SessionRecord.compact`, so nothing here works a band or a reason out.
//!
//! | Form | When | Drawn |
//! |---|---|---|
//! | Card | a breakdown is known | a button; hover opens the card, a press keeps it open |
//! | Popover | no breakdown, Compact offered or off | a button; a press opens the fill and Compact |
//! | Tooltip only | neither | a picture whose tooltip carries the tokens |
//!
//! [`ContextRing`] is a `RenderOnce` the Job head places; what holds the open card, the pending
//! Compact and the keys is a [`Figure`] entity it keeps by the Job's id. A press on Compact is
//! pending in the same frame and is handed to the head's `on_compact`; the head's next [`Fill`]
//! says whether it was refused.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, px, Anchor, App, Bounds, Context, ElementId, Empty, FocusHandle, FontWeight, Hsla,
    PathBuilder, Pixels, Point, Role, SharedString, Subscription, Task, Window,
};
use ipc::{CompactHow, CompactOff, CompactOffer, ContextBreakdown, ContextKind, RingBand, SessionUsage};

use crate::inspect::{self, Fact, Spec, State, Width};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{card, glyph, mono, Btn, Weight};

/// What a Job head hands the ring: how full the window is, what fills it where that is known, and
/// Compact. `None` from [`Fill::of`] is a Job with no window to fill, which draws nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Fill {
    pub tokens: u64,
    pub window: u64,
    /// Fleet's whole percent, clamped 0 to 100, and its band.
    pub percent: u8,
    pub band: RingBand,
    pub breakdown: Option<ContextBreakdown>,
    /// Compact offered, or off and why. Absent: the Job's head offers no Compact at all.
    pub compact: Option<CompactOffer>,
    /// What the last Compact was refused for, as the head says it.
    pub refused: Option<SharedString>,
}

impl Fill {
    pub fn of(usage: &SessionUsage, breakdown: Option<&ContextBreakdown>, compact: Option<CompactOffer>) -> Option<Fill> {
        let (tokens, window) = (usage.context_tokens?, usage.context_window?);
        if window == 0 {
            return None;
        }
        Some(Fill {
            tokens,
            window,
            percent: usage.context_percent?,
            band: usage.context_band?,
            breakdown: breakdown.cloned(),
            compact,
            refused: None,
        })
    }
}

/// What a press on Compact asks of the head: send `/compact` as the Job's kind of Job takes it.
pub type OnCompact = Rc<dyn Fn(CompactHow, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct ContextRing {
    id: SharedString,
    fill: Option<Fill>,
    on_compact: Option<OnCompact>,
    opened: bool,
}

impl ContextRing {
    /// The ring of the Job `id`. The id keeps its open card and its pending Compact between frames.
    pub fn new(id: impl Into<SharedString>, fill: Option<Fill>) -> ContextRing {
        ContextRing { id: id.into(), fill, on_compact: None, opened: false }
    }

    pub fn on_compact(mut self, on_compact: OnCompact) -> ContextRing {
        self.on_compact = Some(on_compact);
        self
    }

    /// A ring that is first drawn with the pointer resting on it, so its card is open: for a walk that
    /// jumps to a state, and a screen that restores one.
    pub fn opened(mut self) -> ContextRing {
        self.opened = true;
        self
    }
}

impl RenderOnce for ContextRing {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let opened = self.opened;
        let figure = window.use_keyed_state((ElementId::from("context-ring"), self.id), cx, move |window, cx| {
            let mut figure = Figure::new(id, window, cx);
            figure.show_open = opened;
            figure
        });
        figure.update(cx, |figure, cx| figure.set(self.fill, self.on_compact, window, cx));
        figure
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Card,
    Popover,
    Tip,
}

fn form_of(fill: &Fill) -> Form {
    if fill.breakdown.is_some() {
        Form::Card
    } else if fill.compact.is_some() {
        Form::Popover
    } else {
        Form::Tip
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Open {
    /// Hover opened it, and leaving shuts it.
    Hover,
    /// A press or Enter kept it.
    Pinned,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hover {
    Ring,
    Compact,
}

/// The part of the ring a Job keeps: what is open, what is pending, and where the keys were.
pub struct Figure {
    id: SharedString,
    fill: Option<Fill>,
    on_compact: Option<OnCompact>,
    open: Option<Open>,
    hover: Option<Hover>,
    over_card: bool,
    pending: bool,
    refusal: Option<SharedString>,
    popup_focus: FocusHandle,
    /// The ring's own keys: they open its card as a hover does.
    ring_focus: FocusHandle,
    /// The card the keys opened, which they shut as they leave.
    by_keys: bool,
    /// Esc shut the card while the keys stayed on the ring: it does not open again until they leave.
    quiet: bool,
    _ring_focus: [Subscription; 2],
    /// Open the card or the tip on the first fill, as a pointer resting on the ring would.
    show_open: bool,
    keys_were: Option<FocusHandle>,
    _leaving: Option<Task<()>>,
    ring_at: Rc<Cell<Option<Bounds<Pixels>>>>,
    compact_at: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl Figure {
    fn new(id: SharedString, window: &mut Window, cx: &mut Context<Self>) -> Figure {
        let ring_focus = cx.focus_handle().tab_stop(true);
        let arrived = cx.on_focus_in(&ring_focus, window, |this, window, cx| this.ring_focused(true, window, cx));
        let left = cx.on_focus_out(&ring_focus, window, |this, _, window, cx| this.ring_focused(false, window, cx));
        Figure {
            ring_focus,
            by_keys: false,
            quiet: false,
            _ring_focus: [arrived, left],
            id,
            fill: None,
            on_compact: None,
            open: None,
            hover: None,
            over_card: false,
            pending: false,
            refusal: None,
            popup_focus: cx.focus_handle(),
            show_open: false,
            keys_were: None,
            _leaving: None,
            ring_at: Rc::default(),
            compact_at: Rc::default(),
        }
    }

    /// The head's data for this frame. A change redraws once; a Compact that was refused, or that
    /// moved the window, ends the pending state.
    fn set(&mut self, fill: Option<Fill>, on_compact: Option<OnCompact>, window: &mut Window, cx: &mut Context<Self>) {
        self.on_compact = on_compact;
        if self.fill == fill {
            return;
        }
        if let Some(new) = &fill {
            // A form with no card to hold the keys puts them back where they were.
            if self.open.is_some() && self.fill.as_ref().is_some_and(|old| form_of(old) != form_of(new)) {
                self.shut(window, cx);
            }
            let old = self.fill.as_ref();
            if old.map(|old| &old.refused) != Some(&new.refused) {
                self.refusal = new.refused.clone();
                if new.refused.is_some() {
                    self.pending = false;
                }
            }
            if self.pending && old.is_some_and(|old| old.tokens != new.tokens) {
                self.pending = false;
                if form_of(new) == Form::Popover {
                    self.shut(window, cx);
                }
            }
        } else {
            self.shut(window, cx);
        }
        if let Some(new) = fill.as_ref().filter(|_| std::mem::take(&mut self.show_open)) {
            match form_of(new) {
                Form::Tip => self.hover = Some(Hover::Ring),
                _ => self.open = Some(Open::Hover),
            }
        }
        self.fill = fill;
        cx.notify();
    }

    fn keep_keys(&mut self, window: &Window, cx: &App) {
        if self.keys_were.is_none() {
            self.keys_were = window.focused(cx);
        }
    }

    fn give_keys_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(was) = self.keys_were.take() {
            window.focus(&was, cx);
        }
    }

    fn shut(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.by_keys = false;
        self.open = None;
        self.hover = None;
        self.over_card = false;
        self.give_keys_back(window, cx);
        cx.notify();
    }

    fn pin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.keep_keys(window, cx);
        self.open = Some(Open::Pinned);
        let focus = self.popup_focus.clone();
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
        cx.notify();
    }

    /// Hover opens the card; leaving shuts it unless pinned, after the time it takes to cross to the card.
    fn ring_hovered(&mut self, over: bool, form: Form, window: &mut Window, cx: &mut Context<Self>) {
        match form {
            Form::Tip => {
                self.hover = over.then_some(Hover::Ring);
                cx.notify();
            }
            Form::Popover => {}
            Form::Card if over => {
                self._leaving = None;
                if self.open.is_none() {
                    self.keep_keys(window, cx);
                    self.open = Some(Open::Hover);
                    cx.notify();
                }
            }
            Form::Card => {
                if self.open == Some(Open::Hover) {
                    self._leaving = Some(cx.spawn_in(window, async move |this, cx| {
                        cx.background_executor().timer(Duration::from_millis(150)).await;
                        let _ = this.update_in(cx, |this, window, cx| {
                            if !this.over_card && this.open == Some(Open::Hover) {
                                this.open = None;
                                this.give_keys_back(window, cx);
                                cx.notify();
                            }
                        });
                    }));
                }
            }
        }
    }

    fn ring_pressed(&mut self, form: Form, window: &mut Window, cx: &mut Context<Self>) {
        if form == Form::Tip {
            return;
        }
        self.by_keys = false;
        if self.open == Some(Open::Pinned) {
            self.shut(window, cx);
        } else {
            self.pin(window, cx);
        }
    }

    /// Compact is pending in this frame; the head is told to send it.
    fn compact_pressed(&mut self, how: CompactHow, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending {
            return;
        }
        self.pending = true;
        self.refusal = None;
        cx.notify();
        if let Some(on_compact) = self.on_compact.clone() {
            on_compact(how, window, cx);
        }
        // The popover goes once Compact is sent (`CompactButton`'s `after`); the card stays to show it pending.
        if self.fill.as_ref().is_some_and(|fill| form_of(fill) == Form::Popover) {
            self.shut(window, cx);
        }
    }

    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open.is_some() {
            self.by_keys = false;
            self.shut(window, cx);
        }
    }

    /// The keys arrived on the ring or left it: from the keyboard the card opens as a hover opens it, and
    /// goes as the keys go.
    fn ring_focused(&mut self, arrived: bool, window: &mut Window, cx: &mut Context<Self>) {
        let card = self.fill.as_ref().is_some_and(|fill| form_of(fill) == Form::Card);
        if arrived {
            if card && window.last_input_was_keyboard() && !self.quiet && self.open.is_none() {
                self.by_keys = true;
                self.ring_hovered(true, Form::Card, window, cx);
            }
        } else {
            self.quiet = false;
            if self.by_keys && self.open == Some(Open::Hover) && !self.over_card {
                self.by_keys = false;
                self.open = None;
                self.keys_were = None;
                cx.notify();
            }
        }
    }
}

// -- words -------------------------------------------------------------------------------------

/// `53840` as `53.8k`, `5000` as `5.0k`, `142000` as `142k`, `1000000` as `1M`.
fn tokens_said(n: u64) -> String {
    if n >= 1_000_000 {
        let m = (n as f64 / 1_000_000.0 * 10.0).round() / 10.0;
        return if m.fract() == 0.0 { format!("{}M", m as u64) } else { format!("{m}M") };
    }
    if n < 1_000 {
        return n.to_string();
    }
    let k = n as f64 / 1_000.0;
    if k >= 100.0 {
        format!("{}k", k.round() as u64)
    } else if k >= 10.0 {
        let tenth = (k * 10.0).round() / 10.0;
        if tenth.fract() == 0.0 { format!("{}k", tenth as u64) } else { format!("{tenth}k") }
    } else {
        format!("{k:.1}k")
    }
}

/// Why Compact is off, as `session-tiles.ts` says it.
fn why_said(why: CompactOff) -> &'static str {
    match why {
        CompactOff::Ended => "The Job ended",
        CompactOff::NotListening => "The terminal is not listening",
        CompactOff::TerminalNotListed => "The terminal has not listed compact",
        CompactOff::AgentNotListed => "The agent has not listed compact",
    }
}

fn tone_of(band: RingBand, look: &Look) -> Hsla {
    match band {
        RingBand::Normal => look.color(token::FG_MUTED),
        RingBand::Warning => look.color(token::STATUS_AWAITING_REVIEW),
        RingBand::Danger => look.color(token::STATUS_COMPLETED_FAILED),
    }
}

/// The heaviest three, by tokens.
fn heaviest<T>(mut items: Vec<(T, u64)>) -> Vec<(T, u64)> {
    items.sort_by(|a, b| b.1.cmp(&a.1));
    items.truncate(3);
    items
}

/// A path's last segment, the last two where another file shares it.
fn file_name(path: &str, all: &[ipc::ContextMemoryFile]) -> String {
    let last = path.rsplit('/').next().unwrap_or(path);
    if all.iter().filter(|one| one.path.rsplit('/').next() == Some(last)).count() > 1 {
        path.rsplit('/').take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("/")
    } else {
        last.to_string()
    }
}

/// MCP servers by what their tools weigh.
fn heaviest_servers(tools: &[ipc::ContextMcpTool]) -> Vec<(String, u64)> {
    let mut by: HashMap<&str, u64> = HashMap::new();
    for tool in tools {
        *by.entry(tool.server.as_str()).or_default() += tool.tokens;
    }
    heaviest(by.into_iter().map(|(name, tokens)| (name.to_string(), tokens)).collect())
}

/// The threshold as a share of the window, where there is one to mark.
fn threshold_of(fill: &Fill) -> Option<f32> {
    let at = fill.breakdown.as_ref()?.autocompact_threshold?;
    (fill.window > 0 && at > 0 && at < fill.window).then(|| at as f32 / fill.window as f32)
}

// -- drawing -----------------------------------------------------------------------------------

/// The ring: a track and an arc laid from twelve o'clock to the fill, a tick where the autocompact
/// threshold is. 16 square, centre 8, radius 5.25, stroke 2 (`SessionFrame.css`, `.armada-context-ring`).
fn ring_canvas(fill: &Fill, tone: Hsla, track: Hsla, tick: Hsla, side: Pixels) -> impl IntoElement {
    let share = (fill.tokens as f32 / fill.window as f32).clamp(0., 1.);
    let threshold = threshold_of(fill);
    canvas(
        |_, _, _| {},
        move |bounds: Bounds<Pixels>, _, window, _| {
            let scale = f32::from(side) / 16.;
            let centre = bounds.center();
            let radius = 5.25 * scale;
            let on = |radius: f32, turn: f32| -> Point<Pixels> {
                let angle = turn * std::f32::consts::TAU;
                point(centre.x + px(radius * angle.sin()), centre.y - px(radius * angle.cos()))
            };
            let stroke = px(2. * scale);
            let mut whole = PathBuilder::stroke(stroke);
            whole.move_to(point(centre.x + px(radius), centre.y));
            whole.arc_to(point(px(radius), px(radius)), px(0.), false, true, point(centre.x - px(radius), centre.y));
            whole.arc_to(point(px(radius), px(radius)), px(0.), false, true, point(centre.x + px(radius), centre.y));
            whole.close();
            if let Ok(path) = whole.build() {
                window.paint_path(path, track);
            }
            if share >= 0.999 {
                let mut full = PathBuilder::stroke(stroke);
                full.move_to(point(centre.x + px(radius), centre.y));
                full.arc_to(point(px(radius), px(radius)), px(0.), false, true, point(centre.x - px(radius), centre.y));
                full.arc_to(point(px(radius), px(radius)), px(0.), false, true, point(centre.x + px(radius), centre.y));
                full.close();
                if let Ok(path) = full.build() {
                    window.paint_path(path, tone);
                }
            } else if share > 0. {
                let mut arc = PathBuilder::stroke(stroke);
                arc.move_to(on(radius, 0.));
                arc.arc_to(point(px(radius), px(radius)), px(0.), share > 0.5, true, on(radius, share));
                if let Ok(path) = arc.build() {
                    window.paint_path(path, tone);
                }
            }
            if let Some(at) = threshold {
                let mut line = PathBuilder::stroke(px(scale));
                line.move_to(on(3.5 * scale, at));
                line.line_to(on(7. * scale, at));
                if let Ok(path) = line.build() {
                    window.paint_path(path, tick);
                }
            }
        },
    )
    .size_full()
}

/// The window as one slim row: a segment for each category in it, the room left, and the autocompact
/// buffer. The categories step down from the card's tone.
fn window_bar(breakdown: &ContextBreakdown, tone: Hsla, look: &Look) -> impl IntoElement {
    const STEPS: [f32; 6] = [1., 0.8, 0.65, 0.5, 0.4, 0.3];
    let shown: Vec<_> = breakdown.categories.iter().filter(|one| one.kind != ContextKind::Deferred && one.tokens > 0).collect();
    div()
        .flex()
        .gap(look.length(token::BORDER_WIDTH))
        .h(look.length(token::SPACE_1))
        .my(look.length(token::SPACE_1))
        .overflow_hidden()
        .rounded(look.length(token::RADIUS_SM))
        .children(shown.into_iter().enumerate().map(|(at, one)| {
            let ground = match one.kind {
                ContextKind::Free => look.color(token::BG_SUNKEN),
                ContextKind::Buffer => look.color(token::BORDER_DEFAULT),
                _ => tone.opacity(STEPS[at.min(STEPS.len() - 1)]),
            };
            div().flex_grow(one.tokens as f32).flex_shrink(1.).flex_basis(px(0.)).min_w(look.length(token::BORDER_WIDTH)).h_full().bg(ground)
        }))
}

/// A tooltip: one line on the overlay ground.
fn tip_card(said: impl Into<SharedString>, look: &Look) -> gpui::Div {
    card(look)
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .child(said.into())
}

impl Figure {
    fn focus_ring(look: &Look) -> Hsla {
        look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour)
    }

    /// Compact at the foot of the card or the popover: pending in this frame, off with its reason on hover.
    fn compact_button(&self, offer: CompactOffer, look: &Look, cx: &Context<Self>) -> gpui::AnyElement {
        let (off, how) = match offer {
            CompactOffer::Offered { how } => (None, Some(how)),
            CompactOffer::Off { why } => (Some(why_said(why)), None),
        };
        let at = Rc::clone(&self.compact_at);
        let body = Btn::new("context-compact", "Compact", Weight::Secondary)
            .disabled(off.is_some())
            .pending(self.pending)
            .draw(look, cx)
            .child(canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
            .relative()
            .on_hover(cx.listener(|this, over: &bool, _, cx| {
                this.hover = over.then_some(Hover::Compact);
                cx.notify();
            }));
        let body = match how {
            Some(how) if !self.pending => body.on_click(cx.listener(move |this, _, window, cx| this.compact_pressed(how, window, cx))),
            _ => body,
        };
        labelled(body, Role::Button, "Compact").into_any_element()
    }

    /// What the last Compact was refused for, under the button.
    fn refusal_line(&self, look: &Look) -> Option<gpui::AnyElement> {
        let said = self.refusal.clone()?;
        Some(
            labelled(
                div()
                    .id("context-refused")
                    .w_full()
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .justify_end()
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                    .child(div().flex_none().h(look.length(token::LEADING_2XS)).flex().items_center().child(glyph("circle-x", look.length(token::SPACE_3), look.color(token::STATUS_COMPLETED_FAILED))))
                    // Fleet's words can run long: they wrap inside the card.
                    .child(div().min_w_0().child(said.clone())),
                Role::Alert,
                said,
            )
            .into_any_element(),
        )
    }

    /// The card: Context, the window, each category, the heaviest memory files and MCP servers, and Compact.
    fn breakdown_card(&self, fill: &Fill, breakdown: &ContextBreakdown, look: &Look, cx: &Context<Self>) -> gpui::AnyElement {
        let tone = tone_of(fill.band, look);
        let wide = fill.band != RingBand::Normal;
        let facts_of = |rows: Vec<(String, u64)>| inspect::facts(rows.into_iter().map(|(term, tokens)| Fact::new(term, tokens_said(tokens)).mono()).collect(), look);
        let categories: Vec<(String, u64)> =
            breakdown.categories.iter().filter(|one| one.kind != ContextKind::Deferred).map(|one| (one.name.clone(), one.tokens)).collect();
        let files = heaviest(breakdown.memory_files.iter().map(|one| (file_name(&one.path, &breakdown.memory_files), one.tokens)).collect());
        let servers = heaviest_servers(&breakdown.mcp_tools);
        let spec = Spec {
            kind_glyph: "layers",
            kind: "Context".into(),
            place: Some(format!("{} / {}", tokens_said(fill.tokens), tokens_said(fill.window)).into()),
            title: "Context window".into(),
            state: Some(State { glyph: None, said: format!("{}% used", fill.percent).into(), ink: tone }),
            edge: wide.then_some(tone),
            width: Width::Wide,
            keys: vec![("Enter", "Keep open")],
        };
        let mut body: Vec<gpui::AnyElement> = vec![
            window_bar(breakdown, tone, look).into_any_element(),
            facts_of(categories).into_any_element(),
            inspect::heading("Memory files", look).into_any_element(),
            facts_of(files).into_any_element(),
            inspect::heading("MCP tools", look).into_any_element(),
            facts_of(servers).into_any_element(),
        ];
        if let Some(offer) = fill.compact {
            body.push(
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap(look.length(token::SPACE_2))
                    .pt(look.length(token::SPACE_2))
                    .child(self.compact_button(offer, look, cx))
                    .children(self.refusal_line(look))
                    .into_any_element(),
            );
        }
        self.popup_frame(spec.draw(body, look), "Context", Role::Region, cx)
    }

    /// The popover of a ring with no breakdown: the fill in words and Compact.
    fn popover_body(&self, fill: &Fill, offer: CompactOffer, look: &Look, cx: &Context<Self>) -> gpui::AnyElement {
        let body = card(look)
            .flex_row()
            .items_center()
            .gap(look.length(token::SPACE_4))
            .p(look.length(token::SPACE_3))
            .child(
                div()
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(format!("{} of {} tokens", tokens_said(fill.tokens), tokens_said(fill.window))),
            )
            .child(div().flex().flex_col().items_end().gap(look.length(token::SPACE_2)).child(self.compact_button(offer, look, cx)).children(self.refusal_line(look)));
        self.popup_frame(body, "Context", Role::Dialog, cx)
    }

    /// The shell of an open card or popover: it takes the keys while it is up, Esc shuts it and Enter keeps it.
    fn popup_frame(&self, body: gpui::Div, name: &'static str, role: Role, cx: &Context<Self>) -> gpui::AnyElement {
        labelled(
            div()
                .id(SharedString::from(format!("context-popup-{}", self.id)))
                .track_focus(&self.popup_focus)
                .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| match event.keystroke.key.as_str() {
                    "escape" => {
                        this.escape(window, cx);
                        cx.stop_propagation();
                    }
                    "enter" if this.open == Some(Open::Hover) => {
                        this.open = Some(Open::Pinned);
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
                    if this.open.is_some() {
                        this.shut(window, cx);
                    }
                }))
                .occlude()
                .child(body),
            role,
            name,
        )
        .into_any_element()
    }
}

impl Render for Figure {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(fill) = self.fill.clone() else { return Empty.into_any_element() };
        let look = cx.look();
        // What floats off the ring hangs from its trailing edge, so it opens leftward: under it, or over it where
        // the ring sits in the window's lower half, as a Radix popover flips (the toolbelt is at the foot).
        let half = window.viewport_size().height / 2.;
        let gap = look.length(token::SPACE_1);
        let hang = move |from: Bounds<Pixels>| {
            let right = from.origin.x + from.size.width;
            if from.origin.y > half {
                (point(right, from.origin.y - gap), Anchor::BottomRight)
            } else {
                (point(right, from.origin.y + from.size.height + gap), Anchor::TopRight)
            }
        };
        let tone = tone_of(fill.band, look);
        let form = form_of(&fill);
        let side = look.length(token::SPACE_4);
        let ring_ink = Self::focus_ring(look);
        let at = Rc::clone(&self.ring_at);
        let name: SharedString = format!("Context {}%", fill.percent).into();
        let face = |body: gpui::Stateful<gpui::Div>| {
            body.flex_none()
                .relative()
                .h(look.length(token::H_CONTROL))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(tone)
                .child(canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                .child(
                    div().flex_none().size(side).child(ring_canvas(
                        &fill,
                        tone,
                        look.color(token::BORDER_DEFAULT),
                        look.color(token::FG_SUBTLE).opacity(0.6),
                        side,
                    )),
                )
                .child(format!("{}%", fill.percent))
        };
        let id = SharedString::from(format!("context-ring-{}", self.id));
        let trigger = match form {
            // The picture still holds the focus a button of the same id had, so a form that changes under
            // a pinned card does not leave the keys on nothing.
            Form::Tip => labelled(
                face(div().id(id)).focusable().on_hover(cx.listener(move |this, over: &bool, window, cx| this.ring_hovered(*over, form, window, cx))),
                Role::Image,
                name.clone(),
            ),
            _ => labelled(
                face(div().id(id))
                    .cursor_pointer()
                    .track_focus(&self.ring_focus)
                    .focus_visible(move |style| style.border_color(ring_ink))
                    // Esc puts a card the keys opened away, and the keys stay on the ring.
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                        if event.keystroke.key == "escape" && this.open == Some(Open::Hover) {
                            this.quiet = true;
                            this.escape(window, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .on_hover(cx.listener(move |this, over: &bool, window, cx| this.ring_hovered(*over, form, window, cx)))
                    .on_click(cx.listener(move |this, _, window, cx| this.ring_pressed(form, window, cx))),
                Role::Button,
                name.clone(),
            ),
        };
        let popup = self.open.and_then(|_| {
            let from = self.ring_at.get()?;
            let body = match (form, fill.breakdown.as_ref(), fill.compact) {
                (Form::Card, Some(breakdown), _) => self.breakdown_card(&fill, breakdown, look, cx),
                (Form::Popover, _, Some(offer)) => self.popover_body(&fill, offer, look, cx),
                _ => return None,
            };
            let (at, anchor) = hang(from);
            Some(deferred(anchored().position(at).anchor(anchor).snap_to_window_with_margin(look.length(token::SPACE_1)).child(body)).with_priority(1).into_any_element())
        });
        let tip = match self.hover {
            Some(Hover::Ring) if form == Form::Tip => self.ring_at.get().map(|from| {
                let (at, anchor) = hang(from);
                let said = format!("Context · {} of {} tokens", tokens_said(fill.tokens), tokens_said(fill.window));
                deferred(
                    anchored().position(at).anchor(anchor).snap_to_window_with_margin(look.length(token::SPACE_1)).child(labelled(
                        div().id("context-tip").child(tip_card(said.clone(), look)),
                        Role::Tooltip,
                        said,
                    )),
                )
                .with_priority(1)
                .into_any_element()
            }),
            Some(Hover::Compact) => match (fill.compact, self.compact_at.get()) {
                (Some(offer), Some(from)) => {
                    let said = match offer {
                        CompactOffer::Offered { .. } => "Compact the Job's context",
                        CompactOffer::Off { why } => why_said(why),
                    };
                    let (at, anchor) = hang(from);
                    Some(
                        deferred(
                            anchored().position(at).anchor(anchor).snap_to_window_with_margin(look.length(token::SPACE_1)).child(labelled(
                                div().id("compact-tip").child(tip_card(said, look)),
                                Role::Tooltip,
                                said,
                            )),
                        )
                        .with_priority(2)
                        .into_any_element(),
                    )
                }
                _ => None,
            },
            _ => None,
        };
        div().relative().flex_none().child(trigger).children(popup).children(tip).into_any_element()
    }
}
