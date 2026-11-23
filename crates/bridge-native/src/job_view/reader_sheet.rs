//! An artifact of parts read in the review panel. **The panel is the one overlay**: the list of what waits on him on the left, the
//! chosen thing on the right, the foot under both. An artifact he opens, from a Needs you row, the Artifacts tab, a card's Open or
//! the For you tab, is picked in that list, with the Job's other artifacts that still ask beside it.
//!
//! Its parts are answered where they stand, one by one, so the band holds no Approve and the notes column is left out: a take is
//! picked by a press or its number, a vote is cast with Send, and the footer under the parts settles or puts the artifact away.
//! Prose keeps a measure and stands at the left; pictures, drawings and takes take the detail's whole inner width.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, Pixels, SharedString, Window};

use super::band_card::Stand;
use super::picture_view::What;
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

/// The zoom of the drawing the panel's detail holds in place, with the scene key it draws under, and the drawing part a press
/// put there in place of the first. Fresh for each item chosen.
pub(super) struct DetailZoom {
    pub zoom: Zoomer,
    pub held: RefCell<String>,
    pub part: Option<String>,
}

impl DetailZoom {
    pub(super) fn new() -> Self {
        DetailZoom { zoom: Zoomer::default(), held: RefCell::new(String::new()), part: None }
    }
}

/// What the stage draws.
pub(super) struct Staged {
    held: String,
    scene: Rc<ipc::Scene>,
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

    /// Read the document `page` of the shown Job in the review panel, with its live page: one press, as a page part is read. A
    /// page the Job holds no card or artifact for opens nothing.
    pub(super) fn open_document(&mut self, page: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
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

    /// The live page filling the detail, where the chosen item is a document or an artifact whose only part is a page: the whole
    /// of the detail between its head and its answer bar, edge to edge, which the page scrolls inside.
    pub(super) fn review_page_stage(&self, item: &Item, cx: &gpui::App) -> Option<AnyElement> {
        let alone = item.kind == Kind::Document || self.page_alone(item);
        let (page, revision, _) = self.panel_pages_of(item, cx).into_iter().next().filter(|_| alone)?;
        let live = self.panel_page(&item.job, &page, revision, cx)?;
        Some(div().id(SharedString::from(format!("review-page:{}", item.key()))).relative().flex_1().min_w_0().min_h_0().child(div().absolute().inset_0().child(live)).into_any_element())
    }

    /// Whether the item is an artifact whose only part is a page, which the panel draws as the page alone: the answer bar
    /// answers a page that asks, so no caption or chip stands over it.
    fn page_alone(&self, item: &Item) -> bool {
        let Some(artifact) = artifact_in(item).and_then(|artifact| self.artifact_of(&item.job, artifact)) else { return false };
        matches!(artifact.parts.as_slice(), [part] if matches!(part.content, ipc::PartContent::Page { .. }))
    }

    /// The parts of the chosen artifact, drawn the way its card draws them, at the detail's width.
    pub(super) fn review_parts(&self, item: &Item, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (Some(artifact), Some(page)) = (artifact_in(item), self.pages.get(item.job.as_str())) else { return div().into_any_element() };
        let Some(newest) = self.artifact_of(&item.job, artifact).and_then(|one| one.revisions.last().map(|last| last.number)) else { return div().into_any_element() };
        self.parts_card(super::acts::SHEET, artifact, newest, true, true, page, look, cx).unwrap_or_else(|| div().into_any_element())
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
                Some(Staged { held, scene })
            }
            Kind::Sketch => {
                let held = self.thread(&item.job, cx)?;
                let card = super::review::sketch_card(&held.head, &held.items, &item.id)?;
                let (shown, _) = super::cards::shown_scene(card, None);
                let held = format!("detail:{shown}");
                let scene = crate::scene::read(&held, &card.drawing)?;
                Some(Staged { held, scene })
            }
            _ => None,
        }
    }

    /// The drawing of the chosen item, filling the detail between its band and its foot: fitted to that box, zoomed in place by
    /// the one zoomer (a pinch or ⌘ and the wheel about the pointer, ⌘+ ⌘- ⌘0, a double press, a drag or a scroll once zoomed).
    /// An ask has no full-window viewer: the owner's decision of 19 Nov 2026.
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
        Some(
            labelled(
                zoomer::bind(div().id("review-stage").relative().flex_1().min_h_0().w_full().overflow_hidden().bg(look.color(token::BG_BASE)), cx, |_, _, _, _| {})
                    .child(zoom.probe())
                    .child(div().absolute().inset_0().child(crate::scene::view(SharedString::from(staged.held), staged.scene, Fit::Reading, Clock::default(), lens)))
                    .children(readout.map(|readout| zoomer::corner(look, readout, |this: &mut JobView, _, cx| zoomer::refit(this, cx), cx))),
                gpui::Role::Group,
                "Drawing",
            )
            .into_any_element(),
        )
    }

    /// The drawing part the panel zooms in place within an artifact of several parts: the one a press put there, else its first.
    /// The detail has one zoomer, so a press on another drawing moves the zoom to it; an ask never opens over the app.
    pub(super) fn in_place_drawing(&self, job: &str, artifact: &str) -> Option<String> {
        let artifact = self.artifact_of(job, artifact)?;
        if artifact.parts.len() < 2 {
            return None;
        }
        let drawings = artifact.parts.iter().filter(|part| matches!(part.content, ipc::PartContent::Drawing { .. }));
        let pressed = self.detail_zoom.as_ref().and_then(|detail| detail.part.clone());
        let mut keys = drawings.map(|part| part_key(&artifact.id, &part.id));
        let first = keys.next()?;
        Some(pressed.filter(|key| *key == first || keys.any(|other| other == *key)).unwrap_or(first))
    }

    /// A press on a drawing part the detail does not zoom: it takes the zoomer, fitted, where it stands in the body.
    pub(super) fn zoom_drawing_in_place(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(detail) = self.detail_zoom.as_mut() else { return };
        detail.part = Some(key.to_string());
        detail.zoom = Zoomer::default();
        cx.notify();
    }

    /// A drawing part zoomed where it stands in the panel's parts: the column's width, as tall as its shape asks up to `tall`.
    /// A pinch or ⌘ and the wheel zoom it, a drag moves it, a double press toggles fit and 100%; a plain wheel at fit scrolls
    /// the parts on past it.
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
                    .children(readout.map(|readout| zoomer::corner(look, readout, |this: &mut JobView, _, cx| zoomer::refit(this, cx), cx))),
                gpui::Role::Group,
                "Drawing",
            )
            .into_any_element(),
        )
    }

    /// Whether the chosen item's detail holds a drawing the one zoomer zooms: staged, or a drawing part zoomed in place.
    fn review_zooms(&self, item: &Item, cx: &gpui::App) -> bool {
        self.staged(item, cx).is_some() || artifact_in(item).and_then(|artifact| self.in_place_drawing(&item.job, artifact)).is_some()
    }

    /// ⌘+ ⌘- ⌘0 zoom the drawing in the detail. False where the key is not one of these, nothing is staged or zoomed in place,
    /// or he is typing in a box of the artifact's parts.
    pub(super) fn review_zoom_key(&mut self, event: &gpui::KeyDownEvent, noting: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.pictures.viewing.is_some() || self.sketch_reading.is_some() || noting {
            return false;
        }
        let Some(item) = self.review_chosen() else { return false };
        if artifact_in(&item).is_some_and(|artifact| self.parts_typing(artifact, window, cx)) {
            return false;
        }
        if !self.review_zooms(&item, cx) {
            return false;
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

    /// A take's radio on the sheet, chosen or not: the vote starts on the Job's pick, whose title drops its mark.
    fn take_bounds(cx: &mut gpui::VisualTestContext, take: &str) -> Bounds<gpui::Pixels> {
        let (title, _) = super::super::parts_card::pick_mark(take);
        let chosen = format!("Chosen: {title}");
        if drawn(cx, Role::RadioButton, &chosen) {
            return bounds_of(cx, Role::RadioButton, &chosen);
        }
        bounds_of(cx, Role::RadioButton, &format!("Pick {title}"))
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
        let chosen = view.read_with(cx, |view, _| view.review_chosen()).expect("an item is picked");
        assert!(chosen.kind == Kind::Artifact && chosen.id.contains(ID), "the artifact is picked: {chosen:?}");
        assert!(drawn(cx, Role::Region, &chosen.name), "the artifact is the detail");
        assert_eq!(view.read_with(cx, |view, _| view.arrangement().into_owned()), before, "no pane gained a tab");
        view.update_in(cx, |view, window, cx| view.close_review(window, cx));
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()));
        assert_eq!(view.read_with(cx, |view, _| view.arrangement().into_owned()), before, "the panes are as they were");
    }

    #[gpui::test]
    fn an_artifact_of_one_drawing_takes_approve_and_ask_for_changes_in_the_answer_bar_at_the_panels_foot(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-folds", JOB, size(px(1400.), px(900.)), |_| {});
        view.update_in(cx, |view, window, cx| assert!(view.open_named("sketch:Step flow", window, cx)));
        redraw(cx);
        let panel = bounds_of(cx, Role::Dialog, "Review");
        let bar = bounds_of(cx, Role::Toolbar, "Answer");
        let approve = bounds_of(cx, Role::Button, "Approve");
        let ask = bounds_of(cx, Role::Button, "Ask for changes");
        let dismiss = bounds_of(cx, Role::Button, "Dismiss");
        assert!((bar.bottom() - panel.bottom()).abs() <= px(2.), "the bar is the panel's foot: {bar:?} {panel:?}");
        for (name, one) in [("Approve", approve), ("Ask for changes", ask), ("Dismiss", dismiss)] {
            assert!(one.top() >= bar.top() && one.bottom() <= bar.bottom(), "{name} stands in the bar: {one:?} {bar:?}");
            assert!(one.top() < dismiss.bottom() && one.bottom() > dismiss.top(), "{name} stands in Dismiss's row: {one:?} {dismiss:?}");
        }
        assert!(approve.right() <= ask.left() && ask.right() < dismiss.left(), "Approve then Ask for changes, and Dismiss after them");
        assert!(approve.left() < bar.left() + bar.size.width * 0.5, "the answer leads the bar");
        assert!(bar.right() - dismiss.right() < bar.size.width * 0.1, "and Dismiss stands at its far end");
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
        // On the sheet the vote starts on the Job's pick; the thread behind holds one line for the sketch, not its takes.
        assert_eq!(view.read_with(cx, |view, _| view.review_chosen().map(|item| item.kind)), Some(Kind::Artifact), "the sheet opens on the artifact the row is");
        let _ = take_bounds(cx, TAKES[1]);
        assert!(drawn(cx, Role::RadioButton, &format!("Pick {}", TAKES[0])), "its takes are drawn, so a pick can be made");
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
    fn a_pick_sent_from_the_answer_bar_carries_the_take_he_chose(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        let key = super::super::parts_card::part_key(ID, "bar");
        let pick = bounds_of(cx, Role::RadioButton, &format!("Pick {}", TAKES[0]));
        assert!(drawn(cx, Role::Button, "Pick B \u{b7} Rail"), "the Job's pick starts chosen, and the bar's answer is named for it");
        let bar = bounds_of(cx, Role::Toolbar, "Answer");
        let panel = bounds_of(cx, Role::Dialog, "Review");
        assert!(bar.top() > pick.top() && bar.top() > panel.top() + panel.size.height * 0.75 && bar.bottom() <= panel.bottom(), "the bar stands under the takes, at the panel's bottom: {pick:?} {bar:?} {panel:?}");
        view.update(cx, |view, cx| view.choose_take(&key, 0, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Button, &format!("Pick {}", TAKES[0])), "the bar's answer follows the take he chose");
        view.update_in(cx, |view, window, cx| {
            let item = view.review_chosen().expect("the artifact is chosen");
            assert!(view.sheet_answer_parts(&item, None, window, cx), "the bar's answer sends");
        });
        view.read_with(cx, |view, _| {
            let sent = view.pages.get(JOB).and_then(|page| page.cards.parts.answer_sent(&key)).expect("the pick went");
            assert_eq!(sent.take.as_deref(), Some("bar"), "the answer holds the take he chose");
        });
    }

    /// A second vote added under the first: "side", of two takes with no scene.
    fn add_side_vote(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) {
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
    }

    /// Fleet holds an artifact to one ask, and the sheet is the one place an ask is answered (19 Nov). An artifact from before
    /// that, with two votes still asking, keeps no tab of its own (his note, 23 Nov): a press on it opens the sheet on it.
    #[gpui::test]
    fn an_artifact_of_two_votes_keeps_no_tab_and_a_press_opens_the_sheet_on_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |_| {});
        add_side_vote(&view, cx);
        view.update_in(cx, |view, window, cx| view.open_artifact_reader(ID, window, cx));
        redraw(cx);
        assert!(!drawn(cx, Role::Region, "Reader: Step bar for Jobs, Revision 2"), "no tab reads it while it asks");
        view.update_in(cx, |view, window, cx| assert!(view.open_named(ID, window, cx)));
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.review_open()), "a press opens the sheet");
        assert_eq!(view.read_with(cx, |view, _| view.review_chosen().map(|item| item.kind)), Some(Kind::Artifact), "on the artifact");
        assert!(drawn(cx, Role::Toolbar, "Answer"), "where it is answered");
    }

    #[gpui::test]
    fn a_single_vote_is_answered_from_the_sheets_bar_and_nothing_draws_a_send_pick(cx: &mut gpui::TestAppContext) {
        let (_, cx) = opened(cx, 1400., 900.);
        assert!(drawn(cx, Role::Button, "Pick B \u{b7} Rail"), "the bar's answer names the take chosen");
        assert!(!drawn(cx, Role::Button, "Send pick"), "the thread behind holds one line, with no Send pick of its own");
        assert!(!drawn(cx, Role::Button, "Send picks"), "and no Send picks");
    }

    #[gpui::test]
    fn cmd_enter_in_an_ask_for_changes_box_sends_the_note(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        // The app's own keys: the registry binds ⌘↵ as Send everywhere, as main.rs installs it.
        cx.update(|_, cx| crate::keymap::install(&crate::keymap::Rebinding::default(), cx));
        // The sheet's own Ask for changes: its box in the bar holds the note for every part that asks.
        let ask = bounds_of(cx, Role::Button, "Ask for changes").center();
        cx.simulate_click(ask, Modifiers::none());
        redraw(cx);
        cx.simulate_input("Shorter, please.");
        redraw(cx);
        cx.simulate_keystrokes("cmd-enter");
        redraw(cx);
        view.read_with(cx, |view, _| {
            let posts: Vec<_> = view.mock_part_posts().iter().map(|post| (post.part.clone(), post.how, post.words.clone())).collect();
            assert!(!posts.is_empty() && posts.iter().all(|(_, how, words)| *how == ipc::PartHow::SentBack && words.as_deref() == Some("Shorter, please.")), "⌘↩ sends the note as Send does: {posts:?}");
            assert!(posts.iter().any(|(part, _, _)| part == "intro"), "the words go back with it: {posts:?}");
        });
    }

    #[gpui::test]
    fn escape_in_an_ask_for_changes_box_closes_the_box_and_the_next_one_closes_the_panel(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        // The sheet's own box, in its bar: the thread behind holds one line for the artifact and no box of its own.
        let ask = bounds_of(cx, Role::Button, "Ask for changes").center();
        cx.simulate_click(ask, Modifiers::none());
        redraw(cx);
        assert!(drawn(cx, Role::TextInput, "Ask for changes"), "the box is open");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!drawn(cx, Role::TextInput, "Ask for changes"), "the first Esc closes the box");
        assert!(view.read_with(cx, |view, _| view.review_open()), "and leaves the panel");
        cx.simulate_keystrokes("escape");
        assert!(!view.read_with(cx, |view, _| view.review_open()), "the next Esc closes the panel");
    }

    /// The sheet's Ask for changes box, open: the rich box the Job composer's words are, holding his note.
    fn sheet_note(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) -> gpui::Entity<super::super::rich_note::RichNote> {
        let ask = bounds_of(cx, Role::Button, "Ask for changes").center();
        cx.simulate_click(ask, Modifiers::none());
        redraw(cx);
        view.read_with(cx, |view, _| view.review().and_then(|review| review.note.as_ref()).map(|held| held.note.clone())).expect("the box is open, a rich one")
    }

    #[gpui::test]
    fn the_sheets_ask_for_changes_box_sends_a_pasted_picture_and_a_tag_with_its_words(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        cx.update(|_, cx| crate::keymap::install(&crate::keymap::Rebinding::default(), cx));
        let note = sheet_note(&view, cx);
        note.update_in(cx, |note, window, cx| {
            let picture = gpui::ClipboardItem::new_image(&gpui::Image::from_bytes(gpui::ImageFormat::Png, vec![1, 2, 3]));
            assert!(note.paste(&picture, cx), "a pasted picture is attached, not typed");
            note.type_words("Shorter, as @Brief", window, cx);
            assert!(note.offering(), "@ opens the list of Jobs and files, as the composer's does");
            note.pick_offered(0, window, cx);
        });
        redraw(cx);
        cx.simulate_keystrokes("cmd-enter");
        redraw(cx);
        view.read_with(cx, |view, _| {
            let posts = view.mock_part_posts();
            assert!(!posts.is_empty(), "the note went back");
            assert_eq!(posts[0].extras.attachments.len(), 1, "the picture goes with the words: {:?}", posts[0].extras);
            assert_eq!(posts[0].extras.mentions.len(), 1, "and the tag: {:?}", posts[0].extras);
            assert!(posts[0].words.as_deref().is_some_and(|words| words.starts_with("Shorter, as ")), "{:?}", posts[0].words);
            assert!(posts[1..].iter().all(|post| post.extras.is_empty()), "once, with the first part sent back");
        });
    }

    #[gpui::test]
    fn escape_in_the_sheets_box_closes_its_tag_list_first_then_the_box_then_the_sheet(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, 1400., 900.);
        let note = sheet_note(&view, cx);
        note.update_in(cx, |note, window, cx| note.type_words("as @Brief", window, cx));
        redraw(cx);
        assert!(note.read_with(cx, |note, _| note.offering()), "the list is open");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!note.read_with(cx, |note, _| note.offering()), "the first Esc closes the list");
        assert!(drawn(cx, Role::TextInput, "Ask for changes"), "and leaves the box");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!drawn(cx, Role::TextInput, "Ask for changes"), "the next closes the box");
        assert!(view.read_with(cx, |view, _| view.review_open()));
        cx.simulate_keystrokes("escape");
        assert!(!view.read_with(cx, |view, _| view.review_open()), "and the next the sheet");
    }

    #[gpui::test]
    fn a_settled_takes_ask_opened_in_the_sheet_says_what_he_picked_with_no_radios_and_no_bar(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts-answered", JOB, size(px(1400.), px(900.)), |_| {});
        view.update_in(cx, |view, window, cx| view.open_artifact_review(ID, window, cx));
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.review_open()), "the sheet opens on the settled artifact");
        assert!(drawn(cx, Role::Status, "Picked: B \u{b7} Rail"), "it says the take he picked");
        for take in TAKES {
            let (title, _) = super::super::parts_card::pick_mark(take);
            for name in [format!("Chosen: {title}"), format!("Pick {title}")] {
                assert!(!drawn(cx, Role::RadioButton, &name), "no radio: {name}");
            }
        }
        assert!(!drawn(cx, Role::Toolbar, "Answer"), "and no answer bar");
    }

    /// The draft Job with its release note waiting on him, opened in the sheet.
    fn draft_opened(cx: &mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &mut gpui::VisualTestContext) {
        let row = r#"[{"id":"draft:dra-1","text":"Release note for 0.42","since":"2026-11-10T22:02:00Z","source":"draft"}]"#;
        let (view, cx) = rooted_job(cx, "drafts", "draft-job", size(px(1400.), px(900.)), |jobs| {
            let held = ipc::decode::<Vec<ipc::WaitingItem>>("waiting", row.as_bytes()).expect("a waiting row");
            jobs.iter_mut().find(|one| one.id.as_str() == "draft-job").expect("the job").waiting_for.extend(held);
        });
        cx.update(|_, cx| crate::keymap::install(&crate::keymap::Rebinding::default(), cx));
        view.update_in(cx, |view, window, cx| view.open_review_on("draft:dra-1", window, cx));
        redraw(cx);
        (view, cx)
    }

    #[gpui::test]
    fn edit_on_a_draft_opens_its_words_in_the_bar_and_approves_them_with_his_fixes(cx: &mut gpui::TestAppContext) {
        let (view, cx) = draft_opened(cx);
        assert_eq!(view.read_with(cx, |view, _| view.review_chosen().map(|item| item.kind)), Some(Kind::Draft));
        let (edit, approve) = (bounds_of(cx, Role::Button, "Edit"), bounds_of(cx, Role::Button, "Approve"));
        assert!(approve.right() <= edit.left(), "Edit stands after Approve in the bar");
        assert!((edit.size.height - approve.size.height).abs() < px(1.) && (edit.center().y - approve.center().y).abs() < px(1.), "one height and one line: {edit:?} {approve:?}");
        assert!(!drawn(cx, Role::Button, "Discuss"), "no Discuss");
        cx.simulate_keystrokes("e");
        redraw(cx);
        assert!(drawn(cx, Role::TextInput, "Draft text"), "E opens the draft's words in the bar");
        let area = view.read_with(cx, |view, _| view.review().and_then(|review| review.edit.as_ref()).map(|(_, area)| area.clone())).expect("the box");
        assert_eq!(area.read_with(cx, |area, _| area.value().to_string()), "Armada 0.42 draws each tool call on a row of its own.", "holding the draft as the Job wrote it");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!drawn(cx, Role::TextInput, "Draft text"), "Esc shuts the box");
        assert!(view.read_with(cx, |view, _| view.review_open()), "and leaves the sheet");
        cx.simulate_keystrokes("e");
        redraw(cx);
        let area = view.read_with(cx, |view, _| view.review().and_then(|review| review.edit.as_ref()).map(|(_, area)| area.clone())).expect("the box again");
        area.update_in(cx, |area, window, cx| area.set_value("Armada 0.42 draws each call on its own row.", window, cx));
        redraw(cx);
        cx.simulate_keystrokes("cmd-enter");
        redraw(cx);
        view.read_with(cx, |view, _| {
            let answer = view.pages.get("draft-job").and_then(|page| page.cards.answers.get("draft:dra-1").cloned());
            assert_eq!(answer, Some(super::super::acts::Answer::Fixed("Armada 0.42 draws each call on its own row.".into())), "approved with his words in place of the Job's");
        });
    }

    #[test]
    fn approve_with_fixes_sends_his_words_as_the_fix() {
        let wire = super::super::review::fixed_body("draft-job", "draft:dra-1", "Mine.".into()).expect("encodes");
        assert!(wire.contains(r#""item_id":"draft:dra-1""#) && wire.contains(r#""fixed":"Mine.""#), "{wire}");
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
    fn escape_closes_the_panel_after_a_press_on_a_take(cx: &mut gpui::TestAppContext) {
        // In the sheet a take's tile is its picture and its name, one press.
        let (view, cx) = opened(cx, 1400., 900.);
        let at = bounds_of(cx, Role::RadioButton, &format!("Pick {}", TAKES[0])).center();
        cx.simulate_click(at, Modifiers::none());
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.pictures.viewing.is_none()), "a press on the take opens nothing over the app");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        if view.read_with(cx, |view, _| view.review_open()) {
            cx.simulate_keystrokes("escape");
            redraw(cx);
        }
        assert!(!view.read_with(cx, |view, _| view.review_open()), "Esc closes the panel after a press on the take");
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
    fn takes_stand_side_by_side_at_2400(cx: &mut gpui::TestAppContext) {
        let (_, cx) = opened(cx, 2400., 1500.);
        let tops: Vec<_> = TAKES.iter().map(|name| take_bounds(cx, name).top()).collect();
        assert_eq!(tops[0], tops[1], "side by side at 2400");
    }

    /// The sheet's takes are cards up to three to a row at any width, so two still share a row at 1100.
    #[gpui::test]
    fn takes_stand_side_by_side_at_1100(cx: &mut gpui::TestAppContext) {
        let (_, cx) = opened(cx, 1100., 700.);
        let tops: Vec<_> = TAKES.iter().map(|name| take_bounds(cx, name).top()).collect();
        assert_eq!(tops[0], tops[1], "side by side at 1100");
    }

    #[gpui::test]
    fn prose_keeps_its_measure_while_pictures_take_the_detail_width(cx: &mut gpui::TestAppContext) {
        let (_, cx) = opened_drawings_at(cx, 2400., 1500.);
        let words = bounds_of(cx, Role::Group, "Words");
        let drawing = bounds_of(cx, Role::Group, "Drawing");
        let pill = bounds_of(cx, Role::Label, "Asks you");
        assert!(f32::from(pill.right() - words.left()) < 800., "the words' caption stands at the measure: {pill:?} {words:?}");
        assert!(f32::from(drawing.size.width) > 1200., "while the drawing runs the detail's width: {drawing:?}");
        assert_eq!(words.left(), drawing.left(), "prose and pictures share a left edge");
    }

    /// The artifact opened in the panel with its takes taken out, so it asks as a drawing and the sheet draws its parts.
    fn opened_drawings<'a>(cx: &'a mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        opened_drawings_at(cx, 1400., 900.)
    }

    /// The same, at `width` by `height`.
    fn opened_drawings_at<'a>(cx: &'a mut gpui::TestAppContext, width: f32, height: f32) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        let (view, cx) = opened(cx, width, height);
        view.update(cx, |view, cx| view.mock_parts_tuned(ID, |artifact| artifact.parts.retain(|part| !matches!(part.content, ipc::PartContent::Takes { .. })), cx));
        redraw(cx);
        (view, cx)
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
        let (view, cx) = opened_drawings(cx);
        let drawing = drawing_in_view(&view, cx);
        let (column, panel) = (bounds_of(cx, Role::Group, "Words"), view.read_with(cx, |view, _| view.sheet.get().height));
        assert!(drawing.left() == column.left() && drawing.size.width >= column.size.width, "the drawing takes the column's width: {drawing:?} beside {column:?}");
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
        let (view, cx) = opened_drawings(cx);
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
    fn f_reads_no_drawing_over_the_app_and_no_see_larger_button_is_drawn(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened_drawings(cx);
        drawing_in_view(&view, cx);
        assert!(!drawn(cx, Role::Button, "See larger"), "an ask has no full-window viewer");
        cx.simulate_keystrokes("f");
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.viewing_drawing().is_none() && view.review_open()), "F opens nothing over the app and the panel stays");
    }

    #[gpui::test]
    fn a_press_on_a_second_drawing_part_zooms_it_in_the_body_and_opens_nothing_over_the_app(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened_drawings(cx);
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
        assert_eq!(view.read_with(cx, |view, _| view.viewing_drawing().map(str::to_string)), None, "an ask opens nothing over the app");
        assert!(view.read_with(cx, |view, _| view.review_open()), "and the panel stays");
        assert_eq!(view.read_with(cx, |view, _| view.in_place_drawing(JOB, ID)), Some(again), "the second takes the zoom where it stands in the body");
        assert!(drawing_in_view(&view, cx).size.width > px(0.), "drawn zoomable in place");
    }

    #[gpui::test]
    fn a_page_part_opened_from_the_artifacts_tab_is_the_live_page_in_the_panel_and_no_tab_opens(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |_| {});
        view.update(cx, |view, cx| view.mock_parts_tuned(ID, |artifact| artifact.parts.retain(|part| !matches!(part.content, ipc::PartContent::Takes { .. })), cx));
        redraw(cx);
        let before = view.read_with(cx, |view, _| view.arrangement().into_owned());
        view.update_in(cx, |view, window, cx| assert!(view.open_named(ID, window, cx)));
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.review_open()), "the panel is up");
        assert!(view.read_with(cx, |view, cx| view.panel_page(JOB, PAGE, 2, cx).is_some()), "it holds the page's live view");
        assert!(drawn(cx, Role::Region, "Page"), "the live page is drawn in the detail");
        let (page, drawing) = (bounds_of(cx, Role::Region, "Page"), bounds_of(cx, Role::Group, "Drawing"));
        assert!(page.left() - drawing.left() <= px(2.) && drawing.right() - page.right() <= px(2.), "it takes the detail's whole width, as a drawing does, less its hairline frame: {page:?} {drawing:?}");
        let panel = view.read_with(cx, |view, _| view.sheet.get().height);
        assert!(page.size.height <= panel * TALL_SHARE + px(1.), "and no taller than a picture stands: {page:?}");
        assert_eq!(view.read_with(cx, |view, _| view.arrangement().into_owned()), before, "no page tab opened");
        assert!(view.read_with(cx, |view, _| view.row_tabs.get(JOB).is_none_or(|tabs| tabs.pages.is_empty())), "no tab holds the page");
        view.update_in(cx, |view, window, cx| view.close_review(window, cx));
        redraw(cx);
        assert!(view.read_with(cx, |view, cx| view.panel_page(JOB, PAGE, 2, cx).is_none()), "the page goes with the panel");
    }

    #[gpui::test]
    fn a_plain_page_opened_from_the_artifacts_tab_is_the_live_page_filling_the_panels_detail_and_no_tab_opens(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "reader", "draft-job", size(px(1400.), px(900.)), |_| {});
        let page = view
            .read_with(cx, |view, _| view.artifacts_of("draft-job").iter().find(|one| one.form == ipc::ArtifactForm::Page && one.parts.is_empty()).map(|one| one.id.clone()))
            .expect("the Job holds a page with no parts");
        let before = view.read_with(cx, |view, _| view.arrangement().into_owned());
        view.update_in(cx, |view, window, cx| assert!(view.open_named(&page, window, cx)));
        redraw(cx);
        let chosen = view.read_with(cx, |view, _| view.review_chosen()).expect("the panel is up with the page picked");
        assert_eq!((chosen.kind, super::page_of_item(&chosen.id)), (Kind::Document, Some(page.as_str())));
        let revision: u32 = chosen.revision.as_deref().and_then(|word| word.trim_start_matches('r').parse().ok()).expect("its revision");
        assert!(view.read_with(cx, |view, cx| view.panel_page("draft-job", &page, revision, cx).is_some()), "it holds the page's live view");
        let live = bounds_of(cx, Role::Region, "Page");
        // Opened, the page is read, so it has no answer bar: it runs down to the detail's own bottom.
        assert!(!drawn(cx, Role::Toolbar, "Answer"), "a page he read asks nothing");
        let (head, detail) = (bounds_of(cx, Role::Heading, &chosen.name), bounds_of(cx, Role::Region, &chosen.name));
        assert!(live.top() - head.bottom() <= px(2.) && detail.bottom() - live.bottom() <= px(2.), "alone in the detail, it takes all of it under the head: {head:?} {live:?} {detail:?}");
        assert_eq!(view.read_with(cx, |view, _| view.arrangement().into_owned()), before, "no page tab opened");
        assert!(view.read_with(cx, |view, _| view.row_tabs.get("draft-job").is_none_or(|tabs| tabs.pages.is_empty())), "no tab holds the page");
        assert!(view.read_with(cx, |view, _| view.mock_seen_posts().iter().any(|one| one.artifact == page && one.revision == revision)), "Fleet is told he opened it, as the reader tells it");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "Esc closes the panel");
    }

    #[gpui::test]
    fn a_page_that_is_an_artifacts_only_part_takes_the_whole_detail_and_one_among_parts_keeps_its_share(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened_drawings(cx);
        let panel = view.read_with(cx, |view, _| view.sheet.get().height);
        assert!(bounds_of(cx, Role::Region, "Page").size.height <= panel * TALL_SHARE + px(1.), "among other parts it stands at its share");
        view.update(cx, |view, cx| view.mock_parts_tuned(ID, |artifact| artifact.parts.retain(|part| part.id == PAGE), cx));
        redraw(cx);
        let name = view.read_with(cx, |view, _| view.review_chosen().map(|item| item.name)).expect("the artifact is chosen");
        let (page, head) = (bounds_of(cx, Role::Region, "Page"), bounds_of(cx, Role::Heading, &name));
        assert!(page.top() - head.bottom() <= px(2.), "alone, it starts under the head: {head:?} {page:?}");
        assert!(page.size.height > panel * TALL_SHARE, "and takes more than a picture's share of the panel: {page:?}");
    }

    #[gpui::test]
    fn a_vote_and_a_press_the_panels_page_posts_are_carried_to_fleet_and_its_answer_goes_back(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened_drawings(cx);
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
        let (view, cx) = opened_drawings(cx);
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
}
