//! The Features place as a gallery, as People in Photos shows a person by a face: each area a band with its features as tiles,
//! each tile the feature's cover, its newest standing picture, with its title under it. Opening a tile shows every standing
//! picture newest first, each with Not this feature, and under Also this? the pictures the librarian thinks may belong to it,
//! each with Yes and Not this feature. `docs/concepts/feature-catalog.md`, *Pictures*.
//!
//! Mock first: it plays on `mock::gallery` and reads no Fleet until he approves the look. The keys are the `features` scope's:
//! the arrows move between tiles and cards, Enter opens a tile or presses the button the keys are on, Tab goes through a card's
//! buttons, and Esc goes back.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::DateTime;
use gpui::{point, px, AppContext as _, Context, Entity, FocusHandle, Image, ImageFormat, KeyDownEvent, ScrollHandle, Subscription, Window};
use gpui_component::input::{InputEvent, InputState};
use ipc::{FeatureAreaRow, FeaturePicture, FeatureRow};

use crate::keymap::Act;
use crate::look::{token, ActiveLook, Look};

/// Whether a card is one of the feature's own pictures or one the librarian thinks may belong to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Standing,
    Suggested,
}

impl Kind {
    /// The card's buttons, in the order Tab reaches them.
    pub(super) fn buttons(self) -> &'static [&'static str] {
        match self {
            Kind::Standing => &[NOT_THIS],
            Kind::Suggested => &[YES, NOT_THIS],
        }
    }
}

pub(super) const YES: &str = "Yes";
pub(super) const NOT_THIS: &str = "Not this feature";

/// The feature open over the gallery, and where the keys are on it.
pub(super) struct Opened {
    pub slug: String,
    /// The card the keys are on: the standing pictures first, then the suggestions.
    pub at: usize,
    /// The button of that card the keys are on, by its place in `Kind::buttons`. None while they are on the card itself.
    pub button: Option<usize>,
    pub why: Option<Why>,
}

/// The box that asks why a picture is not this feature, open under that picture.
pub(super) struct Why {
    pub picture: String,
    pub input: Entity<InputState>,
    _sub: Subscription,
}

pub struct Gallery {
    pub(super) areas: Vec<FeatureAreaRow>,
    /// Every standing picture by feature slug, newest first.
    pub(super) pictures: HashMap<String, Vec<FeaturePicture>>,
    /// What the librarian thinks may belong to a feature, by its slug. No wire carries these yet: the mock's alone.
    pub(super) suggested: HashMap<String, Vec<FeaturePicture>>,
    /// Each picture's bytes as gpui decodes them, made once so the asset cache keeps one decode of each.
    pub(super) images: HashMap<String, Arc<Image>>,
    /// The tile the keys are on: its area's place, and its place in the area.
    pub(super) cursor: (usize, usize),
    pub(super) opened: Option<Opened>,
    pub focus: FocusHandle,
    pub within: FocusHandle,
    pub(super) scroll: ScrollHandle,
    pub(super) page_scroll: ScrollHandle,
}

/// The newest first, by when each was taken.
fn newest_first(pictures: &mut [FeaturePicture]) {
    pictures.sort_by_key(|one| std::cmp::Reverse(DateTime::parse_from_rfc3339(one.taken_at.as_str()).ok()));
}

/// How many tiles a row of an area holds in a window `wide` across: the screen's sides and the area's own are taken off, and
/// as many tiles and the gaps between them as fit are left.
pub(super) fn columns(look: &Look, wide: gpui::Pixels) -> usize {
    let inside = wide - (look.length(token::SPACE_6) + look.length(token::SPACE_4) + look.length(token::BORDER_WIDTH)) * 2.;
    let gap = look.length(token::SPACE_4);
    (((inside + gap) / (tile_width(look) + gap)).floor() as usize).max(1)
}

/// A tile's width: five of the largest space.
pub(super) fn tile_width(look: &Look) -> gpui::Pixels {
    look.length(token::SPACE_12) * 5.
}

impl Gallery {
    pub fn mock(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mock = super::mock::gallery();
        let mut images = HashMap::new();
        for one in mock.pictures.iter().chain(mock.suggested.iter()).flat_map(|(_, shown)| shown.iter()) {
            if let Some(png) = super::mock::picture_png(&one.id) {
                images.insert(one.id.clone(), Arc::new(Image::from_bytes(ImageFormat::Png, png.to_vec())));
            }
        }
        Gallery {
            areas: mock.library.areas,
            pictures: mock.pictures.into_iter().collect(),
            suggested: mock.suggested.into_iter().collect(),
            images,
            cursor: (0, 0),
            opened: None,
            focus: cx.focus_handle(),
            within: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            page_scroll: ScrollHandle::new(),
        }
    }

    pub(super) fn row(&self, slug: &str) -> Option<&FeatureRow> {
        self.areas.iter().flat_map(|area| area.features.iter()).find(|row| row.slug == slug)
    }

    /// The area `slug` is in, by its place.
    pub(super) fn area_of(&self, slug: &str) -> Option<usize> {
        self.areas.iter().position(|area| area.features.iter().any(|row| row.slug == slug))
    }

    /// The opened feature's cards: its standing pictures newest first, then the suggestions.
    pub(super) fn cards(&self, slug: &str) -> Vec<(Kind, &FeaturePicture)> {
        let standing = self.pictures.get(slug).into_iter().flatten().map(|one| (Kind::Standing, one));
        let suggested = self.suggested.get(slug).into_iter().flatten().map(|one| (Kind::Suggested, one));
        standing.chain(suggested).collect()
    }

    fn standing_count(&self, slug: &str) -> usize {
        self.pictures.get(slug).map_or(0, Vec::len)
    }

    fn suggested_count(&self, slug: &str) -> usize {
        self.suggested.get(slug).map_or(0, Vec::len)
    }

    // ---- the gallery --------------------------------------------------------------------------

    /// The tile `by` places along, from one area's last into the next's first.
    fn step_tile(&mut self, by: isize, cx: &mut Context<Self>) {
        let flat: Vec<(usize, usize)> = self.areas.iter().enumerate().flat_map(|(a, area)| (0..area.features.len()).map(move |i| (a, i))).collect();
        let Some(here) = flat.iter().position(|at| *at == self.cursor) else { return };
        let next = (here as isize + by).clamp(0, flat.len().saturating_sub(1) as isize) as usize;
        self.move_to(flat[next], cx);
    }

    /// The tile a row down or up: in the same column of the next row of the area, or of the nearest row of the next area,
    /// as far along as that row reaches.
    fn step_row(&mut self, by: isize, window: &Window, cx: &mut Context<Self>) {
        let cols = columns(cx.look(), window.viewport_size().width);
        let (area, at) = self.cursor;
        let (row, col) = (at / cols, at % cols);
        let len = |a: usize| self.areas.get(a).map_or(0, |one| one.features.len());
        let to = if by > 0 {
            if (row + 1) * cols < len(area) {
                Some((area, (at + cols).min(len(area) - 1)))
            } else {
                (area + 1 < self.areas.len()).then(|| (area + 1, col.min(len(area + 1).saturating_sub(1))))
            }
        } else if row > 0 {
            Some((area, at - cols))
        } else if area > 0 {
            let last = len(area - 1).saturating_sub(1) / cols;
            Some((area - 1, (last * cols + col).min(len(area - 1).saturating_sub(1))))
        } else {
            None
        };
        if let Some(to) = to {
            self.move_to(to, cx);
        }
    }

    fn move_to(&mut self, to: (usize, usize), cx: &mut Context<Self>) {
        if to != self.cursor {
            crate::ui_sounds::moved();
            self.cursor = to;
            self.scroll.scroll_to_item(to.0);
            cx.notify();
        }
    }

    /// Open the tile `(area, at)`: its pictures over the gallery, the keys on the first.
    pub(super) fn open(&mut self, at: (usize, usize), window: &mut Window, cx: &mut Context<Self>) {
        let Some(slug) = self.areas.get(at.0).and_then(|area| area.features.get(at.1)).map(|row| row.slug.clone()) else { return };
        self.cursor = at;
        self.opened = Some(Opened { slug, at: 0, button: None, why: None });
        self.page_scroll.set_offset(point(px(0.), px(0.)));
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Back to the gallery, the keys on the tile that was open.
    pub(super) fn back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(opened) = self.opened.take() {
            if let Some(area) = self.area_of(&opened.slug) {
                self.scroll.scroll_to_item(area);
            }
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    // ---- a feature's pictures -----------------------------------------------------------------

    /// The card `by` places along, the standing pictures then the suggestions, the keys on the card itself.
    fn step_card(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(slug) = self.opened.as_ref().map(|opened| opened.slug.clone()) else { return };
        let last = self.cards(&slug).len().saturating_sub(1);
        let Some(opened) = self.opened.as_mut() else { return };
        let next = (opened.at as isize + by).clamp(0, last as isize) as usize;
        if next != opened.at || opened.button.is_some() {
            crate::ui_sounds::moved();
        }
        opened.at = next;
        opened.button = None;
        cx.notify();
    }

    /// Between the pictures and Also this?, as near the same place along as the other row reaches.
    fn step_section(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(slug) = self.opened.as_ref().map(|opened| opened.slug.clone()) else { return };
        let (standing, suggested) = (self.standing_count(&slug), self.suggested_count(&slug));
        let Some(opened) = self.opened.as_mut() else { return };
        let to = match (by > 0, opened.at < standing) {
            (true, true) if suggested > 0 => Some((standing + opened.at.min(suggested - 1), 1)),
            (false, false) if standing > 0 => Some(((opened.at - standing).min(standing - 1), 0)),
            _ => None,
        };
        if let Some((at, section)) = to {
            crate::ui_sounds::moved();
            opened.at = at;
            opened.button = None;
            // The pictures are the top of the page; Also this? is brought into view.
            if section == 0 {
                self.page_scroll.set_offset(point(px(0.), px(0.)));
            } else {
                self.page_scroll.scroll_to_item(section);
            }
            cx.notify();
        }
    }

    /// Tab and Shift+Tab through the card's buttons and back to the card.
    fn step_button(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(slug) = self.opened.as_ref().map(|opened| opened.slug.clone()) else { return };
        let Some(kind) = self.opened.as_ref().and_then(|opened| self.cards(&slug).get(opened.at).map(|(kind, _)| *kind)) else { return };
        let Some(opened) = self.opened.as_mut() else { return };
        // The card itself is the place before the first button and after the last.
        let places = kind.buttons().len() as isize + 1;
        let now = opened.button.map_or(0, |at| at as isize + 1);
        let next = (now + by).rem_euclid(places);
        opened.button = (next > 0).then(|| next as usize - 1);
        crate::ui_sounds::moved();
        cx.notify();
    }

    /// Enter on the button the keys are on.
    fn press_button(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(opened) = self.opened.as_ref() else { return };
        let Some(button) = opened.button else { return };
        let Some((kind, id)) = self.cards(&opened.slug).get(opened.at).map(|(kind, one)| (*kind, one.id.clone())) else { return };
        match kind.buttons().get(button).copied() {
            Some(YES) => self.keep(&id, cx),
            Some(NOT_THIS) => self.ask_why(id, window, cx),
            _ => {}
        }
    }

    /// Yes on a suggestion: it joins the feature's pictures in its place by when it was taken, and the keys follow it.
    pub(super) fn keep(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(slug) = self.opened.as_ref().map(|opened| opened.slug.clone()) else { return };
        let Some(list) = self.suggested.get_mut(&slug) else { return };
        let Some(at) = list.iter().position(|one| one.id == id) else { return };
        let one = list.remove(at);
        let standing = self.pictures.entry(slug.clone()).or_default();
        standing.push(one);
        newest_first(standing);
        let place = standing.iter().position(|one| one.id == id).unwrap_or(0);
        self.set_cover(&slug);
        if let Some(opened) = self.opened.as_mut() {
            opened.at = place;
            opened.button = None;
        }
        self.page_scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }

    /// Not this feature: the box that asks why opens under the picture, with the keys in it.
    pub(super) fn ask_why(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(slug) = self.opened.as_ref().map(|opened| opened.slug.clone()) else { return };
        let place = self.cards(&slug).iter().position(|(_, one)| one.id == id);
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("What is wrong with it"));
        let picture = id.clone();
        let sub = cx.subscribe_in(&input, window, move |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } => this.remove(&picture, window, cx),
            InputEvent::Change => cx.notify(),
            _ => {}
        });
        let focus = input.clone();
        if let Some(opened) = self.opened.as_mut() {
            opened.at = place.unwrap_or(opened.at);
            opened.button = None;
            opened.why = Some(Why { picture: id, input, _sub: sub });
        }
        window.defer(cx, move |window, cx| focus.update(cx, |input, cx| input.focus(window, cx)));
        cx.notify();
    }

    /// The why box put away with nothing removed, the keys back on the card.
    pub(super) fn shut_why(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(opened) = self.opened.as_mut() {
            opened.why = None;
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Remove the picture from the feature, with what he wrote in the box as why. Fleet's `remove_feature_picture` keeps the
    /// row marked removed and the why as a catalog rule; the mock only takes it off.
    pub(super) fn remove(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(slug) = self.opened.as_ref().map(|opened| opened.slug.clone()) else { return };
        for list in [self.pictures.get_mut(&slug), self.suggested.get_mut(&slug)].into_iter().flatten() {
            list.retain(|one| one.id != id);
        }
        self.set_cover(&slug);
        let last = self.cards(&slug).len().saturating_sub(1);
        if let Some(opened) = self.opened.as_mut() {
            opened.why = None;
            opened.button = None;
            opened.at = opened.at.min(last);
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// The cover `list_features` would carry now: the newest standing picture.
    fn set_cover(&mut self, slug: &str) {
        let cover = self.pictures.get(slug).and_then(|list| list.first()).map(|one| one.id.clone());
        if let Some(row) = self.areas.iter_mut().flat_map(|area| area.features.iter_mut()).find(|row| row.slug == slug) {
            row.cover = cover;
        }
    }

    // ---- keys ---------------------------------------------------------------------------------

    pub(super) fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key != "escape" {
            return;
        }
        let Some(opened) = self.opened.as_mut() else { return };
        if opened.why.is_some() {
            self.shut_why(window, cx);
        } else if opened.button.is_some() {
            opened.button = None;
            cx.notify();
        } else {
            self.back(window, cx);
        }
        cx.stop_propagation();
    }

    pub(super) fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        let opened = self.opened.is_some();
        match act.id {
            "features_chip" if opened => self.step_card(if act.slot == 0 { -1 } else { 1 }, cx),
            "features_chip" => self.step_tile(if act.slot == 0 { -1 } else { 1 }, cx),
            "features_move" if opened => self.step_section(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            "features_move" => self.step_row(if act.slot % 2 == 0 { 1 } else { -1 }, window, cx),
            "features_pane" if opened => self.step_button(if act.slot == 0 { 1 } else { -1 }, cx),
            "features_open" if opened => self.press_button(window, cx),
            "features_open" => self.open(self.cursor, window, cx),
            _ => cx.propagate(),
        }
    }
}
