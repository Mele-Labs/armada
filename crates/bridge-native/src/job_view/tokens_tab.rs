//! A Job's tokens from start to finish as a tab of its panes: the burn of each stretch of time split into the main thread and its
//! subagents, the main thread's context, where it was compacted and where it waited. `job_tokens/` works the graph out and paints it;
//! this holds the tab: its cursor, its keys, its tooltip and its place among the Job's panes.
//!
//! **Each Job has one reading**, which the Context card under the composer's ring shares with the tab, so a card opened first has
//! already read what the tab draws. Both are held app-wide, so the Job view carries no field of them.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use chrono::Local;
use gpui::{
    div, prelude::*, relative, AnyElement, App, Bounds, Context, Entity, EntityId, FocusHandle, FontWeight, Global, MouseMoveEvent, Pixels, Role, SharedString, Subscription, Task,
    WeakEntity, Window,
};

use super::pane_tree::{self as tree, Tab};
use super::JobView;
use crate::fleet::FleetConnection;
use crate::generated::actions::ActionScope;
use crate::job_tokens::{self, bones, chart, margins, Graph, Held, TimelineRead};
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::settings::view::refusal;
use crate::titlebar::parts::{card, mono, sans};

/// The tab's kind in a Job's panes.
pub(super) const KIND: &str = "tokens";

/// How often a live Job's tab asks again while it is drawn.
const EVERY: Duration = Duration::from_secs(60);

/// Each Job view's Tokens tabs, by Job and then by tab id, and each Job's reading.
#[derive(Default)]
struct Held_ {
    tabs: HashMap<(EntityId, String), HashMap<String, Entity<TokensTab>>>,
    reads: HashMap<(EntityId, String), Entity<TimelineRead>>,
}

impl Global for Held_ {}

pub struct TokensTab {
    job: SharedString,
    read: Entity<TimelineRead>,
    view: WeakEntity<JobView>,
    /// The bucket the cursor is on, from the keys or the pointer.
    pub cursor: Option<usize>,
    /// Where the chart was laid out, so a pointer over it reads as a bucket.
    plot: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub focus: FocusHandle,
    /// Set as the tab draws and cleared as the clock ticks, so a tab nobody sees does not ask.
    drawn: bool,
    _poll: Task<()>,
    _watch: Subscription,
}

impl TokensTab {
    fn new(job: SharedString, read: Entity<TimelineRead>, view: WeakEntity<JobView>, cx: &mut Context<Self>) -> TokensTab {
        read.update(cx, |read, cx| read.wake(cx));
        let watch = cx.observe(&read, |_, _, cx| cx.notify());
        let poll = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(EVERY).await;
            let Ok(()) = this.update(cx, |this, cx| {
                if std::mem::take(&mut this.drawn) && this.read.read(cx).live() {
                    this.read.update(cx, |read, cx| read.read(cx));
                }
            }) else {
                break;
            };
        });
        TokensTab { job, read, view, cursor: None, plot: Rc::default(), focus: cx.focus_handle(), drawn: false, _poll: poll, _watch: watch }
    }

    fn graph(&self, cx: &App) -> Option<Rc<Graph>> {
        self.read.read(cx).graph().cloned()
    }

    /// → and ←: the cursor moves a bucket, held at either end.
    pub fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(graph) = self.graph(cx) else { return };
        let next = graph.step(self.cursor, by);
        if next != self.cursor {
            crate::ui_sounds::moved();
            self.cursor = next;
            cx.notify();
        }
    }

    /// The pointer over the chart at `x`: the cursor is on the bucket under it.
    fn hover(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let (Some(graph), Some(bounds)) = (self.graph(cx), self.plot.get()) else { return };
        let across = f32::from((x - bounds.origin.x) / bounds.size.width);
        let next = graph.bucket_at(across);
        if next != self.cursor {
            self.cursor = next;
            cx.notify();
        }
    }

    fn leave(&mut self, cx: &mut Context<Self>) {
        if self.cursor.take().is_some() {
            cx.notify();
        }
    }

    fn on_act(&mut self, act: &Act, _: &mut Window, cx: &mut Context<Self>) {
        match act.id {
            "tokens_move" => self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            _ => cx.propagate(),
        }
    }

    fn title(&self, cx: &App) -> Option<SharedString> {
        self.view.upgrade().and_then(|view| view.read(cx).title_of(&self.job, cx))
    }
}

// ---- drawing --------------------------------------------------------------------------------

fn swatch(look: &Look, ink: Token) -> gpui::Div {
    div().flex_none().size(look.length(token::SPACE_3)).rounded(look.length(token::RADIUS_SM)).bg(look.color(ink))
}

/// What the legend names, each with the mark the chart draws it with.
fn legend(look: &Look) -> AnyElement {
    let hair = look.length(token::BORDER_WIDTH);
    let line = div().flex_none().w(look.length(token::SPACE_4)).h(hair * 1.5).bg(look.color(token::SKETCH_GREY));
    let dashes = div().flex_none().h(look.length(token::SPACE_3)).flex().flex_col().justify_between().children((0..3).map(|_| div().w(hair).h(look.length(token::SPACE_1) / 2.).bg(look.color(token::FG_SUBTLE))));
    let band = div().flex_none().size(look.length(token::SPACE_3)).bg(look.color(token::STATUS_AWAITING_REVIEW).opacity(0.16));
    let dot = div().flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::STATUS_ESCALATED));
    let items: [(&str, AnyElement); 6] = [
        ("Main thread", swatch(look, token::SKETCH_BLUE).into_any_element()),
        ("Subagents", swatch(look, token::SKETCH_ORANGE).into_any_element()),
        ("Context", line.into_any_element()),
        ("Compaction", dashes.into_any_element()),
        ("Pause", band.into_any_element()),
        ("Cold wake", dot.into_any_element()),
    ];
    labelled(
        div()
            .id("tokens-legend")
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(look.length(token::SPACE_4))
            .gap_y(look.length(token::SPACE_1))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .children(items.into_iter().map(|(said, mark)| {
                labelled(div().id(SharedString::from(format!("tokens-legend-{said}"))).flex().items_center().gap(look.length(token::SPACE_2)).child(mark).child(said), Role::Label, said)
            })),
        Role::Group,
        "Legend",
    )
    .into_any_element()
}

/// The four figures under the chart: a label over the value.
fn figures(graph: &Graph, look: &Look) -> AnyElement {
    labelled(
        div().id("tokens-figures").flex_none().flex().flex_wrap().gap_x(look.length(token::SPACE_6)).gap_y(look.length(token::SPACE_3)).children(graph.figures().into_iter().map(|(name, value)| {
            labelled(
                div()
                    .id(SharedString::from(format!("tokens-figure-{name}")))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(name))
                    .child(
                        div()
                            .font_family(mono(look))
                            .text_size(look.length(token::TEXT_LG))
                            .line_height(look.length(token::LEADING_LG))
                            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(SharedString::from(value.clone())),
                    ),
                Role::Group,
                format!("{name}: {value}"),
            )
        })),
        Role::Group,
        "Figures",
    )
    .into_any_element()
}

/// What stands where the figures will be.
fn figure_bones(look: &Look) -> AnyElement {
    div().flex_none().flex().gap_x(look.length(token::SPACE_6)).children((0..4).map(|_| {
        div().flex().flex_col().gap(look.length(token::SPACE_1)).child(crate::settings::view::bone(look, look.length(token::SPACE_12), look.length(token::LEADING_XS))).child(crate::settings::view::bone(look, look.length(token::SPACE_12) * 1.5, look.length(token::LEADING_LG)))
    }))
    .into_any_element()
}

/// The tooltip of the bucket under the cursor, beside its rule.
fn tooltip(graph: &Graph, at: usize, look: &Look) -> Option<AnyElement> {
    let tipped = graph.tip_in(at, &Local)?;
    let centre = graph.centre_of(at)?;
    let said = tipped.said();
    let line = |text: String, ink: Token| div().text_color(look.color(ink)).child(SharedString::from(text));
    let body = card(look)
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .gap(look.length(token::SPACE_1))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(tipped.time.clone())))
        .child(line(tipped.share.clone(), token::FG_DEFAULT))
        .child(line(tipped.split.clone(), token::FG_MUTED))
        .children(tipped.context.clone().map(|text| line(text, token::FG_MUTED)))
        .children(tipped.pause.clone().map(|text| line(text, token::STATUS_AWAITING_REVIEW)));
    let margin = margins(graph, true, look);
    let gap = look.length(token::SPACE_2);
    // The tooltip stands beside the rule, on the side with more room, so it never covers the bucket it names.
    let placed = div().id("tokens-tooltip").absolute().top(margin.strip + gap).max_w(relative(0.5));
    let placed = if centre > 0.55 { placed.right(relative(1. - centre)).mr(gap) } else { placed.left(relative(centre)).ml(gap) };
    Some(labelled(placed.child(body), Role::Tooltip, said).into_any_element())
}

impl gpui::Render for TokensTab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.drawn = true;
        let look = cx.look().clone();
        let look = &look;
        let title = self.title(cx);
        let read = self.read.read(cx);
        let graph = read.graph().cloned();
        let failed = match &read.held {
            Held::Failed(why) if graph.is_none() => Some(why.clone()),
            _ => None,
        };
        let head = match (&graph, title) {
            (Some(graph), title) => {
                let range = graph.header_in(&Local);
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .children(title.map(|title| {
                        labelled(
                            div()
                                .id("tokens-title")
                                .text_size(look.length(token::TEXT_BASE))
                                .line_height(look.length(token::LEADING_BASE))
                                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                .text_color(look.color(token::FG_DEFAULT))
                                .child(title.clone()),
                            Role::Heading,
                            title,
                        )
                    }))
                    .child(labelled(
                        div().id("tokens-range").text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(range.clone())),
                        Role::Label,
                        range,
                    ))
            }
            _ => div().flex_none().flex().flex_col().gap(look.length(token::SPACE_2)).child(crate::settings::view::bone(look, look.length(token::SPACE_12) * 4., look.length(token::LEADING_LG))).child(crate::settings::view::bone(
                look,
                look.length(token::SPACE_12) * 3.,
                look.length(token::LEADING_SM),
            )),
        };
        let field = match (&graph, &failed) {
            (Some(graph), _) => {
                let tipped = self.cursor.and_then(|at| tooltip(graph, at, look));
                let me = cx.entity().downgrade();
                let hover = move |event: &MouseMoveEvent, _: &mut Window, cx: &mut App| {
                    if let Some(me) = me.upgrade() {
                        me.update(cx, |this, cx| this.hover(event.position.x, cx));
                    }
                };
                labelled(
                    div()
                        .id("tokens-chart")
                        .relative()
                        .flex_1()
                        .min_h(look.length(token::SPACE_12) * 9.)
                        .w_full()
                        .on_mouse_move(hover)
                        .on_hover(cx.listener(|this, over: &bool, _, cx| {
                            if !over {
                                this.leave(cx);
                            }
                        }))
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| window.focus(&this.focus, cx)))
                        .child(chart(graph.clone(), true, self.cursor, look, Rc::clone(&self.plot)))
                        .children(tipped),
                    Role::Image,
                    "Tokens over time",
                )
                .into_any_element()
            }
            (None, Some(why)) => labelled(
                div().id("tokens-failed").flex_1().flex().flex_col().gap(look.length(token::SPACE_1)).child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_size(look.length(token::TEXT_SM)).child("Not read")).child(refusal(look, why.clone())),
                Role::Alert,
                "Not read",
            )
            .into_any_element(),
            (None, None) => labelled(div().id("tokens-loading").flex_1().min_h(look.length(token::SPACE_12) * 9.).child(bones(look, look.length(token::SPACE_12) * 9.)), Role::Image, "Loading").into_any_element(),
        };
        let foot = graph.as_ref().map_or_else(|| figure_bones(look), |graph| figures(graph, look));
        // A pane too short for the chart scrolls rather than squeezing it; a tall one gives it the room.
        div()
            .id(SharedString::from(format!("tokens-tab-{}", self.job)))
            .key_context(ActionScope::Tokens.key_context().unwrap_or_default())
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::on_act))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && this.cursor.is_some() {
                    this.leave(cx);
                    cx.stop_propagation();
                }
            }))
            .size_full()
            .overflow_y_scroll()
            .bg(look.color(token::BG_BASE))
            .font_family(sans(look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(div().min_h_full().flex().flex_col().gap(look.length(token::SPACE_4)).p(look.length(token::SPACE_4)).child(head).child(legend(look)).child(field).child(foot))
    }
}

// ---- the Job view's hooks -------------------------------------------------------------------

impl JobView {
    /// The reading Job `job` keeps, made once and shared by its card and its tab.
    pub(super) fn timeline_read(&self, job: &SharedString, cx: &mut Context<Self>) -> Entity<TimelineRead> {
        let key = (cx.entity_id(), job.to_string());
        if let Some(held) = cx.try_global::<Held_>().and_then(|all| all.reads.get(&key)) {
            return held.clone();
        }
        let fleet: Option<Entity<FleetConnection>> = self.fleet.clone();
        let read = cx.new(|_| TimelineRead::new(job.clone(), fleet));
        cx.default_global::<Held_>().reads.insert(key, read.clone());
        read
    }

    /// What the Context card of Job `job` is handed for its graph. None until the Job's reading is made, as the Job draws.
    pub(super) fn tokens_hook(&self, job: &SharedString, cx: &Context<Self>) -> Option<job_tokens::Hook> {
        let read = cx.try_global::<Held_>()?.reads.get(&(cx.entity_id(), job.to_string()))?.clone();
        let me = cx.entity().downgrade();
        Some(job_tokens::Hook {
            read,
            open: Rc::new(move |window, cx| {
                if let Some(me) = me.upgrade() {
                    me.update(cx, |this, cx| this.open_tokens(window, cx));
                }
            }),
        })
    }

    fn tokens_ids(&self, job: &str) -> Vec<String> {
        let Some(arrangement) = self.arrangement_of(job) else { return Vec::new() };
        tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).filter(|tab| tab.kind == KIND).map(|tab| tab.id.clone()).collect()
    }

    /// Each Tokens tab Job `job` has gets its entity, once; one whose tab closed goes with it.
    fn sync_tokens(&self, job: &SharedString, cx: &mut Context<Self>) {
        let me = cx.entity_id();
        let ids = self.tokens_ids(job);
        let read = if ids.is_empty() { None } else { Some(self.timeline_read(job, cx)) };
        let weak = cx.entity().downgrade();
        let all = cx.default_global::<Held_>();
        let mut held = all.tabs.remove(&(me, job.to_string())).unwrap_or_default();
        held.retain(|id, _| ids.contains(id));
        for id in ids {
            if let (false, Some(read)) = (held.contains_key(&id), read.as_ref()) {
                let tab = cx.new(|cx| TokensTab::new(job.clone(), read.clone(), weak.clone(), cx));
                held.insert(id, tab);
            }
        }
        if !held.is_empty() {
            cx.default_global::<Held_>().tabs.insert((me, job.to_string()), held);
        }
    }

    fn held_tokens(&self, job: &str, tab: &str, cx: &Context<Self>) -> Option<Entity<TokensTab>> {
        cx.try_global::<Held_>()?.tabs.get(&(cx.entity_id(), job.to_string()))?.get(tab).cloned()
    }

    /// The body of a Tokens tab in front, its entity made as it first draws.
    pub(super) fn tokens_body(&self, job: &SharedString, tab: &Tab, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.sync_tokens(job, cx);
        let held = self.held_tokens(job, &tab.id, cx)?;
        Some(div().id(SharedString::from(format!("pane-body:{}", tab.id))).flex_1().min_h_0().flex().flex_col().child(div().size_full().child(held)).into_any_element())
    }

    /// Open the Tokens tab of the Job shown, and the keys in it: the one the Job already has comes to the front, and none makes one.
    pub(super) fn open_tokens(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let open = self.tokens_ids(&job).into_iter().next();
        match open {
            Some(id) => {
                self.look_in(&job, move |arrangement| tree::activate(arrangement, &id), cx);
            }
            None => self.add_tab(KIND, window, cx),
        }
        self.sync_tokens(&job, cx);
        let front = self.tokens_ids(&job).into_iter().find_map(|id| self.held_tokens(&job, &id, cx));
        if let Some(tab) = front {
            let keys = tab.read(cx).focus.clone();
            window.focus(&keys, cx);
        }
        cx.notify();
    }

    /// As the Job draws: its reading is made for the card, and a press on its row on the Usage page that left an ask for its tab opens it.
    pub(super) fn sync_tokens_link(&mut self, job: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        self.timeline_read(job, cx);
        if job_tokens::asks(job, cx) && job_tokens::take(job, cx) {
            self.open_tokens(window, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{div, TestAppContext, VisualTestContext};

    use super::*;
    use crate::job_tokens::wire::JobTimeline;
    use crate::labels::LabelBook;

    struct Rooted(Entity<TokensTab>);

    impl gpui::Render for Rooted {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            crate::labels::label_root(div().size_full().child(self.0.clone()))
        }
    }

    fn init(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
    }

    /// The tab on a reading in this state, drawn, with the keys on it.
    fn drawn(cx: &mut TestAppContext, held: Held) -> (Entity<TokensTab>, &mut VisualTestContext) {
        init(cx);
        let (root, cx) = cx.add_window_view(move |_, cx| {
            let read = cx.new(|_| TimelineRead::holding("mock-02".into(), held));
            Rooted(cx.new(|cx| TokensTab::new("mock-02".into(), read, WeakEntity::new_invalid(), cx)))
        });
        let tab = root.read_with(cx, |root, _| root.0.clone());
        redraw(cx);
        let focus = tab.read_with(cx, |tab, _| tab.focus.clone());
        cx.update(|window, cx| window.focus(&focus, cx));
        redraw(cx);
        (tab, cx)
    }

    fn graph_of(timeline: JobTimeline) -> Held {
        Held::Read(Rc::new(Graph::new(&timeline)))
    }

    fn redraw(cx: &mut VisualTestContext) {
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
    }

    fn names(cx: &mut VisualTestContext, role: Role) -> Vec<String> {
        cx.update(|window, cx| {
            let drawn = cx.try_global::<LabelBook>().map(|book| book.drawn(window.window_handle().window_id()).to_vec()).unwrap_or_default();
            drawn.iter().filter(|one| one.role == role).map(|one| one.name.to_string()).collect()
        })
    }

    #[gpui::test]
    fn while_the_reading_is_out_shapes_stand_where_the_graph_will_be(cx: &mut TestAppContext) {
        let (_, cx) = drawn(cx, Held::Reading);
        assert!(names(cx, Role::Image).iter().any(|one| one == "Loading"));
        assert!(!names(cx, Role::Image).iter().any(|one| one == "Tokens over time"), "no graph yet");
        assert!(names(cx, Role::Heading).is_empty() && !names(cx, Role::Group).iter().any(|one| one == "Figures"), "and no words or figures");
    }

    #[gpui::test]
    fn a_reading_fleet_refused_says_so_with_its_words(cx: &mut TestAppContext) {
        let (_, cx) = drawn(cx, Held::Failed("Fleet answered 500".into()));
        assert_eq!(names(cx, Role::Alert), ["Not read"]);
        assert!(!names(cx, Role::Image).iter().any(|one| one == "Loading"));
    }

    #[gpui::test]
    fn a_job_that_spent_nothing_draws_its_header_and_figures_and_no_placeholder(cx: &mut TestAppContext) {
        let (tab, cx) = drawn(cx, graph_of(crate::job_tokens::mock_empty()));
        assert!(names(cx, Role::Image).iter().any(|one| one == "Tokens over time"));
        assert!(names(cx, Role::Group).iter().any(|one| one == "Figures"));
        assert!(!names(cx, Role::Image).iter().any(|one| one == "Loading"));
        cx.simulate_keystrokes("right");
        assert_eq!(tab.read_with(cx, |tab, _| tab.cursor), Some(0), "the one bucket it has");
    }

    #[gpui::test]
    fn right_and_left_move_the_cursor_and_its_tooltip_follows(cx: &mut TestAppContext) {
        let (tab, cx) = drawn(cx, graph_of(crate::job_tokens::mock_timeline()));
        assert!(names(cx, Role::Tooltip).is_empty(), "no cursor, no tooltip");
        cx.simulate_keystrokes("right right right");
        assert_eq!(tab.read_with(cx, |tab, _| tab.cursor), Some(2), "the first right lands on the first bucket");
        let said = names(cx, Role::Tooltip);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].contains("of your week") && said[0].contains("Main thread") && said[0].contains("Context"), "{}", said[0]);
        cx.simulate_keystrokes("left");
        assert_eq!(tab.read_with(cx, |tab, _| tab.cursor), Some(1));
        cx.simulate_keystrokes("left left left");
        assert_eq!(tab.read_with(cx, |tab, _| tab.cursor), Some(0), "held at the start");
        cx.simulate_keystrokes("escape");
        assert_eq!(tab.read_with(cx, |tab, _| tab.cursor), None, "Esc puts the cursor away");
        assert!(names(cx, Role::Tooltip).is_empty());
    }

    #[gpui::test]
    fn the_cursor_on_a_pause_reads_what_it_waited_on(cx: &mut TestAppContext) {
        let (tab, cx) = drawn(cx, graph_of(crate::job_tokens::mock_timeline()));
        tab.update(cx, |tab, cx| (0..39).for_each(|_| tab.step(1, cx)));
        redraw(cx);
        let said = names(cx, Role::Tooltip);
        assert!(said.iter().any(|one| one.ends_with("Waited on you, 52 min. The cache went cold")), "{said:?}");
    }
}
