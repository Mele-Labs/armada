//! Share: `share_package`, `read_package`, `add_package`, `list_unbound_pieces` and
//! `bind_unbound_piece`. `docs/concepts/kit.md`, *Share*.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::served::Served;

/// Make one package from the pieces chosen.
pub(crate) async fn share_package<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::SharePieces = match ipc::decode("the pieces to share", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().share_package(asked).await {
        Ok(made) => answer(StatusCode::OK, &made, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// List what a package would add. Writes nothing.
pub(crate) async fn read_package<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::PackageText = match ipc::decode("a package to read", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().read_package(asked).await {
        Ok(read) => answer(StatusCode::OK, &read, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Add every piece a package holds to the person's Kit.
pub(crate) async fn add_package<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::AddPackage = match ipc::decode("a package to add", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().add_package(asked).await {
        Ok(added) => answer(StatusCode::OK, &added, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The pieces kept unbound.
pub(crate) async fn list_unbound_pieces<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_unbound_pieces().await {
        Ok(pieces) => answer(StatusCode::OK, &pieces, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Choose the repository an unbound piece is for.
pub(crate) async fn bind_unbound_piece<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::BindUnboundPiece = match ipc::decode("a piece to bind", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().bind_unbound_piece(asked).await {
        Ok(added) => answer(StatusCode::OK, &added, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
