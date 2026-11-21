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
use super::review::{page_of_item, Item, Kind, Verdict};
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
    item.artifact.as_deref().filter(|_| item.kind == Kind::Artifact)
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
pub(super) struct Staged {
    held: String,
    scene: Rc<ipc::Scene>,
    enlarge: Enlarge,
}

pub(super) enum Enlarge {
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
            artifact: Some(one.id.clone()),
        })
    }

    /// The artifact of parts a waiting row names, by its `artifact`, or by a title the row's words begin with, as an item.
    pub(super) fn waiting_artifact_item(&self, job: &str, row: &ipc::WaitingItem) -> Option<Item> {
        let named = row.artifact.as_deref().into_iter().chain(std::iter::once(row.text.as_str()));
        let found = named.filter_map(|name| self.artifacts_of(job).into_iter().find(|one| one.id == name || one.title == name || name.starts_with(&format!("{}:", one.title)))).next()?;
        self.artifact_item(job, &found.id)
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

    /// The document `page` as the panel's item: its own `read:<page>` where nothing waits on him for it, so it is read in the
    /// panel, and the card or the artifact it is read from. None where the Job holds neither, or the artifact is one of parts.
    pub(super) fn document_item(&self, job: &str, page: &str, cx: &gpui::App) -> Option<Item> {
        let card = self.thread(job, cx).and_then(|held| super::review::document_card(&held.items, page));
        let (name, revision, since, for_line) = match card {
            Some(card) => (card.title.clone(), card.revision, card.at.as_str().to_string(), card.for_line.as_ref()),
            None => {
                let one = self.artifact_of(job, page).filter(|one| one.parts.is_empty() && one.form == ipc::ArtifactForm::Page)?;
                let last = one.revisions.last()?;
                (one.title.clone(), last.number, last.at.as_str().to_string(), one.for_line.as_ref())
            }
        };
        Some(Item {
            job: job.to_string(),
            id: format!("read:{page}"),
            kind: Kind::Document,
            name,
            since,
            for_line: for_line.and_then(|line| line.text.clone()).filter(|text| !text.trim().is_empty()),
            revision: Some(format!("r{revision}")),
            send_back: false,
            read: true,
            artifact: None,
        })
    }

    /// The key of what the panel picks for the artifact or document `opened`: its artifact of parts, the item waiting on him for
    /// the page, or the document read in place.
    pub(super) fn opened_key(&self, job: &str, opened: &str, cx: &gpui::App) -> String {
        if self.artifact_item(job, opened).is_some() {
            return key_of_artifact(job, opened);
        }
        match self.review_items(job, cx).into_iter().find(|item| page_of_item(&item.id) == Some(opened)).or_else(|| self.document_item(job, opened, cx)) {
            Some(item) => item.key(),
            None => key_of_artifact(job, opened),
        }
    }

    /// Read the document `page` of the shown Job: a page the thread holds a card for reads in a tab of the Job beside the
    /// thread, its margin at its right; any other the Job holds as an artifact reads in the review panel. A page the Job holds
    /// no card or artifact for opens nothing.
    pub(super) fn open_document(&mut self, page: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if self.thread(&job, cx).is_some_and(|held| super::review::document_card(&held.items, page).is_some()) {
            return self.open_document_tab(page, window, cx);
        }
        if self.artifact_item(&job, page).is_none() && self.document_item(&job, page, cx).is_none() {
            return;
        }
        self.set_needs(false, None, window, cx);
        self.open_review_at(Some(page), window, cx);
    }

    /// The pages the chosen item reads live in the panel, each with its revision and the title of what holds it: a page part of an
    /// artifact, or the document itself.
    pub(super) fn panel_pages_of(&self, item: &Item, cx: &gpui::App) -> Vec<(String, u32, String)> {
        match item.kind {
            Kind::Artifact => artifact_in(item)
                .and_then(|artifact| self.artifact_of(&item.job, artifact))
                .map(|one| {
                    one.parts
                        .iter()
                        .filter_map(|part| match &part.content {
                            ipc::PartContent::Page { path } => Some((path.clone(), part.revision, one.title.clone())),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            Kind::Document => {
                let Some(page) = page_of_item(&item.id) else { return Vec::new() };
                let card = self.thread(&item.job, cx).and_then(|held| super::review::document_card(&held.items, page));
                let held = match card {
                    Some(card) => Some((card.revision, card.title.clone())),
                    None => self.artifact_of(&item.job, page).and_then(|one| one.revisions.last().map(|last| (last.number, one.title.clone()))),
                };
                held.map(|(revision, title)| vec![(page.to_string(), revision, title)]).unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }

    /// The live page filling the detail, where the chosen item is a document or an artifact whose only part is a page that asks
    /// nothing: the whole of the detail between its band and its foot, which the page scrolls inside.
    pub(super) fn review_page_stage(&self, item: &Item, cx: &gpui::App) -> Option<AnyElement> {
        let alone = item.kind == Kind::Document || self.page_alone(item);
        let (page, revision, _) = self.panel_pages_of(item, cx).into_iter().next().filter(|_| alone)?;
        let live = self.panel_page(&item.job, &page, revision, cx)?;
        Some(div().id(SharedString::from(format!("review-page:{}", item.key()))).relative().flex_1().min_w_0().min_h_0().child(div().absolute().inset_0().child(live)).into_any_element())
    }

    /// Whether the item is an artifact whose only part is a page that asks nothing, which the panel draws as the page alone.
    fn page_alone(&self, item: &Item) -> bool {
        let Some(artifact) = artifact_in(item).and_then(|artifact| self.artifact_of(&item.job, artifact)) else { return false };
        matches!(artifact.parts.as_slice(), [part] if matches!(part.content, ipc::PartContent::Page { .. }) && !part.asks)
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
        // A page alone draws no parts card in the panel, so the card's focus is the thread's behind it, where Esc never reaches the
        // panel: the keys stay on the list.
        if self.page_alone(item) {
            return;
        }
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
    pub(super) fn staged(&self, item: &Item, cx: &gpui::App) -> Option<Staged> {
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
        let larger = self.larger(look, staged.enlarge, cx);
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

    /// The button at the drawing's top right that reads it over the whole app, as F does.
    fn larger(&self, look: &Look, enlarge: Enlarge, cx: &Context<Self>) -> AnyElement {
        labelled(
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
        )
        .into_any_element()
    }

    /// The drawing part the panel zooms in place within an artifact of several parts: its first. The detail has one zoomer, so
    /// any later drawing part keeps the press that reads it over the app.
    pub(super) fn in_place_drawing(&self, job: &str, artifact: &str) -> Option<String> {
        let artifact = self.artifact_of(job, artifact)?;
        if artifact.parts.len() < 2 {
            return None;
        }
        let part = artifact.parts.iter().find(|part| matches!(part.content, ipc::PartContent::Drawing { .. }))?;
        Some(part_key(&artifact.id, &part.id))
    }

    /// A drawing part zoomed where it stands in the panel's parts: the column's width, as tall as its shape asks up to `tall`.
    /// A pinch or ⌘ and the wheel zoom it, a drag moves it, a double press toggles fit and 100%; a plain wheel at fit scrolls
    /// the parts on past it. The button or F reads it over the whole app.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn drawing_in_place(&self, key: &str, held: &str, text: &str, room: Pixels, tall: Pixels, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let detail = self.detail_zoom.as_ref()?;
        let raw: ipc::RawScene = ipc::decode("a drawing part's scene", text.as_bytes()).ok()?;
        let scene = crate::scene::read(held, &raw)?;
        *detail.held.borrow_mut() = held.to_string();
        let zoom = &detail.zoom;
        // The canvas's shape is known once it has been drawn; the frame before that stands at a plain 16 to 10.
        let ratio = crate::scene::canvas_size(held).map_or(1.6, |(w, h)| (w / h).clamp(0.2, 8.));
        let high = if room / ratio < tall { room / ratio } else { tall };
        let stage = zoom.view().and_then(|view| {
            let (fit, actual) = crate::scene::fit_of(held)?;
            Some(zoomer::Stage { view, fit, actual })
        });
        let readout = stage.filter(|_| !zoom.is_fit()).map(|stage| zoom.readout(stage));
        let lens = Lens { zoom: zoom.scale, pan: (zoom.pan.x, zoom.pan.y) };
        Some(
            labelled(
                zoomer::bind(div().id(SharedString::from(format!("drawing:{key}"))).relative().w_full().h(high).overflow_hidden(), cx, |_, _, _, _| {})
                    .child(zoom.probe())
                    .child(div().absolute().inset_0().child(crate::scene::view(SharedString::from(held.to_string()), scene, Fit::Column, Clock::default(), lens)))
                    .child(self.larger(look, Enlarge::Drawing(key.to_string()), cx))
                    .children(readout.map(|readout| zoomer::corner(look, readout, |this: &mut JobView, _, cx| zoomer::refit(this, cx), cx))),
                gpui::Role::Group,
                "Drawing",
            )
            .into_any_element(),
        )
    }

    /// What F reads over the whole app for the chosen item: the staged drawing or sketch, or the drawing part zoomed in place.
    pub(super) fn review_enlarge(&self, item: &Item, cx: &gpui::App) -> Option<Enlarge> {
        if let Some(staged) = self.staged(item, cx) {
            return Some(staged.enlarge);
        }
        self.in_place_drawing(&item.job, artifact_in(item)?).map(Enlarge::Drawing)
    }

    fn enlarge_stage(&mut self, enlarge: &Enlarge, window: &mut Window, cx: &mut Context<Self>) {
        match enlarge {
            Enlarge::Drawing(key) => self.open_drawing(key, window, cx),
            Enlarge::Sketch(card) => self.read_sketch(card, card, window, cx),
        }
    }

    /// F enlarges the drawing in the detail, and ⌘+ ⌘- ⌘0 zoom it. False where the key is not one of these, nothing is staged
    /// or zoomed in place, or he is typing in a box of the artifact's parts.
    pub(super) fn review_zoom_key(&mut self, event: &gpui::KeyDownEvent, noting: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.pictures.viewing.is_some() || self.sketch_reading.is_some() || noting {
            return false;
        }
        let Some(item) = self.review_chosen() else { return false };
        if artifact_in(&item).is_some_and(|artifact| self.parts_typing(artifact, window, cx)) {
            return false;
        }
        let Some(enlarge) = self.review_enlarge(&item, cx) else { return false };
        let mods = event.keystroke.modifiers;
        if event.keystroke.key == "f" && !mods.modified() {
            self.enlarge_stage(&enlarge, window, cx);
            return true;
        }
        zoomer::keyed(self, &event.keystroke, window, cx)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{px, size, Bounds, Modifiers, Role};

    use ipc::PageToHost;

    use super::super::page_tab::PageTabEvent;
    use super::super::parts_fold::{drawn, redraw, rooted_job};
    use super::*;
    use crate::mock_parts::{ID, JOB, PAGE, TITLE};

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
    fn an_artifact_of_one_drawing_takes_approve_and_ask_for_changes_in_the_panels_foot(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-folds", JOB, size(px(1400.), px(900.)), |_| {});
        view.update_in(cx, |view, window, cx| assert!(view.open_named("sketch:Step flow", window, cx)));
        redraw(cx);
        let panel = bounds_of(cx, Role::Dialog, "Review");
        let approve = bounds_of(cx, Role::Button, "Approve");
        let ask = bounds_of(cx, Role::Button, "Ask for changes");
        let dismiss = bounds_of(cx, Role::Button, "Dismiss");
        let low = panel.top() + panel.size.height * 0.75;
        assert!(approve.top() > low && ask.top() > low && approve.bottom() < panel.bottom(), "both stand in the foot, at the panel's bottom");
        for (name, one) in [("Approve", approve), ("Ask for changes", ask)] {
            assert!(one.top() < dismiss.bottom() && one.bottom() > dismiss.top(), "{name} stands in Dismiss's row: {one:?} {dismiss:?}");
        }
        assert!(approve.right() <= ask.left() && ask.right() <= dismiss.left(), "right-aligned, in the order Approve, Ask for changes, Dismiss");
        assert!(approve.left() > panel.left() + panel.size.width * 0.5, "on the right of the foot");
    }

    #[gpui::test]
    fn a_takes_row_of_the_jobs_review_opens_the_panel_on_this_jobs_items_with_that_row_picked(cx: &mut gpui::TestAppContext) {
        let row = format!(r#"[{{"id":"sketch:takes:bar","text":"Pick a take for the step bar","since":"2026-11-09T14:01:00Z","source":"sketch","artifact":"{ID}"}}]"#);
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |jobs| {
            let held = ipc::decode::<Vec<ipc::WaitingItem>>("waiting", row.as_bytes()).expect("a waiting row");
            jobs.iter_mut().find(|one| one.id.as_str() == JOB).expect("the job").waiting_for.extend(held);
        });
        view.update_in(cx, |view, window, cx| view.open_review_on("sketch:takes:bar", window, cx));
        redraw(cx);
        view.read_with(cx, |view, _| {
            assert!(view.review_open(), "the row opens the panel, though no card of the thread stands for it");
            let chosen = view.review_chosen().expect("one item is chosen");
            assert_eq!((chosen.kind, chosen.artifact.as_deref()), (Kind::Artifact, Some(ID)), "the row stands for the artifact it names");
            let review = view.review().expect("review");
            assert!(review.groups.iter().all(|group| group.job == JOB), "this Job's items alone");
        });
    }

    #[gpui::test]
    fn an_artifact_a_waiting_row_names_is_listed_once_in_the_panel(cx: &mut gpui::TestAppContext) {
        let row = format!(r#"[{{"id":"sketch:takes:bar","text":"Pick a take for the step bar","since":"2026-11-09T14:01:00Z","source":"sketch","artifact":"{ID}"}}]"#);
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |jobs| {
            let held = ipc::decode::<Vec<ipc::WaitingItem>>("waiting", row.as_bytes()).expect("a waiting row");
            jobs.iter_mut().find(|one| one.id.as_str() == JOB).expect("the job").waiting_for.extend(held);
        });
        view.update_in(cx, |view, window, cx| view.open_review_on("sketch:takes:bar", window, cx));
        redraw(cx);
        view.read_with(cx, |view, _| {
            let review = view.review().expect("review");
            let listed: Vec<_> = review.groups.iter().flat_map(|group| group.items.iter()).filter(|item| artifact_in(item) == Some(ID)).map(|item| item.id.clone()).collect();
            assert_eq!(listed.len(), 1, "the waiting row and the asking artifact are one item: {listed:?}");
        });
    }

    #[gpui::test]
    fn a_press_on_an_artifact_a_waiting_row_names_picks_that_row(cx: &mut gpui::TestAppContext) {
        let row = format!(r#"[{{"id":"sketch:takes:bar","text":"Pick a take for the step bar","since":"2026-11-09T14:01:00Z","source":"sketch","artifact":"{ID}"}}]"#);
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |jobs| {
            let held = ipc::decode::<Vec<ipc::WaitingItem>>("waiting", row.as_bytes()).expect("a waiting row");
            jobs.iter_mut().find(|one| one.id.as_str() == JOB).expect("the job").waiting_for.extend(held);
        });
        view.update_in(cx, |view, window, cx| assert!(view.open_named(ID, window, cx)));
        redraw(cx);
        view.read_with(cx, |view, _| {
            let chosen = view.review_chosen().expect("the press picks an item");
            assert_eq!(chosen.id, "sketch:takes:bar", "the row listed in the artifact's place is the one picked");
        });
    }

    /// The Job with a sketch card in its thread and a waiting row for it, the sketch being its artifact of parts too.
    fn with_sketch_card(cx: &mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &mut gpui::VisualTestContext) {
        let row = format!(r#"[{{"id":"{ID}","text":"{TITLE}","since":"2026-11-14T10:01:00Z","source":"sketch"}}]"#);
        let card = format!(r#"{{"kind":"sketch","id":"sk-bar","at":"2026-11-14T10:01:00Z","sketch":"{ID}","title":"{TITLE}","drawing":{{}},"asked":true}}"#);
        super::super::parts_fold::rooted_job_with(
            cx,
            "artifact-parts",
            JOB,
            size(px(1400.), px(900.)),
            |jobs| {
                let held = ipc::decode::<Vec<ipc::WaitingItem>>("waiting", row.as_bytes()).expect("a waiting row");
                jobs.iter_mut().find(|one| one.id.as_str() == JOB).expect("the job").waiting_for.extend(held);
            },
            |threads| threads.get_mut(JOB).expect("the thread").items.push(ipc::decode::<ipc::ThreadItem>("a card", card.as_bytes()).expect("a sketch card")),
        )
    }

    /// Every item of the panel that stands for the artifact, by its card's id or by the artifact it reads.
    fn standing_for_it(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) -> Vec<(String, Kind)> {
        view.read_with(cx, |view, _| {
            let review = view.review().expect("review");
            review.groups.iter().flat_map(|group| group.items.iter()).filter(|item| item.id == ID || artifact_in(item) == Some(ID)).map(|item| (item.id.clone(), item.kind)).collect()
        })
    }

    #[gpui::test]
    fn a_sketch_card_and_the_artifact_of_parts_it_is_are_listed_once_and_the_artifact_stands_with_its_takes(cx: &mut gpui::TestAppContext) {
        let (view, cx) = with_sketch_card(cx);
        view.update_in(cx, |view, window, cx| view.open_review_on(ID, window, cx));
        redraw(cx);
        let listed = standing_for_it(&view, cx);
        assert_eq!(listed.iter().map(|(_, kind)| *kind).collect::<Vec<_>>(), [Kind::Artifact], "one entry, the artifact, which holds every part: {listed:?}");
        assert!(drawn(cx, Role::RadioButton, "Pick B \u{b7} Rail (Job's pick)"), "its takes are drawn, so a pick can be made");
    }

    #[gpui::test]
    fn an_artifact_he_opened_stands_for_its_sketch_card_in_the_panel(cx: &mut gpui::TestAppContext) {
        let (view, cx) = with_sketch_card(cx);
        view.update_in(cx, |view, window, cx| assert!(view.open_named(ID, window, cx)));
        redraw(cx);
        let listed = standing_for_it(&view, cx);
        assert_eq!(listed.iter().map(|(_, kind)| *kind).collect::<Vec<_>>(), [Kind::Artifact], "one entry, the artifact he opened: {listed:?}");
    }

    #[gpui::test]
    fn a_pick_sent_from_the_panels_foot_carries_the_take_and_his_words(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        let key = super::super::parts_card::part_key(ID, "bar");
        let pick = bounds_of(cx, Role::RadioButton, &format!("Pick {}", TAKES[1]));
        let send = bounds_of(cx, Role::Button, "Send pick");
        let words = bounds_of(cx, Role::TextInput, "Note on your pick");
        // The takes scroll in the detail, so a tall one runs past the foot: the foot is what the pick row stands in, at the panel's bottom.
        let panel = bounds_of(cx, Role::Dialog, "Review");
        let low = panel.top() + panel.size.height * 0.75;
        assert!(send.top() > pick.top() && words.top() > pick.top(), "the words box and Send pick stand under the takes' tops");
        assert!(send.top() > low && words.top() > low && send.bottom() < panel.bottom(), "and in the foot, at the panel's bottom");
        view.update_in(cx, |view, window, cx| {
            view.choose_take(&key, 1, cx);
            let input = view.pages.get(JOB).and_then(|page| page.cards.parts.notes(&key)).expect("the words box");
            input.update(cx, |input, cx| input.set_value("Rail, but narrower.", window, cx));
            view.send_pick(&key, window, cx);
        });
        view.read_with(cx, |view, _| {
            let sent = view.pages.get(JOB).and_then(|page| page.cards.parts.answer_sent(&key)).expect("the pick went");
            assert_eq!((sent.take.as_deref(), sent.words.as_deref()), (Some("rail"), Some("Rail, but narrower.")), "one answer holds the pick and the note");
        });
    }

    /// The panel open on the artifact with a second vote added under the first: "side", of two takes with no scene.
    fn opened_with_two_votes<'a>(cx: &'a mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        view.update(cx, |view, cx| {
            view.mock_parts_tuned(
                ID,
                |artifact| {
                    let at = artifact.parts.iter().position(|part| part.id == "bar").expect("the first vote");
                    let take = |id: &str, title: &str| ipc::PartTake { id: id.into(), title: title.into(), scene: None };
                    let content = ipc::PartContent::Takes { question: Some("Which side?".into()), form: ipc::VoteForm::Pick, takes: vec![take("left", "L \u{b7} Left"), take("right", "R \u{b7} Right")], parts: Vec::new() };
                    let side = ipc::ArtifactPart { id: "side".into(), drawn: "side-1".into(), content, ..artifact.parts[at].clone() };
                    artifact.parts.insert(at + 1, side);
                },
                cx,
            )
        });
        redraw(cx);
        (view, cx)
    }

    fn note(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext, key: &str, words: &str) {
        view.update_in(cx, |view, window, cx| {
            let input = view.pages.get(JOB).and_then(|page| page.cards.parts.notes(key)).expect("the vote's words box");
            input.update(cx, |input, cx| input.set_value(words, window, cx));
        });
    }

    #[gpui::test]
    fn several_votes_are_sent_by_one_send_picks_with_each_votes_own_note(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened_with_two_votes(cx);
        let (bar, side) = (super::super::parts_card::part_key(ID, "bar"), super::super::parts_card::part_key(ID, "side"));
        assert!(!drawn(cx, Role::Button, "Send pick"), "no vote has a Send pick of its own");
        view.update_in(cx, |view, _, cx| {
            view.choose_take(&bar, 1, cx);
            view.choose_take(&side, 0, cx);
        });
        note(&view, cx, &bar, "Rail, but narrower.");
        note(&view, cx, &side, "Left reads first.");
        redraw(cx);
        let send = bounds_of(cx, Role::Button, "Send picks").center();
        cx.simulate_click(send, Modifiers::none());
        redraw(cx);
        view.read_with(cx, |view, _| {
            let posts: Vec<_> = view.mock_part_posts().iter().map(|post| (post.part.as_str(), post.take.as_deref(), post.words.as_deref())).collect();
            assert_eq!(posts, [("bar", Some("rail"), Some("Rail, but narrower.")), ("side", Some("left"), Some("Left reads first."))], "one answer to each vote, each with its own note");
            let held = view.artifact_of(JOB, ID).expect("the artifact");
            let state = |id: &str| held.parts.iter().find(|part| part.id == id).map(|part| part.state);
            assert_eq!((state("bar"), state("side")), (Some(ipc::PartState::Picked), Some(ipc::PartState::Picked)), "both votes are answered");
        });
    }

    #[gpui::test]
    fn a_vote_with_no_pick_stays_asking_after_send_picks_and_the_button_waits_for_a_pick(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened_with_two_votes(cx);
        let side = super::super::parts_card::part_key(ID, "side");
        let send = bounds_of(cx, Role::Button, "Send picks").center();
        cx.simulate_click(send, Modifiers::none());
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.mock_part_posts().is_empty()), "with no pick there is nothing to send");
        view.update_in(cx, |view, _, cx| view.choose_take(&side, 1, cx));
        redraw(cx);
        let send = bounds_of(cx, Role::Button, "Send picks").center();
        cx.simulate_click(send, Modifiers::none());
        redraw(cx);
        view.read_with(cx, |view, _| {
            let posts: Vec<_> = view.mock_part_posts().iter().map(|post| (post.part.as_str(), post.take.as_deref())).collect();
            assert_eq!(posts, [("side", Some("right"))], "only the vote with a pick went");
            let held = view.artifact_of(JOB, ID).expect("the artifact");
            let state = |id: &str| held.parts.iter().find(|part| part.id == id).map(|part| part.state);
            assert_eq!((state("bar"), state("side")), (Some(ipc::PartState::Open), Some(ipc::PartState::Picked)), "the other vote still asks");
        });
        assert!(drawn(cx, Role::Button, "Send pick"), "one vote left is a single vote again");
    }

    #[gpui::test]
    fn cmd_enter_sends_every_pick_of_several_votes(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened_with_two_votes(cx);
        let (bar, side) = (super::super::parts_card::part_key(ID, "bar"), super::super::parts_card::part_key(ID, "side"));
        view.update_in(cx, |view, _, cx| {
            view.choose_take(&bar, 0, cx);
            view.choose_take(&side, 1, cx);
        });
        redraw(cx);
        cx.simulate_keystrokes("cmd-enter");
        redraw(cx);
        assert_eq!(view.read_with(cx, |view, _| view.mock_part_posts().len()), 2, "the key sends them all");
    }

    #[gpui::test]
    fn one_takes_part_keeps_its_send_pick(cx: &mut gpui::TestAppContext) {
        let (_, cx) = opened(cx, 1400., 900.);
        assert!(drawn(cx, Role::Button, "Send pick"), "a single vote has its own Send pick");
        assert!(!drawn(cx, Role::Button, "Send picks"), "and no Send picks");
    }

    #[gpui::test]
    fn cmd_enter_in_an_ask_for_changes_box_sends_the_note(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        // The app's own keys: the registry binds ⌘↵ as Send everywhere, as main.rs installs it.
        cx.update(|_, cx| crate::keymap::install(&crate::keymap::Rebinding::default(), cx));
        view.update_in(cx, |view, window, cx| view.part_ask_for_changes(ID, "intro", window, cx));
        redraw(cx);
        cx.simulate_input("Shorter, please.");
        redraw(cx);
        cx.simulate_keystrokes("cmd-enter");
        redraw(cx);
        view.read_with(cx, |view, _| {
            let posts: Vec<_> = view.mock_part_posts().iter().map(|post| (post.part.clone(), post.how, post.words.clone())).collect();
            assert_eq!(posts, [("intro".to_string(), ipc::PartHow::SentBack, Some("Shorter, please.".to_string()))], "⌘↩ sends the note as Send does");
        });
    }

    #[gpui::test]
    fn escape_in_an_ask_for_changes_box_closes_the_box_and_the_next_one_closes_the_panel(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        view.update_in(cx, |view, window, cx| view.part_ask_for_changes(ID, "intro", window, cx));
        redraw(cx);
        let boxed = |view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext| view.read_with(cx, |view, _| view.pages.get(JOB).is_some_and(|page| !page.cards.modes.is_empty()));
        assert!(boxed(&view, cx), "the box is open");
        cx.simulate_keystrokes("escape");
        assert!(!boxed(&view, cx), "the first Esc closes the box");
        assert!(view.read_with(cx, |view, _| view.review_open()), "and leaves the panel");
        cx.simulate_keystrokes("escape");
        assert!(!view.read_with(cx, |view, _| view.review_open()), "the next Esc closes the panel");
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
    fn one_escape_closes_it_from_an_artifact_that_is_a_page_alone(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |_| {});
        view.update(cx, |view, cx| view.mock_parts_tuned(ID, |artifact| artifact.parts.retain(|part| part.id == PAGE), cx));
        redraw(cx);
        view.update_in(cx, |view, window, cx| assert!(view.open_named(ID, window, cx)));
        redraw(cx);
        assert!(drawn(cx, Role::Region, "Page"), "the page fills the detail, and no parts card is drawn in the panel");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "one Esc closes the panel: the keys were not put on the card in the thread behind it");
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
    fn escape_closes_the_panel_after_a_press_on_a_take_and_on_its_picture(cx: &mut gpui::TestAppContext) {
        for name in [format!("Pick {}", TAKES[0]), format!("Picture: {}", TAKES[0])] {
            let (view, cx) = opened(cx, 1400., 900.);
            let at = bounds_of(cx, if name.starts_with("Pick") { Role::RadioButton } else { Role::Button }, &name).center();
            cx.simulate_click(at, Modifiers::none());
            redraw(cx);
            cx.simulate_keystrokes("escape");
            redraw(cx);
            if view.read_with(cx, |view, _| view.review_open()) {
                cx.simulate_keystrokes("escape");
                redraw(cx);
            }
            assert!(!view.read_with(cx, |view, _| view.review_open()), "Esc closes the panel after a press on {name}");
        }
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

    /// The artifact read in the panel with its drawing part in view, and that drawing's bounds.
    fn drawing_in_view(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) -> Bounds<gpui::Pixels> {
        let drawing = bounds_of(cx, Role::Group, "Drawing");
        let (body, offset) = view.read_with(cx, |view, _| (view.body_scroll.bounds(), view.body_scroll.offset()));
        view.update(cx, |view, _| view.body_scroll.set_offset(gpui::point(px(0.), offset.y + body.top() - drawing.top())));
        redraw(cx);
        bounds_of(cx, Role::Group, "Drawing")
    }

    fn detail_scale(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) -> f32 {
        view.read_with(cx, |view, _| view.detail_zoom.as_ref().map_or(0., |detail| detail.zoom.scale))
    }

    #[gpui::test]
    fn a_press_on_a_drawing_among_parts_leaves_it_in_place_with_the_panel_open(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        let drawing = drawing_in_view(&view, cx);
        let (column, panel) = (bounds_of(cx, Role::Group, "Takes"), view.read_with(cx, |view, _| view.sheet.get().height));
        assert_eq!(drawing.size.width, column.size.width, "the drawing takes the column's width: {drawing:?} beside {column:?}");
        assert!(drawing.size.height <= panel * TALL_SHARE + px(1.), "and no more than its share of the panel's height: {drawing:?}");
        cx.simulate_click(drawing.center(), Modifiers::none());
        redraw(cx);
        view.read_with(cx, |view, _| {
            assert!(view.pictures.viewing.is_none(), "a press opens nothing over the app");
            assert!(view.review_open(), "and the panel stays");
        });
        assert_eq!(detail_scale(&view, cx), 1., "one press zooms nothing");
    }

    #[gpui::test]
    fn command_and_the_wheel_or_a_pinch_zoom_the_drawing_in_place_and_a_plain_wheel_scrolls_past_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        let at = drawing_in_view(&view, cx).center();
        let wheel = |modifiers| gpui::ScrollEvent { position: at, delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(120.))), modifiers, ..Default::default() };
        let page = view.read_with(cx, |view, _| view.body_scroll.offset());
        cx.simulate_event(wheel(Modifiers::none()));
        redraw(cx);
        assert_eq!(detail_scale(&view, cx), 1., "a plain wheel at fit leaves the drawing as it was");
        assert_ne!(view.read_with(cx, |view, _| view.body_scroll.offset()), page, "and scrolls the parts");
        let at = bounds_of(cx, Role::Group, "Drawing").center();
        let page = view.read_with(cx, |view, _| view.body_scroll.offset());
        cx.simulate_event(gpui::ScrollEvent { position: at, ..wheel(Modifiers::command()) });
        redraw(cx);
        let zoomed = detail_scale(&view, cx);
        assert!(zoomed > 1.2, "a wheel with command held zooms it: {zoomed}");
        assert_eq!(view.read_with(cx, |view, _| view.body_scroll.offset()), page, "and the parts stand still");
        assert!(drawn(cx, Role::Button, "Fit"), "the corner shows while it is zoomed");
        cx.simulate_event(gpui::PinchEvent { position: at, delta: 0.5, ..Default::default() });
        redraw(cx);
        assert!(detail_scale(&view, cx) > zoomed, "a pinch zooms it further");
        cx.simulate_keystrokes("cmd-0");
        redraw(cx);
        assert_eq!(detail_scale(&view, cx), 1., "Cmd-0 fits it");
        cx.simulate_keystrokes("cmd-=");
        assert!(detail_scale(&view, cx) > 1., "Cmd-= zooms it");
        assert!(view.read_with(cx, |view, _| view.pictures.viewing.is_none() && view.review_open()));
    }

    #[gpui::test]
    fn the_see_larger_button_and_f_read_the_drawing_over_the_app(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        let flow = super::super::parts_card::part_key(ID, "flow");
        drawing_in_view(&view, cx);
        let larger = bounds_of(cx, Role::Button, "See larger").center();
        cx.simulate_click(larger, Modifiers::none());
        redraw(cx);
        assert_eq!(view.read_with(cx, |view, _| view.viewing_drawing().map(str::to_string)), Some(flow.clone()), "the button reads it over the app");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.pictures.viewing.is_none() && view.review_open()), "Esc puts it away and the panel stays");
        cx.simulate_keystrokes("f");
        redraw(cx);
        assert_eq!(view.read_with(cx, |view, _| view.viewing_drawing().map(str::to_string)), Some(flow), "and so does F");
    }

    #[gpui::test]
    fn a_second_drawing_part_still_opens_over_the_app_on_a_press(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        view.update(cx, |view, cx| {
            view.mock_parts_tuned(
                ID,
                |artifact| {
                    let at = artifact.parts.iter().position(|part| part.id == "flow").expect("the drawing part");
                    let again = ipc::ArtifactPart { id: "flow-again".into(), drawn: "flow-again-1".into(), ..artifact.parts[at].clone() };
                    artifact.parts.insert(at + 1, again);
                },
                cx,
            )
        });
        redraw(cx);
        assert!(drawing_in_view(&view, cx).size.width > px(0.), "the first drawing still zooms in place");
        let second = bounds_of(cx, Role::Button, "Drawing");
        let body = view.read_with(cx, |view, _| view.body_scroll.bounds());
        let offset = view.read_with(cx, |view, _| view.body_scroll.offset());
        view.update(cx, |view, _| view.body_scroll.set_offset(gpui::point(px(0.), offset.y + body.top() - second.top())));
        redraw(cx);
        let second = bounds_of(cx, Role::Button, "Drawing").center();
        cx.simulate_click(second, Modifiers::none());
        redraw(cx);
        let again = super::super::parts_card::part_key(ID, "flow-again");
        assert_eq!(view.read_with(cx, |view, _| view.viewing_drawing().map(str::to_string)), Some(again), "a press on the second opens it over the app");
    }

    #[gpui::test]
    fn a_page_part_opened_from_the_artifacts_tab_is_the_live_page_in_the_panel_and_no_tab_opens(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |_| {});
        let before = view.read_with(cx, |view, _| view.arrangement().into_owned());
        view.update_in(cx, |view, window, cx| assert!(view.open_named(ID, window, cx)));
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.review_open()), "the panel is up");
        assert!(view.read_with(cx, |view, cx| view.panel_page(JOB, PAGE, 2, cx).is_some()), "it holds the page's live view");
        assert!(drawn(cx, Role::Region, "Page"), "the live page is drawn in the detail");
        let (page, takes) = (bounds_of(cx, Role::Region, "Page"), bounds_of(cx, Role::Group, "Takes"));
        assert!(page.left() - takes.left() <= px(2.) && takes.right() - page.right() <= px(2.), "it takes the detail's whole width, as the takes do, less its hairline frame: {page:?} {takes:?}");
        let panel = view.read_with(cx, |view, _| view.sheet.get().height);
        assert!(page.size.height <= panel * TALL_SHARE + px(1.), "and no taller than a picture stands: {page:?}");
        assert_eq!(view.read_with(cx, |view, _| view.arrangement().into_owned()), before, "no page tab opened");
        assert!(view.read_with(cx, |view, _| view.row_tabs.get(JOB).is_none_or(|tabs| tabs.pages.is_empty())), "no tab holds the page");
        view.update_in(cx, |view, window, cx| view.close_review(window, cx));
        redraw(cx);
        assert!(view.read_with(cx, |view, cx| view.panel_page(JOB, PAGE, 2, cx).is_none()), "the page goes with the panel");
    }

    #[gpui::test]
    fn a_plain_page_opened_from_the_artifacts_tab_opens_in_a_tab_of_its_job_and_the_panel_stays_shut(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "reader", "draft-job", size(px(1400.), px(900.)), |_| {});
        let page = view
            .read_with(cx, |view, _| view.artifacts_of("draft-job").iter().find(|one| one.form == ipc::ArtifactForm::Page && one.parts.is_empty()).map(|one| one.id.clone()))
            .expect("the Job holds a page with no parts");
        view.update_in(cx, |view, window, cx| assert!(view.open_named(&page, window, cx)));
        redraw(cx);
        // A page whose card stands in the thread reads in a tab beside the thread, with its margin, in place of the panel.
        assert!(!view.read_with(cx, |view, _| view.review_open()), "the panel stays shut");
        assert!(view.read_with(cx, |view, _| view.row_tab_list("draft-job").iter().any(|tab| tab.kind == super::super::row_tabs::RowKind::Document && tab.key == page)), "a tab holds the page");
        assert!(drawn(cx, Role::Region, "Page"), "and draws it live");
    }

    #[gpui::test]
    fn a_page_that_is_an_artifacts_only_part_takes_the_whole_detail_and_one_among_parts_keeps_its_share(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        let panel = view.read_with(cx, |view, _| view.sheet.get().height);
        assert!(bounds_of(cx, Role::Region, "Page").size.height <= panel * TALL_SHARE + px(1.), "among other parts it stands at its share");
        view.update(cx, |view, cx| view.mock_parts_tuned(ID, |artifact| artifact.parts.retain(|part| part.id == PAGE), cx));
        redraw(cx);
        let (page, band) = (bounds_of(cx, Role::Region, "Page"), bounds_of(cx, Role::Group, "Answers"));
        assert!(page.top() - band.bottom() <= px(2.), "alone, it starts under the band: {band:?} {page:?}");
        assert!(page.size.height > panel * TALL_SHARE, "and takes more than a picture's share of the panel: {page:?}");
    }

    #[gpui::test]
    fn a_vote_and_a_press_the_panels_page_posts_are_carried_to_fleet_and_its_answer_goes_back(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        let tab = view.read_with(cx, |view, cx| view.panel_page(JOB, PAGE, 2, cx)).expect("the panel's live page");
        let post = |posted: PageToHost, cx: &mut gpui::VisualTestContext| {
            tab.update(cx, |_, cx| cx.emit(PageTabEvent::Posted(posted)));
            redraw(cx);
        };
        let pick = |take: &str| PageToHost::Vote { artifact: PAGE.into(), revision: 2, vote: "mark".into(), result: ipc::VoteResult::Pick { take: take.into(), notes: Default::default() } };
        post(pick("fade"), cx);
        let kept = view.read_with(cx, |view, _| view.row_tabs.get(JOB).and_then(|tabs| tabs.mock_kept.get(PAGE)).cloned()).expect("the mock Fleet kept the cast");
        assert_eq!(kept.votes.iter().map(|cast| cast.vote.as_str()).collect::<Vec<_>>(), ["mark"], "cast_page_vote reached Fleet");
        assert!(matches!(tab.read_with(cx, |tab, _| tab.answers.last().cloned()), Some(ipc::HostToPage::Voted { cast }) if cast.vote == "mark"), "and its answer went back to the page");
        post(PageToHost::Press { artifact: PAGE.into(), revision: 2, action: "discuss-dark-mode".into(), words: None }, cx);
        assert!(matches!(tab.read_with(cx, |tab, _| tab.answers.last().cloned()), Some(ipc::HostToPage::Pressed { press }) if press.action == "discuss-dark-mode"), "a row action is answered the same way");
        post(PageToHost::Press { artifact: PAGE.into(), revision: 2, action: "drop-checkout".into(), words: None }, cx);
        assert!(
            matches!(tab.read_with(cx, |tab, _| tab.answers.last().cloned()), Some(ipc::HostToPage::Refused { code, .. }) if code == "fleet.page_ask_not_open"),
            "Fleet's refusal goes back in its own words and code"
        );
        assert!(view.read_with(cx, |view, _| view.review_open()), "and the panel stays up");
    }

    #[gpui::test]
    fn a_page_fleet_refuses_says_so_in_the_panel_and_draws_no_picture(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        view.update(cx, |view, cx| {
            view.mock_parts_tuned(
                ID,
                |artifact| {
                    for part in artifact.parts.iter_mut().filter(|part| part.id == PAGE) {
                        part.content = ipc::PartContent::Page { path: "notes".into() };
                    }
                },
                cx,
            )
        });
        redraw(cx);
        assert!(drawn(cx, Role::Alert, "revision 2 of `notes` is not kept"), "Fleet's sentence stands where the page would");
        assert!(view.read_with(cx, |view, cx| view.panel_page(JOB, PAGE, 2, cx).is_none() && view.panel_page(JOB, "notes", 2, cx).is_some()), "the page the part names now is the one held");
    }

    #[gpui::test]
    fn a_takes_picture_fills_its_tile_with_no_frame_between(cx: &mut gpui::TestAppContext) {
        let (_, cx) = opened(cx, 1400., 900.);
        let tile = bounds_of(cx, Role::RadioButton, "Pick A \u{b7} Bar under the title");
        let picture = bounds_of(cx, Role::Button, "Picture: A \u{b7} Bar under the title");
        assert!(picture.left() - tile.left() < px(4.) && tile.right() - picture.right() < px(4.), "no padding: {tile:?} {picture:?}");
    }
}
