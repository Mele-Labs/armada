//! The project's two pages on the Features screen: the rows over the areas that open them, and the page that takes the place of a
//! feature's beside the list. The pages themselves are `project_page.rs` and `project_view.rs`.

use chrono::{DateTime, Utc};
use gpui::{div, prelude::*, px, AnyElement, Context, FontWeight, Role, SharedString, Window};

use super::dossier::selected_shadow;
use super::project_model::ProjectPage;
use super::project_page::{Beside, ProjectPageEvent, ProjectPageView};
use super::project_model::AreaHue;
use super::{FeaturesScreen, Load, Source};
use crate::titlebar;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::view::ring_colour;

impl FeaturesScreen {
    /// The project page open beside the list.
    pub fn project_open(&self, cx: &gpui::App) -> Option<ProjectPage> {
        self.project_page.as_ref().map(|page| page.read(cx).which())
    }

    /// The project page entity, for a test to read.
    #[cfg(test)]
    pub fn project_view(&self) -> Option<gpui::Entity<ProjectPageView>> {
        self.project_page.clone()
    }

    /// When the librarian last ran, which is the time Since you looked compares with until he marks it read.
    fn librarian_ran(&self) -> Option<DateTime<Utc>> {
        match &self.library {
            Load::Read(library) => library.librarian.last_run.as_ref().and_then(|at| DateTime::parse_from_rfc3339(at.as_str()).ok()).map(|at| at.with_timezone(&Utc)),
            _ => None,
        }
    }

    /// A press on a project row, or Enter on it: its page opens beside the list in place of the feature's.
    pub(super) fn open_project(&mut self, which: ProjectPage, window: &mut Window, cx: &mut Context<Self>) {
        self.on_project = Some(which);
        if self.project_open(cx) != Some(which) {
            self.close_page(cx);
            let ran = self.librarian_ran();
            let page = match (&self.source, self.library_project.clone()) {
                (Source::Live(fleet), Some(project)) => Some(cx.new(|cx| ProjectPageView::live(fleet, project, which, ran, cx))),
                (Source::Mock, _) => Some(cx.new(|cx| ProjectPageView::mock(which, cx))),
                _ => None,
            };
            if let Some(page) = page {
                self.project_sub = Some(cx.subscribe_in(&page, window, |this, _, event: &ProjectPageEvent, window, cx| match event {
                    ProjectPageEvent::Open(slug) => this.open_from_project(slug.clone(), window, cx),
                    ProjectPageEvent::Pull { repository, number } => this.open_pull(repository, *number, cx),
                    ProjectPageEvent::Job(id) => this.open_job(id.clone(), cx),
                    ProjectPageEvent::Escape if this.stacked => this.back(window, cx),
                    // Drawn alone as a tab, the page has no list to give the keys to.
                    ProjectPageEvent::Escape if this.alone() => {}
                    ProjectPageEvent::Escape => this.focus_list(window, cx),
                }));
                if self.alone() {
                    page.update(cx, |page, cx| page.focus_body(window, cx));
                }
                self.project_page = Some(page);
                self.tune_project_page(cx);
            }
        }
        cx.notify();
    }

    /// A row of a project page was pressed: the feature opens in its place and has the keys, as the list's own Enter has it.
    pub(super) fn open_from_project(&mut self, slug: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.listed.find(&slug).is_none() {
            return;
        }
        self.open_at(slug, window, cx);
        self.engage_page(cx);
        if self.stacked {
            self.show_page(window, cx);
        } else if let Some(page) = self.page.clone() {
            page.update(cx, |page, cx| page.focus_rail(window, cx));
        }
    }

    /// A pull request number on Who's on what was pressed: the pull request opens in Reviews, as a link to it does.
    fn open_pull(&mut self, repository: &str, number: u64, cx: &mut Context<Self>) {
        if let Some((owner, name)) = repository.split_once('/') {
            crate::deep_link::open(&crate::deep_link::PullLink { owner: owner.to_string(), name: name.to_string(), number }, cx);
        }
    }

    /// A Job chip on Who's on what was pressed: the Job opens on the Jobs screen, as the In this build sheet opens one.
    fn open_job(&mut self, job: String, cx: &mut Context<Self>) {
        if let Some(sink) = cx.try_global::<titlebar::AskedSink>().cloned() {
            (sink.0)(&titlebar::Asked::OpenJob(job), cx);
        }
        crate::palette::show_jobs(cx);
    }

    /// What the open project page takes from the screen: the project's name, the hue the list gives each area, and the room the list leaves it.
    pub(super) fn tune_project_page(&mut self, cx: &mut Context<Self>) {
        let Some(page) = self.project_page.clone() else { return };
        let name = self.library_project.as_ref().and_then(|id| self.chips(cx).into_iter().find(|(chip, _)| chip == id)).map(|(_, name)| name).unwrap_or_default();
        let areas = match &self.library {
            Load::Read(library) => library.areas.iter().map(|area| AreaHue { slug: area.slug.clone(), title: area.title.clone(), hue: self.hue_of(&area.slug) }).collect(),
            _ => Vec::new(),
        };
        let jobs = match &self.library {
            Load::Read(library) => library.areas.iter().flat_map(|area| &area.features).flat_map(|row| &row.jobs_now).map(|job| (job.session_id.as_str().to_string(), job.title.clone())).collect(),
            _ => Vec::new(),
        };
        let beside = Beside { name, areas, flush: !self.stacked, inset: self.inset(cx), jobs };
        page.update(cx, |page, cx| page.set_beside(beside, cx));
    }

    /// The rows over the areas (`.pages`): Overview, Since you looked, then Who's on what. Each is a stop of the keys, as a feature's row is.
    pub(super) fn render_project_rows(&self, look: &Look, focused: bool, cx: &mut Context<Self>) -> AnyElement {
        let open = self.project_open(cx);
        let mut rows = div().flex().flex_col().gap(px(1.)).px(look.length(token::SPACE_2)).pb(look.length(token::SPACE_1));
        for which in ProjectPage::ALL {
            let opened = open == Some(which);
            let under_keys = self.on_project == Some(which);
            let ring = ring_colour(look);
            let row = div()
                .id(SharedString::from(format!("features-page-{}", which.slug())))
                .w_full()
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_1) + px(2.))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .map(|row| {
                    if opened {
                        row.bg(look.color(token::BG_RAISED)).text_color(look.color(token::FG_DEFAULT)).shadow(selected_shadow(look, (under_keys && focused).then_some(ring)))
                    } else if under_keys && focused {
                        row.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)).shadow(selected_shadow(look, Some(ring)))
                    } else {
                        row.text_color(look.color(token::FG_MUTED)).cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER)))
                    }
                })
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .aria_selected(opened)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_project(which, window, cx);
                    if this.stacked {
                        this.show_page(window, cx);
                    } else {
                        this.focus_list(window, cx);
                    }
                }))
                .child(which.title());
            rows = rows.child(labelled(row, Role::ListBoxOption, which.title()));
        }
        labelled(rows.id("features-pages"), Role::ListBox, "Project pages").into_any_element()
    }
}
