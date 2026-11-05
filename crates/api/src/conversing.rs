//! The routes for words typed for a running Job, and for the call a Job puts to a person.
//! `#939`, `#1367`.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{AskingToRun, ReadThought, TellSession};

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Conversations;
use crate::scoped::InManifest;
use crate::served::Served;

/// Read words for which running Job they are for. **Always 200**: a reading that could not be made
/// is the answer `question`.
pub(crate) async fn read_thought<D: Conversations>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let read: ReadThought = match ipc::decode_body("words to read", &body) {
        Ok(read) => read,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let reading = served.daemon().read_thought(read).await;
    answer(StatusCode::OK, &reading, served.run_id())
}

/// A Job on your words passes words to a live Session. 200 with where they went; the send's own refusal otherwise.
pub(crate) async fn tell_session<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let tell: TellSession = match ipc::decode_body("words for a job", &body) {
        Ok(tell) => tell,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().tell_session(tell, scope.manifest()).await {
        Ok(target) => answer(StatusCode::OK, &target, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Put one call to the person, and **hold the response open until they answer**.
///
/// The one route on this surface that waits on a person. It is reached from the
/// agent door and from nowhere else: the caller is the CLI's own permission
/// prompt, not a model. `#1389`.
pub(crate) async fn ask_the_person<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    // **Placed by the connection**, as the door places a Job on your words: a hosted
    // session's ask goes to its own thread.
    let caller = crate::acting::asking().unwrap_or_else(crate::Caller::unplaceable);
    let asking: AskingToRun = match ipc::decode_body("a permission question", &body) {
        Ok(asking) => asking,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .shared()
        .ask_the_person_at(asking, scope.manifest(), caller)
        .await
    {
        Ok(decided) => answer(StatusCode::OK, &decided, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
