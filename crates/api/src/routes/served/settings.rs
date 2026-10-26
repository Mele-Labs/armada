//! The Machine's settings: the rows of [`SERVED`](super::SERVED) under
//! `/settings`, and the older `/limits` and `/preferences` that read and write
//! the same file. `/accounts` rides here too: the account is the Machine's, and
//! its two settings, `account.clientId` and `store.url`, are keys of that file. Fleet's own, not a Job's, so none takes a path parameter; each
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
    // Whether this Mac serves the phone app. `POST /phone` and not `/phone/save`: Bridge's
    // Settings, Phone was built against the path Pocket v2 fixed.
    Route {
        operation: "get_phone",
        method: "GET",
        path: "/phone",
    },
    Route {
        operation: "set_phone",
        method: "POST",
        path: "/phone",
    },
    // Who this machine is signed in as. `get_accounts` is read by whoever draws the
    // title bar menu; the three acts are a person's.
    Route {
        operation: "get_accounts",
        method: "GET",
        path: "/accounts",
    },
    Route {
        operation: "start_sign_in",
        method: "POST",
        path: "/accounts/start_sign_in",
    },
    Route {
        operation: "sign_out",
        method: "POST",
        path: "/accounts/sign_out",
    },
    Route {
        operation: "set_repository_profile",
        method: "POST",
        path: "/accounts/set_repository_profile",
    },
    // The shared home this machine syncs Kit against. A read, Sync, and the person's acts on a row.
    Route {
        operation: "get_shared_home",
        method: "GET",
        path: "/shared_home",
    },
    Route {
        operation: "sync_shared_home",
        method: "POST",
        path: "/shared_home/sync",
    },
    Route {
        operation: "add_to_shared_home",
        method: "POST",
        path: "/shared_home/add",
    },
    Route {
        operation: "bring_from_shared_home",
        method: "POST",
        path: "/shared_home/bring",
    },
    Route {
        operation: "choose_shared_home_side",
        method: "POST",
        path: "/shared_home/choose",
    },
    Route {
        operation: "get_home_kinds",
        method: "GET",
        path: "/shared_home/kinds",
    },
    Route {
        operation: "set_home_kind",
        method: "POST",
        path: "/shared_home/kinds/set",
    },
    Route {
        operation: "set_home_repository",
        method: "POST",
        path: "/shared_home/repository",
    },
];
