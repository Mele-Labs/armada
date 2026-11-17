//! The feature a note belongs to, beside the note. When a note's card is shown its features are read off the main thread
//! (`features::of_path`), and drawn as chips on the card; one is open at a time, its page in a `FeatureOverlay` in a panel
//! to the side of the card. The overlay's own rule holds there: what is opened from inside it replaces its content, and
//! each Esc goes back, the last one closing it. Another Esc then closes the card, as before.
//!
//! The page goes to the Job with the note: `live.rs` appends a `Feature: <slug>` line for each feature the note maps to,
//! which is the line the Job's hooks hand a feature's page by (`docs/concepts/feature-catalog.md`).

use std::collections::HashMap;

use gpui::{anchored, canvas, deferred, div, point, prelude::*, px, Anchor, AnyElement, Bounds, Context, Entity, FocusHandle, FontWeight, Pixels, Role, SharedString, Subscription, Task, Window};

use super::{keyed, Layer};
use crate::features::of_path::{self, Mapped, Place, Wanted};
use crate::features::overlay::{FeatureOverlay, OverlayEvent, Source};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::settings::view::{bone, tip};
use crate::titlebar::parts::{card, glyph};

/// A feature on a note's card, and the keys' place on it.
pub(crate) struct Chip {
    pub feature: Mapped,
    focus: FocusHandle,
}

impl Chip {
    pub(crate) fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }
}

/// What a note's card knows of its features.
#[derive(Default)]
pub(crate) enum Features {
    /// Not read yet: the card was not shown, or no Fleet was given to ask.
    #[default]
    Unread,
    Reading,
    Read(Vec<Chip>),
}

impl Features {
    /// The slugs a send names, in the order they are drawn.
    pub fn slugs(&self) -> Vec<String> {
        match self {
            Features::Read(chips) => chips.iter().map(|chip| chip.feature.slug.clone()).collect(),
            _ => Vec::new(),
        }
    }
}

/// The page that is open beside a card.
pub(super) struct Beside {
    note: u32,
    slug: String,
    overlay: Entity<FeatureOverlay>,
    _sub: Subscription,
}

/// What the layer holds to read and show features: where Fleet is, the reads under way, and the page open.
#[derive(Default)]
pub(super) struct Held {
    pub from: Option<Source>,
    reading: HashMap<u32, Task<()>>,
    beside: Option<Beside>,
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
    /// Where features are read from: Fleet, or the mock's own. With none given a note's card draws no chips.
    pub fn feature_source(&mut self, source: Source, cx: &mut Context<Self>) {
        self.features.from = Some(source);
        cx.notify();
    }

    /// The repository the notes are in, to make a path in a note relative to it.
    fn repository(&self) -> Option<std::path::PathBuf> {
        self.dir.as_ref().and_then(|dir| dir.parent()?.parent()).map(std::path::Path::to_path_buf)
    }

    /// Read the features of note `id`, off the main thread: its card is shown. A note read once is not read again.
    pub(super) fn read_features(&mut self, id: u32, cx: &mut Context<Self>) {
        let Some(source) = self.features.from.clone() else { return };
        let root = self.repository();
        let Some(note) = self.notes.iter().find(|note| note.id == id) else { return };
        if !matches!(note.features, Features::Unread) {
            return;
        }
        let place: Place = of_path::place_of(&note.raw, root.as_deref());
        if place.files.is_empty() && place.components.is_empty() {
            return self.got_features(id, Vec::new(), cx);
        }
        match source {
            Source::Mock => {
                let found = of_path::mock_resolve(&place);
                self.got_features(id, found, cx);
            }
            Source::Live(fleet) => {
                let held = fleet.read(cx).held();
                let Some(port) = held.port() else { return };
                let projects = held.projects.as_ref().map(|read| read.projects.iter().map(|one| one.id.clone()).collect()).unwrap_or_default();
                let project = crate::titlebar::live(cx).and_then(|bar| bar.read(cx).face.scoped_project().map(str::to_string));
                let wanted = Wanted { port, project, projects };
                if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
                    note.features = Features::Reading;
                }
                let task = cx.spawn(async move |this, cx| {
                    let found = of_path::resolve(wanted, place, &*cx).await;
                    let _ = this.update(cx, |this, cx| this.got_features(id, found, cx));
                });
                self.features.reading.insert(id, task);
                cx.notify();
            }
        }
    }

    fn got_features(&mut self, id: u32, found: Vec<Mapped>, cx: &mut Context<Self>) {
        self.features.reading.remove(&id);
        let chips = found.into_iter().map(|feature| Chip { feature, focus: cx.focus_handle().tab_stop(true) }).collect();
        if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
            note.features = Features::Read(chips);
        }
        cx.notify();
    }

    /// The slug of the feature open beside the card.
    pub(crate) fn beside_slug(&self) -> Option<&str> {
        self.features.beside.as_ref().map(|beside| beside.slug.as_str())
    }

    /// The overlay the page beside the card is in.
    pub(crate) fn features_overlay(&self) -> Option<&Entity<FeatureOverlay>> {
        self.features.beside.as_ref().map(|beside| &beside.overlay)
    }

    /// Whether a page is open beside a card.
    pub(crate) fn has_beside(&self) -> bool {
        self.features.beside.is_some()
    }

    /// Open the page of feature `slug` of note `id` beside its card, in place of the one that was open.
    pub(crate) fn open_feature(&mut self, id: u32, slug: String, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(beside) = self.features.beside.as_ref().filter(|beside| beside.note == id && beside.slug == slug) {
            beside.overlay.update(cx, |overlay, cx| overlay.focus_top(window, cx));
            return;
        }
        let Some(source) = self.features.from.clone() else { return };
        let Some(Features::Read(chips)) = self.notes.iter().find(|note| note.id == id).map(|note| &note.features) else { return };
        let Some(mapped) = chips.iter().find(|chip| chip.feature.slug == slug).map(|chip| chip.feature.clone()) else { return };
        let from = self.number(id).map_or("Done note".to_string(), |number| format!("Note {number}"));
        self.put_away_beside(cx);
        let overlay = cx.new(|cx| FeatureOverlay::on_page(source, mapped.project, from, mapped.slug.clone(), mapped.title, window, cx));
        let sub = cx.subscribe_in(&overlay, window, |this, _, event: &OverlayEvent, window, cx| match event {
            OverlayEvent::Closed => this.close_beside(window, cx),
        });
        self.features.beside = Some(Beside { note: id, slug, overlay, _sub: sub });
        cx.notify();
    }

    /// `f`: the first feature of the card's note opens, and each press after it the next, round again.
    pub(crate) fn open_next_feature(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.opened else { return false };
        let slugs = self.note(id).map(|note| note.features.slugs()).unwrap_or_default();
        if slugs.is_empty() {
            return false;
        }
        let at = self.features.beside.as_ref().filter(|beside| beside.note == id).and_then(|beside| slugs.iter().position(|slug| *slug == beside.slug));
        let next = at.map_or(0, |at| (at + 1) % slugs.len());
        self.open_feature(id, slugs[next].clone(), window, cx);
        true
    }

    /// Esc on the card with a page beside it: the page goes back one layer, and the last one closes it.
    pub(crate) fn back_beside(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(beside) = &self.features.beside {
            beside.overlay.update(cx, |overlay, cx| overlay.back(window, cx));
        }
    }

    /// The page closed itself: the keys go back to the card.
    fn close_beside(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.features.beside = None;
        let card = self.card_focus.clone();
        window.focus(&card, cx);
        cx.notify();
    }

    /// The page dropped without a word, as its note's card closes or another note's opens: the page is stamped as looked at.
    pub(super) fn put_away_beside(&mut self, cx: &mut Context<Self>) {
        if let Some(beside) = self.features.beside.take() {
            drop(beside._sub);
            beside.overlay.update(cx, |overlay, cx| overlay.put_away(cx));
        }
    }

    /// The chips of a note's card: bones while the read is out, a chip for each feature, and nothing for a note that maps to none.
    pub(super) fn chips(&self, id: u32, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let row = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2));
        match &self.note(id)?.features {
            Features::Unread => None,
            Features::Reading => Some(
                labelled(
                    row.id("note-features-loading").child(bone(look, px(96.), look.length(token::H_BADGE))).child(bone(look, px(72.), look.length(token::H_BADGE))),
                    Role::Image,
                    "Loading",
                )
                .into_any_element(),
            ),
            Features::Read(chips) if chips.is_empty() => None,
            Features::Read(chips) => {
                let open = self.beside_slug().map(str::to_string);
                let chips = chips.iter().enumerate().map(|(at, chip)| {
                    let slug = chip.feature.slug.clone();
                    let on = open.as_deref() == Some(slug.as_str());
                    let title = chip.feature.title.clone();
                    let name = format!("Feature: {title}");
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
                    labelled(pill.on_click(cx.listener(move |this, _, window, cx| this.open_feature(id, slug.clone(), window, cx))), Role::Button, name)
                });
                Some(row.id("note-features").children(chips).into_any_element())
            }
        }
    }

    /// The page beside the opened note's card, in a panel of its own: to the right of the card, else to its left, and as tall
    /// as the window allows.
    pub(super) fn beside_panel(&self, window: &Window, cx: &Context<Self>) -> Option<impl IntoElement> {
        let beside = self.features.beside.as_ref().filter(|beside| self.opened == Some(beside.note))?;
        let look = cx.look();
        let room = crate::capture::under_title(window, cx);
        let gap = look.length(token::SPACE_2);
        let at = self.card_box.get()?;
        let (x, wide) = side(at, room, gap, look.length(token::SPACE_8) * STEPS_WIDE);
        // As tall as the window allows, down to the bar along its foot.
        let top = room.origin.y + gap;
        let bottom = self.bar_at.get().map_or(room.origin.y + room.size.height - gap, |bar| bar.origin.y - gap);
        let tall = (bottom - top).max(px(0.));
        let panel = card(look)
            .id("note-feature-page")
            .occlude()
            .w(wide)
            .h(tall)
            .overflow_hidden()
            .child(beside.overlay.clone());
        Some(deferred(anchored().anchor(Anchor::TopLeft).position(point(x, top)).child(labelled(panel, Role::Dialog, "Feature page"))).with_priority(super::OVER))
    }

    /// A canvas that keeps where the card is drawn, so the page can stand beside it; the frame is drawn again when it moves.
    pub(super) fn measure_card(&self) -> impl IntoElement {
        let at = std::rc::Rc::clone(&self.card_box);
        canvas(
            move |bounds, window, _| {
                if at.replace(Some(bounds)) != Some(bounds) {
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full()
    }
}
