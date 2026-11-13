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
}

impl Front {
    fn id(self) -> &'static str {
        match self {
            Front::Operator => "operator",
            Front::Jobs => "jobs",
        }
    }
}

pub struct OperatorScreen {
    jobs: Entity<JobsScreen>,
    pub page: Entity<OperatorPageView>,
    pub front: Front,
    pub within: FocusHandle,
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
fn with_places(mut face: Face, here: Front) -> Face {
    if !face.places.iter().any(|place| place.id == "operator") {
        face.places.insert(0, Place { id: "operator", label: "Operator", glyph: "bot", shown: true, hideable: true });
    }
    if !face.places.iter().any(|place| place.id == "jobs") {
        face.places.push(Place { id: "jobs", label: "Jobs", glyph: "box", shown: true, hideable: true });
    }
    face.here = Some(here.id());
    face
}

impl OperatorScreen {
    pub fn live(jobs: Entity<JobsScreen>, fleet: &Entity<FleetConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = cx.new(|cx| OperatorPageView::live(fleet, false, window, cx));
        Self::new(jobs, page, Front::Jobs, window, cx)
    }

    /// On the walk's Fleet, with the page in front.
    pub fn mock(jobs: Entity<JobsScreen>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = cx.new(|cx| OperatorPageView::mock(false, window, cx));
        Self::new(jobs, page, Front::Operator, window, cx)
    }

    /// On the walk's Fleet, with the Jobs screen in front.
    pub fn mock_behind(jobs: Entity<JobsScreen>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = cx.new(|cx| OperatorPageView::mock(false, window, cx));
        Self::new(jobs, page, Front::Jobs, window, cx)
    }

    fn new(jobs: Entity<JobsScreen>, page: Entity<OperatorPageView>, front: Front, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.set_global(Live(page.downgrade()));
        let subs = vec![cx.subscribe_in(&page, window, |this, _, opened: &Opened, window, cx| this.opened(opened, window, cx))];
        let handle = window.window_handle();
        let me = cx.entity().downgrade();
        let behind = cx.try_global::<AskedSink>().cloned();
        cx.set_global(AskedSink(Rc::new(move |asked: &Asked, cx: &mut App| {
            let to = match asked {
                Asked::Operator => Some(None),
                Asked::Place("operator") => Some(Some(Front::Operator)),
                Asked::Place("jobs") => Some(Some(Front::Jobs)),
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
        let mut this = OperatorScreen { jobs, page, front, within: cx.focus_handle(), toasts, _subs: subs };
        this.show_places(cx);
        if front == Front::Operator {
            let keys = this.page.read(cx).focus.clone();
            window.focus(&keys, cx);
        }
        this
    }

    /// The bar's strip says which place is in front: the bar there now, and the one the next screen opens.
    fn show_places(&mut self, cx: &mut Context<Self>) {
        let held = titlebar::live(cx).map(|bar| bar.read(cx).face.clone());
        let face = with_places(held.or_else(|| cx.try_global::<Opening>().map(|opening| opening.0.clone())).unwrap_or_else(Face::bare), self.front);
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
        if to == self.front {
            return;
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
        };
        div().id(SharedString::from("operator-screen")).track_focus(&self.within).size_full().child(shown).child(self.toasts.clone())
    }
}
