//! The Operator's session drawn beside the Job on screen, on the Operator's page: its own thread and box, drawn and answered
//! by the paths a Job's are. While it is drawn, or a key or press reaches it, its parts stand in the view's own places
//! (`shown`, the rows, the box) and go back after, so no path reads the Job in place of the Operator.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{
    div, list, prelude::*, AnyElement, App, Bounds, Context, DispatchPhase, Element, ElementId, Entity, FocusHandle, Focusable, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, LayoutId, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Role, ScrollEvent, SharedString, Subscription, WeakEntity, Window,
};
use gpui_component::input::{InputEvent, TextareaState};
use ipc::operator_talk::TalkToOperator;

use super::composer::{Kept, Popup};
use super::offer::Offer;
use super::row_views::Rows;
use super::{fade, JobView, Page};
use crate::labels::labelled;
use crate::look::{token, ActiveLook as _};

/// The key his words to the Operator are kept under, and under `{KEEP}:{card}` while the box names a card.
pub(crate) const KEEP: &str = "operator:compose";

/// What the beside is keyed by while Fleet has started no Operator session yet: no thread to read, only the box.
pub const UNSTARTED: &str = "operator:unstarted";

/// What VoiceOver, walks and tests name the Operator's thread by.
pub(crate) const TALK: &str = "Talk with the Operator";

/// A card the Operator raised in the talk, drawn whole by the page at the thread's foot.
pub type DrawRaised = Rc<dyn Fn(&str, &mut Window, &mut App) -> AnyElement>;
/// His words to the Operator, by the key the thread holds them under: the page sends them with `talk_to_operator`.
pub type Say = Rc<dyn Fn(TalkToOperator, u64, &mut Window, &mut App)>;
/// Esc in an empty box with nothing over it: the keys go back to the page.
pub type Leave = Rc<dyn Fn(&mut Window, &mut App)>;

/// The parts that stand in the view's own places while the Operator's session is drawn or answered.
struct Held {
    shown: Option<SharedString>,
    rows: Rows,
    focus: FocusHandle,
    input: Entity<TextareaState>,
    frame: FocusHandle,
    offer: Option<Offer>,
    popup: Option<Popup>,
    pane: Option<Pixels>,
    pane_height: Option<Pixels>,
    pop: Option<super::pop::Pop>,
    find: Option<super::find::Find>,
}

pub(super) struct Beside {
    pub id: SharedString,
    held: Held,
    /// Its parts stand in the view's own now.
    inside: bool,
    /// Everything it draws, so a key anywhere in it reaches it.
    root: FocusHandle,
    /// The card Discuss named, by id and title: a send with it is a talk about that card.
    pub about: Option<(String, SharedString)>,
    pub raised: Vec<String>,
    draw: Option<DrawRaised>,
    say: Option<Say>,
    leave: Option<Leave>,
    /// What each send said, by its key, so Retry says it again.
    said: HashMap<u64, TalkToOperator>,
    /// The room its rows are given before the card above gives up any.
    room: Pixels,
    view: Entity<BesideView>,
    _subs: Vec<Subscription>,
}

/// What the page hands the Job view for the Operator's thread and box each time it draws.
pub struct OperatorPane {
    pub session: SharedString,
    /// The Operator's record as the page read it, which the box and the Writing line read.
    pub record: Option<ipc::SessionRecord>,
    /// The walk's thread, which stands for Fleet's.
    pub mock: Option<ipc::JobThread>,
    pub raised: Vec<String>,
    pub draw: DrawRaised,
    pub say: Say,
    pub leave: Leave,
    pub room: Pixels,
}

/// The Operator's thread and box as a view of its own, so the Job view's changes draw it again.
pub(super) struct BesideView {
    job: WeakEntity<JobView>,
}

impl Render for BesideView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let job = self.job.clone();
        let drawn = job.update(cx, |job, cx| job.on_beside(|job| job.render_beside(window, cx))).ok().flatten();
        Routed { child: drawn.unwrap_or_else(|| div().into_any_element()), job }
    }
}

impl JobView {
    /// Whether the Operator's parts stand in the view's own now.
    pub(super) fn beside_in(&self) -> bool {
        self.beside.as_ref().is_some_and(|beside| beside.inside)
    }

    /// Whether `id` is the Operator's session the page draws.
    pub(super) fn is_beside(&self, id: &str) -> bool {
        self.beside.as_ref().is_some_and(|beside| beside.id.as_ref() == id)
    }

    fn trade(&mut self) {
        let Some(beside) = self.beside.as_mut() else { return };
        let held = &mut beside.held;
        std::mem::swap(&mut self.shown, &mut held.shown);
        std::mem::swap(&mut self.rows, &mut held.rows);
        std::mem::swap(&mut self.focus, &mut held.focus);
        std::mem::swap(&mut self.composer.input, &mut held.input);
        std::mem::swap(&mut self.composer.frame, &mut held.frame);
        std::mem::swap(&mut self.composer.offer, &mut held.offer);
        std::mem::swap(&mut self.composer.popup, &mut held.popup);
        std::mem::swap(&mut self.pane, &mut held.pane);
        std::mem::swap(&mut self.pane_height, &mut held.pane_height);
        std::mem::swap(&mut self.pop, &mut held.pop);
        std::mem::swap(&mut self.find, &mut held.find);
        beside.inside = !beside.inside;
    }

    /// Runs `act` with the Operator's parts in place. None where the page draws no Operator.
    pub(super) fn on_beside<R>(&mut self, act: impl FnOnce(&mut Self) -> R) -> Option<R> {
        let was = self.beside.as_ref()?.inside;
        if !was {
            self.trade();
        }
        let done = act(self);
        if !was && self.beside_in() {
            self.trade();
        }
        Some(done)
    }

    /// Runs `act` on the Operator's parts where `beside`, else on the Job's: a menu or picker answers where it was opened.
    pub(super) fn as_side<R>(&mut self, beside: bool, act: impl FnOnce(&mut Self) -> R) -> Option<R> {
        if beside {
            self.on_beside(act)
        } else {
            self.beside_leave();
            Some(act(self))
        }
    }

    /// The Operator's parts in place for a press or key that reached them. Whether they were not before.
    fn beside_enter(&mut self) -> bool {
        if self.beside.as_ref().is_some_and(|beside| !beside.inside) {
            self.trade();
            return true;
        }
        false
    }

    /// The Job's parts back in place.
    pub(super) fn beside_leave(&mut self) {
        if self.beside_in() {
            self.trade();
        }
    }

    /// A key is coming where the Operator's thread or box holds the keys: its parts go in place until it is answered.
    pub(super) fn beside_keys(&mut self, window: &Window, cx: &App) -> bool {
        let Some(beside) = self.beside.as_ref().filter(|beside| !beside.inside) else { return false };
        let keys = beside.root.contains_focused(window, cx) || beside.held.focus.contains_focused(window, cx) || beside.held.frame.contains_focused(window, cx);
        keys && self.beside_enter()
    }

    /// The key his words are kept under: the page's, or the card's the box names.
    pub(super) fn beside_key(&self) -> Option<String> {
        let beside = self.beside.as_ref()?;
        Some(beside.about.as_ref().map_or_else(|| KEEP.to_string(), |(card, _)| format!("{KEEP}:{card}")))
    }

    /// Where a box's files are held: the Operator's under the key his words are kept under, as attaching stores them, a Job's
    /// under its id.
    pub(super) fn box_files_key(&self, id: &SharedString) -> SharedString {
        match self.beside_key() {
            Some(key) if self.is_beside(id) => key.into(),
            _ => id.clone(),
        }
    }

    /// The Operator's session as the page has it now. A new session, after a restart, is a new thread and box.
    pub(super) fn pin_beside(&mut self, pane: OperatorPane, window: &mut Window, cx: &mut Context<Self>) -> Entity<BesideView> {
        self.beside_leave();
        if self.beside.as_ref().is_some_and(|beside| beside.id != pane.session) {
            self.beside = None;
        }
        let id = pane.session.clone();
        let made = self.beside.is_none();
        if made {
            let beside = self.make_beside(id.clone(), window, cx);
            self.beside = Some(beside);
        }
        if let Some(record) = pane.record {
            if self.list.read(cx).operator_record() != Some(&record) {
                self.list.update(cx, |list, _| list.hold_operator(record));
            }
        }
        let mut moved = made;
        match (&self.fleet, pane.mock) {
            (None, Some(read)) => {
                let held = self.mock.get(id.as_ref());
                if held.is_none_or(|held| held.items != read.items || held.head != read.head) {
                    let version = held.map_or(1, |held| held.version + 1);
                    self.mock.insert(id.to_string(), crate::fleet::HeldThread::from_read(read, version));
                    moved = true;
                }
            }
            (Some(fleet), _) => {
                if id.as_ref() != UNSTARTED && !fleet.read(cx).held().threads.contains_key(id.as_ref()) {
                    let fleet = fleet.clone();
                    fleet.update(cx, |fleet, cx| fleet.read_thread(&id, false, cx));
                }
            }
            (None, None) => {}
        }
        let beside = self.beside.as_mut().expect("the Operator's thread was just made");
        moved |= beside.raised != pane.raised;
        beside.raised = pane.raised;
        beside.draw = Some(pane.draw);
        beside.say = Some(pane.say);
        beside.leave = Some(pane.leave);
        beside.room = pane.room;
        let view = beside.view.clone();
        self.pages.entry(id.clone()).or_insert_with(Page::new);
        if made {
            self.on_beside(|this| {
                let kept = this.box_key().map(|key| this.draft_of(&key)).unwrap_or_default();
                this.set_box(&kept, window, cx);
                this.fit_box(window, cx);
                this.want_models(cx);
            });
        }
        if moved {
            self.sync(&id, cx);
        }
        // What the page draws in a raised card may have changed, and its height with it.
        if let Some(page) = self.pages.get(&id) {
            let raised = page.extras.iter().enumerate().filter(|(_, extra)| matches!(extra, super::Extra::Raised(_))).map(|(at, _)| page.entry(page.items + at));
            for at in raised {
                page.list.remeasure_items(at..at + 1);
            }
        }
        view
    }

    fn make_beside(&mut self, id: SharedString, window: &mut Window, cx: &mut Context<Self>) -> Beside {
        let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8).submit_on_enter(true));
        let typed = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
            this.on_beside(|this| this.box_event(event, window, cx));
        });
        let job = cx.weak_entity();
        let view = cx.new(|_| BesideView { job });
        Beside {
            id: id.clone(),
            held: Held {
                shown: Some(id),
                rows: Rows::beside(),
                focus: cx.focus_handle(),
                input,
                frame: cx.focus_handle(),
                offer: None,
                popup: None,
                pane: None,
                pane_height: None,
                pop: None,
                find: None,
            },
            inside: false,
            root: cx.focus_handle(),
            about: None,
            raised: Vec::new(),
            draw: None,
            say: None,
            leave: None,
            said: HashMap::new(),
            room: gpui::px(0.),
            view,
            _subs: vec![typed],
        }
    }

    /// The Operator's thread is drawn again: the Job view changed.
    pub(super) fn beside_changed(&mut self, cx: &mut Context<Self>) {
        if let Some(beside) = self.beside.as_mut() {
            if beside.inside {
                self.rows.changed();
            } else {
                beside.held.rows.changed();
            }
            beside.view.update(cx, |_, cx| cx.notify());
        }
    }

    /// The card the box names, or none: words already in the box stay with it, and an empty box takes what was left for the card.
    pub(super) fn set_about(&mut self, about: Option<(String, SharedString)>, window: &mut Window, cx: &mut Context<Self>) {
        self.on_beside(|this| {
            let was = this.box_key();
            if let Some(beside) = this.beside.as_mut() {
                beside.about = about;
            }
            let now = this.box_key();
            if was != now {
                if this.composer.input.read(cx).value().is_empty() {
                    let kept = now.as_deref().map(|key| this.draft_of(key)).unwrap_or_default();
                    this.set_box(&kept, window, cx);
                } else {
                    if let Some(was) = &was {
                        this.keep_draft(was, Kept::default(), cx);
                    }
                    this.typed(cx);
                }
            }
            cx.notify();
        });
    }

    /// The box names no card: the words in it stay, kept for the page.
    pub(super) fn drop_about(&mut self, cx: &mut Context<Self>) {
        self.on_beside(|this| {
            let was = this.box_key();
            if let Some(beside) = this.beside.as_mut().filter(|beside| beside.about.is_some()) {
                beside.about = None;
            } else {
                return;
            }
            if let Some(was) = &was {
                this.keep_draft(was, Kept::default(), cx);
            }
            this.typed(cx);
            cx.notify();
        });
    }

    /// The Operator's thread scrolled to its last words. Whether it has said any.
    pub(super) fn to_last_reply(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.beside.as_ref().map(|beside| beside.id.clone()) else { return false };
        let Some(at) = self.thread(&id, cx).and_then(|held| held.items.iter().rposition(|item| matches!(item, ipc::ThreadItem::Row { row: ipc::SessionRow::Message { from: ipc::SessionVoice::Agent, .. } }))) else { return false };
        let Some(page) = self.pages.get(&id) else { return false };
        page.list.scroll_to(gpui::ListOffset { item_ix: page.entry(at), offset_in_item: gpui::px(0.) });
        cx.notify();
        true
    }

    /// The Operator's box holds no words and nothing beside them.
    pub(super) fn beside_empty(&self, cx: &App) -> bool {
        let Some(beside) = self.beside.as_ref() else { return true };
        let input = if beside.inside { &self.composer.input } else { &beside.held.input };
        input.read(cx).value().trim().is_empty() && !self.composer.holds(&beside.id)
    }

    /// The words in the Operator's box.
    pub(super) fn beside_words(&self, cx: &App) -> String {
        let Some(beside) = self.beside.as_ref() else { return String::new() };
        let input = if beside.inside { &self.composer.input } else { &beside.held.input };
        input.read(cx).value().to_string()
    }

    /// Whether the Operator's box has the keys.
    pub(super) fn beside_has_keys(&self, window: &Window, cx: &App) -> bool {
        let Some(beside) = self.beside.as_ref() else { return false };
        let input = if beside.inside { &self.composer.input } else { &beside.held.input };
        input.read(cx).focus_handle(cx).contains_focused(window, cx)
    }

    /// Esc with nothing over the Operator's box: the keys go back to the page.
    pub(super) fn beside_leaves(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(leave) = self.beside.as_ref().and_then(|beside| beside.leave.clone()) else { return false };
        window.defer(cx, move |window, cx| leave(window, cx));
        true
    }

    /// His words go to the Operator as a talk, by the page, after this view is done: the page's answer reaches this view.
    pub(super) fn say_to_operator(&mut self, key: u64, talk: TalkToOperator, window: &mut Window, cx: &mut Context<Self>) {
        let Some(beside) = self.beside.as_mut() else { return };
        beside.said.insert(key, talk.clone());
        let Some(say) = beside.say.clone() else { return };
        window.defer(cx, move |window, cx| say(talk, key, window, cx));
    }

    /// What send `key` said, for its Retry.
    pub(super) fn said_to_operator(&self, key: u64) -> Option<TalkToOperator> {
        self.beside.as_ref()?.said.get(&key).cloned()
    }

    /// Fleet answered a send: one it refused stays in the thread marked Not sent, with its Retry. The page's toast said why.
    pub(super) fn operator_answered(&mut self, key: u64, said: Result<(), String>, cx: &mut Context<Self>) {
        let Some(id) = self.beside.as_ref().map(|beside| beside.id.clone()) else { return };
        match said {
            Ok(()) => {
                if let Some(beside) = self.beside.as_mut() {
                    beside.said.remove(&key);
                }
            }
            Err(why) => {
                if let Some(page) = self.pages.get_mut(&id) {
                    if let Some(one) = page.pending.iter_mut().find(|one| one.key == key) {
                        one.failed = true;
                    }
                    page.refused = Some(why.into());
                }
                self.sync(&id, cx);
            }
        }
        cx.notify();
    }

    /// The card the Operator raised that list element `ix` of the shown thread draws, if it draws one.
    pub(super) fn raised_of(&self, ix: usize) -> Option<String> {
        let page = self.pages.get(self.shown.as_ref()?)?;
        match page.extras.get(ix.checked_sub(page.lead + page.before + page.items)?)? {
            super::Extra::Raised(card) => Some(card.clone()),
            _ => None,
        }
    }

    /// Whether list element `ix` of the shown thread is a card the Operator raised.
    pub(super) fn raised_at(&self, ix: usize) -> bool {
        self.raised_of(ix).is_some()
    }

    /// A card the Operator raised, drawn by the page.
    pub(super) fn raised_card(&self, card: &str, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let draw = self.beside.as_ref()?.draw.clone()?;
        Some(draw(card, window, cx))
    }

    /// The Operator's thread and its box, as a Job's thread and box are drawn, under the card on its page.
    fn render_beside(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.rows.begin(&self.shown);
        let Some(id) = self.shown.clone() else { return div().into_any_element() };
        let Some(root) = self.beside.as_ref().map(|beside| beside.root.clone()) else { return div().into_any_element() };
        let room = self.beside.as_ref().map_or(gpui::px(0.), |beside| beside.room);
        if self.pages.get(&id).is_some_and(|page| page.keys.begin_frame(window)) {
            window.focus(&self.focus, cx);
        }
        let look = cx.look().clone();
        // Aa moved how the thread reads: every list measures again, as the Job's thread does.
        let drawn = self.composer.quick.thread();
        if self.composer.thread_drawn != Some(drawn) {
            self.composer.thread_drawn = Some(drawn);
            self.pages.values().for_each(|page| page.list.remeasure());
        }
        let pane = super::head::drawn(window, cx, Role::Log, TALK).map(|bounds| bounds.size.width);
        if pane.is_some() && pane != self.pane {
            self.pane = pane;
            if let Some(page) = self.pages.get(&id) {
                page.list.remeasure();
            }
            window.request_animation_frame();
        }
        let loaded = self.thread(&id, cx).is_some() && self.pages.get(&id).is_some_and(|page| page.version.is_some());
        let refused = !loaded
            && match &self.fleet {
                Some(fleet) => fleet.read(cx).thread_refused(&id),
                None => crate::mock::thread_refused(&id).is_some(),
            };
        // With no session there is no thread to wait for: it draws empty over the box.
        let empty = refused || (!loaded && id.as_ref() == UNSTARTED);
        let body = if loaded {
            let state = self.pages[&id].list.clone();
            labelled(
                div()
                    .id("operator-thread")
                    .key_context("thread_scroll")
                    .track_focus(&self.focus)
                    .on_action(cx.listener(Self::on_page_up))
                    .on_action(cx.listener(Self::on_page_down))
                    .on_action(cx.listener(Self::on_to_start))
                    .on_action(cx.listener(Self::on_to_end))
                    .size_full()
                    .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                    .child(list(state, cx.processor(|this, ix: usize, window, cx| this.on_beside(|this| this.thread_row(ix, window, cx)).unwrap_or_else(|| div().into_any_element()))).size_full())
                    .child(fade(&look, true))
                    .child(fade(&look, false)),
                Role::Log,
                TALK,
            )
            .into_any_element()
        } else if empty {
            div().size_full().into_any_element()
        } else {
            self.render_skeleton(&look)
        };
        let find = self.render_find(&id, cx);
        let dock = (!self.closed(&id, cx)).then(|| self.render_dock(&id, window, cx));
        let mut popups: Vec<AnyElement> = Vec::new();
        popups.extend(self.pop.as_ref().map(|pop| pop.menu.clone().into_any_element()));
        popups.extend(self.render_confirm(window, cx).map(IntoElement::into_any_element));
        popups.extend(self.render_picture(window, cx).map(IntoElement::into_any_element));
        div()
            .id("operator-talk")
            .track_focus(&root)
            .key_context("thread")
            // An act dispatched after its key or press, as the palette's and the menus' are, reaches here with the Job's parts in
            // place: the Operator's go in first, so a send never goes to the Job.
            .capture_action(cx.listener(|this, _: &crate::keymap::Act, _, cx| {
                if this.beside_enter() {
                    let view = cx.entity().downgrade();
                    cx.defer(move |cx| {
                        view.update(cx, |view, _| view.beside_leave()).ok();
                    });
                }
            }))
            .on_action(cx.listener(Self::on_act))
            .on_action(cx.listener(Self::on_search))
            .child(gpui_base::TextSelectionLayer)
            .relative()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .children(find)
            // The rows are given room for the newest of them, up to their least room, before the card above scrolls any.
            .child(div().relative().flex_grow(1.).flex_shrink(1.).flex_basis(room).min_h_0().overflow_hidden().child(body).children(self.render_foot(&id, cx)))
            .children(dock.map(|dock| div().flex_none().child(dock)))
            .children(popups)
            .into_any_element()
    }

    /// The last of the Operator's thread his words were about, before item `before`: a card's id, or none.
    pub(super) fn about_before(&self, items: &[ipc::ThreadItem], before: usize) -> Option<String> {
        items[..before.min(items.len())].iter().rev().find_map(|item| match item {
            ipc::ThreadItem::Row { row: ipc::SessionRow::Message { from: ipc::SessionVoice::You, text, notice: None, .. } } => {
                Some(crate::operator::talk::tag_of(text).map(|(card, ..)| card.to_string()))
            }
            _ => None,
        })?
    }

    /// "About: <card title>", where his words turn to a card: a hairline either side of it.
    pub(super) fn about_row(&self, key: &str, title: &str, cx: &App) -> AnyElement {
        let look = cx.look();
        let said = format!("About: {title}");
        let rule = || div().flex_1().h(look.length(token::BORDER_WIDTH)).bg(look.color(token::OPERATOR_EDGE));
        labelled(
            div()
                .id(SharedString::from(format!("about:{key}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_2))
                .pb(look.length(token::SPACE_3))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(gpui::FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::OPERATOR))
                .child(rule())
                .child(div().flex_none().max_w(gpui::relative(0.7)).truncate().child(said.clone()))
                .child(rule()),
            Role::Splitter,
            said,
        )
        .into_any_element()
    }

    /// The tag Discuss put in the Operator's box: a link glyph, the card's title and an x that drops it.
    pub(super) fn about_chip(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let (_, title) = self.beside.as_ref()?.about.as_ref()?;
        let look = cx.look();
        let said = format!("About {title}");
        let drop = labelled(
            div()
                .id("tag-drop")
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::SPACE_4))
                .rounded_full()
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::OPERATOR_FAINT)))
                .focusable()
                .tab_stop(true)
                .tooltip(super::head::tip("Talk about no card".into()))
                .child(crate::titlebar::parts::glyph("x", look.length(token::SPACE_3), look.color(token::OPERATOR)))
                .on_click(cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    this.set_about(None, window, cx);
                }))
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    let key = &event.keystroke;
                    if matches!(key.key.as_str(), "enter" | "space") && !key.modifiers.modified() {
                        cx.stop_propagation();
                        this.set_about(None, window, cx);
                    }
                })),
            Role::Button,
            "Drop the tag",
        );
        Some(
            labelled(
                div().id("tag").flex().px(look.length(token::SPACE_3)).pt(look.length(token::SPACE_3)).child(
                    div()
                        .flex_none()
                        .max_w_full()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .h(look.length(token::H_CONTROL))
                        .pl(look.length(token::SPACE_3))
                        .pr(look.length(token::SPACE_1))
                        .rounded_full()
                        .bg(look.color(token::OPERATOR_MUTED))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::OPERATOR_EDGE))
                        .text_color(look.color(token::OPERATOR))
                        .text_size(look.length(token::TEXT_XS))
                        .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
                        .child(crate::titlebar::parts::glyph("link", look.length(token::SPACE_3) + look.length(token::SPACE_1), look.color(token::OPERATOR)))
                        .child(div().min_w_0().truncate().child(title.clone()))
                        .child(drop),
                ),
                Role::Group,
                said,
            )
            .into_any_element(),
        )
    }

    /// The Operator's model and effort, picked in its box: the settings Settings shows, and the session running now, both on what was picked.
    pub(super) fn tune_operator(&mut self, id: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let (_, model, effort) = self.tuning(&id, cx);
        // Fleet's restart reads an empty model as a Job's, not the Operator's: the model goes named.
        let Some(model) = model.or_else(|| self.composer.catalog.models.as_ref().map(|models| models.session.clone())) else { return };
        self.composer.box_of(&id).model = Some(Some(model.clone()));
        // No effort picked is the harness's own, which is the setting's first word.
        let effort = effort.unwrap_or_else(|| "harness default".to_string());
        let changes = [
            ("models.operator".to_string(), Some(ipc::SettingValue::Text(model))),
            ("effort.operator".to_string(), Some(ipc::SettingValue::Text(effort))),
        ]
        .into_iter()
        .collect();
        let job = id.clone();
        let saved: super::wire::Done = Box::new(move |this, answer, _, cx| {
            if let (Err(why), Some(page)) = (answer, this.pages.get_mut(&job)) {
                page.refused = Some(why.into());
            }
            cx.notify();
        });
        self.post("/settings/save".into(), ipc::encode(&ipc::SaveSettings { changes }), window, cx, saved);
        self.ask_tune(id, window, cx);
    }
}

/// The Operator's parts in place for every press the pane takes, and back once the press is answered.
struct Routed {
    child: AnyElement,
    job: WeakEntity<JobView>,
}

impl IntoElement for Routed {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn route<E: gpui::MouseEvent>(hitbox: &Hitbox, job: &WeakEntity<JobView>, window: &mut Window) {
    let (hitbox, job) = (hitbox.clone(), job.clone());
    window.on_mouse_event(move |_: &E, phase, window, cx| match phase {
        DispatchPhase::Capture if hitbox.is_hovered(window) => {
            if job.update(cx, |job, _| job.beside_enter()).unwrap_or(false) {
                // Where a press stops going on, the bubble below never comes back here.
                let job = job.clone();
                cx.defer(move |cx| {
                    job.update(cx, |job, _| job.beside_leave()).ok();
                });
            }
        }
        DispatchPhase::Bubble => {
            job.update(cx, |job, _| job.beside_leave()).ok();
        }
        DispatchPhase::Capture => {}
    });
}

impl Element for Routed {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(&mut self, _id: Option<&GlobalElementId>, _inspector_id: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(&mut self, _id: Option<&GlobalElementId>, _inspector_id: Option<&InspectorElementId>, bounds: Bounds<Pixels>, _request_layout: &mut (), window: &mut Window, cx: &mut App) -> Hitbox {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        self.child.prepaint(window, cx);
        hitbox
    }

    fn paint(&mut self, _id: Option<&GlobalElementId>, _inspector_id: Option<&InspectorElementId>, _bounds: Bounds<Pixels>, _request_layout: &mut (), hitbox: &mut Hitbox, window: &mut Window, cx: &mut App) {
        route::<MouseDownEvent>(hitbox, &self.job, window);
        route::<MouseUpEvent>(hitbox, &self.job, window);
        route::<MouseMoveEvent>(hitbox, &self.job, window);
        route::<ScrollEvent>(hitbox, &self.job, window);
        self.child.paint(window, cx);
    }
}
