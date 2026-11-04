//! `get_shown_file`: a file a session showed, read for the phone, which cannot
//! read the Mac's disk as Bridge's panel does.

use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use ipc::WireError;
use serde::Deserialize;

use crate::answers::{answer, problem, refused};
use crate::daemon::{Sessions, ShownFile};
use crate::served::Served;

const SHOWN_TOO_LARGE: &str = "fleet.shown_file_too_large";

/// `?session_id=` and `?path=`, the path as the session's attachment holds it.
#[derive(Deserialize)]
pub(crate) struct WhichShown {
    session_id: String,
    path: String,
}

/// 200 with `ShownText`, a picture's bytes under its type, or `ShownVideo` for a video; 404 for a path
/// the session did not show; 413 for a file over what the phone is sent of its kind.
pub(crate) async fn get_shown_file<D: Sessions>(
    State(served): State<Served<D>>,
    Query(which): Query<WhichShown>,
) -> Response {
    let read = served
        .daemon()
        .get_shown_file(ipc::SessionId::carried(which.session_id), which.path.clone())
        .await;
    match read {
        Ok(ShownFile::Text(text)) => answer(StatusCode::OK, &text, served.run_id()),
        Ok(ShownFile::Picture { media_type, bytes }) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, media_type)], bytes).into_response()
        }
        Ok(ShownFile::Video { media_type, bytes }) => {
            answer(StatusCode::OK, &ipc::ShownVideo { path: which.path, media_type, bytes }, served.run_id())
        }
        Ok(ShownFile::TooLarge { bytes }) => problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            &WireError::raised(
                SHOWN_TOO_LARGE,
                format!(
                    "This file is {bytes} bytes. The phone opens text of up to 1 MiB, pictures of up to 8 MiB \
                     and videos of up to 256 MiB."
                ),
                served.run_id().clone(),
            ),
        ),
        Err(refusal) => refused(refusal),
    }
}
