//! The Features tab's route: a project's library, a request off the main thread on the port the Fleet
//! connection read. `crates/ipc/operations/list_features.toml`.

use gpui::{App, Task};
use ipc::{FeatureAsks, FeatureLibrary, FeaturePage};

use crate::settings::wire::{request, Ask, Refused};

/// `GET /features?project=`.
pub fn library(port: u16, project: &str, cx: &App) -> Task<Result<FeatureLibrary, Refused>> {
    request(port, Ask::Get(format!("/features?project={}", crate::link::percent_encoded(project))), "FeatureLibrary", cx)
}

/// `GET /features/page?feature=<slug>&project=&files=true`: one feature, whole, with each room's files for the floor plan.
/// `crates/ipc/operations/get_feature_page.toml`.
pub fn page(port: u16, project: &str, slug: &str, cx: &App) -> Task<Result<FeaturePage, Refused>> {
    let path = format!("/features/page?feature={}&project={}&files=true", crate::link::percent_encoded(slug), crate::link::percent_encoded(project));
    request(port, Ask::Get(path), "FeaturePage", cx)
}

/// `GET /projects/asks?project=&feature=`: the asks filed under one feature. `crates/ipc/operations/list_feature_asks.toml`.
pub fn asks(port: u16, project: &str, slug: &str, cx: &App) -> Task<Result<FeatureAsks, Refused>> {
    let path = format!("/projects/asks?project={}&feature={}", crate::link::percent_encoded(project), crate::link::percent_encoded(slug));
    request(port, Ask::Get(path), "FeatureAsks", cx)
}

/// `POST /features/upsert`: keep areas and features. `crates/ipc/operations/upsert_features.toml`.
pub fn upsert(port: u16, body: &ipc::UpsertFeatures, cx: &App) -> Task<Result<ipc::FeaturesUpserted, Refused>> {
    request(port, Ask::Post("/features/upsert".into(), crate::settings::wire::body(body)), "FeaturesUpserted", cx)
}

/// `POST /features/change`: merge, rename or move a feature. `crates/ipc/operations/change_feature.toml`.
pub fn change(port: u16, body: &ipc::ChangeFeature, cx: &App) -> Task<Result<ipc::FeatureChanged, Refused>> {
    request(port, Ask::Post("/features/change".into(), crate::settings::wire::body(body)), "FeatureChanged", cx)
}

/// `POST /features/librarian`: run a project's librarian now. `crates/ipc/operations/run_librarian.toml`.
pub fn run_librarian(port: u16, project: &str, cx: &App) -> Task<Result<ipc::LibrarianRan, Refused>> {
    let body = ipc::RunLibrarian { project: project.to_string() };
    request(port, Ask::Post("/features/librarian".into(), crate::settings::wire::body(&body)), "LibrarianRan", cx)
}

/// `POST /features/librarian/ask`: his words to the project's librarian, which does at most one thing with them.
pub fn ask_librarian(port: u16, body: &ipc::AskLibrarian, cx: &App) -> Task<Result<ipc::LibrarianAnswered, Refused>> {
    request(port, Ask::Post("/features/librarian/ask".into(), crate::settings::wire::body(body)), "LibrarianAnswered", cx)
}

/// `POST /features/librarian/undo`: the act of one ask is taken back.
pub fn undo_librarian_ask(port: u16, body: &ipc::UndoLibrarianAsk, cx: &App) -> Task<Result<ipc::LibrarianAskUndone, Refused>> {
    request(port, Ask::Post("/features/librarian/undo".into(), crate::settings::wire::body(body)), "LibrarianAskUndone", cx)
}

/// `POST /features/librarian/refile`: the doc one ask filed moves to another feature.
pub fn refile_librarian_ask(port: u16, body: &ipc::RefileLibrarianAsk, cx: &App) -> Task<Result<ipc::LibrarianAnswered, Refused>> {
    request(port, Ask::Post("/features/librarian/refile".into(), crate::settings::wire::body(body)), "LibrarianAnswered", cx)
}

/// `POST /features/librarian/rank`: the project's might-lists, ranked by the librarian.
pub fn rank_with_librarian(port: u16, project: &str, cx: &App) -> Task<Result<ipc::RankedIssues, Refused>> {
    let body = ipc::RankWithLibrarian { project: project.to_string() };
    request(port, Ask::Post("/features/librarian/rank".into(), crate::settings::wire::body(&body)), "RankedIssues", cx)
}

/// `POST /features/librarian/rank/forget`: the might-lists go back to the ranking by features.
pub fn forget_librarian_ranking(port: u16, project: &str, cx: &App) -> Task<Result<ipc::RankedIssues, Refused>> {
    let body = ipc::RankWithLibrarian { project: project.to_string() };
    request(port, Ask::Post("/features/librarian/rank/forget".into(), crate::settings::wire::body(&body)), "RankedIssues", cx)
}

/// `POST /features/librarian/stop`: the project's running run stops before its next step. `crates/ipc/operations/stop_librarian.toml`.
pub fn stop_librarian(port: u16, project: &str, cx: &App) -> Task<Result<ipc::LibrarianStopping, Refused>> {
    let body = ipc::StopLibrarian { project: project.to_string() };
    request(port, Ask::Post("/features/librarian/stop".into(), crate::settings::wire::body(&body)), "LibrarianStopping", cx)
}

/// `GET /features/overview?project=`: a project's map, busiest features and glossary. `crates/ipc/operations/get_project_overview.toml`.
pub fn overview(port: u16, project: &str, cx: &App) -> Task<Result<ipc::ProjectOverview, Refused>> {
    request(port, Ask::Get(format!("/features/overview?project={}", crate::link::percent_encoded(project))), "ProjectOverview", cx)
}

/// `GET /features/since?project=&since=`: what moved since he looked, by feature. `crates/ipc/operations/get_since_you_looked.toml`.
pub fn since(port: u16, project: &str, since: chrono::DateTime<chrono::Utc>, cx: &App) -> Task<Result<ipc::SinceYouLooked, Refused>> {
    let at = since.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let path = format!("/features/since?project={}&since={}", crate::link::percent_encoded(project), crate::link::percent_encoded(&at));
    request(port, Ask::Get(path), "SinceYouLooked", cx)
}

/// `GET /features/who?project=`: who has an open pull request or a running Job on which feature. `crates/ipc/operations/get_whos_on_what.toml`.
pub fn who(port: u16, project: &str, cx: &App) -> Task<Result<ipc::WhosOnWhat, Refused>> {
    request(port, Ask::Get(format!("/features/who?project={}", crate::link::percent_encoded(project))), "WhosOnWhat", cx)
}

/// `POST /features/retire`: the owner's answer to the librarian's proposal to retire a feature. `crates/ipc/operations/answer_retire.toml`.
pub fn retire(port: u16, body: &ipc::AnswerRetire, cx: &App) -> Task<Result<ipc::RetireAnswered, Refused>> {
    request(port, Ask::Post("/features/retire".into(), crate::settings::wire::body(body)), "RetireAnswered", cx)
}

/// `POST /features/docs/check`: put each doc of a feature to the model against its code. `crates/ipc/operations/check_docs.toml`.
pub fn check_docs(port: u16, body: &ipc::CheckDocs, cx: &App) -> Task<Result<ipc::DocsChecked, Refused>> {
    request(port, Ask::Post("/features/docs/check".into(), crate::settings::wire::body(body)), "DocsChecked", cx)
}

/// `POST /features/docs/dismiss`: quiet a doc's flag until the doc or the code changes. `crates/ipc/operations/dismiss_doc_flag.toml`.
pub fn dismiss_doc_flag(port: u16, body: &ipc::DocFlag, cx: &App) -> Task<Result<ipc::DocFlag, Refused>> {
    request(port, Ask::Post("/features/docs/dismiss".into(), crate::settings::wire::body(body)), "DocFlag", cx)
}

/// `POST /features/docs/job`: start a Job that asks him whether the doc or the code is right. `crates/ipc/operations/start_doc_job.toml`.
pub fn start_doc_job(port: u16, body: &ipc::DocFlag, cx: &App) -> Task<Result<ipc::DocJobStarted, Refused>> {
    request(port, Ask::Post("/features/docs/job".into(), crate::settings::wire::body(body)), "DocJobStarted", cx)
}

/// `GET /features/start?project=`: the modules and docs a project with no feature of its own starts from.
/// `crates/ipc/operations/get_library_start.toml`.
pub fn library_start(port: u16, project: &str, cx: &App) -> Task<Result<ipc::LibraryStart, Refused>> {
    request(port, Ask::Get(format!("/features/start?project={}", crate::link::percent_encoded(project))), "LibraryStart", cx)
}

/// `POST /features/start/draft`: each module of the project kept as a pending feature. `crates/ipc/operations/draft_module_features.toml`.
pub fn draft_modules(port: u16, project: &str, cx: &App) -> Task<Result<ipc::ModulesDrafted, Refused>> {
    let body = ipc::DraftModules { project: project.to_string() };
    request(port, Ask::Post("/features/start/draft".into(), crate::settings::wire::body(&body)), "ModulesDrafted", cx)
}
