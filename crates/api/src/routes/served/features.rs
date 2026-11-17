//! A project's feature library and the figure it is judged by. Rows of [`SERVED`](super::SERVED).

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // The library is a collection under `/features`, and the reads of one feature say which in the last segment. The
    // project rides the query string because a Job that does not know its project's id leaves it out and is placed by
    // its connection.
    Route {
        operation: "list_features",
        method: "GET",
        path: "/features",
    },
    Route {
        operation: "get_feature_page",
        method: "GET",
        path: "/features/page",
    },
    Route {
        operation: "get_feature_brief",
        method: "GET",
        path: "/features/brief",
    },
    Route {
        operation: "get_features_of_path",
        method: "GET",
        path: "/features/of_path",
    },
    // The two acts spell themselves in the last segment, as `/limits/save` does.
    Route {
        operation: "write_feature_sections",
        method: "POST",
        path: "/features/write_sections",
    },
    Route {
        operation: "upsert_features",
        method: "POST",
        path: "/features/upsert",
    },
    Route {
        operation: "seed_features",
        method: "POST",
        path: "/features/seed",
    },
    Route {
        operation: "change_feature",
        method: "POST",
        path: "/features/change",
    },
    // Beside the other usage reads, which are not under `/jobs` either: the question is asked of every Job at once.
    Route {
        operation: "get_first_edit_searches",
        method: "GET",
        path: "/usage/first_edit_searches",
    },
];
