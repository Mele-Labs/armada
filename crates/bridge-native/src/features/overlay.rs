//! The overlay a feature's page and a Job's Story open in, over the tab they were opened from. Every artifact review opens in
//! one full-width overlay, and what is opened from inside it **replaces its content**: the overlay holds a stack of what was
//! opened and draws the top, so there is never a second layer. One Esc goes back, and the last closes the overlay.
//!
//! A feature page opened from a Story tab is the first layer. A page's Story link puts that Job's Story on the stack, and a Story's
//! Filed under row puts a page on it, as many times as he follows them; Esc undoes one at a time.

use gpui::{div, prelude::*, Context, Entity, EventEmitter, FocusHandle, Focusable, KeyDownEvent, Render, Role, SharedString, Subscription, Window};
use ipc::SessionId;

use super::page::{FeaturePageEvent, FeaturePageView};
use crate::fleet::FleetConnection;
use crate::job_view::story::{StoryEvent, StoryTab};
use crate::labels::labelled;
use crate::look::{token, ActiveLook};
use crate::settings::view::{ghost_with, small};
use crate::titlebar::parts::sans;

/// Where the overlay reads from.
#[derive(Clone)]
pub enum Source {
    Live(Entity<FleetConnection>),
    Mock,
}

/// What a layer holds.
pub enum Content {
    Page(Entity<FeaturePageView>),
    Story(Entity<StoryTab>),
}

/// What the overlay tells whoever opened it.
pub enum OverlayEvent {
    /// The last layer went back: the overlay is done.
    Closed,
}

struct Layer {
    /// What this layer is, for its title line.
    title: String,
    /// Where it was opened from, which Back goes to.
    from: String,
    content: Content,
    _sub: Subscription,
}

pub struct FeatureOverlay {
    source: Source,
    project: String,
    stack: Vec<Layer>,
    /// The keys when a layer has none to take them, as after a press on Back.
    focus: FocusHandle,
}

impl EventEmitter<OverlayEvent> for FeatureOverlay {}

impl Focusable for FeatureOverlay {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl FeatureOverlay {
    /// Open on the page of feature `slug`, from the place `from` names.
    pub fn on_page(source: Source, project: String, from: String, slug: String, title: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = FeatureOverlay { source, project, stack: Vec::new(), focus: cx.focus_handle() };
        this.push_page_from(slug, title, from, window, cx);
        this
    }

    /// How many layers are stacked; the top one is the one drawn.
    #[cfg(test)]
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    #[cfg(test)]
    pub fn top(&self) -> Option<&Content> {
        self.stack.last().map(|layer| &layer.content)
    }

    /// The title line of the top layer, and where it was opened from.
    #[cfg(test)]
    pub fn titles(&self) -> Option<(&str, &str)> {
        self.stack.last().map(|layer| (layer.title.as_str(), layer.from.as_str()))
    }

    fn here(&self) -> String {
        self.stack.last().map(|layer| layer.title.clone()).unwrap_or_default()
    }

    /// Put the page of feature `slug` on top.
    pub fn push_page(&mut self, slug: String, title: String, window: &mut Window, cx: &mut Context<Self>) {
        let from = self.here();
        self.push_page_from(slug, title, from, window, cx);
    }

    fn push_page_from(&mut self, slug: String, title: String, from: String, window: &mut Window, cx: &mut Context<Self>) {
        let project = self.project.clone();
        let page = match &self.source {
            Source::Live(fleet) => cx.new(|cx| FeaturePageView::live(fleet, project, slug, window, cx)),
            Source::Mock => cx.new(|cx| FeaturePageView::mock(slug, window, cx)),
        };
        // Opened on purpose, from a Filed under row or a note, so it counts as looked at once it is left.
        page.update(cx, |page, _| page.engage());
        let sub = cx.subscribe_in(&page, window, |this, _, event: &FeaturePageEvent, window, cx| match event {
            FeaturePageEvent::Escape => this.back(window, cx),
            FeaturePageEvent::Story { session_id, ask_id } => this.push_story(session_id.clone(), ask_id.clone(), window, cx),
            FeaturePageEvent::Design { job, artifact } => super::artifact_link::open(job.clone(), artifact.clone(), cx),
            // The overlay holds no list of the library.
            FeaturePageEvent::Retired => {}
        });
        let focus = page.read(cx).within.clone();
        self.stack.push(Layer { title, from, content: Content::Page(page), _sub: sub });
        window.focus(&focus, cx);
        if let Some(Layer { content: Content::Page(page), .. }) = self.stack.last() {
            let rail = page.read(cx).focus_handle(cx);
            window.focus(&rail, cx);
        }
        cx.notify();
    }

    /// Put the Story of Job `job` on top, at the ask `ask_id`. It replaces what is drawn: the page below stays on the stack.
    pub fn push_story(&mut self, job: SessionId, ask_id: String, window: &mut Window, cx: &mut Context<Self>) {
        let from = self.here();
        let name = match &self.source {
            Source::Live(fleet) => fleet.read(cx).held().jobs.iter().find(|one| one.id == job).and_then(|one| one.title.clone()),
            Source::Mock => crate::job_view::story::mock::titles().get(job.as_str()).cloned(),
        };
        let title = format!("Story of {}", name.unwrap_or_else(|| "this Job".into()));
        let id = job.as_str().to_string();
        let story = match &self.source {
            Source::Live(fleet) => cx.new(|cx| StoryTab::live(fleet, id, true, window, cx)),
            Source::Mock => cx.new(|cx| StoryTab::mock(id, true, window, cx)),
        };
        story.update(cx, |story, cx| story.put_ask(ask_id, cx));
        let sub = cx.subscribe_in(&story, window, |this, _, event: &StoryEvent, window, cx| match event {
            StoryEvent::Escape => this.back(window, cx),
            StoryEvent::Feature { slug, title } => this.push_page(slug.clone(), title.clone(), window, cx),
            // A Job's pull requests and Jobs open from its own view, which this overlay is not in.
            StoryEvent::ReadPull { .. } | StoryEvent::OpenPull { .. } | StoryEvent::OpenJob(_) => {}
        });
        let focus = story.read(cx).focus_handle(cx);
        self.stack.push(Layer { title, from, content: Content::Story(story), _sub: sub });
        window.focus(&focus, cx);
        cx.notify();
    }

    /// Esc, or Back: the top layer goes, and the one under it is drawn again. The last layer going closes the overlay.
    pub fn back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(gone) = self.stack.pop() else { return cx.emit(OverlayEvent::Closed) };
        // A page put away is looked at: that is when it is stamped.
        if let Content::Page(page) = &gone.content {
            page.update(cx, |page, cx| page.leave(cx));
        }
        if self.stack.is_empty() {
            cx.emit(OverlayEvent::Closed);
        } else {
            self.focus_top(window, cx);
        }
        cx.notify();
    }

    /// Every layer put away at once, each page stamped as looked at, with no event: for whoever holds the overlay and is
    /// about to drop it.
    pub fn put_away(&mut self, cx: &mut Context<Self>) {
        for gone in self.stack.drain(..) {
            if let Content::Page(page) = &gone.content {
                page.update(cx, |page, cx| page.leave(cx));
            }
        }
    }

    /// The keys go to the top layer, as they do after a layer goes back.
    pub fn focus_top(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.stack.last() {
            Some(Layer { content: Content::Page(page), .. }) => {
                let rail = page.read(cx).focus_handle(cx);
                window.focus(&rail, cx);
            }
            Some(Layer { content: Content::Story(story), .. }) => {
                let focus = story.read(cx).focus_handle(cx);
                window.focus(&focus, cx);
            }
            None => {}
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" {
            self.back(window, cx);
            cx.stop_propagation();
        }
    }
}

impl Render for FeatureOverlay {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let top = self.stack.last();
        let (title, from) = top.map(|layer| (layer.title.clone(), layer.from.clone())).unwrap_or_default();
        let body = top.map(|layer| match &layer.content {
            Content::Page(page) => div().size_full().child(page.clone()).into_any_element(),
            Content::Story(story) => div().size_full().child(story.clone()).into_any_element(),
        });
        let story = matches!(top.map(|layer| &layer.content), Some(Content::Story(_)));
        let back = labelled(ghost_with(&look, "overlay-back", Some("arrow-left"), "Back", false).on_click(cx.listener(|this, _, window, cx| this.back(window, cx))), Role::Button, "Back");
        let strip = div()
            .id("overlay-strip")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_SUNKEN))
            .child(back)
            // A page names itself in its own top strip; a Story is named here.
            .children(story.then(|| {
                labelled(
                    div().id("overlay-title").text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(title.clone())),
                    Role::Heading,
                    title.clone(),
                )
            }))
            .child(small(&look, if from.is_empty() { String::new() } else { format!("From {from}") }, look.color(token::FG_SUBTLE)));
        div()
            .id("feature-overlay")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(strip)
            .child(div().flex_1().min_h_0().children(body))
    }
}
