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
        operation: "get_project_overview",
        method: "GET",
        path: "/features/overview",
    },
    Route {
        operation: "get_since_you_looked",
        method: "GET",
        path: "/features/since",
    },
    Route {
        operation: "get_feature_brief",
        method: "GET",
        path: "/features/brief",
    },
    Route {
        operation: "get_glossary",
        method: "GET",
        path: "/features/glossary",
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
        operation: "answer_retire",
        method: "POST",
        path: "/features/retire",
    },
    Route {
        operation: "check_docs",
        method: "POST",
        path: "/features/docs/check",
    },
    Route {
        operation: "dismiss_doc_flag",
        method: "POST",
        path: "/features/docs/dismiss",
    },
    Route {
        operation: "start_doc_job",
        method: "POST",
        path: "/features/docs/job",
    },
    Route {
        operation: "seed_features",
        method: "POST",
        path: "/features/seed",
    },
    Route {
        operation: "run_librarian",
        method: "POST",
        path: "/features/librarian",
    },
    Route {
        operation: "stop_librarian",
        method: "POST",
        path: "/features/librarian/stop",
    },
    Route {
        operation: "change_feature",
        method: "POST",
        path: "/features/change",
    },
    // The mod's hook asks as a Job is about to read, so it spells itself beside the two acts.
    Route {
        operation: "hand_over_features",
        method: "POST",
        path: "/features/hand_over",
    },
    Route {
        operation: "check_commit_feature",
        method: "POST",
        path: "/features/commit_check",
    },
    // Beside the other usage reads, which are not under `/jobs` either: the question is asked of every Job at once.
    Route {
        operation: "get_first_edit_searches",
        method: "GET",
        path: "/usage/first_edit_searches",
    },
    Route {
        operation: "get_whos_on_what",
        method: "GET",
        path: "/features/who",
    },
];
