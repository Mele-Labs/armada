//! settings.json, read and saved: `limiting`'s shape over the whole file.
//!
//! **No `Resolved` extractor and no path parameter.** The settings are the
//! Machine's, beside `/limits` and for its reason: nothing about them is a Job's.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::SaveSettings;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Settings;
use crate::served::Served;

/// Every setting, and the last hand edit refused, if one stands.
pub(crate) async fn get_settings<D: Settings>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_settings().await {
        Ok(settings) => answer(StatusCode::OK, &settings, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Change keys and answer with every setting. **A body that is not a set of
/// changes is undecodable**; a key or value the table refuses is the daemon's
/// refusal, by name, and nothing is written.
pub(crate) async fn save_settings<D: Settings>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let save: SaveSettings = match ipc::decode("settings to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_settings(save).await {
        Ok(settings) => answer(StatusCode::OK, &settings, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Whether this Mac serves the phone app.
pub(crate) async fn get_phone<D: Settings>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_phone().await {
        Ok(phone) => answer(StatusCode::OK, &phone, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Opt this Mac in or out of serving the phone, and answer where it stands.
pub(crate) async fn set_phone<D: Settings>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let set: ipc::SetPhoneGateway = match ipc::decode("whether to serve the phone", &body) {
        Ok(set) => set,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().set_phone(set).await {
        Ok(phone) => answer(StatusCode::OK, &phone, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
