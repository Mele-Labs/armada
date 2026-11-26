//! The Jobs list by feature and by area: each project's library (`list_features`) read off the main thread, a heading for
//! each feature or area, and a Job under every feature whose `jobs_now` holds it.

use super::*;

/// How often the libraries are read again at most while the list wants them: one read can take a second.
const EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// What a heading leads to, beyond its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Lead {
    /// A feature, by its project and slug: a press opens its board.
    Feature { project: String, slug: String },
    Area,
    /// The Jobs no feature holds, last.
    NoFeature,
}

impl Lead {
    /// The id the keys carry while they are on a heading that opens a board, which no Job's id can be.
    pub(super) fn stop(&self) -> Option<SharedString> {
        match self {
            Lead::Feature { project, slug } => Some(format!("lead:feature:{project}/{slug}").into()),
            Lead::Area | Lead::NoFeature => None,
        }
    }
}

/// What the feature groupings read: each listed project's library, as Fleet last answered.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Leads {
    pub(super) libraries: Vec<ipc::FeatureLibrary>,
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

impl Leads {
    /// Every feature, in the libraries' order, area by area.
    pub(super) fn shelves(&self) -> Vec<Shelf<'_>> {
        let mut out = Vec::new();
        for library in &self.libraries {
            for area in &library.areas {
                for feature in &area.features {
                    out.push(Shelf {
                        key: format!("feature:{}/{}", library.project, feature.slug),
                        title: &feature.title,
                        area_key: format!("area:{}/{}", library.project, area.slug),
                        area_title: &area.title,
                        project: &library.project,
                        slug: &feature.slug,
                        jobs: feature.jobs_now.iter().map(|job| job.session_id.as_str()).collect(),
                    });
                }
            }
        }
        out
    }

    /// The titles of the features a Job writes to now, in the libraries' order.
    pub(super) fn titles_of(&self, job: &str) -> Vec<String> {
        self.shelves().into_iter().filter(|shelf| shelf.jobs.contains(&job)).map(|shelf| shelf.title.to_string()).collect()
    }

    /// A feature by its project and slug, with its area's title.
    pub(super) fn feature(&self, project: &str, slug: &str) -> Option<(&ipc::FeatureRow, &str)> {
        let library = self.libraries.iter().find(|library| library.project == project)?;
        library.areas.iter().find_map(|area| area.features.iter().find(|feature| feature.slug == slug).map(|feature| (feature, area.title.as_str())))
    }
}

/// The muted line of a row under feature `under`: the other features the Job writes to now. None where it writes to no other.
pub(super) fn also_under(titles: &[String], under: &str) -> Option<SharedString> {
    let others: Vec<&str> = titles.iter().map(String::as_str).filter(|title| *title != under).collect();
    (!others.is_empty()).then(|| format!("Also under {}", others.join(", ")).into())
}

impl JobsList {
    /// Whether the list draws anything read from the libraries now.
    fn wants_leads(&self) -> bool {
        matches!(self.group, Group::Feature | Group::Area)
    }

    /// The projects of the Jobs Fleet serves live, by id, each once.
    fn lead_projects(&self) -> Vec<String> {
        let mut projects: Vec<String> =
            self.jobs.iter().filter(|job| job.state == SessionState::Live).filter_map(|job| self.names.project_of(job)).map(str::to_string).collect();
        projects.sort();
        projects.dedup();
        projects
    }

    /// Read the libraries again where the list draws from them, no sooner than [`EVERY`] after the last read and never two at once.
    /// `now` reads at once, as a new grouping does.
    pub(super) fn refresh_leads(&mut self, now: bool, cx: &mut Context<Self>) {
        if !self.wants_leads() || self.leads_read.is_some() || self.fleet.is_none() {
            return;
        }
        if !now && self.leads_at.is_some_and(|at| at.elapsed() < EVERY) {
            return;
        }
        let Some(port) = self.port(cx) else { return };
        let projects = self.lead_projects();
        self.leads_at = Some(std::time::Instant::now());
        let reads: Vec<_> = projects.iter().map(|project| crate::features::wire::library(port, project, cx)).collect();
        self.leads_read = Some(cx.spawn(async move |this, cx| {
            let mut libraries = Vec::new();
            for read in reads {
                match read.await {
                    Ok(library) => libraries.push(library),
                    Err(why) => Note::warn("a project's features were not read").field("why", why.said).emit(),
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.leads_read = None;
                this.hold_libraries(libraries, cx);
            });
        }));
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
    /// A feature's, an area's or No feature's heading: the list's own heading look, a feature's with the `layers` glyph and
    /// an area's with the dot, and no count.
    pub(super) fn render_lead(&self, at: usize, head: &Head, cx: &mut Context<Self>) -> gpui::AnyElement {
        let look = cx.look();
        let name = head.name.clone();
        let mark: gpui::AnyElement = match head.lead {
            Some(Lead::Feature { .. }) => svg().path(icon_path("layers")).flex_none().size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)).into_any_element(),
            _ => div().flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::STATUS_NOT_STARTED)).into_any_element(),
        };
        labelled(group_line(look, at == 0).id(("lead", at)).gap(look.length(token::SPACE_2)).child(mark).child(div().min_w_0().truncate().child(name.clone())), Role::Heading, name)
            .into_any_element()
    }
}
