//! The rows of [`SERVED`](super::SERVED) an adversarial review is started and its findings settled
//! through. Its own table for the parent's reason: past 1200 lines.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    Route {
        operation: "start_adversarial_review",
        method: "POST",
        path: "/adversarial_review",
    },
    Route {
        operation: "send_finding",
        method: "POST",
        path: "/adversarial_review/send",
    },
    Route {
        operation: "dismiss_finding",
        method: "POST",
        path: "/adversarial_review/dismiss",
    },
];
