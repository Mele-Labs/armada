//! ⌘[ and ⌘], Back and Forward through the places he has been, as Classic's `history.ts` has them: a fold of
//! visits over {back, at, forward}, capped at [`CAP`]. A place is where the window says he is; the fold knows
//! nothing of how he got there. Places are recorded by observation, so every way of getting somewhere (a key, a
//! press, a toast, a link) is a visit, and a restore is never one.
//!
//! The Job browser keeps ⌘[ and ⌘] while its tab is in front in the pane the keys are in (`browser_act`); this
//! takes what that lets go, from any focus, a text box's included.

use gpui::{AnyWindowHandle, App, Context, Entity, Global, SharedString, Subscription, WeakEntity, Window};

use crate::keymap::Act;
use crate::operator::{Front, OperatorScreen};
use crate::settings::overlay::SettingsOverlay;

/// The most places kept behind him.
pub const CAP: usize = 50;

/// Where he has been, where he is and where Forward goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History<P> {
    pub back: Vec<P>,
    pub at: P,
    pub forward: Vec<P>,
}

pub enum Event<P> {
    Visited(P),
    Back,
    Forward,
}

impl<P: Clone + PartialEq> History<P> {
    pub fn start(at: P) -> Self {
        History { back: Vec::new(), at, forward: Vec::new() }
    }

    pub fn fold(mut self, event: Event<P>) -> Self {
        match event {
            Event::Visited(place) => {
                // The same place twice is one visit.
                if place == self.at {
                    return self;
                }
                let left = std::mem::replace(&mut self.at, place);
                self.stack(left);
                self.forward.clear();
            }
            Event::Back => {
                let Some(to) = self.back.pop() else { return self };
                let left = std::mem::replace(&mut self.at, to);
                self.forward.insert(0, left);
            }
            Event::Forward => {
                if self.forward.is_empty() {
                    return self;
                }
                let to = self.forward.remove(0);
                let left = std::mem::replace(&mut self.at, to);
                self.stack(left);
            }
        }
        self
    }

    /// `left` goes behind him, the oldest dropped past [`CAP`].
    fn stack(&mut self, left: P) {
        self.back.push(left);
        if self.back.len() > CAP {
            self.back.drain(..self.back.len() - CAP);
        }
    }

    /// `place` replacing the layers over home: what is behind the last visit to home stays, the layers after it go,
    /// and `place` stands where they were, so Back from a sibling returns to home and not to the last layer. The place
    /// being left counts as a visit, so a press made on home itself keeps it behind. With no home in the trail it is
    /// an ordinary visit. Native has no layered pages over a Cockpit yet, so nothing calls this outside its tests.
    #[allow(dead_code)]
    pub fn collapse(self, place: P, is_home: impl Fn(&P) -> bool) -> Self {
        let mut trail = self.back.clone();
        trail.push(self.at.clone());
        let Some(home) = trail.iter().rposition(|one| is_home(one)) else {
            return self.fold(Event::Visited(place));
        };
        trail.truncate(if is_home(&place) { home } else { home + 1 });
        History { back: trail, at: place, forward: Vec::new() }
    }
}

/// Where he is, as far as Back and Forward are concerned: the place in front, the Job open on it where that is the
/// Jobs screen, and whether Settings is over it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub front: Front,
    pub job: Option<SharedString>,
    pub settings: bool,
}

/// The window's history. It observes the screen, its Jobs list and Settings, and folds each change of place in.
pub struct Keeper {
    screen: Entity<OperatorScreen>,
    overlay: Entity<SettingsOverlay>,
    held: History<Place>,
    _subs: Vec<Subscription>,
}

/// The keeper and its window, for a key from anywhere.
struct Live(WeakEntity<Keeper>, AnyWindowHandle);

impl Global for Live {}

/// ⌘[ and ⌘] from any focus: what the Job browser lets go of reaches here (`actions.toml`, scope `anywhere`).
pub fn bind(cx: &mut App) {
    cx.on_action(|key: &Act, cx: &mut App| {
        if !key.is("history") {
            return cx.propagate();
        }
        let Some(Live(keeper, window)) = cx.try_global::<Live>().map(|live| Live(live.0.clone(), live.1)) else { return cx.propagate() };
        let forward = key.slot == 1;
        // The key is being dispatched inside this window's own update, so the move waits until it is over.
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                if let Some(keeper) = keeper.upgrade() {
                    keeper.update(cx, |keeper, cx| keeper.go(forward, window, cx));
                }
            });
        });
    });
}

impl Keeper {
    pub fn new(screen: Entity<OperatorScreen>, overlay: Entity<SettingsOverlay>, window: AnyWindowHandle, cx: &mut Context<Self>) -> Self {
        let held = History::start(here(&screen, &overlay, cx));
        let list = screen.read(cx).list(cx);
        let subs = vec![
            cx.observe(&screen, |this, _, cx| this.visit(cx)),
            // A Job opened or let go of on the Jobs screen.
            cx.observe(&list, |this, _, cx| this.visit(cx)),
            cx.observe(&overlay, |this, _, cx| this.visit(cx)),
        ];
        let me = cx.entity().downgrade();
        cx.set_global(Live(me, window));
        Keeper { screen, overlay, held, _subs: subs }
    }

    /// The window moved: a new place is a visit, and the same place is nothing. Fleet's events notify the list far more
    /// often than the place changes, so most calls only read.
    fn visit(&mut self, cx: &mut Context<Self>) {
        let now = here(&self.screen, &self.overlay, cx);
        if now != self.held.at {
            self.held = self.held.clone().fold(Event::Visited(now));
        }
    }

    /// Back or Forward: the place the fold lands on is brought back, and where that left him is where he is.
    fn go(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let moved = self.held.clone().fold(if forward { Event::Forward } else { Event::Back });
        if moved == self.held {
            return;
        }
        self.held = moved;
        let to = self.held.at.clone();
        if !to.settings {
            self.overlay.update(cx, |overlay, cx| overlay.close(window, cx));
        }
        self.screen.update(cx, |screen, cx| screen.restore(to.front, to.job, window, cx));
        if to.settings {
            self.overlay.update(cx, |overlay, cx| {
                if !overlay.open {
                    let project = overlay.scope.project.clone();
                    overlay.show(project, false, window, cx);
                }
            });
        }
        // A Job that is gone, or a place the screen does not hold, leaves him elsewhere: that is where he is.
        self.held.at = here(&self.screen, &self.overlay, cx);
    }
}

/// Where he is now in the window that keeps a history, for what is drawn there (`annotate::drawn_by`).
pub fn now(cx: &App) -> Option<Place> {
    let keeper = cx.try_global::<Live>()?.0.upgrade()?;
    Some(keeper.read(cx).held.at.clone())
}

fn here(screen: &Entity<OperatorScreen>, overlay: &Entity<SettingsOverlay>, cx: &App) -> Place {
    let (front, job) = screen.read(cx).at(cx);
    Place { front, job, settings: overlay.read(cx).open }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visit(history: History<&'static str>, place: &'static str) -> History<&'static str> {
        history.fold(Event::Visited(place))
    }

    fn home(place: &&str) -> bool {
        *place == "overview"
    }

    use gpui::{AppContext, Focusable, TestAppContext, VisualTestContext};

    use crate::jobs::{JobsList, JobsScreen};
    use crate::settings::SettingsScreen;

    /// The app's own stack over mock data: Jobs in front, the Operator's places in the strip, Settings over all of it.
    struct Window {
        screen: Entity<OperatorScreen>,
        overlay: Entity<SettingsOverlay>,
        list: Entity<JobsList>,
        keeper: Entity<Keeper>,
        jobs: Vec<SharedString>,
    }

    fn open(cx: &mut TestAppContext) -> (Window, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
            crate::titlebar::bind(cx);
            bind(cx);
        });
        let records = crate::mock::jobs("jobs-many").expect("the fixture decodes");
        let jobs = records.iter().take(2).map(|record| SharedString::from(record.id.as_str().to_string())).collect();
        let mut held = None;
        let (settings, cx) = cx.add_window_view(|window, cx| {
            let list = cx.new(|cx| JobsList::mock(&records, cx));
            let jobs = cx.new(|cx| JobsScreen::new(list, None, window, cx));
            let screen = cx.new(|cx| OperatorScreen::mock_reviews(jobs, window, cx));
            held = Some(screen.clone());
            SettingsScreen::mock(screen.into(), window, cx)
        });
        let screen = held.expect("the screen was built");
        let overlay = settings.read_with(cx, |settings, _| settings.overlay.clone());
        let list = screen.read_with(cx, |screen, cx| screen.list(cx));
        let keeper = cx.update(|window, cx| {
            let handle = window.window_handle();
            let (screen, overlay) = (screen.clone(), overlay.clone());
            cx.new(|cx| Keeper::new(screen, overlay, handle, cx))
        });
        cx.run_until_parked();
        (Window { screen, overlay, list, keeper, jobs }, cx)
    }

    fn front(window: &Window, cx: &mut VisualTestContext) -> Front {
        window.screen.read_with(cx, |screen, _| screen.front)
    }

    fn opened(window: &Window, cx: &mut VisualTestContext) -> Option<SharedString> {
        window.list.read_with(cx, |list, _| list.opened_id())
    }

    #[gpui::test]
    fn cmd_bracket_goes_back_through_the_places_and_cmd_close_bracket_forward(cx: &mut TestAppContext) {
        let (window, cx) = open(cx);
        assert_eq!(front(&window, cx), Front::Jobs);
        cx.simulate_keystrokes("cmd-4");
        cx.simulate_keystrokes("cmd-5");
        cx.run_until_parked();
        assert_eq!(front(&window, cx), Front::Features);
        cx.simulate_keystrokes("cmd-[");
        cx.run_until_parked();
        assert_eq!(front(&window, cx), Front::Kit, "Back goes to the place before");
        cx.simulate_keystrokes("cmd-[");
        cx.run_until_parked();
        assert_eq!(front(&window, cx), Front::Jobs);
        cx.simulate_keystrokes("cmd-[");
        cx.run_until_parked();
        assert_eq!(front(&window, cx), Front::Jobs, "Back at the first place stays");
        cx.simulate_keystrokes("cmd-]");
        cx.simulate_keystrokes("cmd-]");
        cx.run_until_parked();
        assert_eq!(front(&window, cx), Front::Features, "Forward goes the way Back came");
        window.keeper.read_with(cx, |keeper, _| assert_eq!(keeper.held.back.len(), 2, "going back and forth is no visit"));
    }

    #[gpui::test]
    fn a_visit_after_back_forgets_forward(cx: &mut TestAppContext) {
        let (window, cx) = open(cx);
        cx.simulate_keystrokes("cmd-4");
        cx.simulate_keystrokes("cmd-[");
        cx.simulate_keystrokes("cmd-5");
        cx.run_until_parked();
        cx.simulate_keystrokes("cmd-]");
        cx.run_until_parked();
        assert_eq!(front(&window, cx), Front::Features, "Forward has nowhere to go after a new visit");
        cx.simulate_keystrokes("cmd-[");
        cx.run_until_parked();
        assert_eq!(front(&window, cx), Front::Jobs, "Kit was dropped from the trail");
    }

    #[gpui::test]
    fn the_job_open_on_the_jobs_screen_is_part_of_the_place(cx: &mut TestAppContext) {
        let (window, cx) = open(cx);
        let (first, second) = (window.jobs[0].clone(), window.jobs[1].clone());
        window.list.update(cx, |list, cx| list.open_job(&first, cx));
        cx.run_until_parked();
        window.list.update(cx, |list, cx| list.open_job(&second, cx));
        cx.run_until_parked();
        cx.simulate_keystrokes("cmd-4");
        cx.run_until_parked();
        assert_eq!(front(&window, cx), Front::Kit);
        cx.simulate_keystrokes("cmd-[");
        cx.run_until_parked();
        assert_eq!((front(&window, cx), opened(&window, cx)), (Front::Jobs, Some(second.clone())), "back to the Job that was open");
        cx.simulate_keystrokes("cmd-[");
        cx.run_until_parked();
        assert_eq!(opened(&window, cx), Some(first), "and the one before");
        cx.simulate_keystrokes("cmd-[");
        cx.run_until_parked();
        assert_eq!(opened(&window, cx), None, "and the list with none open");
        cx.simulate_keystrokes("cmd-]");
        cx.simulate_keystrokes("cmd-]");
        cx.simulate_keystrokes("cmd-]");
        cx.run_until_parked();
        assert_eq!(front(&window, cx), Front::Kit, "three presses forward reach Kit again");
    }

    #[gpui::test]
    fn a_job_that_is_gone_is_left_shut_and_that_is_where_he_is(cx: &mut TestAppContext) {
        let (window, cx) = open(cx);
        let first = window.jobs[0].clone();
        window.list.update(cx, |list, cx| list.open_job(&first, cx));
        cx.simulate_keystrokes("cmd-4");
        cx.run_until_parked();
        window.list.update(cx, |list, cx| list.gone(&[first.to_string()], cx));
        cx.run_until_parked();
        cx.simulate_keystrokes("cmd-[");
        cx.run_until_parked();
        assert_eq!((front(&window, cx), opened(&window, cx)), (Front::Jobs, None));
        window.keeper.read_with(cx, |keeper, _| assert_eq!(keeper.held.at, Place { front: Front::Jobs, job: None, settings: false }));
    }

    #[gpui::test]
    fn settings_is_a_place_and_back_shuts_it_from_its_search_box(cx: &mut TestAppContext) {
        let (window, cx) = open(cx);
        cx.simulate_keystrokes("cmd-,");
        cx.run_until_parked();
        assert!(window.overlay.read_with(cx, |overlay, _| overlay.open), "⌘, opens Settings");
        let search = window.overlay.read_with(cx, |overlay, cx| overlay.query.read(cx).focus_handle(cx));
        cx.update(|window, cx| window.focus(&search, cx));
        cx.simulate_keystrokes("cmd-[");
        cx.run_until_parked();
        assert!(!window.overlay.read_with(cx, |overlay, _| overlay.open), "Back shuts Settings, though a text box has the keys");
        assert_eq!(front(&window, cx), Front::Jobs);
        cx.simulate_keystrokes("cmd-]");
        cx.run_until_parked();
        assert!(window.overlay.read_with(cx, |overlay, _| overlay.open), "Forward opens it again");
    }

    #[test]
    fn a_visit_pushes_where_you_were() {
        let history = visit(History::start("a"), "b");
        assert_eq!(history, History { back: vec!["a"], at: "b", forward: vec![] });
    }

    #[test]
    fn the_same_place_twice_is_one_visit() {
        assert!(visit(History::start("a"), "a").back.is_empty());
    }

    #[test]
    fn back_then_forward_returns() {
        let history = visit(visit(History::start("a"), "b"), "c");
        let back = history.clone().fold(Event::Back);
        assert_eq!(back.at, "b");
        assert_eq!(back.forward, vec!["c"]);
        assert_eq!(back.fold(Event::Forward), history);
    }

    #[test]
    fn a_new_visit_after_back_drops_forward() {
        let history = visit(History::start("a"), "b").fold(Event::Back);
        assert!(visit(history, "c").forward.is_empty());
    }

    #[test]
    fn back_and_forward_at_the_ends_do_nothing() {
        let history = History::start("a");
        assert_eq!(history.clone().fold(Event::Back), history);
        assert_eq!(history.clone().fold(Event::Forward), history);
    }

    #[test]
    fn the_stack_is_capped_and_the_oldest_dropped() {
        let places: Vec<String> = (0..=CAP + 10).map(|n| n.to_string()).collect();
        let mut history = History::start(places[0].clone());
        for place in &places[1..] {
            history = history.fold(Event::Visited(place.clone()));
        }
        assert_eq!(history.back.len(), CAP);
        assert_eq!(history.back[0], "10");
    }

    #[test]
    fn back_from_a_sibling_returns_to_the_cockpit() {
        let layered = visit(visit(History::start("overview"), "checks"), "lessons");
        let sibling = layered.collapse("sessions", home);
        assert_eq!(sibling, History { back: vec!["overview"], at: "sessions", forward: vec![] });
        assert_eq!(sibling.fold(Event::Back).at, "overview");
    }

    #[test]
    fn a_press_on_the_cockpit_drops_the_layers_over_it_and_keeps_what_was_behind_it() {
        let mut history = visit(History::start("x"), "overview");
        history = visit(visit(history, "checks"), "lessons");
        let forward = history.fold(Event::Back);
        assert_eq!(forward.forward.len(), 1);
        assert_eq!(forward.collapse("overview", home), History { back: vec!["x"], at: "overview", forward: vec![] });
    }

    #[test]
    fn a_press_made_on_the_cockpit_itself_keeps_it_behind_the_sibling() {
        let history = visit(History::start("x"), "overview");
        assert_eq!(history.collapse("sessions", home), History { back: vec!["x", "overview"], at: "sessions", forward: vec![] });
    }

    #[test]
    fn with_no_cockpit_visit_anywhere_a_collapse_is_a_plain_visit_that_forgets_forward() {
        let history = visit(visit(History::start("a"), "b"), "c").fold(Event::Back);
        assert_eq!(history.collapse("d", home), History { back: vec!["a", "b"], at: "d", forward: vec![] });
    }
}
