//! The Helm routes: its Session found, spoken into, started over, and the questions it puts to a
//! person. `#939`, `#1367`. **Helm is a hosted Session**, so its thread is a Session's.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{AskHelm, AskingToRun, ReadThought, TellSession, ThoughtSent};

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Conversations;
use crate::scoped::InManifest;
use crate::served::Served;

/// The repository's live Helm Session and its thread, or none.
pub(crate) async fn get_helm_session<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.shared().get_helm_session(scope.manifest()).await {
        Ok(session) => answer(StatusCode::OK, &session, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The session as one quotable record. `#1367`.
pub(crate) async fn get_helm_debug_info<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().get_helm_debug_info(scope.manifest()).await {
        Ok(info) => answer(StatusCode::OK, &info, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Take a message for the repository's Helm Session, starting it if none is live. 200 with the
/// Session's record; the reply is the Session's thread.
pub(crate) async fn ask_helm<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: AskHelm = match ipc::decode_body("a message to Helm", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().ask_helm(asked, scope.manifest()).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Read words for where they belong. **Always 200**: a reading that could not be made is the
/// answer `question`, so the words go to Helm and are never lost.
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

/// Say that words were delivered to a Session or a Job, so the thread keeps where they went.
pub(crate) async fn record_thought_sent<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let sent: ThoughtSent = match ipc::decode_body("where words went", &body) {
        Ok(sent) => sent,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().record_thought_sent(sent, scope.manifest()).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Helm passes words to a live Session. 200 with where they went; the send's own refusal otherwise.
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
    // **Placed by the connection**, as the door places a Helm session: a hosted
    // session's ask goes to its own thread and not to Helm's dock.
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

/// What is waiting on a person right now, Fleet-wide.
pub(crate) async fn list_helm_calls<D: Conversations>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_helm_calls().await {
        Ok(waiting) => answer(StatusCode::OK, &waiting, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A person's answer, into the call that is waiting on it.
pub(crate) async fn answer_helm_call<D: Conversations>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let said: ipc::AnswerHelmCall = match ipc::decode_body("an answer to a Helm call", &body) {
        Ok(said) => said,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().answer_helm_call(said).await {
        Ok(waiting) => answer(StatusCode::OK, &waiting, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// End the repository's live Helm Session.
pub(crate) async fn start_helm_fresh<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().start_helm_fresh(scope.manifest()).await {
        Ok(session) => answer(StatusCode::OK, &session, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
