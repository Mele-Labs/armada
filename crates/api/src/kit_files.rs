//! A Kit piece's file: read, saved and opened elsewhere. Machine-wide, as the inventory the piece
//! is found in is, and no body or query carries a path.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{GetKitFile, OpenKitPiece, SaveKitFile};

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::served::Served;

/// One piece's file, whole.
pub(crate) async fn get_kit_file<D: Queries>(
    State(served): State<Served<D>>,
    Query(asked): Query<GetKitFile>,
) -> Response {
    match served.daemon().get_kit_file(asked).await {
        Ok(file) => answer(StatusCode::OK, &file, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A corrected file to disk, where it is still what the edit read.
pub(crate) async fn save_kit_file<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: SaveKitFile = match ipc::decode_body("a file to save", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_kit_file(asked).await {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A piece in the person's editor, or shown in Finder.
pub(crate) async fn open_kit_piece<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: OpenKitPiece = match ipc::decode_body("a piece to open", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().open_kit_piece(asked).await {
        Ok(opened) => answer(StatusCode::OK, &opened, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
