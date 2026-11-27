//! The Jobs list by feature and by area: a heading for each feature or area the listed Jobs write to, and a Job under every
//! feature its row's `features` names. Over every grouping, each sub project (`list_sub_projects`) heads its Jobs: the ones he
//! picked, and the live Jobs that write to one of its features. A board reads its project's library (`list_features`).

use super::*;

/// How often the libraries are read again at most while a board wants them: one read can take a second.
const EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// What a heading leads to, beyond its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Lead {
    /// A feature, by its project and slug: a press opens its board.
    Feature { project: String, slug: String },
    Area,
    /// The Jobs no feature holds, last.
    NoFeature,
    /// A sub project, by its id: a press opens its board.
    SubProject { id: String },
}

impl Lead {
    /// The id the keys carry while they are on a heading that opens a board, which no Job's id can be.
    pub(super) fn stop(&self) -> Option<SharedString> {
        match self {
            Lead::Feature { project, slug } => Some(format!("lead:feature:{project}/{slug}").into()),
            Lead::SubProject { id } => Some(format!("lead:sub:{id}").into()),
            Lead::Area | Lead::NoFeature => None,
        }
    }
}

/// What the boards read: each listed project's library, as Fleet last answered, and the sub projects.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Leads {
    pub(super) libraries: Vec<ipc::FeatureLibrary>,
    /// Every sub project Fleet keeps, oldest first.
    pub(super) subs: Vec<ipc::SubProject>,
}

/// One feature as a heading stands for it.
pub(super) struct Shelf<'a> {
    pub key: String,
    pub title: &'a str,
    pub area_key: String,
    pub area_title: &'a str,
    pub project: &'a str,
    pub slug: &'a str,
    pub jobs: Vec<&'a str>,
}

/// Every feature the Jobs write to, from their rows, in the libraries' order: project by project, area by area.
pub(super) fn shelves<'a>(jobs: impl IntoIterator<Item = &'a SessionRecord>) -> Vec<Shelf<'a>> {
    let mut out: Vec<(&ipc::SessionFeature, Shelf<'a>)> = Vec::new();
    for job in jobs {
        for feature in &job.features {
            match out.iter_mut().find(|(held, _)| held.project == feature.project && held.slug == feature.slug) {
                Some((_, shelf)) => shelf.jobs.push(job.id.as_str()),
                None => out.push((
                    feature,
                    Shelf {
                        key: format!("feature:{}/{}", feature.project, feature.slug),
                        title: &feature.title,
                        area_key: format!("area:{}/{}", feature.project, feature.area),
                        area_title: &feature.area_title,
                        project: &feature.project,
                        slug: &feature.slug,
                        jobs: vec![job.id.as_str()],
                    },
                )),
            }
        }
    }
    out.sort_by(|(a, _), (b, _)| (&a.project, a.area_place, &a.area, a.place, &a.slug).cmp(&(&b.project, b.area_place, &b.area, b.place, &b.slug)));
    out.into_iter().map(|(_, shelf)| shelf).collect()
}

/// The titles of the features a Job writes to now, in the libraries' order.
pub(super) fn titles_of(shelves: &[Shelf], job: &str) -> Vec<String> {
    shelves.iter().filter(|shelf| shelf.jobs.contains(&job)).map(|shelf| shelf.title.to_string()).collect()
}

impl Leads {
    /// A feature by its project and slug, with its area's title.
    pub(super) fn feature(&self, project: &str, slug: &str) -> Option<(&ipc::FeatureRow, &str)> {
        let library = self.libraries.iter().find(|library| library.project == project)?;
        library.areas.iter().find_map(|area| area.features.iter().find(|feature| feature.slug == slug).map(|feature| (feature, area.title.as_str())))
    }
}

impl Leads {
    /// A sub project's Jobs among `listed`: the ones he picked, in his order, then each live one that writes to one of its
    /// features now, with that feature's title.
    pub(super) fn members<'a>(&self, sub: &ipc::SubProject, listed: &[&'a SessionRecord]) -> Vec<(&'a SessionRecord, Option<String>)> {
        let mut out: Vec<(&'a SessionRecord, Option<String>)> =
            sub.jobs.iter().filter_map(|id| listed.iter().find(|job| job.id == *id)).map(|job| (*job, None)).collect();
        for job in listed.iter().filter(|job| job.state == SessionState::Live) {
            if out.iter().any(|(held, _)| held.id == job.id) {
                continue;
            }
            if let Some(feature) = job.features.iter().find(|feature| feature.project == sub.project && sub.features.contains(&feature.slug)) {
                out.push((*job, Some(feature.title.clone())));
            }
        }
        out
    }

    /// The titles of a sub project's features, as its library names them now, else as a Job writing to one names it.
    pub(super) fn feature_titles(&self, sub: &ipc::SubProject, jobs: &[SessionRecord]) -> Vec<String> {
        let written = |slug: &str| jobs.iter().flat_map(|job| &job.features).find(|feature| feature.project == sub.project && feature.slug == slug).map(|feature| feature.title.clone());
        sub.features.iter().filter_map(|slug| self.feature(&sub.project, slug).map(|(feature, _)| feature.title.clone()).or_else(|| written(slug))).collect()
    }
}

/// The muted line of a joined row: the feature that brought it in.
pub(super) fn joined(title: &str) -> SharedString {
    format!("Joined: it writes to {title}").into()
}

/// The muted line of a row under feature `under`: the other features the Job writes to now. None where it writes to no other.
pub(super) fn also_under(titles: &[String], under: &str) -> Option<SharedString> {
    let others: Vec<&str> = titles.iter().map(String::as_str).filter(|title| *title != under).collect();
    (!others.is_empty()).then(|| format!("Also under {}", others.join(", ")).into())
}

impl JobsList {
    /// Whether the list draws anything read from the libraries now: a feature's or a sub project's board. The headings draw
    /// from the Jobs' own rows.
    fn wants_features(&self) -> bool {
        self.board().is_some()
    }

    /// The projects of the Jobs Fleet serves live, by id, each once.
    fn lead_projects(&self) -> Vec<String> {
        let mut projects: Vec<String> =
            self.jobs.iter().filter(|job| job.state == SessionState::Live).filter_map(|job| self.names.project_of(job)).map(str::to_string).collect();
        projects.sort();
        projects.dedup();
        projects
    }

    /// Read the sub projects again, and the libraries where a board draws from them, no sooner than [`EVERY`] after the last
    /// read and never two at once. `now` reads at once, as a new grouping or a board opening does.
    pub(super) fn refresh_leads(&mut self, now: bool, cx: &mut Context<Self>) {
        if self.leads_read.is_some() || self.fleet.is_none() {
            return;
        }
        if !now && self.leads_at.is_some_and(|at| at.elapsed() < EVERY) {
            return;
        }
        let Some(port) = self.port(cx) else { return };
        let projects = if self.wants_features() { self.lead_projects() } else { Vec::new() };
        self.leads_at = Some(std::time::Instant::now());
        let subs = crate::settings::wire::request::<ipc::SubProjects>(port, crate::settings::wire::Ask::Get("/sub_projects".into()), "SubProjects", cx);
        let reads: Vec<_> = projects.iter().map(|project| crate::features::wire::library(port, project, cx)).collect();
        let read_any = !reads.is_empty();
        self.leads_read = Some(cx.spawn(async move |this, cx| {
            let subs = match subs.await {
                Ok(held) => Some(held.sub_projects),
                Err(why) => {
                    Note::warn("the sub projects were not read").field("why", why.said).emit();
                    None
                }
            };
            let mut libraries = Vec::new();
            for read in reads {
                match read.await {
                    Ok(library) => libraries.push(library),
                    Err(why) => Note::warn("a project's features were not read").field("why", why.said).emit(),
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.leads_read = None;
                if let Some(subs) = subs {
                    this.hold_subs(subs, cx);
                }
                if read_any {
                    this.hold_libraries(libraries, cx);
                }
            });
        }));
    }

    /// The sub projects Fleet answered: the list draws from them, where they changed.
    pub(crate) fn hold_subs(&mut self, subs: Vec<ipc::SubProject>, cx: &mut Context<Self>) {
        if self.leads.subs != subs {
            self.leads.subs = subs;
            self.relist(cx);
        }
    }

    /// The libraries Fleet answered: the list draws from them, where they changed.
    pub(crate) fn hold_libraries(&mut self, libraries: Vec<ipc::FeatureLibrary>, cx: &mut Context<Self>) {
        if self.leads.libraries != libraries {
            self.leads.libraries = libraries;
            self.relist(cx);
        }
    }
}

impl JobsList {
    /// A feature's, an area's, No feature's or a sub project's heading: the list's own heading look, a feature's with the
    /// `layers` glyph, a sub project's with `folder-tree` in the accent, an area's with the dot, and no count.
    pub(super) fn render_lead(&self, at: usize, head: &Head, cx: &mut Context<Self>) -> gpui::AnyElement {
        let look = cx.look();
        let name = head.name.clone();
        let glyph = |name: &str, ink: crate::look::Token| svg().path(icon_path(name)).flex_none().size(look.length(token::SPACE_3)).text_color(look.color(ink)).into_any_element();
        let sub = matches!(head.lead, Some(Lead::SubProject { .. }));
        let mark: gpui::AnyElement = match head.lead {
            Some(Lead::Feature { .. }) => glyph("layers", token::FG_MUTED),
            Some(Lead::SubProject { .. }) => glyph("boxes", token::ACCENT),
            _ => div().flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::STATUS_NOT_STARTED)).into_any_element(),
        };
        let lead = head.lead.clone().filter(|lead| lead.stop().is_some());
        let focused = lead.as_ref().and_then(Lead::stop).is_some_and(|stop| self.focused.as_ref() == Some(&stop));
        let hovered = look.color(token::BG_HOVER);
        let line = group_line(look, at == 0)
            .id(("lead", at))
            .gap(look.length(token::SPACE_2))
            .rounded_t(look.length(token::RADIUS_MD))
            .when(sub, |line| line.text_color(look.color(token::ACCENT)))
            .when(focused, |line| line.bg(hovered))
            .child(mark)
            .child(div().min_w_0().truncate().child(name.clone()));
        // A feature's or a sub project's heading opens its board.
        let line = match lead {
            Some(lead) => line.cursor_pointer().hover(move |style| style.bg(hovered)).on_click(cx.listener(move |this, _, window, cx| this.open_board(lead.clone(), window, cx))),
            None => line,
        };
        labelled(line, Role::Heading, name).into_any_element()
    }
}

impl JobsList {
    /// ⇧S on the row under the keys, or on the selection where that row is in it.
    pub(super) fn on_start_sub_project(&mut self, _: &StartSubProject, window: &mut Window, cx: &mut Context<Self>) {
        match self.focused.clone().filter(|id| self.selectable(id)) {
            Some(id) => {
                let at = self.row_under(&id);
                self.open_name_sheet(self.reached_by(&id), at, window, cx);
            }
            None => cx.propagate(),
        }
    }

    /// The naming sheet for these Jobs, under the row as the project picker opens, or in the window's upper middle where the row
    /// is not drawn.
    pub(crate) fn open_name_sheet(&mut self, ids: Vec<SharedString>, at: Option<Point<Pixels>>, window: &mut Window, cx: &mut Context<Self>) {
        let ids: Vec<SharedString> = ids.into_iter().filter(|id| self.record(id).is_some_and(|job| job.state == SessionState::Live)).collect();
        if ids.is_empty() {
            return;
        }
        let (at, corner) = match at {
            Some(at) => (at, Anchor::TopLeft),
            None => {
                let size = window.viewport_size();
                (point(size.width / 2., size.height / 4.), Anchor::TopCenter)
            }
        };
        self.card = None;
        let list = cx.weak_entity();
        let sheet = cx.new(|cx| name_sheet::NameSheet::new(list, ids, at, corner, window, cx));
        let dismissed = cx.subscribe_in(&sheet, window, |this, shut, _: &DismissEvent, window, cx| {
            if this.naming.as_ref().is_some_and(|(open, _)| open == shut) {
                this.naming = None;
            }
            window.focus(&this.focus, cx);
            cx.notify();
        });
        self.naming = Some((sheet, dismissed));
        cx.notify();
    }

    /// The naming sheet while it is open, for a test.
    #[cfg(test)]
    pub(crate) fn name_sheet(&self) -> Option<Entity<name_sheet::NameSheet>> {
        self.naming.as_ref().map(|(sheet, _)| sheet.clone())
    }

    /// What Start it sends: the project of the first Job, the name, and the Jobs in his order.
    pub(super) fn sub_project_body(&self, ids: &[SharedString], name: &str) -> ipc::StartSubProject {
        let project = ids.iter().filter_map(|id| self.record(id)).find_map(|job| self.names.project_of(job)).unwrap_or_default().to_string();
        ipc::StartSubProject { project, name: name.trim().to_string(), jobs: ids.iter().map(|id| ipc::SessionId::carried(id.to_string())).collect() }
    }

    /// Start a sub project of these Jobs. A live Fleet keeps it and the heading stands on its answer; a refusal comes back in
    /// Fleet's words. On mock data the list keeps it itself, as a walk's stand-in for Fleet's yes. The selection is let go.
    pub(crate) fn start_sub_project(&mut self, ids: &[SharedString], name: &str, cx: &mut Context<Self>) -> Task<Result<(), String>> {
        let body = self.sub_project_body(ids, name);
        if self.fleet.is_none() {
            let features = shelves(&self.jobs).into_iter().filter(|shelf| shelf.project == body.project && body.jobs.iter().any(|id| shelf.jobs.contains(&id.as_str()))).map(|shelf| shelf.slug.to_string()).collect();
            let sub = ipc::SubProject {
                id: format!("mock-sub-{}", self.leads.subs.len() + 1),
                project: body.project,
                name: body.name,
                jobs: body.jobs,
                features,
                started_at: ipc::Instant::carried(chrono::Utc::now().to_rfc3339()),
            };
            self.hold_sub(sub, cx);
            self.clear_selection(cx);
            return Task::ready(Ok(()));
        }
        let Some(port) = self.port(cx) else { return Task::ready(Err("Fleet is not connected".into())) };
        let sent = crate::settings::wire::request::<ipc::SubProject>(port, crate::settings::wire::Ask::Post("/sub_projects".into(), crate::settings::wire::body(&body)), "SubProject", cx);
        cx.spawn(async move |this, cx| match sent.await {
            Ok(sub) => {
                let _ = this.update(cx, |this, cx| {
                    this.hold_sub(sub, cx);
                    this.clear_selection(cx);
                });
                Ok(())
            }
            Err(why) => Err(why.said),
        })
    }

    /// A sub project kept: it heads its Jobs at once, ahead of the next read.
    fn hold_sub(&mut self, sub: ipc::SubProject, cx: &mut Context<Self>) {
        let mut subs = self.leads.subs.clone();
        match subs.iter_mut().find(|held| held.id == sub.id) {
            Some(held) => *held = sub,
            None => subs.push(sub),
        }
        self.hold_subs(subs, cx);
    }
}

/// What a Job waits on now, as its record says it: what it asks of him first, else what keeps it open (a watch, a Job it
/// started, its pull request), else a held item, a slot, a pause, or the state it stands in.
pub(super) fn holds(job: &SessionRecord) -> String {
    if let Some(item) = job.waiting_for.iter().find(|item| item.source != WaitingSource::Done) {
        return match item.source {
            WaitingSource::ReadyToTry => "Ready to try: waits on your Looks good".into(),
            _ => format!("Waits on you: {}", crate::job_view::plain_of(&item.text)),
        };
    }
    if let Some(held) = job.held_open_by.first() {
        return match held {
            ipc::HeldOpenBy::Watch { name } => format!("Waits on its watch {name}"),
            ipc::HeldOpenBy::Job { name } => format!("Waits on its Job {name}"),
            ipc::HeldOpenBy::PullRequest { number, on: ipc::PullRequestWait::Queue } => format!("Pull request #{number} in the merge queue"),
            ipc::HeldOpenBy::PullRequest { number, on: ipc::PullRequestWait::AutoMerge } => format!("Pull request #{number} merges once its Checks pass"),
            ipc::HeldOpenBy::PullRequest { number, on: ipc::PullRequestWait::Checks { .. } } => format!("Pull request #{number} waits on its Checks"),
        };
    }
    if let Some(wait) = job.claim_waits.first() {
        return if wait.holder.is_empty() { format!("Waits on {}", wait.item) } else { format!("Waits on {}, held by {}", wait.item, wait.holder) };
    }
    if let Some(wait) = &job.slot_wait {
        return crate::slot_wait::said(wait).to_string();
    }
    if job.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some()) {
        return "Paused".into();
    }
    mark_of(job).map(|mark| glyph_of(mark.state, None).1.to_string()).unwrap_or_default()
}

impl JobsList {
    /// The heading under the keys, where they are on a feature's or a sub project's.
    pub(super) fn focused_lead(&self) -> Option<Lead> {
        let focused = self.focused.as_ref()?;
        self.lines.iter().find_map(|line| match line {
            Line::Group(head) => head.lead.clone().filter(|lead| lead.stop().as_ref() == Some(focused)),
            _ => None,
        })
    }

    /// The board standing in the right column, where no Job was opened since it opened.
    pub(crate) fn board(&self) -> Option<Entity<board::LeadBoard>> {
        self.board.as_ref().filter(|(_, at)| *at == self.picks).map(|(board, _)| board.clone())
    }

    /// Open a feature's or a sub project's board where a Job opens, with the keys on it and its heading under the list's keys.
    pub(super) fn open_board(&mut self, lead: Lead, window: &mut Window, cx: &mut Context<Self>) {
        self.focused = lead.stop();
        self.card = None;
        let me = cx.entity();
        let board = cx.new(|cx| board::LeadBoard::new(&me, lead, cx));
        let keys = board.read(cx).focus().clone();
        self.board = Some((board, self.picks));
        window.focus(&keys, cx);
        self.refresh_leads(true, cx);
        cx.notify();
    }

    /// Esc on a board, or its sub project ungrouped: the board goes and the keys go back to the list.
    pub(super) fn close_board(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.board = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Ungroup: Fleet drops the sub project and its heading leaves; its Jobs are untouched. A refusal comes back in Fleet's
    /// words. On mock data the list drops it itself.
    pub(super) fn ungroup_sub_project(&mut self, id: &str, cx: &mut Context<Self>) -> Task<Result<(), String>> {
        if self.fleet.is_none() {
            let subs = self.leads.subs.iter().filter(|sub| sub.id != id).cloned().collect();
            self.hold_subs(subs, cx);
            return Task::ready(Ok(()));
        }
        let Some(port) = self.port(cx) else { return Task::ready(Err("Fleet is not connected".into())) };
        let body = crate::settings::wire::body(&ipc::UngroupSubProject { id: id.to_string() });
        let sent = cx.background_executor().spawn(async move {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
            runtime.block_on(crate::transport::post(port, UNGROUP, body))
        });
        let id = id.to_string();
        cx.spawn(async move |this, cx| {
            match sent.await {
                Ok((status, _)) if (200..300).contains(&status) => {}
                Ok((status, bytes)) => return Err(crate::refusal::Refusal::read("POST", UNGROUP, status, &bytes).to_string()),
                Err(why) => return Err(why),
            }
            let _ = this.update(cx, |this, cx| {
                let subs = this.leads.subs.iter().filter(|sub| sub.id != id).cloned().collect();
                this.hold_subs(subs, cx);
            });
            Ok(())
        })
    }
}

/// `ungroup_sub_project`'s route.
const UNGROUP: &str = "/sub_projects/ungroup";
