//! The Logbook's reads and acts. `docs/concepts/logbook.md`.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Logbook;
use crate::served::Served;

/// `?repository=`, `?section=` and `?state=` on `get_logbook`. A value that is not one of a set's
/// is refused, as any query that will not read is.
#[derive(Deserialize)]
pub(crate) struct Listing {
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    section: Option<ipc::LogbookSection>,
    #[serde(default)]
    state: Option<ipc::LogbookState>,
}

/// The entry an act names.
#[derive(Deserialize)]
pub(crate) struct NamedEntry {
    entry_id: String,
}

/// Every entry asked for, of every state.
pub(crate) async fn get_logbook<D: Logbook>(State(served): State<Served<D>>, Query(listing): Query<Listing>) -> Response {
    match served.daemon().get_logbook(listing.repository, listing.section, listing.state).await {
        Ok(logbook) => answer(StatusCode::OK, &logbook, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Replace an entry's text; answers with the entry.
pub(crate) async fn edit_logbook_entry<D: Logbook>(
    State(served): State<Served<D>>,
    Path(NamedEntry { entry_id }): Path<NamedEntry>,
    body: Bytes,
) -> Response {
    let edit: ipc::EditLogbookEntry = match ipc::decode("an entry's new text", &body) {
        Ok(edit) => edit,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().edit_logbook_entry(entry_id, edit).await {
        Ok(entry) => answer(StatusCode::OK, &entry, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Lock an entry or let it go; answers with the entry.
pub(crate) async fn lock_logbook_entry<D: Logbook>(
    State(served): State<Served<D>>,
    Path(NamedEntry { entry_id }): Path<NamedEntry>,
    body: Bytes,
) -> Response {
    let lock: ipc::LockLogbookEntry = match ipc::decode("whether to lock an entry", &body) {
        Ok(lock) => lock,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().lock_logbook_entry(entry_id, lock).await {
        Ok(entry) => answer(StatusCode::OK, &entry, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Forget an entry for good.
pub(crate) async fn forget_logbook_entry<D: Logbook>(
    State(served): State<Served<D>>,
    Path(NamedEntry { entry_id }): Path<NamedEntry>,
) -> Response {
    match served.daemon().forget_logbook_entry(entry_id).await {
        Ok(forgotten) => answer(StatusCode::OK, &forgotten, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Start tidying the Logbook; answers at once that a pass is running.
pub(crate) async fn tidy_logbook<D: Logbook>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let tidy: ipc::TidyLogbook = match ipc::decode("which Logbook to tidy", &body) {
        Ok(tidy) => tidy,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().tidy_logbook(tidy).await {
        Ok(tidied) => answer(StatusCode::OK, &tidied, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Teach an entry by hand; it stands at once.
pub(crate) async fn teach_logbook_entry<D: Logbook>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let teach: ipc::TeachLogbookEntry = match ipc::decode("an entry to teach", &body) {
        Ok(teach) => teach,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().teach_logbook_entry(teach).await {
        Ok(entry) => answer(StatusCode::OK, &entry, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
