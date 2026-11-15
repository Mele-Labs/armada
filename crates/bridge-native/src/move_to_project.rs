//! Moving Jobs into one of his projects, or out to none (`set_session_project`; `docs/practices/protocol/projects.md`).
//!
//! One call for any number of Jobs, live or ended. Fleet answers the Jobs' rows and publishes each as
//! `session.changed`, and the Jobs list groups by the project on the record, so a move regroups the rows by
//! itself. A refusal is said in a toast in Fleet's own sentence.
//!
//! Every door asks [`movable`] whether the act is there, [`entries`] for the picker's rows and
//! [`crate::jobs::JobsList::move_to_project`] to make the move: the row's menu, `P`, the palette and the Job head.

use gpui::{App, SharedString};
use ipc::SessionRecord;

use crate::dispatch::live::{projects_of, ProjectRepositories};
use crate::dropdown::{Entry, Row};
use crate::titlebar::id_of;

/// The route the move is asked on. Fleet keeps it on the Mac that holds the Job.
pub const ROUTE: &str = "/sessions/project";

/// The id of the row that takes the Jobs out of any project.
const NONE: &str = "project:";

/// Whether the act is there for `job`: one Fleet hosts that has a repository, since Fleet refuses a Job with none.
/// An ended Job is moved as a live one is.
pub fn movable(job: &SessionRecord) -> bool {
    job.hosted.is_some() && job.manifest_id.is_some()
}

/// The project the Jobs being moved stand in now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Every one stands in this project, or in none.
    In(Option<String>),
    /// They stand in different places, or there is no Job.
    Mixed,
}

/// Where the Jobs stand: the one project they all share, else mixed.
pub fn standing<'a>(jobs: impl IntoIterator<Item = &'a SessionRecord>) -> Standing {
    let mut found: Option<Option<String>> = None;
    for job in jobs {
        let project = job.hosted.as_ref().and_then(|hosted| hosted.project.clone());
        match &found {
            Some(first) if *first != project => return Standing::Mixed,
            Some(_) => {}
            None => found = Some(project),
        }
    }
    found.map_or(Standing::Mixed, Standing::In)
}

/// The picker's rows: the field that finds a project, each of his projects with its repositories on a line under
/// the name, then No project. The row every Job already stands in is ticked.
pub fn entries(projects: &[ProjectRepositories], standing: &Standing) -> Vec<Entry> {
    let ticked = |project: Option<&str>| matches!(standing, Standing::In(held) if held.as_deref() == project);
    let mut entries = vec![Entry::Search { label: "Find a project", to_rule: false }];
    for (project, repositories) in projects {
        let row = Row::new(id_of(format!("project:{}", project.id)), project.name.clone()).icon("layers").ticked(ticked(Some(project.id.as_ref())));
        let row = if repositories.is_empty() {
            row
        } else {
            row.under(repositories.iter().map(SharedString::as_ref).collect::<Vec<&str>>().join(", "))
        };
        entries.push(Entry::Row(row));
    }
    entries.push(Entry::Row(Row::new(NONE, "No project").icon("git-branch").ticked(ticked(None))));
    entries
}

/// The project a picker row names: `Some(None)` for No project, and none for a row that is not one of the picker's.
pub fn project_of(row: &str) -> Option<Option<String>> {
    let id = row.strip_prefix("project:")?;
    Some((!id.is_empty()).then(|| id.to_string()))
}

/// His projects as the title bar holds them, each with its repositories. Mock data has the bar's scene when a
/// walk draws one, else the mock's own projects.
pub fn projects(mock: bool, cx: &App) -> Vec<ProjectRepositories> {
    if let Some(bar) = crate::titlebar::live(cx) {
        let face = &bar.read(cx).face;
        return projects_of(&face.projects, &face.repositories);
    }
    if !mock {
        return Vec::new();
    }
    let served = crate::mock::repositories();
    let repositories = crate::titlebar::repositories_of(&served, &[]);
    projects_of(&crate::titlebar::projects_of(&served, &crate::mock::projects()), &repositories)
}

/// What is posted to `/sessions/project`: `SetSessionProject`, with none for the project that takes the Jobs out.
pub fn body(jobs: &[SharedString], project: Option<&str>) -> Result<String, ipc::Unencodable> {
    ipc::encode(&ipc::SetSessionProject {
        session_ids: jobs.iter().map(|id| ipc::SessionId::carried(id.to_string())).collect(),
        project: project.map(str::to_string),
    })
}

/// The move asked of Fleet. It answers the rows and publishes each, so nothing is kept here; a refusal is said in a
/// toast in Fleet's words.
pub fn post(port: Option<u16>, body: Result<String, ipc::Unencodable>, cx: &mut App) {
    let Some(port) = port else {
        crate::toasts::tell("Fleet is not connected", cx);
        return;
    };
    let body = match body {
        Ok(body) => body,
        Err(why) => {
            crate::toasts::tell(format!("{why:?}"), cx);
            return;
        }
    };
    cx.spawn(async move |cx| {
        let answer = cx
            .background_executor()
            .spawn(async move {
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                runtime.block_on(crate::transport::post(port, ROUTE, body))
            })
            .await;
        let said = match answer {
            Ok((status, _)) if (200..300).contains(&status) => return,
            Ok((status, bytes)) => crate::snooze::refusal_said(status, &bytes),
            Err(why) => why,
        };
        cx.update(|cx| crate::toasts::tell(said, cx));
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::face::InProject;

    fn job(id: &str, project: Option<&str>) -> SessionRecord {
        let mut job = crate::mock::jobs("project-filter").expect("the fixture decodes").into_iter().find(|job| job.id.as_str() == id).expect("a Job of the fixture");
        job.hosted.as_mut().expect("hosted").project = project.map(str::to_string);
        job
    }

    fn held() -> Vec<ProjectRepositories> {
        let project = |id: &str, name: &str| InProject { id: id.to_string().into(), name: name.to_string().into() };
        vec![
            (project("armada", "Armada"), vec!["armada".into()]),
            (project("storefront", "Storefront"), vec!["sample-web".into(), "sample-api".into()]),
            (project("ledger", "Ledger"), Vec::new()),
        ]
    }

    fn rows(entries: &[Entry]) -> Vec<(&'static str, String, Option<String>, Option<bool>)> {
        entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Row(row) => Some((row.id, row.label.to_string(), row.under.as_ref().map(|under| under.to_string()), row.ticked)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_picker_finds_projects_lists_each_with_its_repositories_and_ends_on_no_project() {
        let entries = entries(&held(), &Standing::Mixed);
        assert!(matches!(entries.first(), Some(Entry::Search { label: "Find a project", .. })));
        assert_eq!(
            rows(&entries),
            vec![
                ("project:armada", "Armada".into(), Some("armada".into()), Some(false)),
                ("project:storefront", "Storefront".into(), Some("sample-web, sample-api".into()), Some(false)),
                ("project:ledger", "Ledger".into(), None, Some(false)),
                ("project:", "No project".into(), None, Some(false)),
            ],
            "no row is ticked where the Jobs differ"
        );
        let glyphs: Vec<Option<&str>> = entries.iter().filter_map(|entry| match entry { Entry::Row(row) => Some(row.icon), _ => None }).collect();
        assert_eq!(glyphs, vec![Some("layers"), Some("layers"), Some("layers"), Some("git-branch")]);
    }

    #[test]
    fn the_row_every_job_stands_in_is_ticked() {
        let ticked = |standing: Standing| rows(&entries(&held(), &standing)).into_iter().filter(|row| row.3 == Some(true)).map(|row| row.0).collect::<Vec<_>>();
        assert_eq!(ticked(Standing::In(Some("storefront".into()))), vec!["project:storefront"]);
        assert_eq!(ticked(Standing::In(None)), vec!["project:"], "Jobs in no project tick No project");
        assert!(ticked(Standing::Mixed).is_empty());
    }

    #[test]
    fn jobs_in_one_project_stand_there_and_jobs_in_two_stand_nowhere() {
        let (a, b, c) = (job("mock-01", Some("armada")), job("mock-02", Some("armada")), job("mock-03", Some("storefront")));
        assert_eq!(standing([&a, &b]), Standing::In(Some("armada".into())));
        assert_eq!(standing([&a, &c]), Standing::Mixed);
        let (none_a, none_b) = (job("mock-01", None), job("mock-02", None));
        assert_eq!(standing([&none_a, &none_b]), Standing::In(None));
        assert_eq!(standing([&a, &none_a]), Standing::Mixed, "one in a project and one in none");
        assert_eq!(standing(std::iter::empty()), Standing::Mixed);
    }

    #[test]
    fn a_picker_row_names_its_project_and_the_last_names_none() {
        assert_eq!(project_of("project:storefront"), Some(Some("storefront".to_string())));
        assert_eq!(project_of("project:"), Some(None));
        assert_eq!(project_of("move_project"), None);
    }

    #[test]
    fn the_body_names_the_jobs_and_leaves_the_project_out_for_none() {
        let ids: Vec<SharedString> = vec!["mock-01".into(), "mock-02".into()];
        assert_eq!(body(&ids, Some("storefront")).expect("encodes"), r#"{"session_ids":["mock-01","mock-02"],"project":"storefront"}"#);
        assert_eq!(body(&ids, None).expect("encodes"), r#"{"session_ids":["mock-01","mock-02"]}"#);
    }

    #[test]
    fn only_a_hosted_job_with_a_repository_can_move() {
        assert!(movable(&job("mock-01", None)));
        let mut unhosted = job("mock-01", None);
        unhosted.hosted = None;
        assert!(!movable(&unhosted));
        let mut bare = job("mock-01", None);
        bare.manifest_id = None;
        assert!(!movable(&bare));
        let mut ended = job("mock-15", None);
        ended.state = ipc::SessionState::Ended;
        assert!(movable(&ended), "Fleet moves an ended Job as it moves a live one");
    }
}
