//! The words the merge line view leaves to the client (`horizon.ts`, `forge.tsx`, `YourCheckout.tsx`,
//! `chip-card.tsx`). Fleet sends states and never a sentence (`merge_line_view.rs`), so each state a
//! chip can be in is said here, verbatim from Electron.

use ipc::{HubPullCi, HubPullRequest, HubQueueState, HubReview};

use crate::generated::vocabulary::rendering;
use crate::look::{token, Token};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullState {
    Queued,
    Running,
    Failing,
    Ready,
    Blocked,
    Open,
}

/// What the line says of one pull request: its state and the words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    pub state: PullState,
    pub says: &'static str,
}

const fn said(state: PullState, says: &'static str) -> Reading {
    Reading { state, says }
}

/// `horizon.ts`, `QUEUE`.
pub fn queue_reading(state: HubQueueState) -> Reading {
    match state {
        HubQueueState::WaitingForCi => said(PullState::Running, "Waiting for ci to join the merge queue"),
        HubQueueState::InQueue => said(PullState::Queued, "In the merge queue"),
        HubQueueState::Queued => said(PullState::Queued, "In the merge queue, waiting its turn"),
        HubQueueState::AwaitingChecks => said(PullState::Running, "In the merge queue, running its checks"),
        HubQueueState::Mergeable => said(PullState::Ready, "In the merge queue, ready to merge"),
        HubQueueState::Unmergeable => said(PullState::Failing, "In the merge queue, cannot merge"),
    }
}

/// `horizon.ts`, `CI`.
pub fn ci_reading(ci: HubPullCi) -> Reading {
    match ci {
        HubPullCi::Passed => said(PullState::Ready, "ci passed"),
        HubPullCi::Running => said(PullState::Running, "ci running"),
        HubPullCi::Failed => said(PullState::Failing, "ci failed"),
        HubPullCi::WaitingOnMain => said(PullState::Blocked, "ci red because main is"),
    }
}

/// The queue's reading when the pull request is in it, else its ci's, else `Open`.
pub fn reading_of(pull: &HubPullRequest) -> Reading {
    match (&pull.queue, pull.ci) {
        (Some(queue), _) => queue_reading(queue.state),
        (None, Some(ci)) => ci_reading(ci),
        (None, None) => said(PullState::Open, "Open"),
    }
}

/// The check line inside the queue: where ci stands.
pub fn check_of(pull: &HubPullRequest) -> Option<&'static str> {
    pull.queue.as_ref().and(pull.ci).map(|ci| ci_reading(ci).says)
}

/// Held by a queue, the forge's, rather than open and waiting. Drawn solid, the rest a ring.
pub fn is_queued(pull: &HubPullRequest) -> bool {
    pull.queue.as_ref().is_some_and(|queue| queue.state != HubQueueState::WaitingForCi)
}

pub const MERGE_BY_HAND: &str = "Needs merging by hand";

/// The state's mark in the card, from the registry: `queued` is `LAND_STATE.waiting`, `running`
/// `LAND_STATE.gating`, `failing` `CHECK_OUTCOME.failed`, `ready` `CHECK_OUTCOME.passed`, `blocked`
/// `QUEUED_REASON.blocked_by_dependency`, and `open` has none.
pub fn mark_of(state: PullState) -> Option<&'static str> {
    let (vocabulary, variant) = match state {
        PullState::Queued => ("land_state", "waiting"),
        PullState::Running => ("land_state", "gating"),
        PullState::Failing => ("check_outcome", "failed"),
        PullState::Ready => ("check_outcome", "passed"),
        PullState::Blocked => ("queued_reason", "blocked_by_dependency"),
        PullState::Open => return None,
    };
    rendering(vocabulary, variant).and_then(|one| one.glyph)
}

/// The dot's colour for a state: `deck.css` `.deck-chip__dot[data-state]`. Queued and open take the
/// quiet default.
pub fn hue_of(state: PullState) -> Token {
    match state {
        PullState::Blocked => token::STATUS_ESCALATED,
        PullState::Running => token::STATUS_RUNNING,
        PullState::Ready => token::STATUS_COMPLETED_SUCCESS,
        PullState::Failing => token::STATUS_COMPLETED_FAILED,
        PullState::Queued | PullState::Open => token::FG_SUBTLE,
    }
}

pub fn review_said(review: HubReview) -> Option<&'static str> {
    match review {
        HubReview::Approved => Some("Approved"),
        HubReview::ChangesRequested => Some("Changes requested"),
        HubReview::ReviewRequired => Some("Required"),
        HubReview::None => None,
    }
}

/// `Read just now`, `Read 40s ago`, `Read 3m ago`, `Read 2h 05m ago`.
pub fn read_said(seconds: i64) -> String {
    if seconds < 5 {
        "Read just now".into()
    } else if seconds < 60 {
        format!("Read {seconds}s ago")
    } else {
        format!("Read {} ago", span_said(seconds))
    }
}

/// A span in Electron's `spanOf`: `3m`, `2h 05m`, `1d 4h`.
pub fn span_said(seconds: i64) -> String {
    let minutes = seconds / 60;
    match minutes {
        0..=59 => format!("{minutes}m"),
        60..=1439 => format!("{}h {:02}m", minutes / 60, minutes % 60),
        _ => format!("{}d {}h", minutes / 1440, (minutes % 1440) / 60),
    }
}

/// `1 commit`, `3 commits`.
pub fn commits(n: u32) -> String {
    if n == 1 { "1 commit".into() } else { format!("{n} commits") }
}
