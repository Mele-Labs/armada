//! The adversarial review's three routes: start one, and send or dismiss a finding. A subject is in
//! the body, never the path, because a pull request or a document may become one later.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{AdversarialReviews, Refusal};
use crate::served::Served;

fn answered(done: Result<ipc::AdversarialReview, Refusal>, served_run: &ipc::RunId) -> Response {
    match done {
        Ok(review) => answer(StatusCode::OK, &review, served_run),
        Err(refusal) => refused(refusal),
    }
}

/// The person presses the review. 200 with the review, running or kept; 404 for no such Job or no plan; 409 for a
/// plan that is not the current version.
pub(crate) async fn start_adversarial_review<D: AdversarialReviews>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body::<ipc::StartAdversarialReview>("an adversarial review to start", &body) {
        Ok(start) => answered(served.shared().start_adversarial_review(start).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

/// The person sends a finding to the agent as his message. 200 with the review; 404 for a finding it does not hold.
pub(crate) async fn send_finding<D: AdversarialReviews>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body::<ipc::SendFinding>("a finding to send", &body) {
        Ok(send) => answered(served.shared().send_finding(send).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

/// The person dismisses a finding; the agent is told nothing. 200 with the review.
pub(crate) async fn dismiss_finding<D: AdversarialReviews>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body::<ipc::DismissFinding>("a finding to dismiss", &body) {
        Ok(dismiss) => answered(served.shared().dismiss_finding(dismiss).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}
