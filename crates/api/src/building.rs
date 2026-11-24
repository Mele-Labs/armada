//! The build Fleet runs on: the read, the restart onto another, the revert to the one before, and the
//! snapshot of picked Jobs with its swap and keep.
//!
//! **No `Resolved` extractor and no path parameter**, `limiting`'s reason:
//! the build is Fleet's own, and no Job's. The change answers 202, because it
//! starts a restart that stops the Fleet answering it.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{AnswerReadyToTry, ChangeFleetBuild, MarkBuildNoteLookedAt, PickFleetBuildJob, SendBuildNoteFeedback, SkipPreviewRefresh, UpdateMac};

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, OtherMacs, Queries};
use crate::served::Served;

/// Which build Fleet runs on, and where it stands against `origin/main`.
pub(crate) async fn get_fleet_build<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_fleet_build().await {
        Ok(report) => answer(StatusCode::OK, &report, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The branches Jobs offered for the preview and the one Refresh row.
pub(crate) async fn get_preview_offers<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_preview_offers().await {
        Ok(offers) => answer(StatusCode::OK, &offers, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Skip the Refresh row with this id until the set of ready offers changes.
pub(crate) async fn skip_preview_refresh<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: SkipPreviewRefresh = match ipc::decode_body("a Refresh row to skip", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().skip_preview_refresh(asked).await {
        Ok(skipped) => answer(StatusCode::OK, &skipped, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The notes of the branches the running preview build holds.
pub(crate) async fn get_build_notes<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_build_notes().await {
        Ok(notes) => answer(StatusCode::OK, &notes, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Mark the note of a (branch, tip) looked at, and answer the notes now.
pub(crate) async fn mark_build_note_looked_at<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: MarkBuildNoteLookedAt = match ipc::decode_body("a note to mark looked at", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().mark_build_note_looked_at(asked).await {
        Ok(notes) => answer(StatusCode::OK, &notes, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Send the owner's words on a note to its Job, or to a new one where it has ended.
pub(crate) async fn send_build_note_feedback<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: SendBuildNoteFeedback = match ipc::decode_body("feedback on a note", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().send_build_note_feedback(asked).await {
        Ok(sent) => answer(StatusCode::OK, &sent, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// His answer on a Ready to try card in a Job's thread, and the card as it stands after. **A card of a Job another Mac
/// holds is answered there**, where the build he runs here holds its branch at its tip. The route is his alone, so no
/// Job's call reaches this.
pub(crate) async fn answer_ready_to_try<D: Commands + OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: AnswerReadyToTry = match ipc::decode_body("an answer on a ready to try card", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    if let Some(mac) = served.daemon().mac_holding_job(asked.session_id.as_str()).await {
        if let Err(refusal) = served.daemon().ready_in_this_build(&asked) {
            return refused(refusal);
        }
        return crate::mac_acts::across(&served, mac, ipc::JobAct::ReadyAnswer(asked)).await;
    }
    match served.shared().answer_ready_to_try(asked).await {
        Ok(card) => answer(StatusCode::OK, &card, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Start a restart onto `main` or the preview, and answer that it has begun.
pub(crate) async fn change_fleet_build<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ChangeFleetBuild = match ipc::decode_body("build to change to", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().change_fleet_build(asked).await {
        Ok(changing) => answer(StatusCode::ACCEPTED, &changing, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Move this Mac, another of his Macs, or both onto one build, keeping running Jobs, and answer that it has begun.
pub(crate) async fn update_mac<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: UpdateMac = match ipc::decode_body("a Mac to update", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().update_mac(asked).await {
        Ok(changing) => answer(StatusCode::ACCEPTED, &changing, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Start the undo of the last restart, and answer that it has begun. No body: the build reverted
/// to is the one `builds/previous` names.
pub(crate) async fn revert_fleet_build<D: Commands>(State(served): State<Served<D>>) -> Response {
    match served.daemon().revert_fleet_build().await {
        Ok(reverting) => answer(StatusCode::ACCEPTED, &reverting, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Pick a Job's branch for the next snapshot, or take it out, and start building the snapshot.
pub(crate) async fn pick_fleet_build_job<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: PickFleetBuildJob = match ipc::decode_body("a branch to pick for the snapshot", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().pick_fleet_build_job(asked).await {
        Ok(snapshotting) => answer(StatusCode::ACCEPTED, &snapshotting, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Start the swap onto the kept snapshot, and answer that it has begun. No body: the build swapped
/// onto is the one `snapshot` names.
pub(crate) async fn swap_fleet_build<D: Commands>(State(served): State<Served<D>>) -> Response {
    match served.daemon().swap_fleet_build().await {
        Ok(swapping) => answer(StatusCode::ACCEPTED, &swapping, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Keep the running build's tried Jobs, and answer the titles of the Jobs told to land them.
pub(crate) async fn keep_fleet_build<D: Commands>(State(served): State<Served<D>>) -> Response {
    match served.shared().keep_fleet_build().await {
        Ok(kept) => answer(StatusCode::OK, &kept, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
