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

/// A copy this Mac holds of another Mac's shared project, as the picker lists it: the project with its repositories
/// here, and the name of the Mac that holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Copied {
    pub project: ProjectRepositories,
    pub from: String,
}

/// The picker's rows: the field that finds a project, each of his projects with its repositories on a line under
/// the name, then the copies of another Mac's projects with that Mac named on the line, then No project. The row
/// every Job already stands in is ticked.
pub fn entries(projects: &[ProjectRepositories], copies: &[Copied], standing: &Standing) -> Vec<Entry> {
    let ticked = |project: Option<&str>| matches!(standing, Standing::In(held) if held.as_deref() == project);
    let mut entries = vec![Entry::Search { label: "Find a project", to_rule: false }];
    let own = projects.iter().map(|one| (one, None));
    let away = copies.iter().map(|one| (&one.project, Some(one.from.as_str())));
    for ((project, repositories), from) in own.chain(away) {
        let row = Row::new(id_of(format!("project:{}", project.id)), project.name.clone()).icon("layers").ticked(ticked(Some(project.id.as_ref())));
        let mut under = repositories.iter().map(SharedString::as_ref).collect::<Vec<&str>>().join(", ");
        if let Some(mac) = from {
            if !under.is_empty() {
                under.push_str(" · ");
            }
            under.push_str(&format!("from {mac}"));
        }
        entries.push(Entry::Row(if under.is_empty() { row } else { row.under(under) }));
    }
    entries.push(Entry::Row(Row::new(NONE, "No project").icon("git-branch").ticked(ticked(None))));
    entries
}

/// The project a picker row names: `Some(None)` for No project, and none for a row that is not one of the picker's.
pub fn project_of(row: &str) -> Option<Option<String>> {
    let id = row.strip_prefix("project:")?;
    Some((!id.is_empty()).then(|| id.to_string()))
}

/// His own projects as the title bar holds them, each with its repositories. Mock data has the bar's scene when a
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

/// The copies of another Mac's projects the title bar holds, each with the name of the Mac that holds it. A Mac the
/// list does not know is said as another Mac.
pub fn copies(cx: &App) -> Vec<Copied> {
    let Some(bar) = crate::titlebar::live(cx) else {
        return Vec::new();
    };
    let face = &bar.read(cx).face;
    copied(&face.copies, &face.repositories, crate::pairing::macs(cx))
}

/// [`copies`] from what it reads.
fn copied(copies: &[crate::titlebar::ProjectCopy], repositories: &[crate::titlebar::Repository], macs: &[ipc::Mac]) -> Vec<Copied> {
    let projects: Vec<crate::titlebar::Project> = copies.iter().map(|one| one.project.clone()).collect();
    projects_of(&projects, repositories)
        .into_iter()
        .zip(copies)
        .map(|(project, one)| Copied { project, from: crate::job_view::name_of(macs, &one.mac, false) })
        .collect()
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
        let entries = entries(&held(), &[], &Standing::Mixed);
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

    fn copy(id: &str, name: &str, mac: &str, members: Vec<crate::titlebar::Member>) -> crate::titlebar::ProjectCopy {
        crate::titlebar::ProjectCopy {
            mac: mac.to_string().into(),
            project: crate::titlebar::Project { id: id.to_string().into(), name: name.to_string().into(), members, tracker: None, kept: None, look: None },
        }
    }

    fn macs() -> Vec<ipc::Mac> {
        let mac = |id: &str, name: &str, this_mac: bool| ipc::Mac { id: id.into(), name: name.into(), this_mac, ..Default::default() };
        vec![mac("mac-here", "Studio", true), mac("mac-pro", "Nicks-MacBook-Pro-2026", false)]
    }

    #[test]
    fn a_copy_is_listed_after_his_own_projects_with_its_mac_named_and_ticks_for_a_job_in_it() {
        let served = crate::mock::repositories();
        let repositories = crate::titlebar::repositories_of(&served, &[]);
        let copies = vec![
            copy("armada-pro", "Armada", "mac-pro", vec![crate::titlebar::Member { repository: 0, folder: None }]),
            copy("lost", "Lost", "mac-gone", Vec::new()),
        ];
        let copies = copied(&copies, &repositories, &macs());
        assert_eq!(copies[0].from, "Nicks-MacBook-Pro-2026");
        assert_eq!(copies[1].from, "another Mac", "a Mac the list does not know");
        let entries = entries(&held(), &copies, &Standing::In(Some("armada-pro".into())));
        let listed = rows(&entries);
        let first = &repositories[0].name;
        assert_eq!(
            listed[3..],
            [
                ("project:armada-pro", "Armada".into(), Some(format!("{first} · from Nicks-MacBook-Pro-2026")), Some(true)),
                ("project:lost", "Lost".into(), Some("from another Mac".into()), Some(false)),
                ("project:", "No project".into(), None, Some(false)),
            ],
            "copies follow his own, a copy with no repository here is still listed, and No project stays last"
        );
        let glyphs: Vec<Option<&str>> = entries.iter().filter_map(|entry| match entry { Entry::Row(row) => Some(row.icon), _ => None }).collect();
        assert_eq!(glyphs[3], Some("layers"));
    }

    #[test]
    fn the_switcher_still_leaves_a_copy_out() {
        let served = crate::mock::repositories();
        let mut read = crate::mock::projects();
        let mut other = read.projects[0].clone();
        (other.id, other.mac) = ("armada-pro".to_string(), Some("mac-pro".to_string()));
        read.projects.push(other);
        let switcher: Vec<String> = crate::titlebar::projects_of(&served, &read).iter().map(|one| one.id.to_string()).collect();
        assert!(!switcher.contains(&"armada-pro".to_string()), "the switcher cannot scope the window to a copy");
        let copies: Vec<String> = crate::titlebar::copies_of(&served, &read).iter().map(|one| one.project.id.to_string()).collect();
        assert_eq!(copies, vec!["armada-pro".to_string()]);
    }

    #[test]
    fn the_row_every_job_stands_in_is_ticked() {
        let ticked = |standing: Standing| rows(&entries(&held(), &[], &standing)).into_iter().filter(|row| row.3 == Some(true)).map(|row| row.0).collect::<Vec<_>>();
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
