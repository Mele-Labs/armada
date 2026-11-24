//! The board's routes, each a request off the main thread on the port the Fleet connection read: the runs, the manifest, the
//! issue searches and the review lists Fleet already served, and the layout and the ranked issues of the project.

use gpui::{App, AppContext as _, Task};
use ipc::{AppRun, AppRunDraft, AppRunList, BoardLayout, IssueList, IssueQueries, ManifestConfig, NamedAppRun, ProjectBoard, RankedIssues, SaveProjectBoard, SearchIssues};

use crate::link::percent_encoded;
use crate::settings::wire::{body, request, scoped, Ask, Refused};

/// `GET /app_runs?manifest_id=` for a repository's main checkout, `?job_id=` for the worktree a Job holds.
pub fn runs(port: u16, manifest: &str, job: Option<&str>, cx: &App) -> Task<Result<AppRunList, Refused>> {
    let path = match job {
        Some(job) => format!("/app_runs?job_id={}", percent_encoded(job)),
        None => scoped("/app_runs", manifest),
    };
    request(port, Ask::Get(path), "AppRunList", cx)
}

/// `GET /app_runs/draft?manifest_id=`: the run Fleet proposes for a repository that declares none. Saves nothing.
pub fn draft(port: u16, manifest: &str, cx: &App) -> Task<Result<AppRunDraft, Refused>> {
    request(port, Ask::Get(scoped("/app_runs/draft", manifest)), "AppRunDraft", cx)
}

/// `POST /app_runs/start`, `/stop` or `/restart`, by the route's last word.
pub fn run_act(port: u16, route: &'static str, manifest: &str, name: &str, job: Option<&str>, cx: &App) -> Task<Result<AppRun, Refused>> {
    let named = NamedAppRun { name: name.to_string(), job_id: job.map(str::to_string) };
    request(port, Ask::Post(scoped(route, manifest), body(&named)), "AppRun", cx)
}

/// `GET /manifests/:id`: what the repository's manifest declares.
pub fn manifest(port: u16, manifest: &str, cx: &App) -> Task<Result<ManifestConfig, Refused>> {
    request(port, Ask::Get(format!("/manifests/{}", percent_encoded(manifest))), "ManifestConfig", cx)
}

/// `GET /projects/project/:id/board`: the layout the project kept, none where it kept none.
pub fn board(port: u16, project: &str, cx: &App) -> Task<Result<ProjectBoard, Refused>> {
    request(port, Ask::Get(format!("/projects/project/{}/board", percent_encoded(project))), "ProjectBoard", cx)
}

/// `POST /projects/project/:id/board`: keep the layout, whole, or forget it with none.
pub fn save_board(port: u16, project: &str, layout: Option<BoardLayout>, cx: &App) -> Task<Result<ProjectBoard, Refused>> {
    let path = format!("/projects/project/{}/board", percent_encoded(project));
    request(port, Ask::Post(path, body(&SaveProjectBoard { layout })), "ProjectBoard", cx)
}

/// `GET /projects/project/:id/ranked_issues`: the open issues nobody holds and the pull requests of others, each with the
/// features it matched.
pub fn ranked_issues(port: u16, project: &str, cx: &App) -> Task<Result<RankedIssues, Refused>> {
    request(port, Ask::Get(format!("/projects/project/{}/ranked_issues", percent_encoded(project))), "RankedIssues", cx)
}

/// `GET /issue_queries/:repository`: the tracker's own Assigned to me, among what its guide says.
pub fn issue_queries(port: u16, manifest: &str, cx: &App) -> Task<Result<IssueQueries, Refused>> {
    request(port, Ask::Get(format!("/issue_queries/{}", percent_encoded(manifest))), "IssueQueries", cx)
}

/// `POST /issue_queries/:repository/search`: one query in the tracker's own language.
pub fn search_issues(port: u16, manifest: &str, query: &str, cx: &App) -> Task<Result<IssueList, Refused>> {
    let path = format!("/issue_queries/{}/search", percent_encoded(manifest));
    request(port, Ask::Post(path, body(&SearchIssues { query: query.to_string() })), "IssueList", cx)
}

/// `POST /manifest/start_run?manifest_id=`: one Check or Command, run in the repository's main checkout.
pub fn run_command(port: u16, manifest: &str, name: &str, cx: &App) -> Task<Result<ipc::CheckoutRunUnderway, Refused>> {
    let asked = ipc::StartCheckoutRun { name: name.to_string(), workspace: None };
    request(port, Ask::Post(scoped("/manifest/start_run", manifest), body(&asked)), "CheckoutRunUnderway", cx)
}

/// The output a server has written, from `/servers/:id/observe`: the history the socket opens with, read until the server has
/// said all of it or the socket has gone quiet, then the last `most` lines.
pub fn log_tail(port: u16, server: String, most: usize, cx: &App) -> Task<Result<Vec<String>, Refused>> {
    cx.background_spawn(async move {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| Refused::said(why.to_string()))?;
        runtime.block_on(async move {
            let io = crate::transport::upgrade(port, &format!("/servers/{}/observe", percent_encoded(&server))).await.map_err(Refused::said)?;
            let mut messages = crate::frames::Messages::new(io);
            let mut lines: Vec<String> = Vec::new();
            loop {
                // A message half-read when the timer fires would lose its bytes, so the quiet is told by a read that does not come.
                let next = tokio::time::timeout(std::time::Duration::from_millis(600), messages.next()).await;
                let Ok(Some(text)) = next else { break };
                match ipc::decode::<ipc::ServerMessage>("server message", text.as_bytes()) {
                    Ok(ipc::ServerMessage::Lines(more)) => lines.extend(more.lines),
                    Ok(ipc::ServerMessage::Closed(_)) => break,
                    Ok(_) => {}
                    Err(why) => return Err(Refused::said(why.to_string())),
                }
            }
            let from = lines.len().saturating_sub(most);
            Ok(lines.split_off(from))
        })
    })
}
