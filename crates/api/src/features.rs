//! A project's feature library over the wire. `docs/concepts/feature-catalog.md`.
//!
//! Every route is a decode and one call. **The caller is the connection** a Job's call arrived on, so a read with no
//! `project` is answered for the project that Job works in.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;

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

/// `?project=` on `list_features`.
#[derive(serde::Deserialize)]
pub(crate) struct OfProject {
    #[serde(default)]
    project: Option<String>,
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

pub(crate) async fn get_feature_page<D: Features>(State(served): State<Served<D>>, Query(asked): Query<OfFeature>) -> Response {
    match served.daemon().get_feature_page(crate::acting::asking(), asked.project, asked.feature).await {
        Ok(page) => answer(StatusCode::OK, &page, served.run_id()),
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

pub(crate) async fn upsert_features<D: Features>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let upsert: ipc::UpsertFeatures = match ipc::decode_body("areas and features to keep", &body) {
        Ok(upsert) => upsert,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().upsert_features(upsert).await {
        Ok(kept) => answer(StatusCode::OK, &kept, served.run_id()),
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

pub(crate) async fn get_first_edit_searches<D: Features>(State(served): State<Served<D>>, Query(asked): Query<Weeks>) -> Response {
    match served.daemon().get_first_edit_searches(asked.weeks).await {
        Ok(figure) => answer(StatusCode::OK, &figure, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
