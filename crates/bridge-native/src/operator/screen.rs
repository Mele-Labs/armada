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
    /// The owner's Claude Code setup, kind by kind.
    Kit,
    /// The project's feature library, on the project the bar is scoped to.
    Features,
    /// Where the burn was wasted. No place in the strip: it is reached by name from the palette.
    Usage,
}

impl Front {
    fn id(self) -> &'static str {
        match self {
            Front::Operator => "operator",
            Front::Jobs => "jobs",
            Front::Reviews => "reviews",
            Front::Kit => "kit",
            Front::Features => "features",
            Front::Usage => "usage",
        }
    }
}

pub struct OperatorScreen {
    jobs: Entity<JobsScreen>,
    pub page: Entity<OperatorPageView>,
    pub front: Front,
    pub within: FocusHandle,
    reviews: Option<Entity<crate::reviews::ReviewsScreen>>,
    kit: Option<Entity<crate::kit::KitScreen>>,
    features: Option<Entity<crate::features::FeaturesScreen>>,
    usage: Option<Entity<crate::usage_page::UsageScreen>>,
    toasts: Entity<crate::toasts::Toasts>,
    /// The Fleet whose repositories a pasted pull request link is read against; the walks' own where none.
    fleet: Option<Entity<FleetConnection>>,
    _subs: Vec<gpui::Subscription>,
}

/// The page in front, for a walk to move between scenes and for Jobs screens to hear what arrives.
struct Live(WeakEntity<OperatorPageView>);

impl Global for Live {}

/// The screen and its window, for a press from outside them: a banner's.
struct Here(WeakEntity<OperatorScreen>, AnyWindowHandle);

impl Global for Here {}

pub fn live(cx: &App) -> Option<Entity<OperatorPageView>> {
    cx.try_global::<Live>().and_then(|live| live.0.upgrade())
}

/// The strip's places now that the Operator has a screen: Operator first, as Electron's `DECK_TABS` has them.
fn with_places(mut face: Face, here: Front, reviews: bool, kit: bool, features: bool) -> Face {
    if !face.places.iter().any(|place| place.id == "operator") {
        face.places.insert(0, Place { id: "operator", label: "Operator", glyph: "bot", shown: true, hideable: true });
    }
    if !face.places.iter().any(|place| place.id == "jobs") {
        face.places.push(Place { id: "jobs", label: "Jobs", glyph: "box", shown: true, hideable: true });
    }
    if reviews && !face.places.iter().any(|place| place.id == "reviews") {
        face.places.push(Place { id: "reviews", label: "Reviews", glyph: "git-pull-request", shown: true, hideable: true });
    }
    if kit && !face.places.iter().any(|place| place.id == "kit") {
        face.places.push(Place { id: "kit", label: "Kit", glyph: "briefcase", shown: true, hideable: true });
    }
    // Issues has no native screen yet, so Features follows the last place there is.
    if features && !face.places.iter().any(|place| place.id == "features") {
        face.places.push(Place { id: "features", label: "Features", glyph: "library", shown: true, hideable: true });
    }
    // Settings > Layout's choice for a place that has a row there, read each time the places are drawn.
    for place in face.places.iter_mut().filter(|place| matches!(place.id, "kit" | "features")) {
        place.shown = !crate::settings::layout::hidden("rail", place.id);
    }
    face.here = Some(here.id());
    face
}

impl OperatorScreen {
    pub fn live(jobs: Entity<JobsScreen>, fleet: &Entity<FleetConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = cx.new(|cx| OperatorPageView::live(fleet, window, cx));
        let reviews = cx.new(|cx| crate::reviews::ReviewsScreen::live(fleet, window, cx));
        let kit = cx.new(|cx| crate::kit::KitScreen::live(fleet, window, cx));
        let features = cx.new(|cx| crate::features::FeaturesScreen::live(fleet, None, window, cx));
        let mut this = Self::new(jobs, page, Front::Jobs, Some(reviews), Some(kit), Some(features), window, cx);
        this.fleet = Some(fleet.clone());
        // A link that launched the app is held for this screen, and decided as Fleet's answers arrive.
        this._subs.push(cx.observe(fleet, |_, _, cx| crate::deep_link::settle(cx)));
        cx.defer(crate::deep_link::settle);
        // A layout saved in Settings arrives with Fleet's answer: the strip draws again where it hides or shows a place.
        let layout = cx.observe(fleet, |this, _, cx| this.show_places(cx));
        this._subs.push(layout);
        this
    }

    /// On the walk's Fleet, with the page in front.
    pub fn mock(jobs: Entity<JobsScreen>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = cx.new(|cx| OperatorPageView::mock(window, cx));
        Self::new(jobs, page, Front::Operator, None, None, None, window, cx)
    }

    /// On the walk's Fleet, with the Jobs screen in front.
    pub fn mock_behind(jobs: Entity<JobsScreen>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = cx.new(|cx| OperatorPageView::mock(window, cx));
        Self::new(jobs, page, Front::Jobs, None, None, None, window, cx)
    }

    /// On the walk's Fleet, with the Jobs screen in front and Reviews in the strip, on its own mock,
    /// whose work lands on the walk's `Lands` steps.
    pub fn mock_reviews(jobs: Entity<JobsScreen>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Made first: the mocks after it name scenarios of their own, and this one reads the scenario the walk named.
        let usage = cx.new(crate::usage_page::UsageScreen::mock);
        let page = cx.new(|cx| OperatorPageView::mock(window, cx));
        let reviews = cx.new(|cx| {
            let mut reviews = crate::reviews::ReviewsScreen::mock(window, cx);
            reviews.walk(cx);
            reviews
        });
        let kit = cx.new(|cx| crate::kit::KitScreen::mock(window, cx));
        let features = cx.new(|cx| crate::features::FeaturesScreen::mock(window, cx));
        let mut this = Self::new(jobs, page, Front::Jobs, Some(reviews), Some(kit), Some(features), window, cx);
        this.usage = Some(usage);
        this.name_jobs(cx);
        this
    }

    /// As [`Self::mock_reviews`], with the Usage page in front: `--mock usage-week` and its kin.
    pub fn mock_usage(jobs: Entity<JobsScreen>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::mock_reviews(jobs, window, cx);
        this.bring(Some(Front::Usage), window, cx);
        this
    }

    fn new(
        jobs: Entity<JobsScreen>,
        page: Entity<OperatorPageView>,
        front: Front,
        reviews: Option<Entity<crate::reviews::ReviewsScreen>>,
        kit: Option<Entity<crate::kit::KitScreen>>,
        features: Option<Entity<crate::features::FeaturesScreen>>,
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
        cx.set_global(Here(me.clone(), handle));
        let behind = cx.try_global::<AskedSink>().cloned();
        let drawn = reviews.as_ref().map(|reviews| reviews.downgrade());
        cx.set_global(AskedSink(Rc::new(move |asked: &Asked, cx: &mut App| {
            // A project or a repository picked: the inbox draws only its pull requests, so it draws again once the bar has taken the pick.
            if matches!(asked, Asked::Project(_) | Asked::Repository(_)) {
                if let Some(reviews) = drawn.clone() {
                    cx.defer(move |cx| {
                        if let Some(reviews) = reviews.upgrade() {
                            reviews.update(cx, |_, cx| cx.notify());
                        }
                    });
                }
            }
            // The palette's Kit: a kind brings Kit to the front on that kind.
            if let Asked::KitAt(kind) = asked {
                let (me, kind) = (me.clone(), *kind);
                cx.defer(move |cx| {
                    let _ = handle.update(cx, |_, window, cx| {
                        if let Some(me) = me.upgrade() {
                            me.update(cx, |this, cx| this.open_kit(Some(kind), window, cx));
                        }
                    });
                });
                return;
            }
            let to = match asked {
                Asked::Operator => Some(None),
                Asked::Place("operator") => Some(Some(Front::Operator)),
                Asked::Place("jobs") => Some(Some(Front::Jobs)),
                Asked::Place("reviews") => Some(Some(Front::Reviews)),
                Asked::Place("kit") => Some(Some(Front::Kit)),
                Asked::Place("features") => Some(Some(Front::Features)),
                Asked::Place("usage") => Some(Some(Front::Usage)),
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
        let mut subs = subs;
        // ⌘V on a pull request's link, outside a text box, opens it in Reviews.
        let here = window.window_handle().window_id();
        let pasted_into = cx.entity().downgrade();
        subs.push(cx.intercept_keystrokes(move |event, window, cx| {
            let key = &event.keystroke;
            if key.key != "v" || !key.modifiers.platform || key.modifiers.alt || key.modifiers.shift || window.window_handle().window_id() != here {
                return;
            }
            if window.context_stack().iter().any(|context| context.contains(crate::keymap::TEXT_BOX)) {
                return;
            }
            let Some(me) = pasted_into.upgrade() else { return };
            let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else { return };
            let asked = match &me.read(cx).fleet {
                Some(fleet) => crate::paste_link::asked_by(&text, fleet.read(cx).held().repositories.as_deref().unwrap_or_default()),
                None => crate::paste_link::asked_by(&text, &crate::mock::repositories()),
            };
            if asked.is_some() {
                open_asked(asked, cx);
                cx.stop_propagation();
            }
        }));
        let mut this = OperatorScreen { jobs, page, front, within: cx.focus_handle(), reviews, kit, features, usage: None, toasts, fleet: None, _subs: subs };
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
        // A card's chips name every Job it is about, ended ones too.
        let titles: std::collections::HashMap<String, String> = list.read(cx).records().iter().filter_map(|job| job.title.clone().map(|title| (job.id.as_str().to_string(), title))).collect();
        if let Some(usage) = &self.usage {
            usage.update(cx, |usage, cx| usage.set_titles(&titles, cx));
        }
        self.page.update(cx, |page, cx| {
            page.set_jobs(named, cx);
            page.set_titles(titles, cx);
        });
    }

    /// The bar's strip says which place is in front: the bar there now, and the one the next screen opens.
    fn show_places(&mut self, cx: &mut Context<Self>) {
        let held = titlebar::live(cx).map(|bar| bar.read(cx).face.clone());
        let face = with_places(held.or_else(|| cx.try_global::<Opening>().map(|opening| opening.0.clone())).unwrap_or_else(Face::bare), self.front, self.reviews.is_some(), self.kit.is_some(), self.features.is_some());
        cx.set_global(Opening(face.clone()));
        if let Some(bar) = titlebar::live(cx) {
            // Fleet's events come often: the bar draws again only when its places or the one in front changed.
            if bar.read(cx).face.places == face.places && bar.read(cx).face.here == face.here {
                return;
            }
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
        if to == self.front || (to == Front::Reviews && self.reviews.is_none()) || (to == Front::Kit && self.kit.is_none()) || (to == Front::Features && self.features.is_none()) || (to == Front::Usage && self.usage.is_none()) {
            return;
        }
        if self.front == Front::Reviews {
            if let Some(reviews) = &self.reviews {
                reviews.update(cx, |reviews, cx| reviews.leave(cx));
            }
        }
        if self.front == Front::Kit {
            if let Some(kit) = &self.kit {
                kit.update(cx, |kit, cx| kit.leave(cx));
            }
        }
        if self.front == Front::Features {
            if let Some(features) = &self.features {
                features.update(cx, |features, cx| features.leave(cx));
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
            Front::Kit => {
                if let Some(kit) = &self.kit {
                    kit.update(cx, |kit, cx| kit.enter(window, cx));
                }
            }
            Front::Features => {
                if let Some(features) = &self.features {
                    features.update(cx, |features, cx| features.enter(window, cx));
                }
            }
            Front::Usage => {
                if let Some(usage) = &self.usage {
                    usage.update(cx, |usage, cx| usage.enter(window, cx));
                }
            }
        }
        cx.notify();
    }

    /// The Usage page this screen holds.
    #[cfg(test)]
    pub fn usage_for_test(&self) -> Option<Entity<crate::usage_page::UsageScreen>> {
        self.usage.clone()
    }

    /// Kit in front, on a kind of its own when one is named.
    pub fn open_kit(&mut self, kind: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        self.bring(Some(Front::Kit), window, cx);
        if let (Some(kind), Some(kit)) = (kind, &self.kit) {
            kit.update(cx, |kit, cx| kit.show_kind(kind, window, cx));
        }
    }

    /// A toast's press: the Jobs screen in front, the Job open on it.
    pub fn open_job(&mut self, job: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.bring(Some(Front::Jobs), window, cx);
        let list = self.jobs.read(cx).list.clone();
        list.update(cx, |list, cx| {
            list.open_job(job, cx);
        });
    }

    /// A press on a review asked of him, on its toast or its banner: Reviews in front, that pull request open. A
    /// request no longer standing brings Reviews alone.
    pub fn open_review(&mut self, asked: Option<&ipc::ReviewAsked>, window: &mut Window, cx: &mut Context<Self>) {
        self.bring(Some(Front::Reviews), window, cx);
        if let (Some(asked), Some(reviews)) = (asked, &self.reviews) {
            reviews.update(cx, |reviews, cx| reviews.open_asked(asked, window, cx));
        }
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

/// What a link is decided with: whether `features.deepLinks` is on and the repositories Fleet serves, each `None`
/// until Fleet has said. `None` where the screen is not up yet, or has no Fleet to read.
pub fn link_state(cx: &App) -> Option<(Option<bool>, Option<Vec<ipc::RepositorySummary>>)> {
    let screen = cx.try_global::<Here>()?.0.upgrade()?;
    let fleet = screen.read(cx).fleet.clone()?;
    let held = fleet.read(cx).held();
    Some((held.deep_links, held.repositories.clone()))
}

/// The Fleet a link is linked through, where the screen has one.
pub fn link_fleet(cx: &App) -> Option<Entity<FleetConnection>> {
    cx.try_global::<Here>()?.0.upgrade()?.read(cx).fleet.clone()
}

/// A banner's press on a review asked of him: [`OperatorScreen::open_review`] from outside the screen.
pub fn open_asked(asked: Option<ipc::ReviewAsked>, cx: &mut App) {
    let Some(Here(me, handle)) = cx.try_global::<Here>().map(|here| Here(here.0.clone(), here.1)) else { return };
    cx.defer(move |cx| {
        let _ = handle.update(cx, |_, window, cx| {
            let _ = me.update(cx, |this, cx| this.open_review(asked.as_ref(), window, cx));
        });
    });
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
            Front::Kit => {
                let title = crate::chrome::title_row(window, cx);
                let kit = self.kit.clone().map(|kit| kit.into_any_element());
                div().size_full().flex().flex_col().bg(cx.look().color(token::BG_BASE)).child(title).child(div().flex_1().min_h_0().children(kit)).into_any_element()
            }
            Front::Features => {
                let title = crate::chrome::title_row(window, cx);
                let features = self.features.clone().map(|features| features.into_any_element());
                div().size_full().flex().flex_col().bg(cx.look().color(token::BG_BASE)).child(title).child(div().flex_1().min_h_0().children(features)).into_any_element()
            }
            Front::Usage => {
                let title = crate::chrome::title_row(window, cx);
                let usage = self.usage.clone().map(|usage| usage.into_any_element());
                div().size_full().flex().flex_col().bg(cx.look().color(token::BG_BASE)).child(title).child(div().flex_1().min_h_0().children(usage)).into_any_element()
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
    fn redraw(cx: &mut VisualTestContext) {
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
    }

    fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
        cx.update(|window, cx| cx.try_global::<LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, name)).map(|one| one.bounds))
    }

    fn entered(screen: &Entity<OperatorScreen>, cx: &mut VisualTestContext) -> bool {
        screen.read_with(cx, |screen, cx| screen.features.as_ref().expect("the screen holds Features").read(cx).entered())
    }

    #[test]
    fn features_is_a_place_of_its_own_after_the_last_one_and_may_be_hidden() {
        let face = with_places(Face::bare(), Front::Jobs, true, true, true);
        let ids: Vec<_> = face.places.iter().map(|place| place.id).collect();
        assert_eq!(ids, ["operator", "jobs", "reviews", "kit", "features"], "no place before it moves a digit");
        let place = face.places.last().expect("a place");
        assert_eq!((place.label, place.glyph, place.shown, place.hideable), ("Features", "library", true, true));
        assert!(face.may_hide("features"));
        assert_eq!(with_places(Face::bare(), Front::Jobs, true, true, false).places.len(), 4, "a screen without Features draws no place for it");
    }

    #[test]
    fn settings_layout_hides_features_from_the_strip_as_it_hides_kit() {
        let drawn_ids = |face: &Face| face.drawn_places().map(|place| place.id).collect::<Vec<_>>();
        crate::settings::layout::hold(r#"{"version":1,"rail":{"hidden":["features"]}}"#);
        let hidden = with_places(Face::bare(), Front::Jobs, true, true, true);
        assert_eq!(drawn_ids(&hidden), ["operator", "jobs", "reviews", "kit"]);
        // Places already in the bar are read again, so a layout saved after they were first drawn takes hold.
        crate::settings::layout::hold(r#"{"version":1,"rail":{"hidden":["kit"]}}"#);
        let swapped = with_places(hidden, Front::Jobs, true, true, true);
        assert_eq!(drawn_ids(&swapped), ["operator", "jobs", "reviews", "features"], "Features is back and Kit is gone");
        crate::settings::layout::hold("");
        assert_eq!(drawn_ids(&with_places(swapped, Front::Jobs, true, true, true)), ["operator", "jobs", "reviews", "kit", "features"]);
    }

    #[gpui::test]
    fn features_is_a_tab_named_features_and_a_press_brings_it_to_the_front_and_reads_the_library(cx: &mut TestAppContext) {
        ready(cx);
        let (screen, cx) = cx.add_window_view(|window, cx| {
            let jobs = jobs_screen(window, cx);
            OperatorScreen::mock_reviews(jobs, window, cx)
        });
        redraw(cx);
        let tab = drawn(cx, Role::Tab, "Features").expect("the strip draws Features, named as the person reads it");
        assert!(!entered(&screen, cx));
        cx.simulate_click(tab.center(), gpui::Modifiers::default());
        redraw(cx);
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Features);
        assert!(entered(&screen, cx), "the screen follows Fleet once it is in front");
        assert!(drawn(cx, Role::ListBoxOption, "Start a Job").is_some(), "the library's features are drawn, each named");
    }

    #[gpui::test]
    fn another_place_taking_the_front_has_features_leave(cx: &mut TestAppContext) {
        ready(cx);
        let (screen, cx) = cx.add_window_view(|window, cx| {
            let jobs = jobs_screen(window, cx);
            OperatorScreen::mock_reviews(jobs, window, cx)
        });
        redraw(cx);
        cx.update(|window, cx| screen.update(cx, |screen, cx| screen.bring(Some(Front::Features), window, cx)));
        assert!(entered(&screen, cx));
        cx.update(|window, cx| screen.update(cx, |screen, cx| screen.bring(Some(Front::Kit), window, cx)));
        assert!(!entered(&screen, cx), "Kit in front, Features stops following Fleet");
        cx.update(|window, cx| screen.update(cx, |screen, cx| screen.bring(Some(Front::Features), window, cx)));
        cx.update(|window, cx| screen.update(cx, |screen, cx| screen.bring(Some(Front::Jobs), window, cx)));
        assert!(!entered(&screen, cx), "so does Jobs in front");
    }

    #[gpui::test]
    fn command_5_is_features_with_operator_jobs_reviews_and_kit_before_it(cx: &mut TestAppContext) {
        ready(cx);
        let (screen, cx) = cx.add_window_view(|window, cx| {
            let jobs = jobs_screen(window, cx);
            OperatorScreen::mock_reviews(jobs, window, cx)
        });
        redraw(cx);
        cx.simulate_keystrokes("cmd-5");
        redraw(cx);
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Features);
        cx.simulate_keystrokes("cmd-4");
        redraw(cx);
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Kit, "the digits before it keep their places");
        assert!(!entered(&screen, cx));
    }

    #[gpui::test]
    fn a_live_features_screen_reads_its_library_when_it_comes_to_the_front(cx: &mut TestAppContext) {
        ready(cx);
        let (screen, cx) = cx.add_window_view(|window, cx| {
            let jobs = jobs_screen(window, cx);
            let fleet = cx.new(crate::fleet::FleetConnection::idle);
            OperatorScreen::live(jobs, &fleet, window, cx)
        });
        let not_read = |screen: &Entity<OperatorScreen>, cx: &mut VisualTestContext| {
            screen.read_with(cx, |screen, cx| screen.features.as_ref().and_then(|features| features.read(cx).not_read().map(str::to_string)))
        };
        assert_eq!(not_read(&screen, cx), None, "nothing is asked of Fleet before the screen is in front");
        cx.update(|window, cx| screen.update(cx, |screen, cx| screen.bring(Some(Front::Features), window, cx)));
        assert_eq!(not_read(&screen, cx).as_deref(), Some("Fleet is not connected"), "coming to the front reads, and an idle Fleet says so");
    }

    const PASTED: &str = "https://forge.example/acme-labs/sample-api/pull/41";

    #[gpui::test]
    fn pasting_a_pull_request_link_outside_a_text_box_opens_reviews(cx: &mut TestAppContext) {
        ready(cx);
        let (screen, cx) = cx.add_window_view(|window, cx| {
            let jobs = jobs_screen(window, cx);
            OperatorScreen::mock_reviews(jobs, window, cx)
        });
        cx.run_until_parked();
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Jobs);
        cx.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(PASTED.to_string())));
        cx.simulate_keystrokes("cmd-v");
        cx.run_until_parked();
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Reviews);
    }

    #[gpui::test]
    fn pasting_a_pull_request_link_into_a_text_box_stays_the_boxs(cx: &mut TestAppContext) {
        ready(cx);
        let (screen, cx) = cx.add_window_view(|window, cx| {
            let jobs = jobs_screen(window, cx);
            OperatorScreen::mock_reviews(jobs, window, cx)
        });
        cx.update(|window, cx| {
            screen.update(cx, |screen, cx| {
                screen.bring(Some(Front::Operator), window, cx);
                let composer = screen.page.read(cx).composer.clone();
                composer.update(cx, |composer, cx| composer.focus(window, cx));
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(PASTED.to_string())));
        cx.simulate_keystrokes("cmd-v");
        cx.run_until_parked();
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Operator, "the field took the paste");
    }
}
