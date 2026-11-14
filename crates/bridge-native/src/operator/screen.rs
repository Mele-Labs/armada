//! The window's two places, Operator and Jobs, in the title bar's strip. It mounts the Operator's page
//! beside the Jobs screen the way the dispatch screen mounts its sheet: the bar's Operator button, ⌘J and
//! the Operator place bring the page to the front, and the rest of what the bar asks goes on to the Jobs
//! screen's sink.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyWindowHandle, App, Context, Entity, FocusHandle, Global, SharedString, WeakEntity, Window};

use super::page::{Opened, OperatorPageView};
use crate::fleet::FleetConnection;
use crate::jobs::JobsScreen;
use crate::log::Note;
use crate::look::{token, ActiveLook};
use crate::titlebar::scenes::Opening;
use crate::titlebar::{self, Asked, AskedSink, Face, Place};

/// Which place is in front.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Front {
    Operator,
    Jobs,
    /// The pull requests waiting on him, where a screen holds them.
    Reviews,
}

impl Front {
    fn id(self) -> &'static str {
        match self {
            Front::Operator => "operator",
            Front::Jobs => "jobs",
            Front::Reviews => "reviews",
        }
    }
}

pub struct OperatorScreen {
    jobs: Entity<JobsScreen>,
    pub page: Entity<OperatorPageView>,
    pub front: Front,
    pub within: FocusHandle,
    reviews: Option<Entity<crate::reviews::ReviewsScreen>>,
    toasts: Entity<crate::toasts::Toasts>,
    _subs: Vec<gpui::Subscription>,
}

/// The page in front, for a walk to move between scenes and for Jobs screens to hear what arrives.
struct Live(WeakEntity<OperatorPageView>);

impl Global for Live {}

pub fn live(cx: &App) -> Option<Entity<OperatorPageView>> {
    cx.try_global::<Live>().and_then(|live| live.0.upgrade())
}

/// The strip's places now that the Operator has a screen: Operator first, as Electron's `DECK_TABS` has them.
fn with_places(mut face: Face, here: Front, reviews: bool) -> Face {
    if !face.places.iter().any(|place| place.id == "operator") {
        face.places.insert(0, Place { id: "operator", label: "Operator", glyph: "bot", shown: true, hideable: true });
    }
    if !face.places.iter().any(|place| place.id == "jobs") {
        face.places.push(Place { id: "jobs", label: "Jobs", glyph: "box", shown: true, hideable: true });
    }
    if reviews && !face.places.iter().any(|place| place.id == "reviews") {
        face.places.push(Place { id: "reviews", label: "Reviews", glyph: "git-pull-request", shown: true, hideable: true });
    }
    face.here = Some(here.id());
    face
}

impl OperatorScreen {
    pub fn live(jobs: Entity<JobsScreen>, fleet: &Entity<FleetConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = cx.new(|cx| OperatorPageView::live(fleet, false, window, cx));
        Self::new(jobs, page, Front::Jobs, None, window, cx)
    }

    /// On the walk's Fleet, with the page in front.
    pub fn mock(jobs: Entity<JobsScreen>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = cx.new(|cx| OperatorPageView::mock(false, window, cx));
        Self::new(jobs, page, Front::Operator, None, window, cx)
    }

    /// On the walk's Fleet, with the Jobs screen in front.
    pub fn mock_behind(jobs: Entity<JobsScreen>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = cx.new(|cx| OperatorPageView::mock(false, window, cx));
        Self::new(jobs, page, Front::Jobs, None, window, cx)
    }

    /// On the walk's Fleet, with the Jobs screen in front and Reviews in the strip, on its own mock.
    pub fn mock_reviews(jobs: Entity<JobsScreen>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = cx.new(|cx| OperatorPageView::mock(false, window, cx));
        let reviews = cx.new(|cx| crate::reviews::ReviewsScreen::mock(window, cx));
        Self::new(jobs, page, Front::Jobs, Some(reviews), window, cx)
    }

    fn new(
        jobs: Entity<JobsScreen>,
        page: Entity<OperatorPageView>,
        front: Front,
        reviews: Option<Entity<crate::reviews::ReviewsScreen>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.set_global(Live(page.downgrade()));
        let list = jobs.read(cx).list.clone();
        let subs = vec![
            cx.subscribe_in(&page, window, |this, _, opened: &Opened, window, cx| this.opened(opened, window, cx)),
            // The Jobs the box's `@` names are the ones the list holds.
            cx.observe(&list, |this, _, cx| this.name_jobs(cx)),
        ];
        let handle = window.window_handle();
        let me = cx.entity().downgrade();
        let behind = cx.try_global::<AskedSink>().cloned();
        cx.set_global(AskedSink(Rc::new(move |asked: &Asked, cx: &mut App| {
            let to = match asked {
                Asked::Operator => Some(None),
                Asked::Place("operator") => Some(Some(Front::Operator)),
                Asked::Place("jobs") => Some(Some(Front::Jobs)),
                Asked::Place("reviews") => Some(Some(Front::Reviews)),
                _ => None,
            };
            match to {
                Some(to) => go(&me, handle, to, cx),
                None => {
                    if let Some(behind) = &behind {
                        (behind.0)(asked, cx);
                    }
                }
            }
        })));
        let toasts = crate::toasts::Toasts::mount(&page, cx.entity().downgrade(), window, cx);
        let mut this = OperatorScreen { jobs, page, front, within: cx.focus_handle(), reviews, toasts, _subs: subs };
        this.name_jobs(cx);
        this.show_places(cx);
        if front == Front::Operator {
            let keys = this.page.read(cx).focus.clone();
            window.focus(&keys, cx);
        }
        this
    }

    /// The page's box offers the Jobs the list holds, but those that have ended.
    fn name_jobs(&mut self, cx: &mut Context<Self>) {
        let list = self.jobs.read(cx).list.clone();
        let named: Vec<(String, String)> = list
            .read(cx)
            .records()
            .iter()
            .filter(|job| job.state != ipc::SessionState::Ended)
            .filter_map(|job| job.title.clone().map(|title| (job.id.as_str().to_string(), title)))
            .collect();
        self.page.update(cx, |page, cx| page.set_jobs(named, cx));
    }

    /// The bar's strip says which place is in front: the bar there now, and the one the next screen opens.
    fn show_places(&mut self, cx: &mut Context<Self>) {
        let held = titlebar::live(cx).map(|bar| bar.read(cx).face.clone());
        let face = with_places(held.or_else(|| cx.try_global::<Opening>().map(|opening| opening.0.clone())).unwrap_or_else(Face::bare), self.front, self.reviews.is_some());
        cx.set_global(Opening(face.clone()));
        if let Some(bar) = titlebar::live(cx) {
            bar.update(cx, |bar, cx| {
                bar.face.places = face.places.clone();
                bar.face.here = face.here;
                cx.notify();
            });
        }
    }

    /// Bring a place to the front; `None` is the Operator's own key, which goes back where it came from.
    pub fn bring(&mut self, to: Option<Front>, window: &mut Window, cx: &mut Context<Self>) {
        let to = to.unwrap_or(if self.front == Front::Operator { Front::Jobs } else { Front::Operator });
        if to == self.front || (to == Front::Reviews && self.reviews.is_none()) {
            return;
        }
        if self.front == Front::Reviews {
            if let Some(reviews) = &self.reviews {
                reviews.update(cx, |reviews, cx| reviews.leave(cx));
            }
        }
        self.front = to;
        self.show_places(cx);
        match to {
            Front::Operator => {
                let keys = self.page.read(cx).focus.clone();
                window.focus(&keys, cx);
                self.page.update(cx, |page, cx| page.in_front(cx));
            }
            Front::Jobs => {
                let keys = self.jobs.read(cx).list.read(cx).focus_handle().clone();
                window.focus(&keys, cx);
            }
            Front::Reviews => {
                if let Some(reviews) = &self.reviews {
                    reviews.update(cx, |reviews, cx| reviews.enter(window, cx));
                }
            }
        }
        cx.notify();
    }

    /// A toast's press: the Jobs screen in front, the Job open on it.
    pub fn open_job(&mut self, job: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.bring(Some(Front::Jobs), window, cx);
        let list = self.jobs.read(cx).list.clone();
        list.update(cx, |list, cx| {
            list.open_job(job, cx);
        });
    }

    /// What the page asks: a Job opens on the Jobs screen.
    fn opened(&mut self, opened: &Opened, window: &mut Window, cx: &mut Context<Self>) {
        let job = match opened {
            Opened::Job(id) => Some(id.clone()),
            Opened::Act(act, session) => {
                Note::info("an item's act was left to the Job that holds it").field("act", format!("{:?}", act.kind)).field("target", act.target.as_str()).emit();
                session.clone()
            }
        };
        let Some(job) = job else { return };
        self.bring(Some(Front::Jobs), window, cx);
        let list = self.jobs.read(cx).list.clone();
        list.update(cx, |list, cx| {
            list.open_job(job.as_str(), cx);
        });
    }
}

/// A press on the banner of a call no Job holds, such as another Mac waiting to pair: the Operator's page in
/// front with that call's card first and the keys on its Confirm. Never opens a Job.
pub fn open_call(key: String, cx: &mut App) {
    if let Some(sink) = cx.try_global::<AskedSink>().cloned() {
        (sink.0)(&Asked::Place("operator"), cx);
    }
    if let Some(page) = live(cx) {
        page.update(cx, |page, cx| page.show_pairing(&key, cx));
    }
}

fn go(me: &WeakEntity<OperatorScreen>, handle: AnyWindowHandle, to: Option<Front>, cx: &mut App) {
    let me = me.clone();
    cx.defer(move |cx| {
        let _ = handle.update(cx, |_, window, cx| {
            if let Some(me) = me.upgrade() {
                me.update(cx, |this, cx| this.bring(to, window, cx));
            }
        });
    });
}

impl Render for OperatorScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shown = match self.front {
            Front::Jobs => self.jobs.clone().into_any_element(),
            Front::Operator => {
                let title = crate::chrome::title_row(window, cx);
                div().size_full().flex().flex_col().bg(cx.look().color(token::BG_BASE)).child(title).child(div().flex_1().min_h_0().child(self.page.clone())).into_any_element()
            }
            Front::Reviews => {
                let title = crate::chrome::title_row(window, cx);
                let reviews = self.reviews.clone().map(|reviews| reviews.into_any_element());
                div().size_full().flex().flex_col().bg(cx.look().color(token::BG_BASE)).child(title).child(div().flex_1().min_h_0().children(reviews)).into_any_element()
            }
        };
        // ⌘K and the bar's search open the Jobs screen's palette over whichever place is in front; the Jobs screen
        // draws it itself.
        let palette = (self.front != Front::Jobs).then(|| self.jobs.read(cx).palette().clone());
        div().id(SharedString::from("operator-screen")).track_focus(&self.within).size_full().child(shown).child(self.toasts.clone()).children(palette)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Role, TestAppContext, VisualTestContext};

    use super::*;
    use crate::jobs::JobsList;
    use crate::labels::LabelBook;

    fn ready(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
            crate::titlebar::bind(cx);
        });
    }

    fn jobs_screen(window: &mut Window, cx: &mut App) -> Entity<JobsScreen> {
        let records = crate::mock::jobs("jobs-many").expect("the fixture decodes");
        let list = cx.new(|cx| JobsList::mock(&records, cx));
        cx.new(|cx| JobsScreen::new(list, None, window, cx))
    }

    /// Whether the palette's box was drawn in this window.
    fn palette_drawn(cx: &mut VisualTestContext) -> bool {
        cx.update(|window, cx| cx.default_global::<LabelBook>().find(window.window_handle().window_id(), Role::Dialog, "Command palette").is_some())
    }

    #[gpui::test]
    fn the_palette_opens_over_the_operator_page_and_esc_gives_the_page_its_keys_back(cx: &mut TestAppContext) {
        ready(cx);
        let (screen, cx) = cx.add_window_view(|window, cx| {
            let jobs = jobs_screen(window, cx);
            OperatorScreen::mock(jobs, window, cx)
        });
        cx.run_until_parked();
        let (page_keys, palette) = screen.read_with(cx, |screen, cx| (screen.page.read(cx).focus.clone(), screen.jobs.read(cx).palette().clone()));
        assert!(!palette_drawn(cx));
        cx.simulate_keystrokes("cmd-k");
        cx.run_until_parked();
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Operator, "the page stays in front");
        assert!(palette.read_with(cx, |palette, _| palette.is_open()));
        assert!(palette_drawn(cx), "the palette is drawn over the page");
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        assert!(!palette.read_with(cx, |palette, _| palette.is_open()));
        assert!(cx.update(|window, _| page_keys.is_focused(window)), "the keys go back to the page");
    }

    #[gpui::test]
    fn the_palette_opens_over_settings_and_esc_shuts_the_palette_alone(cx: &mut TestAppContext) {
        ready(cx);
        let (screen, cx) = cx.add_window_view(|window, cx| {
            let jobs = jobs_screen(window, cx);
            let held = cx.new(|cx| OperatorScreen::mock(jobs, window, cx));
            crate::settings::SettingsScreen::mock(held.into(), window, cx)
        });
        cx.run_until_parked();
        let settings = screen.read_with(cx, |screen, _| screen.overlay.clone());
        cx.simulate_keystrokes("cmd-,");
        cx.run_until_parked();
        assert!(settings.read_with(cx, |settings, _| settings.open), "⌘, opens Settings");
        let keys = cx.update(|window, cx| window.focused(cx)).expect("Settings holds the keys");
        cx.simulate_keystrokes("cmd-k");
        cx.run_until_parked();
        assert!(cx.update(|_, cx| crate::palette::is_up(cx)), "⌘K opens the palette over Settings");
        assert!(palette_drawn(cx), "the palette is drawn over Settings");
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        assert!(!cx.update(|_, cx| crate::palette::is_up(cx)));
        assert!(settings.read_with(cx, |settings, _| settings.open), "Esc on the palette leaves Settings open");
        assert!(cx.update(|window, _| keys.is_focused(window)), "the keys go back into Settings");
    }
}
