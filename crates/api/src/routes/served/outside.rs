//! The rows of [`SERVED`](super::SERVED) about what lies outside any Job: the
//! machine's sleep, a repository's open issues read from its tracker, its projects, and Requests: an address
//! anywhere, sent to filled from a repository's environments.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // The plan's windows, from the newest reading of any session.
    Route {
        operation: "get_plan_usage",
        method: "GET",
        path: "/plan_usage",
    },
    // The default model's step down as the weekly window runs out, and the four things the owner does to it.
    Route {
        operation: "get_model_downshift",
        method: "GET",
        path: "/model_downshift",
    },
    Route {
        operation: "accept_downshift",
        method: "POST",
        path: "/model_downshift/accept",
    },
    Route {
        operation: "restore_downshift",
        method: "POST",
        path: "/model_downshift/restore",
    },
    Route {
        operation: "keep_downshift",
        method: "POST",
        path: "/model_downshift/keep",
    },
    Route {
        operation: "settle_downshift",
        method: "POST",
        path: "/model_downshift/settle",
    },
    // The agents' account, signed out for every Job at once.
    Route {
        operation: "get_agent_sign_in",
        method: "GET",
        path: "/agent/sign_in",
    },
    Route {
        operation: "start_agent_sign_in",
        method: "POST",
        path: "/agent/sign_in",
    },
    // Sleep mode: the switch, the night it keeps, and a correction of one decision.
    Route {
        operation: "get_sleep",
        method: "GET",
        path: "/sleep",
    },
    Route {
        operation: "set_sleep",
        method: "POST",
        path: "/sleep",
    },
    Route {
        operation: "override_sleep",
        method: "POST",
        path: "/sleep/override",
    },
    Route {
        operation: "review_sleep",
        method: "POST",
        path: "/sleep/review",
    },
    // What happened across the Fleet, windowed by instant: three optional bounds, so a query string.
    Route {
        operation: "get_fleet_timeline",
        method: "GET",
        path: "/timeline",
    },
    // One slot's leases and checkouts: the repository and the slot are both asked for, so a query string.
    Route {
        operation: "get_slot_history",
        method: "GET",
        path: "/slot_history",
    },
    // Which served repository a request's words are about: typed text, so a
    // query string for `search_files`' reason, and under `/repositories`
    // beside the list it picks from.
    Route {
        operation: "read_request_repository",
        method: "GET",
        path: "/repositories/of_request",
    },
    // A repository's open issues, read from its tracker. #91.
    Route {
        operation: "list_issues",
        method: "GET",
        path: "/issues/:repository",
    },
    // One issue by reference, closed ones included, for a # that names one the list does not hold.
    Route {
        operation: "get_issue",
        method: "GET",
        path: "/issues/:repository/:reference",
    },
    // Saved issue queries and the query writer. The queries live under their own path so a
    // reference can never be read as one. `docs/concepts/tracker.md`, *Saved queries*.
    Route {
        operation: "search_issues",
        method: "POST",
        path: "/issue_queries/:repository/search",
    },
    Route {
        operation: "list_issue_queries",
        method: "GET",
        path: "/issue_queries/:repository",
    },
    Route {
        operation: "install_tool",
        method: "POST",
        path: "/issue_queries/:repository/install_tool",
    },
    Route {
        operation: "write_issue_query",
        method: "POST",
        path: "/issue_queries/:repository/write",
    },
    Route {
        operation: "save_issue_query",
        method: "POST",
        path: "/issue_queries/:repository",
    },
    Route {
        operation: "forget_issue_query",
        method: "POST",
        path: "/issue_queries/:repository/:id/forget",
    },
    // A repository's projects, kept on this Mac. `docs/practices/protocol/projects.md`.
    Route {
        operation: "list_projects",
        method: "GET",
        path: "/projects/:repository",
    },
    Route {
        operation: "add_project",
        method: "POST",
        path: "/projects/:repository",
    },
    Route {
        operation: "rename_project",
        method: "POST",
        path: "/projects/:repository/:project/rename",
    },
    Route {
        operation: "delete_project",
        method: "POST",
        path: "/projects/:repository/:project/delete",
    },
    Route {
        operation: "add_project_task",
        method: "POST",
        path: "/projects/:repository/:project/tasks",
    },
    Route {
        operation: "link_project_issue",
        method: "POST",
        path: "/projects/:repository/:project/issues",
    },
    Route {
        operation: "add_project_subtask",
        method: "POST",
        path: "/projects/:repository/:project/rows/:row/subtasks",
    },
    Route {
        operation: "link_task_issue",
        method: "POST",
        path: "/projects/:repository/:project/rows/:row/issues",
    },
    Route {
        operation: "unlink_task_issue",
        method: "POST",
        path: "/projects/:repository/:project/rows/:row/issues/unlink",
    },
    Route {
        operation: "link_task_job",
        method: "POST",
        path: "/projects/:repository/:project/rows/:row/job",
    },
    Route {
        operation: "unlink_task_job",
        method: "POST",
        path: "/projects/:repository/:project/rows/:row/job/unlink",
    },
    Route {
        operation: "link_task_pull",
        method: "POST",
        path: "/projects/:repository/:project/rows/:row/pulls",
    },
    Route {
        operation: "unlink_task_pull",
        method: "POST",
        path: "/projects/:repository/:project/rows/:row/pulls/unlink",
    },
    Route {
        operation: "edit_project_task",
        method: "POST",
        path: "/projects/:repository/:project/rows/:row/edit",
    },
    Route {
        operation: "remove_project_row",
        method: "POST",
        path: "/projects/:repository/:project/rows/:row/remove",
    },
    Route {
        operation: "move_project_row",
        method: "POST",
        path: "/projects/:repository/:project/rows/:row/move",
    },
    Route {
        operation: "tick_project_task",
        method: "POST",
        path: "/projects/:repository/:project/rows/:row/tick",
    },
    // Requests. A repository's two files, read on each call; a secret's value is the Keychain's.
    Route {
        operation: "send_request",
        method: "POST",
        path: "/requests/send",
    },
    Route {
        operation: "get_requests",
        method: "GET",
        path: "/requests",
    },
    Route {
        operation: "save_request_collection",
        method: "POST",
        path: "/requests/collections",
    },
    Route {
        operation: "save_request_to_collection",
        method: "POST",
        path: "/requests/collections/request",
    },
    Route {
        operation: "save_request_environment",
        method: "POST",
        path: "/requests/environments",
    },
];
