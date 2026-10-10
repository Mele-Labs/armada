//! The Machine's settings: the rows of [`SERVED`](super::SERVED) under
//! `/settings`, and the older `/limits` and `/preferences` that read and write
//! the same file. Fleet's own, not a Job's, so none takes a path parameter; each
//! save spells its act in the last segment, as `/manifest/save_file` does.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    Route {
        operation: "get_settings",
        method: "GET",
        path: "/settings",
    },
    Route {
        operation: "save_settings",
        method: "POST",
        path: "/settings/save",
    },
    // The four numbers `/capacity` is measured against, beside it and for its
    // reason. Four keys of the same file, kept in their older shape.
    Route {
        operation: "get_limits",
        method: "GET",
        path: "/limits",
    },
    Route {
        operation: "save_limits",
        method: "POST",
        path: "/limits/save",
    },
    // A person's Bridge preferences, `/limits`' shape one section over.
    Route {
        operation: "get_preferences",
        method: "GET",
        path: "/preferences",
    },
    Route {
        operation: "save_preferences",
        method: "POST",
        path: "/preferences/save",
    },
];
