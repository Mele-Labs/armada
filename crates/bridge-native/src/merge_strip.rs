//! The merge queue strip along the window's bottom edge (`DK/Deck.tsx`, `MergeQueue`): a merge button,
//! main's light, and a chip for each pull request, nearest main first. **It draws Fleet's merge line
//! view** (`get_merge_line_view`) **and calls** (`get_calls`): which pull requests, in what order,
//! whether main reads green, red or nothing, and which chip a call pulses are Fleet's, and what is
//! left here is the words (`words.rs`), which Fleet leaves to a client.
//!
//! One repository is a single row. On All with several, one row for each, and the strip shows one at
//! a time: it moves to the next every 15 seconds while the pointer and the keys are elsewhere, a main
//! turning red brings its row in, and ↑ and ↓ move it by hand. A chip's card opens on hover and its
//! press selects the pull request; main's light opens a card of the red band on hover and a press
//! keeps it open.
//!
//! The strip is one entity the Jobs screen mounts as its last child ([`strip`]); a screen that
//! draws it with nothing served draws nothing.

mod fix;
mod scenes;
mod words;

pub use scenes::{apply, begin, end};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, px, Anchor, Animation, AnimationExt as _, AnyElement, App, Bounds, ClipboardItem, Context, DismissEvent,
    Empty, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, Global, Hsla, MouseButton, Pixels, Point, Role, ScrollHandle, SharedString, Subscription, Task,
    WeakEntity, Window,
};
use ipc::{
    HeldCheckout, HubPullRequest, MainRedJob, MainView, MergeLineView, StandingCall, StandingCallKind,
};

use crate::dropdown::{Dropdown, Entry, Picked, Row};
use crate::inspect::{self, Fact, Spec, State, Width};
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, mono, no_drag, small, tip, Btn, Weight};
use words::{check_of, commits, hue_of, is_queued, reading_of, read_said, review_said, span_said, PullState, MERGE_BY_HAND};

/// How long the stacked line rests on one repository before it moves to the next.
const ROTATE: Duration = Duration::from_secs(15);

/// A chip's tone, which the strip's dot takes over its state: a pull request that is red, one the
/// person can merge, or neither. Fleet sends no field for it, so the head that knows the Job gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Calm,
    Ask,
    Bad,
}

/// What the Jobs know of one pull request, which Fleet's view does not carry: the Job that holds it,
/// and the tone its dot takes.
#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    pub number: u64,
    /// The Job that holds the pull request, by id and title.
    pub job: Option<(SharedString, SharedString)>,
    pub tone: Tone,
}

/// One repository's line: Fleet's view of it, and what the Jobs add.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub view: MergeLineView,
    pub held: Vec<Held>,
}

/// A Job the red main's work could be sent back to.
#[derive(Clone, Debug, PartialEq)]
pub struct Recent {
    pub id: SharedString,
    pub title: SharedString,
    pub branch: Option<SharedString>,
}

/// What the strip is given.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Strip {
    /// The repositories in scope, in Fleet's order. The person's pick is a client filter, so the head
    /// hands the lines of the pick.
    pub lines: Vec<Line>,
    /// Fleet's calls; a call no Job holds pulses the chip it is about.
    pub calls: Vec<StandingCall>,
    /// The scope is All, so several repositories stack as rows.
    pub on_all: bool,
    /// The repository of the Job or pull request that is selected: on All, only its row draws.
    pub selected_root: Option<String>,
    /// The pull requests of the open Job, which lead the rest.
    pub related: Vec<u64>,
    /// Waiting on Fleet's first answer: placeholder chips draw.
    pub loading: bool,
    /// The Jobs a red main's work can be sent back to.
    pub recent: Vec<Recent>,
}

/// What a press on the strip asks of the rest of the app. The wiring sends these; the walk reads none.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    /// The Merge line page.
    MergeLine,
    /// A pull request, in the side panel.
    OpenPull { root: String, number: u64 },
    OpenJob(SharedString),
    /// A new Job for the red main of this repository, from this brief.
    DispatchFix { root: String, request: String },
    /// The red main's work, to a Job as a message.
    SendBack { root: String, session: SharedString, message: String },
    /// Replace the claim another machine holds: a new Job from this brief.
    TakeOver { root: String, request: String },
    /// Fleet reads the pull request from the forge now (the chip menu's Refresh).
    RefreshPull { root: String, number: u64 },
}

impl EventEmitter<Asked> for MergeStrip {}

#[derive(Clone, PartialEq, Eq)]
enum Card {
    Main(usize),
    Chip(usize, u64),
}

/// The one strip of the window, for the keys that work from anywhere and for a walk's scenes.
struct Live(WeakEntity<MergeStrip>);

impl Global for Live {}

/// The face a strip opens with while a walk is playing.
pub struct Opening(pub Strip);

impl Global for Opening {}

pub fn live(cx: &App) -> Option<gpui::Entity<MergeStrip>> {
    cx.try_global::<Live>().and_then(|live| live.0.upgrade())
}

/// The strip for the Jobs screen to draw as its last child.
pub fn strip(window: &mut Window, cx: &mut App) -> AnyElement {
    window.use_keyed_state("merge-strip", cx, |window, cx| MergeStrip::new(window, cx)).into_any_element()
}

/// `q`, the merge queue's panel key (`deck_panel`, scope deck, its fourth slot): the keys go to the
/// strip. The deck's own handler takes `l` and lets the rest reach here.
pub fn bind(cx: &mut App) {
    cx.on_action(|key: &Act, cx: &mut App| {
        if !(key.is("deck_panel") && key.slot == 3) {
            return cx.propagate();
        }
        let Some(strip) = live(cx) else { return cx.propagate() };
        let (window, focus) = {
            let strip = strip.read(cx);
            (strip.window, strip.focus.clone())
        };
        // The key is being dispatched inside this window's own update, so the focus moves just after.
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| window.focus(&focus, cx));
        });
    });
}

/// The menu a right press on a chip, or Shift+F10 on it, opens.
struct ChipMenu {
    dropdown: Entity<Dropdown>,
    _picked: Subscription,
    _dismissed: Subscription,
}

pub struct MergeStrip {
    pub data: Strip,
    /// The window the strip is drawn in, which a key that works from anywhere puts the keys in.
    window: gpui::AnyWindowHandle,
    /// The pull request the side panel shows.
    selected: Option<u64>,
    card: Option<Card>,
    pinned: bool,
    over_card: bool,
    _leaving: Option<Task<()>>,
    _ticking: Option<Task<()>>,
    /// The stacked row on screen.
    row: usize,
    /// The pointer or the keys are on the strip, so it does not rotate.
    over: bool,
    focused: bool,
    focus: FocusHandle,
    popup_focus: FocusHandle,
    keys_were: Option<FocusHandle>,
    scroll: ScrollHandle,
    bounds: Rc<RefCell<HashMap<SharedString, Bounds<Pixels>>>>,
    /// The repository whose red main has a new Job being dispatched, and any refusal.
    fixing: Option<String>,
    /// The keys of each chip and light, so a card opens as the keys arrive on it.
    chip_focus: HashMap<SharedString, (FocusHandle, Subscription, Subscription)>,
    /// A card Esc shut while the keys stayed on its chip: it does not open again until they leave.
    quiet: Option<Card>,
    /// The card the keys opened, which they shut again as they leave; one a hover or a press opened is theirs to leave.
    by_keys: Option<Card>,
    menu: Option<ChipMenu>,
    /// The dialog a red band's act opened (`fix.rs`).
    fix: Option<fix::FixDialog>,
    _rotate: Task<()>,
    _focus_in: Subscription,
    _focus_out: Subscription,
}

impl MergeStrip {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> MergeStrip {
        let data = cx.try_global::<Opening>().map(|opening| opening.0.clone()).unwrap_or_default();
        cx.set_global(Live(cx.entity().downgrade()));
        let focus = cx.focus_handle();
        let focus_in = cx.on_focus_in(&focus, window, |this, _, cx| {
            this.focused = true;
            cx.notify();
        });
        let focus_out = cx.on_focus_out(&focus, window, |this, _, _, cx| {
            this.focused = false;
            cx.notify();
        });
        let rotate = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(ROTATE).await;
            let moved = this.update(cx, |this, cx| {
                if this.grouped() && !this.over && !this.focused && this.card.is_none() {
                    this.row = (this.row + 1) % this.shown().len().max(1);
                    cx.notify();
                }
            });
            if moved.is_err() {
                break;
            }
        });
        MergeStrip {
            data,
            window: window.window_handle(),
            selected: None,
            card: None,
            pinned: false,
            over_card: false,
            _leaving: None,
            _ticking: None,
            row: 0,
            over: false,
            focused: false,
            focus,
            popup_focus: cx.focus_handle(),
            keys_were: None,
            scroll: ScrollHandle::new(),
            bounds: Rc::default(),
            fixing: None,
            chip_focus: HashMap::new(),
            quiet: None,
            by_keys: None,
            menu: None,
            fix: None,
            _rotate: rotate,
            _focus_in: focus_in,
            _focus_out: focus_out,
        }
    }

    /// New data, as Fleet's last answer made it. A main that turned red brings its row into view.
    #[allow(dead_code)]
    pub fn set(&mut self, data: Strip, cx: &mut Context<Self>) {
        let was_red: Vec<bool> = self.data.lines.iter().map(|line| matches!(line.view.main, Some(MainView::Red { .. }))).collect();
        self.data = data;
        let shown = self.shown();
        for (at, line) in self.data.lines.iter().enumerate() {
            let red = matches!(line.view.main, Some(MainView::Red { .. }));
            if red && !was_red.get(at).copied().unwrap_or(false) {
                if let Some(place) = shown.iter().position(|one| *one == at) {
                    self.row = place;
                }
            }
        }
        self.row = self.row.min(shown.len().saturating_sub(1));
        cx.notify();
    }

    /// Back to a first strip with nothing open, as a walk does when it starts a screen over.
    pub fn reset(&mut self, data: Strip) {
        self.data = data;
        self.selected = None;
        self.card = None;
        self.by_keys = None;
        self.quiet = None;
        self.menu = None;
        self.pinned = false;
        self.over_card = false;
        self.row = 0;
        self.keys_were = None;
        self.fixing = None;
        self.fix = None;
    }

    /// A walk's scene: new data with nothing open, on the row of a main that is red.
    pub fn jump(&mut self, data: Strip, cx: &mut Context<Self>) {
        self.reset(data);
        let shown = self.shown();
        let red = shown.iter().position(|at| matches!(self.data.lines[*at].view.main, Some(MainView::Red { .. })));
        self.row = red.unwrap_or(0);
        cx.notify();
    }

    /// A walk's scene that comes with main's card open, as a hover leaves it.
    pub fn show_main(&mut self, cx: &mut Context<Self>) {
        if let Some(line) = self.shown().into_iter().find(|at| matches!(self.data.lines[*at].view.main, Some(MainView::Red { .. }))) {
            self.card = Some(Card::Main(line));
            cx.notify();
        }
    }

    /// The lines the strip draws: on All with several and a Job or pull request selected, only that
    /// repository's, else every one.
    fn shown(&self) -> Vec<usize> {
        let all: Vec<usize> = (0..self.data.lines.len()).collect();
        if !(self.data.on_all && all.len() > 1) {
            return all;
        }
        let scoped: Vec<usize> = match &self.data.selected_root {
            Some(root) => all.iter().copied().filter(|at| &self.data.lines[*at].view.root == root).collect(),
            None => Vec::new(),
        };
        if scoped.is_empty() { all } else { scoped }
    }

    fn grouped(&self) -> bool {
        self.shown().len() > 1
    }

    /// A call no Job holds, still to open, pulses the chip it is about: `main:<root>`, `checkout:<root>`
    /// or `pull:<number>`.
    fn calling(&self, key: &str) -> bool {
        self.data.calls.iter().any(|call| call.kind != StandingCallKind::Job && call.key == key && call.owner == call.key && !call.opened)
    }

    // -- keys and focus ----------------------------------------------------------------------------

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
        self.card = None;
        self.by_keys = None;
        self.pinned = false;
        self.over_card = false;
        self._ticking = None;
        self.give_keys_back(window, cx);
        cx.notify();
    }

    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.card.is_some() {
            // The keys go back to the chip, and its focus is not a reason to open the card again.
            self.quiet = self.card.clone();
            self.shut(window, cx);
        }
    }

    /// The keys arrived on a chip or light: from the keyboard its card opens, as a hover opens it.
    fn focused_in(&mut self, card: Card, window: &mut Window, cx: &mut Context<Self>) {
        if !window.last_input_was_keyboard() || self.pinned || self.quiet.as_ref() == Some(&card) || self.menu.is_some() {
            return;
        }
        self._leaving = None;
        if self.card.as_ref() != Some(&card) {
            self.keep_keys(window, cx);
            self.by_keys = Some(card.clone());
            self.card = Some(card);
            self.tick(cx);
            cx.notify();
        }
    }

    /// The keys left a chip or light: its card shuts with them, unless it was kept open.
    fn focused_out(&mut self, card: Card, cx: &mut Context<Self>) {
        if self.quiet.as_ref() == Some(&card) {
            self.quiet = None;
        }
        if self.by_keys.as_ref() == Some(&card) && self.card.as_ref() == Some(&card) && !self.pinned && !self.over_card {
            self.card = None;
            self.by_keys = None;
            self._ticking = None;
            self.keys_were = None;
            cx.notify();
        }
    }

    /// The keys for a chip or light, made once and kept.
    fn focus_for(&mut self, key: SharedString, card: Card, window: &mut Window, cx: &mut Context<Self>) {
        if self.chip_focus.contains_key(&key) {
            return;
        }
        let handle = cx.focus_handle().tab_stop(true);
        let into = card.clone();
        let arrived = cx.on_focus_in(&handle, window, move |this, window, cx| this.focused_in(into.clone(), window, cx));
        let left = cx.on_focus_out(&handle, window, move |this, _, _, cx| this.focused_out(card.clone(), cx));
        self.chip_focus.insert(key, (handle, arrived, left));
    }

    fn focus_of(&self, key: &str) -> Option<FocusHandle> {
        self.chip_focus.get(key).map(|(handle, _, _)| handle.clone())
    }

    // -- the chip's menu ---------------------------------------------------------------------------

    /// A right press on a chip, or Shift+F10 on it: Open in browser and Copy link where the chip knows its
    /// address, and Refresh, under the heading `Pull request #n`.
    fn open_chip_menu(&mut self, line: usize, number: u64, at: Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(one) = self.data.lines.get(line) else { return };
        let Some(pull) = one.view.pulls.iter().find(|pull| pull.number == number) else { return };
        let (root, url) = (one.view.root.clone(), pull.url.clone());
        self.keep_keys(window, cx);
        self.card = None;
        self.pinned = false;
        self.over_card = false;
        self._ticking = None;
        let mut entries = vec![Entry::Label(crate::titlebar::id_of(format!("Pull request #{number}")))];
        if !url.is_empty() {
            entries.push(Entry::Row(Row::new("browser", "Open in browser").icon("external-link")));
            entries.push(Entry::Row(Row::new("copy", "Copy link").icon("link")));
        }
        entries.push(Entry::Row(Row::new("refresh", "Refresh")));
        let name = format!("Pull request #{number}");
        let dropdown = cx.new(|cx| Dropdown::new(name, entries, at, Anchor::TopLeft, from_keys, cx));
        let picked = cx.subscribe_in(&dropdown, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.menu = None;
            match *id {
                "browser" => cx.open_url(&url),
                "copy" => cx.write_to_clipboard(ClipboardItem::new_string(url.clone())),
                "refresh" => cx.emit(Asked::RefreshPull { root: root.clone(), number }),
                _ => {}
            }
            this.give_keys_back(window, cx);
            cx.notify();
        });
        let dismissed = cx.subscribe_in(&dropdown, window, |this, _, _: &DismissEvent, window, cx| {
            this.menu = None;
            this.give_keys_back(window, cx);
            cx.notify();
        });
        let keys = dropdown.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.menu = Some(ChipMenu { dropdown, _picked: picked, _dismissed: dismissed });
        cx.notify();
    }

    fn step_row(&mut self, by: isize, cx: &mut Context<Self>) {
        let count = self.shown().len();
        if count < 2 {
            return;
        }
        self.row = (self.row as isize + by).rem_euclid(count as isize) as usize;
        cx.notify();
    }

    // -- the cards -------------------------------------------------------------------------------------

    /// Hover opens a card; leaving shuts it unless pinned, after the time it takes to cross to it.
    fn hovered(&mut self, card: Card, over: bool, window: &mut Window, cx: &mut Context<Self>) {
        if over {
            self._leaving = None;
            if !self.pinned && self.card.as_ref() != Some(&card) {
                self.keep_keys(window, cx);
                self.by_keys = None;
                self.card = Some(card);
                self.tick(cx);
                cx.notify();
            }
        } else if self.card.as_ref() == Some(&card) && !self.pinned {
            self._leaving = Some(cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(150)).await;
                let _ = this.update_in(cx, |this, window, cx| {
                    if !this.over_card && !this.pinned && this.card.as_ref() == Some(&card) {
                        this.card = None;
                        this._ticking = None;
                        this.give_keys_back(window, cx);
                        cx.notify();
                    }
                });
            }));
        }
    }

    /// A press on main's light keeps its card open, or shuts a pinned one.
    fn light_pressed(&mut self, line: usize, window: &mut Window, cx: &mut Context<Self>) {
        let card = Card::Main(line);
        if self.pinned && self.card.as_ref() == Some(&card) {
            self.shut(window, cx);
            return;
        }
        self.keep_keys(window, cx);
        self.by_keys = None;
        self.card = Some(card);
        self.pinned = true;
        self.tick(cx);
        let focus = self.popup_focus.clone();
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
        cx.notify();
    }

    /// The card's ages move as they are looked at: a redraw every 5 seconds while one is open.
    fn tick(&mut self, cx: &mut Context<Self>) {
        if self._ticking.is_some() {
            return;
        }
        self._ticking = Some(cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(5)).await;
            let open = this.update(cx, |this, cx| {
                cx.notify();
                this.card.is_some()
            });
            if !matches!(open, Ok(true)) {
                break;
            }
        }));
    }

    /// A pull request's chip pressed: the side panel shows it, and the chip says so in this frame.
    fn chip_pressed(&mut self, line: usize, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.data.lines.get(line).map(|line| line.view.root.clone()) else { return };
        self.selected = Some(number);
        self.card = None;
        self.pinned = false;
        self._ticking = None;
        self.give_keys_back(window, cx);
        cx.emit(Asked::OpenPull { root, number });
        cx.notify();
    }

    /// Dispatch a new Job for a red main: pending in this frame.
    fn fix_pressed(&mut self, line: usize, cx: &mut Context<Self>) {
        let Some(one) = self.data.lines.get(line) else { return };
        let Some(MainView::Red { failed, merge, .. }) = &one.view.main else { return };
        if self.fixing.is_some() {
            return;
        }
        let (root, request) = (one.view.root.clone(), fix::brief_of(failed, merge.as_ref()));
        self.fixing = Some(root.clone());
        cx.emit(Asked::DispatchFix { root, request });
        cx.notify();
    }

    /// The Job the red main asked for has started, or was refused: the button comes back.
    #[allow(dead_code)]
    pub fn fix_answered(&mut self, cx: &mut Context<Self>) {
        self.fixing = None;
        cx.notify();
    }
}

// -- what a chip says --------------------------------------------------------------------------

/// One pull request as a chip draws it.
struct Dot<'a> {
    pull: &'a HubPullRequest,
    state: PullState,
    says: &'static str,
    queued: bool,
    place: Option<u32>,
    tone: Tone,
    job: Option<&'a (SharedString, SharedString)>,
    related: bool,
}

impl Dot<'_> {
    /// The chip's accessible name: the card's rows on one line.
    fn tip(&self) -> String {
        let title = if self.pull.title.is_empty() { String::new() } else { format!(" {}", self.pull.title) };
        let place = self.place.map(|place| format!(", place {place}")).unwrap_or_default();
        let mut parts = vec![format!("#{}{title}", self.pull.number), self.pull.branch.clone(), format!("{}{place}", self.says)];
        if self.pull.merge_by_hand {
            parts.push(MERGE_BY_HAND.to_string());
        }
        if let Some((_, title)) = self.job {
            parts.push(format!("Job: {title}"));
        }
        parts.join(" · ")
    }

    fn hue(&self, look: &Look) -> Hsla {
        match self.tone {
            Tone::Bad => look.color(token::STATUS_COMPLETED_FAILED),
            Tone::Ask => look.color(token::STATUS_AWAITING_REVIEW),
            Tone::Calm => look.color(hue_of(self.state)),
        }
    }
}

fn dots_of<'a>(line: &'a Line, related: &[u64]) -> Vec<Dot<'a>> {
    line.view
        .pulls
        .iter()
        .map(|pull| {
            let reading = reading_of(pull);
            let held = line.held.iter().find(|one| one.number == pull.number);
            Dot {
                pull,
                state: reading.state,
                says: reading.says,
                queued: is_queued(pull),
                place: pull.queue.as_ref().and_then(|queue| queue.position),
                tone: held.map_or(Tone::Calm, |one| one.tone),
                job: held.and_then(|one| one.job.as_ref()),
                related: related.contains(&pull.number),
            }
        })
        // A draft is left off unless it is in the queue or the open Job holds it.
        .filter(|dot| !dot.pull.draft || dot.queued || dot.related)
        .collect()
}

/// Each repository whose forge has stopped answering, and when the stalest read was.
struct Forge {
    read_at: Option<chrono::DateTime<chrono::Utc>>,
    quiet: Vec<(String, chrono::DateTime<chrono::Utc>)>,
}

fn instant(at: &ipc::Instant) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(at.as_str()).ok().map(|at| at.with_timezone(&chrono::Utc))
}

fn forge_of(lines: &[&Line]) -> Forge {
    Forge {
        read_at: lines.iter().filter_map(|line| line.view.read_at.as_ref().and_then(instant)).min(),
        quiet: lines.iter().filter_map(|line| Some((line.view.name.clone(), line.view.quiet_since.as_ref().and_then(instant)?))).collect(),
    }
}

/// `GitHub not answering since 18:40:00`, naming each repository only where more than one is served.
fn quiet_said(forge: &Forge, several: bool) -> Option<String> {
    let time = |at: &chrono::DateTime<chrono::Utc>| at.with_timezone(&chrono::Local).format("%H:%M:%S").to_string();
    match forge.quiet.as_slice() {
        [] => None,
        [(_, since)] if !several => Some(format!("GitHub not answering since {}", time(since))),
        many => Some(format!("GitHub not answering for {}", many.iter().map(|(name, since)| format!("{name} since {}", time(since))).collect::<Vec<_>>().join(", "))),
    }
}

/// `Checkout 3 commits behind main` as the light says it, from the held checkout's call.
fn checkout_title(checkout: &HeldCheckout) -> String {
    format!("{} behind {}", commits(checkout.behind), checkout.base)
}

/// The fact under a held checkout's title (`checkout.ts`).
fn checkout_fact(held: &HeldCheckout) -> String {
    if !held.clashing.is_empty() {
        "Uncommitted changes main also changed".into()
    } else if !held.uncommitted.is_empty() {
        "Uncommitted changes".into()
    } else if held.ahead > 0 {
        format!("{} not on {}", commits(held.ahead), held.base)
    } else {
        held.on.as_ref().map(|on| format!("On {on}")).unwrap_or_default()
    }
}

// -- drawing -----------------------------------------------------------------------------------

/// A chip's body: the pill every light and pull request wears (`.deck-chip`).
fn pill(look: &Look) -> gpui::Div {
    div()
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .h(look.length(token::H_CONTROL_SM))
        .pl(look.length(token::SPACE_3))
        .pr(look.length(token::SPACE_4))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_GLASS))
        .rounded_full()
        .bg(look.color(token::FG_DEFAULT).opacity(0.06))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .text_color(look.color(token::FG_MUTED))
}

/// The lamp on a chip: a ring where the pull request is open, filled where a queue holds it.
fn lamp(look: &Look, ink: Hsla, filled: bool) -> gpui::Div {
    let size = look.length(token::SPACE_2);
    let dot = div().flex_none().size(size).rounded_full();
    if filled { dot.bg(ink) } else { dot.border(look.length(token::BORDER_WIDTH)).border_color(ink) }
}

/// The wash and the pulse of a chip a call stands for.
fn calling_wash(look: &Look) -> gpui::Div {
    div()
        .absolute()
        .inset_0()
        .rounded_full()
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::STATUS_AWAITING_REVIEW).opacity(0.6))
        .bg(look.color(token::STATUS_AWAITING_REVIEW).opacity(0.14))
}

fn pulse(dot: gpui::Div, id: SharedString, look: &Look) -> AnyElement {
    dot.with_animation(id, Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.35, 1.)), |dot, at| dot.opacity(at))
        .into_any_element()
}

impl MergeStrip {
    /// A pull request's chip: its place, its lamp, `#<n>` and the by-hand mark; a button the forge's
    /// pull request opens in the side panel.
    fn chip(&self, line: usize, dot: &Dot, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let number = dot.pull.number;
        let key = format!("pull:{number}");
        let calling = self.calling(&key);
        let selected = self.selected == Some(number);
        let hue = dot.hue(look);
        let name: SharedString = dot.tip().into();
        let id = SharedString::from(format!("chip-{line}-{number}"));
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let store = Rc::clone(&self.bounds);
        let kept = id.clone();
        let lit = selected || dot.related;
        let dot_mark = lamp(look, hue, dot.queued || dot.state == PullState::Open);
        let keys = self.focus_of(&id);
        let key_id = id.clone();
        let body = pill(look)
            .id(id.clone())
            .when_some(keys, |chip, keys| chip.track_focus(&keys))
            .when(selected, |chip| chip.border_color(look.color(token::FG_DEFAULT)).bg(look.color(token::BG_HOVER)))
            .when(dot.related, |chip| chip.border_color(look.color(token::FG_DEFAULT)).bg(look.color(token::FG_DEFAULT).opacity(0.22)))
            .when(lit, |chip| chip.text_color(look.color(token::FG_DEFAULT)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))))
            .child(canvas(move |bounds, _, _| { store.borrow_mut().insert(kept, bounds); }, |_, _, _, _| {}).absolute().size_full())
            .when(calling, |chip| chip.child(calling_wash(look)))
            .children(dot.queued.then(|| dot.place).flatten().map(|place| {
                div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(place.to_string())
            }))
            .child(if calling { pulse(dot_mark, SharedString::from(format!("calling-{id}")), look) } else { dot_mark.into_any_element() })
            .child(format!("#{number}"))
            .children(dot.pull.merge_by_hand.then(|| {
                labelled(
                    div()
                        .id(SharedString::from(format!("hand-{id}")))
                        .flex_none()
                        .child(glyph("user-check", look.length(token::SPACE_3), look.color(token::STATUS_AWAITING_APPROVAL)))
                        .tooltip(tip(MERGE_BY_HAND)),
                    Role::Image,
                    MERGE_BY_HAND,
                )
            }))
            .cursor_pointer()
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.border_color(ring))
            .hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.12)).text_color(look.color(token::FG_DEFAULT)))
            .on_hover(cx.listener(move |this, over: &bool, window, cx| this.hovered(Card::Chip(line, number), *over, window, cx)))
            .on_click(cx.listener(move |this, _, window, cx| this.chip_pressed(line, number, window, cx)))
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.open_chip_menu(line, number, event.position, false, window, cx);
            }))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                let key = &event.keystroke;
                // The ContextMenu key is not one gpui delivers on macOS, so Shift+F10 alone.
                if key.key != "f10" || !key.modifiers.shift {
                    return;
                }
                cx.stop_propagation();
                let chip = this.bounds.borrow().get(&key_id).copied();
                let at = chip.map_or_else(Point::default, |chip| point(chip.origin.x + chip.size.width / 2., chip.origin.y + chip.size.height));
                this.open_chip_menu(line, number, at, true, window, cx);
            }));
        labelled(body, Role::Button, name).into_any_element()
    }

    /// Main's light. Green with the forge answering is a picture; red or a held checkout is a button
    /// that opens a card.
    fn light(&self, line: usize, grouped: bool, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let one = &self.data.lines[line];
        let name = if grouped { one.view.name.clone() } else { "main".to_string() };
        let red = matches!(one.view.main, Some(MainView::Red { .. }));
        let held = one.view.checkout.as_ref();
        let forge = forge_of(&[one]);
        let quiet = quiet_said(&forge, false);
        let where_ = if grouped { format!(" in {}", one.view.name) } else { String::new() };
        let mut said = if red { format!("Main is red{where_}") } else { format!("Main is green{where_}") };
        if red && !grouped {
            said = format!("Main is red in {}", one.view.name);
        }
        if let Some(checkout) = held {
            said = format!("{said}, checkout {}", checkout_title(checkout));
        }
        let calling = self.calling(&format!("main:{}", one.view.root)) || self.calling(&format!("checkout:{}", one.view.root));
        let ink = if red { look.color(token::STATUS_COMPLETED_FAILED) } else { look.color(token::STATUS_COMPLETED_SUCCESS) };
        let stale = quiet.is_some();
        let lamp_mark = {
            let mark = lamp(look, ink, !stale).when(stale, |mark| mark.opacity(0.4));
            if calling { pulse(mark, SharedString::from(format!("calling-main-{line}")), look) } else { mark.into_any_element() }
        };
        let id = SharedString::from(format!("light-{line}"));
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let store = Rc::clone(&self.bounds);
        let kept = id.clone();
        let keys = self.focus_of(&id);
        let body = pill(look)
            .id(id.clone())
            .child(canvas(move |bounds, _, _| { store.borrow_mut().insert(kept, bounds); }, |_, _, _, _| {}).absolute().size_full())
            .when(calling, |chip| chip.child(calling_wash(look)))
            .child(lamp_mark)
            .children(quiet.clone().map(|text| {
                labelled(
                    div().id(SharedString::from(format!("quiet-{id}"))).flex_none().child(glyph("unplug", look.length(token::SPACE_3), look.color(token::STATUS_ESCALATED))).tooltip(tip(text.clone())),
                    Role::Image,
                    text,
                )
            }))
            .children(held.map(|checkout| {
                let said = format!("Checkout {}", checkout_title(checkout));
                labelled(
                    div().id(SharedString::from(format!("held-{id}"))).flex_none().child(glyph("folder-git-2", look.length(token::SPACE_3), look.color(token::STATUS_AWAITING_REVIEW))).tooltip(tip(said.clone())),
                    Role::Image,
                    said,
                )
            }))
            .child(name);
        if !red && held.is_none() {
            // A picture: its tooltip is the sentence, then how old the reading is.
            let tooltip = match forge.read_at {
                None => said.clone(),
                Some(at) => format!("{said}. {}", read_said((chrono::Utc::now() - at).num_seconds().max(0))),
            };
            return labelled(body.tooltip(tip(tooltip)), Role::Image, said).into_any_element();
        }
        labelled(
            body.when_some(keys, |chip, keys| chip.track_focus(&keys))
                .cursor_pointer()
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                .hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.12)).text_color(look.color(token::FG_DEFAULT)))
                .on_hover(cx.listener(move |this, over: &bool, window, cx| this.hovered(Card::Main(line), *over, window, cx)))
                .on_click(cx.listener(move |this, _, window, cx| this.light_pressed(line, window, cx))),
            Role::Button,
            said,
        )
        .into_any_element()
    }

    /// One repository's chips: the queue in its order inside one track, a rule, then the rest.
    fn chips_of(&self, line: usize, cx: &Context<Self>) -> Vec<AnyElement> {
        let look = cx.look();
        let one = &self.data.lines[line];
        let dots = dots_of(one, &self.data.related);
        let mut queued: Vec<&Dot> = dots.iter().filter(|dot| dot.queued).collect();
        queued.sort_by_key(|dot| dot.place.unwrap_or(u32::MAX));
        let mut open: Vec<&Dot> = dots.iter().filter(|dot| !dot.queued).collect();
        open.sort_by_key(|dot| !dot.related);
        let mut out: Vec<AnyElement> = Vec::new();
        if !queued.is_empty() {
            out.push(
                labelled(
                    div()
                        .id(SharedString::from(format!("track-{line}")))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_1))
                        .py(look.length(token::SPACE_1) / 2.)
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .rounded_full()
                        .bg(look.color(token::FG_DEFAULT).opacity(0.08))
                        .children(queued.iter().map(|dot| self.chip(line, dot, cx))),
                    Role::List,
                    "Merge queue",
                )
                .into_any_element(),
            );
        }
        if !queued.is_empty() && !open.is_empty() {
            out.push(div().flex_none().w(look.length(token::BORDER_WIDTH)).h(look.length(token::H_CONTROL_SM)).bg(look.color(token::BORDER_DEFAULT)).into_any_element());
        }
        out.extend(open.iter().map(|dot| self.chip(line, dot, cx)));
        out
    }

    /// The row of one repository: main's light, then its chips, scrolling across.
    fn row_of(&self, line: usize, grouped: bool, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let mut items = vec![self.light(line, grouped, cx)];
        items.extend(self.chips_of(line, cx));
        div()
            .id(SharedString::from(format!("chips-{line}")))
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .overflow_x_scroll()
            .track_scroll(&self.scroll)
            // A wheel turned over the line moves it along, since it scrolls only across.
            .on_scroll(cx.listener(|this, event: &gpui::ScrollEvent, _, cx| {
                let delta = event.delta.pixel_delta(px(16.));
                if delta.y != px(0.) && delta.x == px(0.) {
                    let mut offset = this.scroll.offset();
                    offset.x = (offset.x + delta.y).clamp(-this.scroll.max_offset().x, px(0.));
                    this.scroll.set_offset(offset);
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .children(items)
            .into_any_element()
    }

    /// Placeholder chips while Fleet has not answered: the shapes the line will take.
    fn skeleton(&self, look: &Look) -> AnyElement {
        let shapes = [3., 5., 5., 4.];
        div()
            .flex_1()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .children(shapes.into_iter().enumerate().map(|(at, width)| {
                div()
                    .flex_none()
                    .h(look.length(token::H_CONTROL_SM))
                    .w(look.length(token::SPACE_6) * width)
                    .rounded_full()
                    .bg(look.color(token::FG_DEFAULT).opacity(0.06))
                    .with_animation(
                        SharedString::from(format!("skeleton-{at}")),
                        Animation::new(look.duration(token::DURATION_PULSE) * 2).repeat().with_easing(gpui::pulsating_between(0.5, 1.)),
                        |shape, at| shape.opacity(at),
                    )
            }))
            .into_any_element()
    }

    /// The card a hover or a press has open, over the chip it belongs to.
    fn open_card(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let card = self.card.clone()?;
        let look = cx.look();
        let (key, body) = match &card {
            Card::Main(line) => (format!("light-{line}"), self.main_card(*line, look, cx)?),
            Card::Chip(line, number) => (format!("chip-{line}-{number}"), self.chip_card(*line, *number, look, cx)?),
        };
        let from = self.bounds.borrow().get(&SharedString::from(key)).copied()?;
        // The strip is the window's foot, so the card stands over the chip.
        let at = point(from.origin.x, from.origin.y - look.length(token::SPACE_1));
        let name: SharedString = match &card {
            Card::Main(_) => "main".into(),
            Card::Chip(_, number) => format!("Pull request #{number}").into(),
        };
        Some(
            deferred(
                anchored().position(at).anchor(Anchor::BottomLeft).snap_to_window_with_margin(look.length(token::SPACE_1)).child(
                    labelled(
                        div()
                            .id("strip-card")
                            .track_focus(&self.popup_focus)
                            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| match event.keystroke.key.as_str() {
                                "escape" => {
                                    this.escape(window, cx);
                                    cx.stop_propagation();
                                }
                                "enter" if !this.pinned && matches!(this.card, Some(Card::Main(_))) => {
                                    this.pinned = true;
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
                                if this.pinned {
                                    this.shut(window, cx);
                                }
                            }))
                            .occlude()
                            .child(body),
                        Role::Region,
                        name,
                    ),
                ),
            )
            .with_priority(1)
            .into_any_element(),
        )
    }

    /// What hovering a pull request's chip shows: the ledger's card (`chip-card.tsx`).
    fn chip_card(&self, line: usize, number: u64, look: &Look, cx: &Context<Self>) -> Option<gpui::Div> {
        let one = self.data.lines.get(line)?;
        let dots = dots_of(one, &self.data.related);
        let dot = dots.iter().find(|dot| dot.pull.number == number)?;
        let pull = dot.pull;
        let ink = match dot.state {
            PullState::Ready => look.color(token::STATUS_COMPLETED_SUCCESS),
            PullState::Failing => look.color(token::STATUS_COMPLETED_FAILED),
            PullState::Running => look.color(token::STATUS_RUNNING),
            PullState::Blocked => look.color(token::STATUS_ESCALATED),
            PullState::Queued | PullState::Open => look.color(token::FG_MUTED),
        };
        let now = chrono::Utc::now();
        let ago = |at: &Option<ipc::Instant>| at.as_ref().and_then(instant).map(|at| format!("{} ago", span_said((now - at).num_seconds().max(0))));
        let mut rows: Vec<Fact> = Vec::new();
        if let Some(author) = &pull.author {
            rows.push(Fact::new("Author", author.clone()));
        }
        if let Some(check) = check_of(pull) {
            rows.push(Fact::new("Checks", check));
        }
        if let Some(place) = dot.place {
            rows.push(Fact::new("Place", place.to_string()));
        }
        if let Some(opened) = ago(&pull.opened_at) {
            rows.push(Fact::new("Opened", opened));
        }
        if let Some(updated) = ago(&pull.updated_at) {
            rows.push(Fact::new("Updated", updated));
        }
        if let Some(queued) = ago(&pull.queue.as_ref().and_then(|queue| queue.enqueued_at.clone())) {
            rows.push(Fact::new("In queue", queued));
        }
        rows.push(Fact::new("Branch", pull.branch.clone()).mono());
        if let Some(review) = pull.review.and_then(review_said) {
            rows.push(Fact::new("Review", review));
        }
        if pull.mergeable == Some(ipc::HubMergeable::Conflicting) {
            rows.push(Fact::new("Main", "Conflicts").tone(look.color(token::STATUS_COMPLETED_FAILED)));
        }
        if pull.merge_by_hand {
            rows.push(Fact::new("Merge", MERGE_BY_HAND));
        }
        let job = dot.job.cloned();
        let spec = Spec {
            kind_glyph: "git-pull-request",
            kind: "Pull request".into(),
            place: Some(format!("#{number}").into()),
            title: if pull.title.is_empty() { format!("#{number}").into() } else { pull.title.clone().into() },
            state: Some(State {
                glyph: Some("git-pull-request"),
                said: if pull.draft { format!("{}, draft", dot.says).into() } else { dot.says.into() },
                ink,
            }),
            edge: (!matches!(dot.state, PullState::Queued | PullState::Open)).then_some(ink),
            width: Width::Narrow,
            keys: vec![("Enter", "Open")],
        };
        let mut body = vec![inspect::facts(rows, look).into_any_element()];
        if let Some((id, title)) = job {
            let accent = look.color(token::ACCENT);
            let hover = look.color(token::ACCENT_HOVER);
            body.push(
                small(look)
                    .flex()
                    .justify_between()
                    .gap(look.length(token::SPACE_3))
                    .child(div().text_color(look.color(token::FG_MUTED)).child("Job"))
                    .child(labelled(
                        div()
                            .id("chip-card-job")
                            .cursor_pointer()
                            .text_color(accent)
                            .hover(move |style| style.text_color(hover).underline())
                            .child(title.clone())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.card = None;
                                this.give_keys_back(window, cx);
                                cx.emit(Asked::OpenJob(id.clone()));
                                cx.notify();
                            })),
                        Role::Button,
                        title,
                    ))
                    .into_any_element(),
            );
        }
        Some(spec.draw(body, look))
    }

    /// Main's card: the red band of each red repository, then the held checkout, then forge age and quiet.
    fn main_card(&self, line: usize, look: &Look, cx: &Context<Self>) -> Option<gpui::Div> {
        let one = self.data.lines.get(line)?;
        let red = matches!(one.view.main, Some(MainView::Red { .. }));
        let forge = forge_of(&[one]);
        let ink = if red { look.color(token::STATUS_COMPLETED_FAILED) } else { look.color(token::STATUS_COMPLETED_SUCCESS) };
        let spec = Spec {
            kind_glyph: "git-merge",
            kind: "Main".into(),
            place: None,
            title: "main".into(),
            state: Some(State { glyph: Some(if red { "circle-x" } else { "circle-check" }), said: if red { "Red" } else { "Green" }.into(), ink }),
            edge: Some(ink),
            width: Width::Wide,
            keys: if self.pinned { vec![("Esc", "Close")] } else { vec![("Enter", "Keep open")] },
        };
        let mut body: Vec<AnyElement> = Vec::new();
        if let Some(MainView::Red { failed, also, merge, taken_by, elsewhere, checking, .. }) = &one.view.main {
            body.push(self.red_band(line, failed, also, merge.as_ref(), taken_by.as_ref(), elsewhere.as_ref(), !checking.is_empty(), checking, look, cx));
        }
        if let Some(checkout) = &one.view.checkout {
            body.push(checkout_section(checkout, look));
        }
        let quiet = quiet_said(&forge, false);
        if forge.read_at.is_some() || quiet.is_some() {
            body.push(
                small(look)
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .text_color(look.color(token::FG_MUTED))
                    .children(forge.read_at.map(|at| read_said((chrono::Utc::now() - at).num_seconds().max(0))))
                    .children(quiet.map(|text| {
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .child(glyph("unplug", look.length(token::SPACE_3), look.color(token::STATUS_ESCALATED)))
                            .child(text)
                    }))
                    .into_any_element(),
            );
        }
        Some(spec.draw(body, look))
    }

    /// The red band: what failed, the pull request that broke it and the Job on it, and the act that
    /// puts a new Job on it (`MainHead.tsx`). Held, it is caution and offers nothing.
    #[allow(clippy::too_many_arguments)]
    fn red_band(
        &self,
        line: usize,
        failed: &MainRedJob,
        also: &[MainRedJob],
        merge: Option<&ipc::MainMerge>,
        taken_by: Option<&ipc::HubSession>,
        elsewhere: Option<&ipc::FixingElsewhere>,
        held: bool,
        checking: &[ipc::MainChecking],
        look: &Look,
        cx: &Context<Self>,
    ) -> AnyElement {
        let hue = if held { look.color(token::STATUS_AWAITING_REVIEW) } else { look.color(token::STATUS_COMPLETED_FAILED) };
        let title: SharedString = if held {
            let list = checking
                .iter()
                .map(|one| one.pull_request.as_ref().map_or_else(|| one.commit.chars().take(10).collect::<String>(), |merge| format!("#{}", merge.number)))
                .collect::<Vec<_>>()
                .join(" ");
            format!("New checks are running on main: {list}").into()
        } else {
            "Main is red".into()
        };
        // The acts draw where nothing makes them moot: not a held red, not a Job on it, and another machine's claim leaves only Take over.
        let free = !held && taken_by.is_none();
        let asks = free && elsewhere.is_none();
        let takes_over = free && elsewhere.is_some();
        let pending = self.fixing.as_ref() == self.data.lines.get(line).map(|line| &line.view.root);
        let text = |value: String| div().min_w_0().child(value).into_any_element();
        let mut rows: Vec<(String, AnyElement)> = Vec::new();
        for job in std::iter::once(failed).chain(also.iter()) {
            rows.push((if job.unmapped { "CI job" } else { "Check" }.into(), text(job.check.clone())));
            if let Some(test) = &job.test {
                rows.push(("Test".into(), text(test.clone())));
            }
        }
        if let Some(merge) = merge {
            let branch = merge.branch.clone().map(|branch| format!(" {branch}")).unwrap_or_default();
            let owner = merge.session.as_ref().map(|session| {
                let title = session.title.clone().unwrap_or_else(|| session.id.clone());
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(labelled(
                        div().id("merge-owner-mark").flex_none().child(glyph("square-terminal", look.length(token::SPACE_3), look.color(token::FG_MUTED))).tooltip(tip("Job")),
                        Role::Image,
                        "Job",
                    ))
                    .child(title)
            });
            let send_back = asks.then(|| {
                let label = if merge.session.is_some() { "Send back" } else { "Send back to…" };
                labelled(
                    Btn::new("send-back", label, Weight::Secondary).draw(look, cx).on_click(cx.listener(move |this, _, window, cx| this.open_fix(line, fix::Which::Back, window, cx))),
                    Role::Button,
                    label,
                )
            });
            rows.push((
                "Broke in".into(),
                div().flex().flex_wrap().items_center().gap_x(look.length(token::SPACE_3)).gap_y(look.length(token::SPACE_1)).child(format!("#{}{branch}", merge.number)).children(owner).children(send_back).into_any_element(),
            ));
        }
        // Another machine's claim: who holds it and for how long, and the one act left, to take it over.
        let claim = (elsewhere.is_some() && taken_by.is_none()).then(|| {
            let act = takes_over.then(|| {
                labelled(
                    Btn::new("take-over", "Take over", Weight::Ghost)
                        .draw(look, cx)
                        .bg(look.color(token::BG_SUNKEN))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .on_click(cx.listener(move |this, _, window, cx| this.open_fix(line, fix::Which::TakeOver, window, cx))),
                    Role::Button,
                    "Take over",
                )
                .into_any_element()
            });
            elsewhere.map(|held| fix::held_by(held, look, act)).unwrap_or_else(|| div().into_any_element())
        });
        let taken = taken_by.map(|session| session.title.clone().unwrap_or_else(|| session.id.clone()));
        let band = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(hue.opacity(0.4))
            .bg(hue.opacity(0.10))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(look.length(token::SPACE_3))
                    .child(
                        div()
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .child(title.clone()),
                    )
                    .when(asks, |head| {
                        head.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(look.length(token::SPACE_2))
                                .child(labelled(
                                    Btn::new("dispatch-fix", "Dispatch a new Job", Weight::Secondary)
                                        .pending(pending)
                                        .draw(look, cx)
                                        .on_click(cx.listener(move |this, _, _, cx| this.fix_pressed(line, cx))),
                                    Role::Button,
                                    "Dispatch a new Job",
                                ))
                                .children(merge.is_none().then(|| {
                                    labelled(
                                        Btn::new("send-back-to", "Send back to…", Weight::Secondary)
                                            .draw(look, cx)
                                            .on_click(cx.listener(move |this, _, window, cx| this.open_fix(line, fix::Which::Back, window, cx))),
                                        Role::Button,
                                        "Send back to…",
                                    )
                                })),
                        )
                    }),
            )
            .child(
                div().flex().flex_col().gap(look.length(token::BORDER_WIDTH) * 2.).font_family(mono(look)).children(rows.into_iter().map(|(term, value)| {
                    small(look)
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .child(div().flex_none().w(look.length(token::SPACE_12) * 1.5).text_color(look.color(token::FG_MUTED)).child(term))
                        .child(div().flex_1().min_w_0().child(value))
                })),
            )
            .children(claim.map(|claim| {
                small(look)
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .child(div().flex_none().w(look.length(token::SPACE_12) * 1.5).font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child("Fixing"))
                    .child(div().flex_1().min_w_0().child(claim))
            }))
            .children(taken.map(|who| {
                small(look)
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .child(div().flex_none().w(look.length(token::SPACE_12) * 1.5).font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child("Fixing"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .child(labelled(
                                div().id("fixing-mark").flex_none().child(glyph("hammer", look.length(token::SPACE_3), look.color(token::STATUS_RUNNING))).tooltip(tip("Working on it")),
                                Role::Image,
                                "Working on it",
                            ))
                            .child(who),
                    )
            }));
        labelled(band.id("red-band"), Role::Status, title).into_any_element()
    }
}

/// A held checkout's section (`YourCheckout.tsx`): its head, the title, and the fact under it.
fn checkout_section(held: &HeldCheckout, look: &Look) -> AnyElement {
    let hue = look.color(token::STATUS_AWAITING_REVIEW);
    let fact = checkout_fact(held);
    labelled(
        div()
            .id("your-checkout")
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(hue.opacity(0.4))
            .bg(hue.opacity(0.10))
            .child(
                small(look)
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_color(look.color(token::FG_MUTED))
                    .child(glyph("folder-git-2", look.length(token::SPACE_3), hue))
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Your checkout")),
            )
            .child(
                div()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(checkout_title(held)),
            )
            .when(!fact.is_empty(), |section| section.child(small(look).text_color(look.color(token::FG_MUTED)).child(fact))),
        Role::Region,
        "Your checkout",
    )
    .into_any_element()
}

impl Render for MergeStrip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shown = self.shown();
        for &line in &shown {
            self.focus_for(SharedString::from(format!("light-{line}")), Card::Main(line), window, cx);
            let numbers: Vec<u64> = dots_of(&self.data.lines[line], &self.data.related).iter().map(|dot| dot.pull.number).collect();
            for number in numbers {
                self.focus_for(SharedString::from(format!("chip-{line}-{number}")), Card::Chip(line, number), window, cx);
            }
        }
        let look = cx.look();
        if shown.is_empty() && !self.data.loading {
            // None served: nothing, and no sentence about the absence.
            return Empty.into_any_element();
        }

        let grouped = shown.len() > 1;
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let row = if self.data.loading {
            self.skeleton(look)
        } else if grouped {
            let at = self.row.min(shown.len() - 1);
            self.row_of(shown[at], true, cx)
        } else {
            self.row_of(shown[0], false, cx)
        };
        let merge = labelled(
            no_drag(
                div()
                    .id("merge-line")
                    .flex_none()
                    .size(look.length(token::H_CONTROL))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(gpui::transparent_black())
                    .text_color(look.color(token::FG_SUBTLE))
                    .cursor_pointer()
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.border_color(ring))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                    .child(glyph("git-merge", look.length(token::SPACE_4), look.color(token::FG_SUBTLE)))
                    .tooltip(tip("Merge line"))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(Asked::MergeLine))),
            ),
            Role::Button,
            "Merge line",
        );
        let card = self.open_card(cx);
        labelled(
            div()
                .id("merge-strip")
                .key_context("strip")
                .track_focus(&self.focus)
                // Tab from the strip goes to its own chips, not the first stop in the window.
                .tab_group()
                .flex_none()
                .w_full()
                .flex()
                .flex_col()
                .bg(look.color(token::BG_SUNKEN))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .pb(look.length(token::SPACE_1))
                .on_hover(cx.listener(|this, over: &bool, _, _| this.over = *over))
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| match event.keystroke.key.as_str() {
                    "up" => {
                        this.step_row(-1, cx);
                        cx.stop_propagation();
                    }
                    "down" => {
                        this.step_row(1, cx);
                        cx.stop_propagation();
                    }
                    "escape" if this.card.is_some() => {
                        this.escape(window, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .min_h(look.length(token::H_CONTROL) + look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_4))
                        .text_color(look.color(token::FG_SUBTLE))
                        .child(merge)
                        .child(row),
                )
                .children(card)
                .children(self.menu.as_ref().map(|menu| menu.dropdown.clone()))
                .children(self.fix_layer(look, window.viewport_size(), cx)),
            Role::Region,
            "Merge queue",
        )
        .into_any_element()
    }
}
