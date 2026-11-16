//! An artifact of parts read in the review panel. **The panel is the one overlay**: the list of what waits on him on the left, the
//! chosen thing on the right, the foot under both. An artifact he opens, from a Needs you row, the Artifacts tab, a card's Open or
//! the For you tab, is picked in that list, with the Job's other artifacts that still ask beside it.
//!
//! Its parts are answered where they stand, one by one, so the band holds no Approve and the notes column is left out: a take is
//! picked by a press or its number, a vote is cast with Send, and the footer under the parts settles or puts the artifact away.
//! Prose keeps a measure and stands at the left; pictures, drawings and takes take the detail's whole inner width.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, MouseButton, Pixels, SharedString, Window};

use super::band_card::Stand;
use super::picture_view::What;
use super::rows::{glyph, tip, Size as GlyphSize};
use super::zoomer::{self, Zoomer};
use crate::labels::labelled;
use crate::scene::{Clock, Fit, Lens};
use super::parts_card::{effective_part, part_key, stand_of};
use super::review::{Item, Kind, Verdict};
use super::JobView;
use crate::look::{token, Look};

/// How wide prose stands in the panel: sixteen of `--space-12`, about 760 points.
pub(super) fn measure(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 16.
}

/// The share of the panel's height a picture may stand at most.
pub(super) const TALL_SHARE: f32 = 0.7;

/// What an artifact's id is on the panel: `parts:<artifact>`, which no waiting item's id begins with.
const PREFIX: &str = "parts:";

/// The panel's key for artifact `artifact` of Job `job`.
pub(super) fn key_of_artifact(job: &str, artifact: &str) -> String {
    super::review::key_of(job, &format!("{PREFIX}{artifact}"))
}

/// The artifact an item stands for, where it is one.
pub(super) fn artifact_in(item: &Item) -> Option<&str> {
    item.id.strip_prefix(PREFIX).filter(|_| item.kind == Kind::Artifact)
}

/// The zoom of the drawing the panel's detail holds in place, with the scene key it draws under. Fresh for each item chosen.
pub(super) struct DetailZoom {
    pub zoom: Zoomer,
    pub held: RefCell<String>,
}

impl DetailZoom {
    pub(super) fn new() -> Self {
        DetailZoom { zoom: Zoomer::default(), held: RefCell::new(String::new()) }
    }
}

/// What the stage draws and how it is enlarged over the whole app.
struct Staged {
    held: String,
    scene: Rc<ipc::Scene>,
    enlarge: Enlarge,
}

enum Enlarge {
    Drawing(String),
    Sketch(String),
}

impl JobView {
    /// The item for artifact `artifact` of Job `job`, as its card reads where it stands.
    fn artifact_item(&self, job: &str, artifact: &str) -> Option<Item> {
        let one = self.artifact_of(job, artifact)?;
        if one.parts.is_empty() {
            return None;
        }
        let revision = one.revisions.last();
        Some(Item {
            job: job.to_string(),
            id: format!("{PREFIX}{}", one.id),
            kind: Kind::Artifact,
            name: one.title.clone(),
            since: revision.map_or_else(String::new, |last| last.at.as_str().to_string()),
            for_line: one.for_line.as_ref().and_then(|line| line.text.clone()).filter(|text| !text.trim().is_empty()),
            revision: revision.map(|last| format!("r{}", last.number)),
            send_back: false,
            read: false,
        })
    }

    /// The artifacts of parts Job `job` holds that still ask, and the one he opened, as the panel's items.
    pub(super) fn artifact_items(&self, job: &str, opened: Option<&str>) -> Vec<Item> {
        let Some(page) = self.pages.get(job) else { return Vec::new() };
        self.artifacts_of(job)
            .into_iter()
            .filter(|one| {
                let parts: Vec<_> = one.parts.iter().map(|part| effective_part(part, &part_key(&one.id, &part.id), page)).collect();
                opened == Some(one.id.as_str()) || stand_of(&parts, one.state) == Stand::Asking
            })
            .filter_map(|one| self.artifact_item(job, &one.id))
            .collect()
    }

    /// Where the artifact an item stands for stands: None for an item that is none, Some(None) while it asks.
    pub(super) fn artifact_verdict(&self, item: &Item) -> Option<Option<Verdict>> {
        let artifact = self.artifact_of(&item.job, artifact_in(item)?)?;
        let page = self.pages.get(item.job.as_str())?;
        let parts: Vec<_> = artifact.parts.iter().map(|part| effective_part(part, &part_key(&artifact.id, &part.id), page)).collect();
        let dismissed = page.cards.answers.get(artifact.id.as_str()) == Some(&super::acts::Answer::Withdrawn);
        let stand = if dismissed { Stand::Dismissed } else { stand_of(&parts, artifact.state) };
        Some(match stand {
            Stand::Asking => None,
            Stand::Approved => Some(Verdict::Approved),
            Stand::SentBack => Some(Verdict::ChangesAsked),
            Stand::Dismissed | Stand::Withdrawn => Some(Verdict::Dismissed),
            _ => Some(Verdict::Answered),
        })
    }

    /// Read artifact `artifact` of the shown Job in the review panel: it opens with the artifact picked, or moves to it where it is up.
    pub(super) fn open_artifact_review(&mut self, artifact: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if self.artifact_item(&job, artifact).is_none() {
            return;
        }
        self.open_review_at(Some(artifact), window, cx);
    }

    /// The parts of the chosen artifact, drawn the way its card draws them, at the detail's width.
    pub(super) fn review_parts(&self, item: &Item, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (Some(artifact), Some(page)) = (artifact_in(item), self.pages.get(item.job.as_str())) else { return div().into_any_element() };
        let Some(newest) = self.artifact_of(&item.job, artifact).and_then(|one| one.revisions.last().map(|last| last.number)) else { return div().into_any_element() };
        self.parts_card(super::acts::SHEET, artifact, newest, true, true, page, look, cx).unwrap_or_else(|| div().into_any_element())
    }

    /// The foot of the chosen artifact: what settles it, and Dismiss or Archive.
    pub(super) fn review_parts_foot(&self, item: &Item, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let (artifact, page) = (artifact_in(item)?, self.pages.get(item.job.as_str())?);
        let foot = self.parts_pinned_footer(super::acts::SHEET, artifact, page, look, cx)?;
        Some(div().flex_none().border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(foot).into_any_element())
    }

    /// The keys go to the chosen artifact's parts: a number picks a take, A and C answer a part, and a take put large is put back.
    pub(super) fn review_parts_focus(&mut self, item: &Item, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(artifact) = artifact_in(item).map(str::to_string) {
            self.focus_card(&artifact, window, cx);
        }
    }

    /// Dismiss the chosen artifact, or archive it once it no longer asks.
    pub(super) fn review_parts_dismiss(&mut self, item: &Item, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(artifact) = artifact_in(item).map(str::to_string) {
            self.dismiss_artifact(&artifact, window, cx);
        }
    }
}

impl JobView {
    /// A drawing the detail fills with: an artifact of one drawing and nothing else, or a sketch.
    fn staged(&self, item: &Item, cx: &gpui::App) -> Option<Staged> {
        match item.kind {
            Kind::Artifact => {
                let artifact = self.artifact_of(&item.job, artifact_in(item)?)?;
                let [part] = artifact.parts.as_slice() else { return None };
                if !matches!(part.content, ipc::PartContent::Drawing { .. }) {
                    return None;
                }
                let key = part_key(&artifact.id, &part.id);
                let (held, scene) = self.scene_of_as(&What::Drawing { key: key.clone() }, "detail")?;
                Some(Staged { held, scene, enlarge: Enlarge::Drawing(key) })
            }
            Kind::Sketch => {
                let held = self.thread(&item.job, cx)?;
                let card = super::review::sketch_card(&held.head, &held.items, &item.id)?;
                let (shown, _) = super::cards::shown_scene(card, None);
                let held = format!("detail:{shown}");
                let scene = crate::scene::read(&held, &card.drawing)?;
                Some(Staged { held, scene, enlarge: Enlarge::Sketch(card.id.clone()) })
            }
            _ => None,
        }
    }

    /// The drawing of the chosen item, filling the detail between its band and its foot: fitted to that box, zoomed in place by
    /// the one zoomer (a pinch or ⌘ and the wheel about the pointer, ⌘+ ⌘- ⌘0, a double press, a drag or a scroll once zoomed),
    /// and read over the whole app by the button or F.
    pub(super) fn review_stage(&self, item: &Item, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let staged = self.staged(item, cx)?;
        let detail = self.detail_zoom.as_ref()?;
        *detail.held.borrow_mut() = staged.held.clone();
        let zoom = &detail.zoom;
        let stage = zoom.view().and_then(|view| {
            let (fit, actual) = crate::scene::fit_of(&staged.held)?;
            Some(zoomer::Stage { view, fit, actual })
        });
        let readout = stage.map(|stage| zoom.readout(stage));
        let lens = Lens { zoom: zoom.scale, pan: (zoom.pan.x, zoom.pan.y) };
        let enlarge = staged.enlarge;
        let larger = labelled(
            div()
                .id("stage-larger")
                .absolute()
                .top(look.length(token::SPACE_3))
                .right(look.length(token::SPACE_3))
                .size(look.length(token::H_CONTROL))
                .flex()
                .items_center()
                .justify_center()
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("See it larger, F"))
                .child(glyph(look, "maximize-2", GlyphSize::Small, look.color(token::FG_DEFAULT)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, _, window, cx| this.enlarge_stage(&enlarge, window, cx))),
            gpui::Role::Button,
            "See larger",
        );
        Some(
            labelled(
                zoomer::bind(div().id("review-stage").relative().flex_1().min_h_0().w_full().overflow_hidden().bg(look.color(token::BG_BASE)), cx, |_, _, _, _| {})
                    .child(zoom.probe())
                    .child(div().absolute().inset_0().child(crate::scene::view(SharedString::from(staged.held), staged.scene, Fit::Reading, Clock::default(), lens)))
                    .child(larger)
                    .children(readout.map(|readout| zoomer::corner(look, readout, |this: &mut JobView, _, cx| zoomer::refit(this, cx), cx))),
                gpui::Role::Group,
                "Drawing",
            )
            .into_any_element(),
        )
    }

    fn enlarge_stage(&mut self, enlarge: &Enlarge, window: &mut Window, cx: &mut Context<Self>) {
        match enlarge {
            Enlarge::Drawing(key) => self.open_drawing(key, window, cx),
            Enlarge::Sketch(card) => self.read_sketch(card, card, window, cx),
        }
    }

    /// F enlarges the drawing in the detail, and ⌘+ ⌘- ⌘0 zoom it. False where the key is not one of these or nothing is staged.
    pub(super) fn review_zoom_key(&mut self, event: &gpui::KeyDownEvent, noting: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.pictures.viewing.is_some() || self.sketch_reading.is_some() || noting {
            return false;
        }
        let Some(item) = self.review_chosen() else { return false };
        let Some(staged) = self.staged(&item, cx) else { return false };
        let mods = event.keystroke.modifiers;
        if event.keystroke.key == "f" && !mods.modified() {
            self.enlarge_stage(&staged.enlarge, window, cx);
            return true;
        }
        zoomer::keyed(self, &event.keystroke, window, cx)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{px, size, Bounds, Modifiers, Role};

    use super::super::parts_fold::{drawn, redraw, rooted_job};
    use super::*;
    use crate::mock_parts::{ID, JOB, TITLE};

    const TAKES: [&str; 2] = ["A \u{b7} Bar under the title", "B \u{b7} Rail (Job's pick)"];

    /// The Job with its artifact of parts read in the review panel at `width` by `height`.
    fn opened<'a>(cx: &'a mut gpui::TestAppContext, width: f32, height: f32) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(width), px(height)), |_| {});
        view.update_in(cx, |view, window, cx| assert!(view.open_named(ID, window, cx)));
        redraw(cx);
        (view, cx)
    }

    /// What the last frame drew with this role and name, the last drawn: the thread behind draws its own beneath the panel's.
    fn bounds_of(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> Bounds<gpui::Pixels> {
        cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds))
            .unwrap_or_else(|| panic!("{name} was not drawn"))
    }

    #[gpui::test]
    fn an_artifact_opens_the_review_panel_across_the_window_with_it_picked_and_closing_leaves_the_panes(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |_| {});
        let before = view.read_with(cx, |view, _| view.arrangement().into_owned());
        view.update_in(cx, |view, window, cx| assert!(view.open_named(ID, window, cx)));
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.review_open()), "the review panel is up");
        let panel = bounds_of(cx, Role::Dialog, "Review");
        assert_eq!((panel.left(), panel.right()), (px(24.), px(1400. - 24.)), "24 points from each side: {panel:?}");
        assert!(drawn(cx, Role::Region, TITLE), "the artifact is the detail");
        assert_eq!(view.read_with(cx, |view, _| view.arrangement().into_owned()), before, "no pane gained a tab");
        view.update_in(cx, |view, window, cx| view.close_review(window, cx));
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()));
        assert_eq!(view.read_with(cx, |view, _| view.arrangement().into_owned()), before, "the panes are as they were");
    }

    #[gpui::test]
    fn one_escape_closes_it_from_the_parts_and_from_the_list(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "one Esc from the parts, where the keys were put");
        view.update_in(cx, |view, window, cx| assert!(view.open_named(ID, window, cx)));
        redraw(cx);
        view.update_in(cx, |view, window, cx| window.focus(&view.review_focus.list, cx));
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "one Esc from the list");
    }

    #[gpui::test]
    fn escape_puts_the_keys_back_where_the_panel_was_opened_from(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |_| {});
        let from = view.update_in(cx, |view, window, cx| {
            window.focus(&view.focus, cx);
            view.focus.clone()
        });
        view.update_in(cx, |view, window, cx| assert!(view.open_named(ID, window, cx)));
        redraw(cx);
        assert!(!cx.update(|window, _| from.is_focused(window)), "the keys are in the panel");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(cx.update(|window, _| from.is_focused(window)), "and Esc gives them back");
    }

    #[gpui::test]
    fn the_zoomer_hears_its_escape_first(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        let key = super::super::parts_card::part_key(ID, "bar");
        view.update_in(cx, |view, window, cx| view.open_take(&key, 0, window, cx));
        redraw(cx);
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.review_open()), "the zoomer shut and the panel stayed");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "the second Esc closes the panel");
    }

    #[gpui::test]
    fn the_close_button_closes_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        let at = bounds_of(cx, Role::Button, "Close").center();
        cx.simulate_click(at, Modifiers::none());
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()));
    }

    #[gpui::test]
    fn takes_stand_side_by_side_at_2400_and_one_to_a_row_at_1100(cx: &mut gpui::TestAppContext) {
        let (_, cx) = opened(cx, 2400., 1500.);
        let tops: Vec<_> = TAKES.iter().map(|name| bounds_of(cx, Role::RadioButton, &format!("Pick {name}")).top()).collect();
        assert_eq!(tops[0], tops[1], "side by side at 2400");
    }

    #[gpui::test]
    fn takes_stand_one_to_a_row_at_1100(cx: &mut gpui::TestAppContext) {
        let (_, cx) = opened(cx, 1100., 700.);
        let tiles: Vec<_> = TAKES.iter().map(|name| bounds_of(cx, Role::RadioButton, &format!("Pick {name}"))).collect();
        assert!(tiles[1].top() >= tiles[0].bottom(), "one to a row at 1100: {tiles:?}");
    }

    #[gpui::test]
    fn prose_keeps_its_measure_while_pictures_take_the_detail_width(cx: &mut gpui::TestAppContext) {
        let (_, cx) = opened(cx, 2400., 1500.);
        let words = bounds_of(cx, Role::Group, "Words");
        let takes = bounds_of(cx, Role::Group, "Takes");
        let pill = bounds_of(cx, Role::Label, "Asks you");
        assert!(f32::from(pill.right() - words.left()) < 800., "the words' caption stands at the measure: {pill:?} {words:?}");
        assert!(f32::from(takes.size.width) > 1200., "while the takes run the detail's width: {takes:?}");
        assert_eq!(words.left(), takes.left(), "prose and pictures share a left edge");
    }

    #[gpui::test]
    fn a_takes_picture_fills_its_tile_with_no_frame_between(cx: &mut gpui::TestAppContext) {
        let (_, cx) = opened(cx, 1400., 900.);
        let tile = bounds_of(cx, Role::RadioButton, "Pick A \u{b7} Bar under the title");
        let picture = bounds_of(cx, Role::Button, "Picture: A \u{b7} Bar under the title");
        assert!(picture.left() - tile.left() < px(4.) && tile.right() - picture.right() < px(4.), "no padding: {tile:?} {picture:?}");
    }
}
