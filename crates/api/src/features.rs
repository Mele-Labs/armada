//! A project's feature library over the wire. `docs/concepts/feature-catalog.md`.
//!
//! Every route is a decode and one call. **The caller is the connection** a Job's call arrived on, so a read with no
//! `project` is answered for the project that Job works in.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Features;
use crate::served::Served;

/// `?project=&feature=` on the two reads of one feature.
#[derive(serde::Deserialize)]
pub(crate) struct OfFeature {
    #[serde(default)]
    project: Option<String>,
    feature: String,
}

/// `?project=&feature=&section=&files=` on `get_feature_page`.
#[derive(serde::Deserialize)]
pub(crate) struct OfFeaturePage {
    #[serde(default)]
    project: Option<String>,
    feature: String,
    #[serde(default)]
    section: Option<String>,
    /// The floor plan's rooms filled with their files. Only Bridge asks; an agent's read stays small.
    #[serde(default)]
    files: bool,
}

/// `?project=` on `list_features`.
#[derive(serde::Deserialize)]
pub(crate) struct OfProject {
    #[serde(default)]
    project: Option<String>,
}

/// `?project=&words=` on `get_features_by_meaning`.
#[derive(serde::Deserialize)]
pub(crate) struct OfWords {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    words: String,
}

/// `?project=&since=` on `get_since_you_looked`.
#[derive(serde::Deserialize)]
pub(crate) struct OfSince {
    #[serde(default)]
    project: Option<String>,
    since: String,
}

/// `?path=&project=&manifest_id=` on `get_features_of_path`.
#[derive(serde::Deserialize)]
pub(crate) struct OfPath {
    path: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    manifest_id: Option<String>,
}

/// `?weeks=` on `get_first_edit_searches`.
#[derive(serde::Deserialize)]
pub(crate) struct Weeks {
    #[serde(default)]
    weeks: Option<u32>,
}

pub(crate) async fn list_features<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfProject>) -> Response {
    match served.daemon().list_features(crate::acting::asking(), asked.project).await {
        Ok(library) => answer(StatusCode::OK, &library, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_features_by_meaning<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfWords>) -> Response {
    match served.daemon().get_features_by_meaning(crate::acting::asking(), asked.project, asked.words).await {
        Ok(found) => answer(StatusCode::OK, &found, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_feature_page<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfFeaturePage>) -> Response {
    match served.daemon().get_feature_page(crate::acting::asking(), asked.project, asked.feature, asked.section, asked.files).await {
        Ok(page) => answer(StatusCode::OK, &page, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_glossary<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfProject>) -> Response {
    match served.daemon().get_glossary(crate::acting::asking(), asked.project).await {
        Ok(glossary) => answer(StatusCode::OK, &glossary, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_project_overview<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfProject>) -> Response {
    match served.daemon().get_project_overview(crate::acting::asking(), asked.project).await {
        Ok(overview) => answer(StatusCode::OK, &overview, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_since_you_looked<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfSince>) -> Response {
    match served.daemon().get_since_you_looked(crate::acting::asking(), asked.project, asked.since).await {
        Ok(looked) => answer(StatusCode::OK, &looked, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_whos_on_what<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfProject>) -> Response {
    match served.daemon().get_whos_on_what(crate::acting::asking(), asked.project).await {
        Ok(who) => answer(StatusCode::OK, &who, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_feature_brief<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfFeature>) -> Response {
    match served.daemon().get_feature_brief(crate::acting::asking(), asked.project, asked.feature).await {
        Ok(brief) => answer(StatusCode::OK, &brief, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_features_of_path<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfPath>) -> Response {
    match served.daemon().get_features_of_path(crate::acting::asking(), asked.project, asked.manifest_id, asked.path).await {
        Ok(found) => answer(StatusCode::OK, &found, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A Job rewrites sections of a feature. **The session is placed by the connection**, as `all_done`'s is.
pub(crate) async fn write_feature_sections<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let write: ipc::WriteFeatureSections = match ipc::decode_body("sections of a feature", &body) {
        Ok(write) => write,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().write_feature_sections(crate::acting::asking(), write).await {
        Ok(written) => answer(StatusCode::OK, &written, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The Operator or the owner keeps areas and features. **Placed as `change_feature` is.**
pub(crate) async fn upsert_features<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let upsert: ipc::UpsertFeatures = match ipc::decode_body("areas and features to keep", &body) {
        Ok(upsert) => upsert,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().upsert_features(crate::acting::asking(), upsert).await {
        Ok(kept) => answer(StatusCode::OK, &kept, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner or the Operator answers a proposal to retire a feature. **Placed as `upsert_features` is.**
pub(crate) async fn answer_retire<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let retiring: ipc::AnswerRetire = match ipc::decode_body("an answer to a proposal to retire a feature", &body) {
        Ok(retiring) => retiring,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().answer_retire(crate::acting::asking(), retiring).await {
        Ok(answered) => answer(StatusCode::OK, &answered, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner's Check on a feature's docs, or the Operator's. **Placed as `answer_retire` is.**
pub(crate) async fn check_docs<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let check: ipc::CheckDocs = match ipc::decode_body("a feature whose docs to check", &body) {
        Ok(check) => check,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().check_docs(crate::acting::asking(), check).await {
        Ok(checked) => answer(StatusCode::OK, &checked, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner's Dismiss on a doc that disagrees with the code. **Placed as `answer_retire` is.**
pub(crate) async fn dismiss_doc_flag<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let flag: ipc::DocFlag = match ipc::decode_body("a doc whose flag to dismiss", &body) {
        Ok(flag) => flag,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().dismiss_doc_flag(crate::acting::asking(), flag).await {
        Ok(dismissed) => answer(StatusCode::OK, &dismissed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner's Start a Job on a doc that disagrees with the code. **Placed as `answer_retire` is.**
pub(crate) async fn start_doc_job<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let flag: ipc::DocFlag = match ipc::decode_body("a doc to start a Job on", &body) {
        Ok(flag) => flag,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_doc_job(crate::acting::asking(), flag).await {
        Ok(started) => answer(StatusCode::OK, &started, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_library_start<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfProject>) -> Response {
    match served.daemon().get_library_start(crate::acting::asking(), asked.project).await {
        Ok(start) => answer(StatusCode::OK, &start, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner's Draft as Pending on a project with no feature of its own, or the Operator's. **Placed as `answer_retire` is.**
pub(crate) async fn draft_module_features<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let draft: ipc::DraftModules = match ipc::decode_body("a project whose modules to draft", &body) {
        Ok(draft) => draft,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().draft_module_features(crate::acting::asking(), draft).await {
        Ok(drafted) => answer(StatusCode::OK, &drafted, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner groups the Jobs he picked in the Jobs list.
pub(crate) async fn start_sub_project<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let start: ipc::StartSubProject = match ipc::decode_body("a name and the Jobs to group under it", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().start_sub_project(start).await {
        Ok(started) => answer(StatusCode::OK, &started, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_sub_projects<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfProject>) -> Response {
    match served.daemon().list_sub_projects(asked.project).await {
        Ok(listed) => answer(StatusCode::OK, &listed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn ungroup_sub_project<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let ungroup: ipc::UngroupSubProject = match ipc::decode_body("a sub project to ungroup", &body) {
        Ok(ungroup) => ungroup,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().ungroup_sub_project(ungroup).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(refusal) => refused(refusal),
    }
}

/// The Operator seeds a project. **Placed by its connection**, as `raise_card` is.
pub(crate) async fn seed_features<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let seed: ipc::SeedFeatures = match ipc::decode_body("a project to seed", &body) {
        Ok(seed) => seed,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().seed_features(crate::acting::asking(), seed).await {
        Ok(seeded) => answer(StatusCode::OK, &seeded, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner or the Operator presses Catch up: the project's librarian runs now. **Placed as `upsert_features` is.**
pub(crate) async fn run_librarian<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let run: ipc::RunLibrarian = match ipc::decode_body("a project to run the librarian for", &body) {
        Ok(run) => run,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().run_librarian(crate::acting::asking(), run).await {
        Ok(ran) => answer(StatusCode::OK, &ran, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner or the Operator presses Stop on a running Catch up. **Placed as `run_librarian` is.**
pub(crate) async fn stop_librarian<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let stop: ipc::StopLibrarian = match ipc::decode_body("a project whose librarian to stop", &body) {
        Ok(stop) => stop,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().stop_librarian(crate::acting::asking(), stop).await {
        Ok(stopping) => answer(StatusCode::OK, &stopping, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// He asks the librarian from the Ask box. **Placed as `run_librarian` is.**
pub(crate) async fn ask_librarian<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::AskLibrarian = match ipc::decode_body("an ask of the librarian", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().ask_librarian(crate::acting::asking(), asked).await {
        Ok(answered) => answer(StatusCode::OK, &answered, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// He presses Undo under an answer. **Placed as `run_librarian` is.**
pub(crate) async fn undo_librarian_ask<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::UndoLibrarianAsk = match ipc::decode_body("an ask to undo", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().undo_librarian_ask(crate::acting::asking(), asked).await {
        Ok(answered) => answer(StatusCode::OK, &answered, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// He presses Move under a filed doc. **Placed as `run_librarian` is.**
pub(crate) async fn refile_librarian_ask<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::RefileLibrarianAsk = match ipc::decode_body("a filed doc to move", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().refile_librarian_ask(crate::acting::asking(), asked).await {
        Ok(answered) => answer(StatusCode::OK, &answered, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// He presses Rank with the librarian. **Placed as `run_librarian` is.**
pub(crate) async fn rank_with_librarian<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::RankWithLibrarian = match ipc::decode_body("a project to rank", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().rank_with_librarian(crate::acting::asking(), asked).await {
        Ok(answered) => answer(StatusCode::OK, &answered, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// He presses Reset on the ranked lists. **Placed as `run_librarian` is.**
pub(crate) async fn forget_librarian_ranking<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::RankWithLibrarian = match ipc::decode_body("a project whose ranking to forget", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().forget_librarian_ranking(crate::acting::asking(), asked).await {
        Ok(answered) => answer(StatusCode::OK, &answered, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The mod's hook asks which pages a Job is owed. **The Job is named in the body**: the hook runs in the harness's process,
/// not on the agent's connection.
pub(crate) async fn hand_over_features<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let hand: ipc::HandOverFeatures = match ipc::decode_body("feature pages to hand over", &body) {
        Ok(hand) => hand,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().hand_over_features(hand).await {
        Ok(handed) => answer(StatusCode::OK, &handed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The Operator or the owner merges, renames or moves a feature. **A door call is placed by its connection**, as `raise_card` is; a call by no door is the owner's, from Bridge.
pub(crate) async fn change_feature<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let change: ipc::ChangeFeature = match ipc::decode_body("a change to a feature", &body) {
        Ok(change) => change,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().change_feature(crate::acting::asking(), change).await {
        Ok(changed) => answer(StatusCode::OK, &changed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The mod's hook asks before a `git commit` runs. **The Job is named in the body**, as `hand_over_features`'s is.
pub(crate) async fn check_commit_feature<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let check: ipc::CheckCommitFeature = match ipc::decode_body("a commit about to run", &body) {
        Ok(check) => check,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().check_commit_feature(check).await {
        Ok(checked) => answer(StatusCode::OK, &checked, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_first_edit_searches<D: Features>(State(served): State<Served<D>>, Query(asked): Query<Weeks>) -> Response {
    match served.daemon().get_first_edit_searches(asked.weeks).await {
        Ok(figure) => answer(StatusCode::OK, &figure, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
