//! The feature a note belongs to, beside the note. When a note's card is shown its features are read off the main thread
//! (`features::of_path`), and drawn as chips in a strip under the card; one is open at a time, its page in a
//! `FeatureOverlay` in a panel to the side of the card. The overlay's own rule holds there: what is opened from inside it
//! replaces its content, and each Esc goes back, the last one closing it. Another Esc then closes the card, as before.
//!
//! All of it is one entity the layer holds, and the layer calls it from two places: `Beside::new` where it builds its
//! children and [`attach`] where it draws, which hangs the entity on the layer's root and takes the keys of the card.
//! The entity learns which card is open by observing the layer.
//!
//! The page goes to the Job with the note: `live.rs` appends a `Feature: <slug>` line for each feature the note maps to,
//! which is the line the Job's hooks hand a feature's page by (`docs/concepts/feature-catalog.md`). A send runs off the
//! main thread, so the slugs it needs are in [`slugs_of`] by the note's key.

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Mutex;

use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, px, Anchor, AnyElement, App, Bounds, Context, Div, Entity, FocusHandle, FontWeight, KeyDownEvent, Pixels, Role, SharedString,
    Stateful, Subscription, Task, WeakEntity, Window,
};
use serde_json::Value;

use super::{keyed, Layer};
use crate::features::of_path::{self, Mapped, Place, Wanted};
use crate::features::overlay::{FeatureOverlay, OverlayEvent, Source};
use crate::labels::{labelled, LabelBook};
use crate::look::{token, ActiveLook, Look};
use crate::settings::view::{bone, tip};
use crate::titlebar::parts::{card, glyph};

/// Each note's features by slug, as last read: what `live::note_of` hands the Job, on the thread the send runs on.
static SLUGS: Mutex<BTreeMap<String, Vec<String>>> = Mutex::new(BTreeMap::new());

/// The slugs of the features the note with this key maps to, in the order they are drawn.
pub(super) fn slugs_of(key: &str) -> Vec<String> {
    SLUGS.lock().map(|held| held.get(key).cloned().unwrap_or_default()).unwrap_or_default()
}

pub(super) fn remember(key: &str, slugs: Vec<String>) {
    if let Ok(mut held) = SLUGS.lock() {
        held.insert(key.to_string(), slugs);
    }
}

/// A feature in the strip, and the keys' place on it.
struct Chip {
    feature: Mapped,
    focus: FocusHandle,
}

/// What a note's card knows of its features: absent from the entity until its card is shown.
enum Read {
    Reading,
    Done(Vec<Chip>),
}

/// The page that is open beside a card.
struct Page {
    slug: String,
    overlay: Entity<FeatureOverlay>,
    _sub: Subscription,
}

/// What a read needs of a note, taken off the layer.
struct Subject {
    id: u32,
    key: String,
    raw: Value,
    root: Option<PathBuf>,
}

pub(super) struct Beside {
    layer: WeakEntity<Layer>,
    /// Where features are read from: Fleet, or the mock's own. With none given a note's card draws no chips.
    from: Option<Source>,
    /// The note whose card is open, by id and by key.
    shown: Option<(u32, String)>,
    reads: HashMap<String, Read>,
    reading: HashMap<String, Task<()>>,
    page: Option<Page>,
    /// Where the opened card was last drawn, so the strip and the page can stand by it.
    card: Rc<Cell<Option<Bounds<Pixels>>>>,
    _follow: Subscription,
}

/// The widest the page is drawn, in `SPACE_8` steps: the rail of sections and a pane of rows.
const STEPS_WIDE: f32 = 26.;

/// Where the page stands beside a card inside `room`: its left edge and its width. To the right of the card where
/// `want` fits, else to the left, else on the side with more room at what that gives, and where neither side holds
/// three fifths of `want` over the card's right edge.
pub(super) fn side(card: Bounds<Pixels>, room: Bounds<Pixels>, gap: Pixels, want: Pixels) -> (Pixels, Pixels) {
    let from_right = card.origin.x + card.size.width + gap;
    let right = room.origin.x + room.size.width - gap - from_right;
    let left = card.origin.x - gap - (room.origin.x + gap);
    if right >= want {
        (from_right, want)
    } else if left >= want {
        (card.origin.x - gap - want, want)
    } else {
        let least = want * 0.6;
        let (best, edge) = if right >= left { (right, from_right) } else { (left, room.origin.x + gap) };
        if best >= least {
            (edge, best)
        } else {
            (room.origin.x + room.size.width - gap - least, least)
        }
    }
}

impl Layer {
    fn subject(&self, id: u32) -> Option<Subject> {
        let note = self.note(id)?;
        let root = self.dir.as_ref().and_then(|dir| dir.parent()?.parent()).map(std::path::Path::to_path_buf);
        Some(Subject { id, key: note.key.clone(), raw: note.raw.clone(), root })
    }

    /// Where features are read from: Fleet, or the mock's own. With none given a note's card draws no chips.
    pub fn feature_source(&mut self, source: Source, cx: &mut Context<Self>) {
        let opened = self.opened.and_then(|id| self.subject(id));
        self.beside.update(cx, |beside, cx| beside.from_source(source, opened, cx));
    }

    /// Whether a source of features was given, so a note's card draws its chips.
    pub(crate) fn has_feature_source(&self, cx: &App) -> bool {
        self.beside.read(cx).from.is_some()
    }

    /// Notes read back from where they are kept, as the layer reads them when it comes on.
    pub(crate) fn read_notes(&mut self, found: Vec<Value>, cx: &mut Context<Self>) {
        self.read_back(found, cx);
    }

    /// The slug of the feature open beside the card.
    pub(crate) fn beside_slug(&self, cx: &App) -> Option<String> {
        self.beside.read(cx).page.as_ref().map(|page| page.slug.clone())
    }

    /// The overlay the page beside the card is in.
    pub(crate) fn features_overlay(&self, cx: &App) -> Option<Entity<FeatureOverlay>> {
        self.beside.read(cx).page.as_ref().map(|page| page.overlay.clone())
    }

    /// `f`: the first feature of the card's note opens, and each press after it the next, round again.
    pub(crate) fn open_next_feature(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.opened else { return false };
        let from = self.named(id);
        self.beside.update(cx, |beside, cx| beside.open_next(from, window, cx))
    }

    /// How a page says which note it was opened from.
    fn named(&self, id: u32) -> String {
        self.number(id).map_or("Done note".to_string(), |number| format!("Note {number}"))
    }
}

impl Beside {
    /// The entity the layer holds. It watches the layer, so it knows which card is open and when it closes.
    pub(super) fn new(cx: &mut Context<Layer>) -> Entity<Beside> {
        let layer = cx.entity();
        let weak = cx.weak_entity();
        cx.new(|cx| Beside {
            layer: weak,
            from: None,
            shown: None,
            reads: HashMap::new(),
            reading: HashMap::new(),
            page: None,
            card: Rc::default(),
            _follow: cx.observe(&layer, Beside::follow),
        })
    }

    /// The layer changed: another card opened, or the card closed. The page of the one before is put away.
    fn follow(&mut self, layer: Entity<Layer>, cx: &mut Context<Self>) {
        let opened = layer.read(cx).opened;
        if opened == self.shown.as_ref().map(|(id, _)| *id) {
            return;
        }
        let subject = opened.and_then(|id| layer.read(cx).subject(id));
        self.shown = subject.as_ref().map(|subject| (subject.id, subject.key.clone()));
        self.put_away(cx);
        if let Some(subject) = subject {
            self.read(subject, cx);
        }
        cx.notify();
    }

    fn from_source(&mut self, source: Source, opened: Option<Subject>, cx: &mut Context<Self>) {
        self.from = Some(source);
        if let Some(subject) = opened {
            self.shown = Some((subject.id, subject.key.clone()));
            self.read(subject, cx);
        }
        cx.notify();
    }

    /// Read the features of a note, off the main thread: its card is shown. A note read once is not read again.
    fn read(&mut self, subject: Subject, cx: &mut Context<Self>) {
        let Some(source) = self.from.clone() else { return };
        if self.reads.contains_key(&subject.key) {
            return;
        }
        let place: Place = of_path::place_of(&subject.raw, subject.root.as_deref());
        if place.files.is_empty() && place.components.is_empty() {
            return self.got(subject.key, Vec::new(), cx);
        }
        match source {
            Source::Mock => {
                let found = of_path::mock_resolve(&place);
                self.got(subject.key, found, cx);
            }
            Source::Live(fleet) => {
                let held = fleet.read(cx).held();
                let Some(port) = held.port() else { return };
                let projects = held.projects.as_ref().map(|read| read.projects.iter().map(|one| one.id.clone()).collect()).unwrap_or_default();
                let project = crate::titlebar::live(cx).and_then(|bar| bar.read(cx).face.scoped_project().map(str::to_string));
                let wanted = Wanted { port, project, projects };
                let key = subject.key.clone();
                self.reads.insert(key.clone(), Read::Reading);
                let task = cx.spawn(async move |this, cx| {
                    let found = of_path::resolve(wanted, place, &*cx).await;
                    let _ = this.update(cx, |this, cx| this.got(subject.key, found, cx));
                });
                self.reading.insert(key, task);
                cx.notify();
            }
        }
    }

    fn got(&mut self, key: String, found: Vec<Mapped>, cx: &mut Context<Self>) {
        self.reading.remove(&key);
        remember(&key, found.iter().map(|feature| feature.slug.clone()).collect());
        let chips = found.into_iter().map(|feature| Chip { feature, focus: cx.focus_handle().tab_stop(true) }).collect();
        self.reads.insert(key, Read::Done(chips));
        cx.notify();
    }

    /// The features of the note `key`, once read: none while the read is out.
    pub(super) fn features_of(&self, key: &str) -> Vec<Mapped> {
        match self.reads.get(key) {
            Some(Read::Done(chips)) => chips.iter().map(|chip| chip.feature.clone()).collect(),
            _ => Vec::new(),
        }
    }

    /// The chips of the opened note's card, once read.
    fn chips_of_shown(&self) -> &[Chip] {
        match self.shown.as_ref().and_then(|(_, key)| self.reads.get(key)) {
            Some(Read::Done(chips)) => chips,
            _ => &[],
        }
    }

    /// Which chip the keys are on.
    fn on_chip(&self, window: &Window) -> Option<usize> {
        self.chips_of_shown().iter().position(|chip| chip.focus.is_focused(window))
    }

    /// Open the page of feature `slug` of the opened note beside its card, in place of the one that was open.
    fn open(&mut self, slug: String, from: String, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(page) = self.page.as_ref().filter(|page| page.slug == slug) {
            page.overlay.update(cx, |overlay, cx| overlay.focus_top(window, cx));
            return;
        }
        let Some(source) = self.from.clone() else { return };
        let Some(mapped) = self.chips_of_shown().iter().find(|chip| chip.feature.slug == slug).map(|chip| chip.feature.clone()) else { return };
        self.put_away(cx);
        let overlay = cx.new(|cx| FeatureOverlay::on_page(source, mapped.project, from, mapped.slug.clone(), mapped.title, window, cx));
        let sub = cx.subscribe_in(&overlay, window, |this, _, event: &OverlayEvent, window, cx| match event {
            OverlayEvent::Closed => this.closed(window, cx),
        });
        self.page = Some(Page { slug, overlay, _sub: sub });
        cx.notify();
    }

    fn open_next(&mut self, from: String, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let slugs: Vec<String> = self.chips_of_shown().iter().map(|chip| chip.feature.slug.clone()).collect();
        if slugs.is_empty() {
            return false;
        }
        let at = self.page.as_ref().and_then(|page| slugs.iter().position(|slug| *slug == page.slug));
        let next = at.map_or(0, |at| (at + 1) % slugs.len());
        self.open(slugs[next].clone(), from, window, cx);
        true
    }

    /// Esc on the card with a page beside it: the page goes back one layer, and the last one closes it.
    fn back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(page) = &self.page {
            page.overlay.update(cx, |overlay, cx| overlay.back(window, cx));
        }
    }

    /// The page closed itself: the keys go back to the card.
    fn closed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.page = None;
        if let Some(card) = self.layer.upgrade().map(|layer| layer.read(cx).card_focus.clone()) {
            window.focus(&card, cx);
        }
        cx.notify();
    }

    /// The page dropped without a word, as its note's card closes or another note's opens: the page is stamped as looked at.
    fn put_away(&mut self, cx: &mut Context<Self>) {
        if let Some(page) = self.page.take() {
            drop(page._sub);
            page.overlay.update(cx, |overlay, cx| overlay.put_away(cx));
        }
    }

    /// The chips: bones while the read is out, a chip for each feature, and nothing for a note that maps to none.
    fn chips(&self, key: &str, look: &Look, window: &Window, from: &str, cx: &Context<Self>) -> Option<AnyElement> {
        let row = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2));
        match self.reads.get(key)? {
            Read::Reading => Some(
                labelled(
                    row.id("note-features-loading").child(bone(look, px(96.), look.length(token::H_BADGE))).child(bone(look, px(72.), look.length(token::H_BADGE))),
                    Role::Image,
                    "Loading",
                )
                .into_any_element(),
            ),
            Read::Done(chips) if chips.is_empty() => None,
            Read::Done(chips) => {
                let open = self.page.as_ref().map(|page| page.slug.as_str());
                let chips = chips.iter().enumerate().map(|(at, chip)| {
                    let slug = chip.feature.slug.clone();
                    let on = open == Some(slug.as_str());
                    let title = chip.feature.title.clone();
                    let name = format!("Feature: {title}");
                    let from = from.to_string();
                    let pill = div()
                        .id(SharedString::from(format!("note-feature-{at}")))
                        .flex_none()
                        .max_w_full()
                        .h(look.length(token::H_BADGE))
                        .px(look.length(token::SPACE_2))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(if on { look.color(token::ACCENT) } else { gpui::transparent_black() })
                        .bg(look.color(token::ACCENT_MUTED))
                        .text_size(look.length(token::TEXT_2XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .text_color(look.color(token::ACCENT))
                        .whitespace_nowrap()
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::ACCENT_MUTED).opacity(0.8)))
                        .tooltip(tip(format!("Open the page of {title}")))
                        .child(glyph("book-open", look.length(token::SPACE_3), look.color(token::ACCENT)))
                        .child(div().truncate().child(title));
                    let pill = keyed(look, pill, Some((&chip.focus, chip.focus.is_focused(window))));
                    labelled(pill.on_click(cx.listener(move |this, _, window, cx| this.open(slug.clone(), from.clone(), window, cx))), Role::Button, name)
                });
                Some(row.id("note-features").children(chips).into_any_element())
            }
        }
    }

    /// Keeps where the card is drawn, read from the label book once the card has been prepainted, so the strip and the
    /// page can stand by it; the frame is drawn again when it moves.
    fn measure(&self) -> impl IntoElement {
        let seen = Rc::clone(&self.card);
        deferred(
            canvas(
                move |_, window, cx| {
                    let id = window.window_handle().window_id();
                    let now = cx.default_global::<LabelBook>().drawn(id).iter().rev().find(|one| one.role == Role::Dialog && one.name == "Note").map(|one| one.bounds);
                    if seen.replace(now) != now {
                        window.refresh();
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_0(),
        )
        .with_priority(super::OVER)
    }

    /// The page, in a panel of its own: to the right of the card, else to its left, and as tall as the window allows.
    fn panel(&self, at: Bounds<Pixels>, bar: Option<Bounds<Pixels>>, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let page = self.page.as_ref()?;
        let room = crate::capture::under_title(window, cx);
        let gap = look.length(token::SPACE_2);
        let (x, wide) = side(at, room, gap, look.length(token::SPACE_8) * STEPS_WIDE);
        // As tall as the window allows, down to the bar along its foot.
        let top = room.origin.y + gap;
        let bottom = bar.map_or(room.origin.y + room.size.height - gap, |bar| bar.origin.y - gap);
        let tall = (bottom - top).max(px(0.));
        let panel = card(look).id("note-feature-page").occlude().w(wide).h(tall).overflow_hidden().child(page.overlay.clone());
        Some(deferred(anchored().anchor(Anchor::TopLeft).position(point(x, top)).child(labelled(panel, Role::Dialog, "Feature page"))).with_priority(super::OVER).into_any_element())
    }
}

impl Render for Beside {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let nothing = div().absolute();
        let Some((id, key)) = self.shown.clone() else { return nothing.into_any_element() };
        if !matches!(self.reads.get(&key), Some(Read::Reading)) && self.chips_of_shown().is_empty() {
            return nothing.into_any_element();
        }
        let look = cx.look();
        let gap = look.length(token::SPACE_2);
        let (from, bar) = self.layer.upgrade().map(|layer| (layer.read(cx).named(id), layer.read(cx).bar_at.get())).unwrap_or_default();
        // Under the card, as wide as it is; where the card is not known yet, only what measures it.
        let drawn = self.card.get().map(|at| {
            let strip = self.chips(&key, look, window, &from, cx).map(|chips| {
                let below = point(at.origin.x, at.origin.y + at.size.height + gap);
                let row = div().id("note-features-strip").occlude().w(at.size.width).child(chips);
                deferred(anchored().anchor(Anchor::TopLeft).position(below).snap_to_window_with_margin(gap).child(row)).with_priority(super::OVER)
            });
            (strip, self.panel(at, bar, look, window, cx))
        });
        let (strip, panel) = drawn.unwrap_or_default();
        nothing.children(strip).children(panel).child(self.measure()).into_any_element()
    }
}

/// The layer's one call into the entity, made where it draws its root: the entity is a child of the root, and the card's
/// keys are taken there. Esc goes to the page before it goes to the card, and from a chip it closes the card; `f` opens
/// the page of the note's feature, where the keys are on the card or a chip.
pub(super) fn attach(root: Stateful<Div>, layer: &Layer, window: &Window, cx: &mut Context<Layer>) -> Stateful<Div> {
    let beside = layer.beside.read(cx);
    let keys_here = layer.card_focus.contains_focused(window, cx) || beside.on_chip(window).is_some();
    let takes_f = keys_here && !beside.chips_of_shown().is_empty();
    root.child(layer.beside.clone())
        .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
            let beside = this.beside.read(cx);
            let (page, chip) = (beside.page.is_some(), beside.on_chip(window).is_some());
            if event.keystroke.key != "escape" || !(chip || page && this.card_focus.contains_focused(window, cx)) {
                return;
            }
            cx.stop_propagation();
            if page {
                this.beside.update(cx, |beside, cx| beside.back(window, cx));
            } else {
                this.close_card(window, cx);
            }
        }))
        .when(takes_f, |root| {
            root.key_context(crate::generated::actions::ActionScope::Features.key_context().unwrap_or_default()).on_action(cx.listener(|this, act: &crate::keymap::Act, window, cx| {
                if !(act.is("note_feature") && this.open_next_feature(window, cx)) {
                    cx.propagate();
                }
            }))
        })
}

#[cfg(test)]
mod feature_tests;
