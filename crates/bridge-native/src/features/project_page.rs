//! A project's own pages on the Features screen: its overview, what moved since he looked, and who is on what. An entity of its own, as a feature's
//! page is, so the screen fills the slot beside the list with whichever is chosen.
//!
//! What is read is Fleet's: `get_project_overview`, `get_since_you_looked` or `get_whos_on_what`, off the main thread, and the page draws bones until it
//! answers. `project_model.rs` works out what is drawn. **The time he last looked is kept on this Mac per project** (`looked.rs`),
//! and only Mark read moves it.

use chrono::{DateTime, Duration, Utc};
use gpui::{App, Context, Entity, EventEmitter, FocusHandle, Focusable, KeyDownEvent, Pixels, Subscription, Task, Window};
use ipc::{ProjectOverview, SinceYouLooked, WhosOnWhat};

use super::page::Load;
use super::project_model::{self, AreaHue, ProjectPage};
use super::who_model::{self, Stop, WhoView};
use super::{looked, mock, who_kept, wire};
use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::log::Note;
use crate::settings::wire::Refused;

/// What the page tells whoever holds it.
pub enum ProjectPageEvent {
    /// A row was pressed: the feature to open beside the list.
    Open(String),
    /// A pull request number was pressed: the pull request to open, by its repository and number.
    Pull { repository: String, number: u64 },
    /// A Job chip was pressed: the Job to open.
    Job(String),
    /// Esc, or Tab: the keys go back to the list.
    Escape,
}

enum Source {
    Live(Entity<FleetConnection>),
    Mock,
}

/// What the keys can be on.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Target {
    MarkRead,
    Feature(String),
    /// A place on Who's on what, in the layout drawn.
    Who(Stop),
}

/// What the screen knows that the page does not: the project's name, the hue of each area, and the room the list leaves it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Beside {
    pub name: String,
    pub areas: Vec<AreaHue>,
    pub flush: bool,
    pub inset: Pixels,
    /// The owner's Jobs the library names, as `(id, title)`, for the Job chips of Who's on what to say what they are.
    pub jobs: Vec<(String, String)>,
}

pub struct ProjectPageView {
    pub within: FocusHandle,
    /// The keys are on the page's rows.
    pub(super) body: FocusHandle,
    source: Source,
    pub(super) which: ProjectPage,
    project: String,
    pub(super) beside: Beside,
    pub(super) overview: Load<ProjectOverview>,
    pub(super) since: Load<SinceYouLooked>,
    pub(super) who: Load<WhosOnWhat>,
    /// The layout Who's on what is drawn in: the one he chose last.
    pub(super) view: WhoView,
    /// The time Since you looked compares with.
    pub(super) since_at: DateTime<Utc>,
    /// The place the keys are on, in `targets`.
    pub(super) at: Option<usize>,
    read_port: Option<u16>,
    work: Option<Task<()>>,
    /// The page a read was sent for that has not answered: one sent again over it drops it, which the log says.
    out: Option<ProjectPage>,
    _subs: Vec<Subscription>,
}

impl EventEmitter<ProjectPageEvent> for ProjectPageView {}

impl Drop for ProjectPageView {
    fn drop(&mut self) {
        if let Some(page) = self.out {
            Note::info("a project page was put away before its read answered").field("page", page.slug()).emit();
        }
    }
}

impl Focusable for ProjectPageView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.body.clone()
    }
}

impl ProjectPageView {
    /// On Fleet: `which` page of `project`, by Fleet's id. `ran` is when the librarian last ran, which is the time to compare with
    /// the first time, where he has marked nothing read.
    pub fn live(fleet: &Entity<FleetConnection>, project: String, which: ProjectPage, ran: Option<DateTime<Utc>>, cx: &mut Context<Self>) -> Self {
        looked::persist(cx);
        who_kept::persist(cx);
        let since_at = project_model::first_since(looked::before_project(&project, cx), ran, Utc::now());
        let mut this = Self::new(Source::Live(fleet.clone()), project, which, since_at, cx);
        let watch = cx.observe(fleet, |this, _, cx| {
            if this.port(cx) != this.read_port {
                this.read(cx);
            }
        });
        this._subs.push(watch);
        this.read(cx);
        this
    }

    /// On the mock pages in `mock.rs`.
    pub fn mock(which: ProjectPage, cx: &mut Context<Self>) -> Self {
        let since_at = Utc::now() - Duration::days(mock::SINCE_DAYS_AGO);
        let mut this = Self::new(Source::Mock, mock::PROJECT.to_string(), which, since_at, cx);
        this.read(cx);
        this
    }

    fn new(source: Source, project: String, which: ProjectPage, since_at: DateTime<Utc>, cx: &mut Context<Self>) -> Self {
        ProjectPageView {
            within: cx.focus_handle(),
            body: cx.focus_handle(),
            source,
            which,
            project,
            beside: Beside::default(),
            overview: Load::Reading,
            since: Load::Reading,
            who: Load::Reading,
            view: who_kept::get(cx).unwrap_or(WhoView::Grid),
            since_at,
            at: None,
            read_port: None,
            work: None,
            out: None,
            _subs: Vec::new(),
        }
    }

    pub fn which(&self) -> ProjectPage {
        self.which
    }

    /// The time Since you looked compares with.
    #[cfg(test)]
    pub fn since_at(&self) -> DateTime<Utc> {
        self.since_at
    }

    /// Stand in for Fleet's Who's on what.
    #[cfg(test)]
    pub fn fill_who(&mut self, who: WhosOnWhat, cx: &mut Context<Self>) {
        self.who = Load::Read(who);
        self.keep_at();
        cx.notify();
    }

    /// Say Who's on what was refused.
    #[cfg(test)]
    pub fn refuse_who(&mut self, said: &str, cx: &mut Context<Self>) {
        self.who = Load::Failed(said.into());
        cx.notify();
    }

    /// Stand in for what Fleet served, so a test draws a state the mock never has.
    #[cfg(test)]
    pub fn fill(&mut self, overview: Option<ProjectOverview>, since: Option<SinceYouLooked>, cx: &mut Context<Self>) {
        if let Some(overview) = overview {
            self.overview = Load::Read(overview);
        }
        if let Some(since) = since {
            self.since = Load::Read(since);
        }
        cx.notify();
    }

    /// What the screen knows of the project: its name, the hue of each area, and the room the list leaves the page.
    pub fn set_beside(&mut self, beside: Beside, cx: &mut Context<Self>) {
        if self.beside != beside {
            self.beside = beside;
            cx.notify();
        }
    }

    // ---- reading -----------------------------------------------------------------------------

    fn port(&self, cx: &App) -> Option<u16> {
        match &self.source {
            Source::Live(fleet) => fleet.read(cx).held().port(),
            Source::Mock => None,
        }
    }

    /// Read the page from Fleet again, keeping what is drawn until the answer comes.
    pub fn read(&mut self, cx: &mut Context<Self>) {
        let Source::Live(_) = &self.source else {
            self.overview = Load::Read(mock::overview());
            self.since = Load::Read(mock::since(self.since_at));
            self.who = Load::Read(mock::who());
            self.keep_at();
            cx.notify();
            return;
        };
        let port = self.port(cx);
        self.read_port = port;
        let Some(port) = port else {
            self.failed("Fleet is not connected");
            cx.notify();
            return;
        };
        if let Some(was) = self.out.replace(self.which) {
            Note::info("a project page was read again before its last read answered").field("page", was.slug()).emit();
        }
        Note::info("a project page was read").field("page", self.which.slug()).field("port", port.to_string()).emit();
        match self.which {
            ProjectPage::Overview => {
                let ask = wire::overview(port, &self.project, cx);
                self.work = Some(cx.spawn(async move |this, cx| {
                    let answer = ask.await;
                    let _ = this.update(cx, |this, cx| this.took_overview(answer, cx));
                }));
            }
            ProjectPage::Since => {
                let asked = self.since_at;
                let ask = wire::since(port, &self.project, asked, cx);
                self.work = Some(cx.spawn(async move |this, cx| {
                    let answer = ask.await;
                    let _ = this.update(cx, |this, cx| this.took_since(asked, answer, cx));
                }));
            }
            ProjectPage::Who => {
                let ask = wire::who(port, &self.project, cx);
                self.work = Some(cx.spawn(async move |this, cx| {
                    let answer = ask.await;
                    let _ = this.update(cx, |this, cx| this.took_who(answer, cx));
                }));
            }
        }
    }

    fn failed(&mut self, said: &str) {
        match self.which {
            ProjectPage::Overview if !matches!(self.overview, Load::Read(_)) => self.overview = Load::Failed(said.into()),
            ProjectPage::Since if !matches!(self.since, Load::Read(_)) => self.since = Load::Failed(said.into()),
            ProjectPage::Who if !matches!(self.who, Load::Read(_)) => self.who = Load::Failed(said.into()),
            _ => {}
        }
    }

    /// The log says how a read ended, so a page that stays on its bones leaves a trail.
    fn answered<T>(&mut self, page: ProjectPage, answer: &Result<T, Refused>) {
        self.out = None;
        let said = match answer {
            Ok(_) => "read".to_string(),
            Err(why) => why.said.clone(),
        };
        Note::info("a project page read answered").field("page", page.slug()).field("answer", said).emit();
    }

    fn took_overview(&mut self, answer: Result<ProjectOverview, Refused>, cx: &mut Context<Self>) {
        self.answered(ProjectPage::Overview, &answer);
        match answer {
            Ok(read) => self.overview = Load::Read(read),
            // A read that failed keeps what was read before, where there was one.
            Err(why) => self.failed(&why.said),
        }
        self.keep_at();
        cx.notify();
    }

    /// An answer for a time he has since moved on from is not the page's.
    fn took_since(&mut self, asked: DateTime<Utc>, answer: Result<SinceYouLooked, Refused>, cx: &mut Context<Self>) {
        self.answered(ProjectPage::Since, &answer);
        if asked != self.since_at {
            return;
        }
        match answer {
            Ok(read) => self.since = Load::Read(read),
            Err(why) => self.failed(&why.said),
        }
        self.keep_at();
        cx.notify();
    }

    fn took_who(&mut self, answer: Result<WhosOnWhat, Refused>, cx: &mut Context<Self>) {
        self.answered(ProjectPage::Who, &answer);
        match answer {
            Ok(read) => self.who = Load::Read(read),
            Err(why) => self.failed(&why.said),
        }
        self.keep_at();
        cx.notify();
    }

    fn keep_at(&mut self) {
        let count = self.targets().len();
        self.at = self.at.filter(|at| *at < count);
    }

    /// The page is in front again: Fleet is read again.
    pub fn enter(&mut self, cx: &mut Context<Self>) {
        self.read(cx);
    }

    // ---- what the keys are on ----------------------------------------------------------------

    /// The places the keys can be on, in the order they are drawn: Mark read, then each feature a row names.
    pub(super) fn targets(&self) -> Vec<Target> {
        if let (ProjectPage::Who, Load::Read(read)) = (self.which, &self.who) {
            return who_model::who(read, &self.beside.jobs).stops(self.view).into_iter().map(Target::Who).collect();
        }
        match (self.which, &self.overview, &self.since) {
            (ProjectPage::Overview, Load::Read(read), _) => read.busiest.iter().map(|one| Target::Feature(one.slug.clone())).collect(),
            (ProjectPage::Since, _, Load::Read(read)) => std::iter::once(Target::MarkRead).chain(read.features.iter().map(|one| Target::Feature(one.slug.clone()))).collect(),
            _ => Vec::new(),
        }
    }

    pub fn focus_body(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.body, cx);
        cx.notify();
    }

    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let count = self.targets().len() as isize;
        if count == 0 {
            return;
        }
        let at = self.at.map_or(if by > 0 { -1 } else { count }, |at| at as isize);
        self.at = Some((at + by).clamp(0, count - 1) as usize);
        cx.notify();
    }

    pub(super) fn pick(&mut self, at: usize, cx: &mut Context<Self>) {
        self.at = Some(at);
        self.press(cx);
    }

    /// Enter, or a press: Mark read marks the page read, and a row opens its feature.
    fn press(&mut self, cx: &mut Context<Self>) {
        match self.at.and_then(|at| self.targets().get(at).cloned()) {
            Some(Target::MarkRead) => self.mark_read(cx),
            Some(Target::Feature(slug) | Target::Who(Stop::Feature(slug))) => cx.emit(ProjectPageEvent::Open(slug)),
            Some(Target::Who(Stop::Pull { repository, number })) => cx.emit(ProjectPageEvent::Pull { repository, number }),
            Some(Target::Who(Stop::Job(id))) => cx.emit(ProjectPageEvent::Job(id)),
            None => {}
        }
    }

    pub fn view(&self) -> WhoView {
        self.view
    }

    /// Draw Who's on what in `view`, and keep the choice for the next page and the next start. The keys start again at the top, as the
    /// places they were on are not the ones the new layout draws.
    pub fn set_view(&mut self, view: WhoView, cx: &mut Context<Self>) {
        if self.view != view {
            self.view = view;
            self.at = None;
            who_kept::set(view, cx);
            cx.notify();
        }
    }

    /// The key that cycles the three layouts: Grid, By feature, By person, and round.
    pub fn cycle_view(&mut self, cx: &mut Context<Self>) {
        self.set_view(self.view.next(), cx);
    }

    /// Mark read: the time he last looked is now, kept on this Mac, and the page is read again for it.
    pub fn mark_read(&mut self, cx: &mut Context<Self>) {
        let now = Utc::now();
        looked::stamp_project(&self.project, now, cx);
        self.since_at = now;
        self.read(cx);
        cx.notify();
    }

    // ---- keys --------------------------------------------------------------------------------

    pub(super) fn on_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" {
            cx.emit(ProjectPageEvent::Escape);
            cx.stop_propagation();
        }
    }

    /// The registry's `features` acts, pressed while the keys are on the page. All of them stop here except the filter, the project
    /// chips and the librarian's Catch up, which are the list's. The page has one place to be in, so Tab goes back to the list.
    pub(super) fn on_act(&mut self, act: &Act, _: &mut Window, cx: &mut Context<Self>) {
        if !act.id.starts_with("features_") || ["features_filter", "features_project", "features_catch_up"].contains(&act.id) {
            cx.propagate();
            return;
        }
        match act.id {
            "features_move" => self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            "features_open" => self.press(cx),
            "features_pane" => cx.emit(ProjectPageEvent::Escape),
            "features_who_view" if self.which == ProjectPage::Who => self.cycle_view(cx),
            "features_who_view" => cx.propagate(),
            _ => {}
        }
    }
}
