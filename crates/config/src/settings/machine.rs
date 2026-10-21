//! Which agent CLI a Drone runs as and on what `PATH`, the switches that turn
//! a behaviour on or off, the programs Bridge opens a file or a folder in, and
//! where a pull request link opens, and the name this machine goes by.

use super::{Applies, Entry, Flag, Json, Kind, List, Options, Section, Shipped, Words};

pub const HARNESS_AGENT: Words = Words("harness.agent");
pub const HARNESS_BINARY_PATH: Words = Words("harness.binaryPath");
pub const HARNESS_DRONE_PATH: List = List("harness.dronePath");

pub const HELM_CAN_ACT: Flag = Flag("features.helmCanAct");
pub const DRAFT_PULL_REQUESTS: Flag = Flag("features.draftPullRequests");
pub const OPEN_GUIDES_FIRST_TIME: Flag = Flag("features.openGuidesFirstTime");
pub const WORKFLOWS: Flag = Flag("features.workflows");
pub const DISPUTE_REVIEWS: Flag = Flag("features.disputeReviews");
/// Read by Bridge alone: whether a job needing the owner posts a macOS notification.
pub const NOTIFICATIONS: Flag = Flag("features.notifications");
/// Read by Bridge alone: whether moves and presses make their quiet sound.
pub const UI_SOUNDS: Flag = Flag("features.uiSounds");
/// The folders every session may reach past its slots with no card, and the ones it may not:
/// `ipc::reach_folders`. Written by an "always" answer on a session's card.
pub const REACH_FOLDERS: Json = Json("features.reachFolders");

pub const EDITOR_COMMAND: Words = Words("editor.command");
pub const TERMINAL_APP: Words = Words("terminal.app");
pub const TERMINAL_COMMAND: Words = Words("terminal.command");

pub const OPEN_PULL_REQUESTS_IN: Words = Words("links.openPullRequestsIn");

pub const MACHINE_NAME: Words = Words("machine.name");

pub const ACCOUNT_CLIENT_ID: Words = Words("account.clientId");

pub const STORE_URL: Words = Words("store.url");

/// The terminal programs Bridge knows how to open a folder in, and the one
/// that runs `terminal.command` instead.
pub const TERMINALS: &[&str] = &["Terminal", "iTerm", "Ghostty", "Warp", "WezTerm", "Custom"];

/// Where a pull request link opens: Bridge's own panel, or the default browser.
pub const PULL_REQUEST_PLACES: &[&str] = &["armada", "browser"];

pub(super) const HARNESS: &[Entry] = &[
    // One adapter exists, so one choice; it is real plumbing so a second adapter
    // is a new option rather than a new setting.
    Entry {
        key: HARNESS_AGENT.0,
        section: Section::Harness,
        title: "Agent",
        description: "Which agent CLI Drones, the Judge and Helm run as.",
        kind: Kind::Choice(Options::Harnesses),
        shipped: Shipped::Supplied,
        applies: Applies::AtRestart,
        row: Some("agentharness-binary-path-and-version-pin"),
        env: None,
        tip: None,
    },
    // The default is the CLI as installed on `PATH`, a name only `adapters` may
    // spell. Fleet refuses to start where nothing runnable is there, before it
    // takes a port: `armada::agent` says why both names are probed.
    Entry {
        key: HARNESS_BINARY_PATH.0,
        section: Section::Harness,
        title: "Agent program",
        description: "The agent CLI's program: a name looked up on a Drone's PATH, or a path. Fleet will not start if nothing runnable is there.",
        kind: Kind::Text,
        shipped: Shipped::Supplied,
        applies: Applies::AtRestart,
        row: Some("agentharness-binary-path-and-version-pin"),
        env: Some("ARMADA_AGENT_BINARY"),
        tip: None,
    },
    // The system directories, always last; the per-user ones and Fleet's own
    // `PATH` go before them — `armada::serve::drone_path_with`.
    Entry {
        key: HARNESS_DRONE_PATH.0,
        section: Section::Harness,
        title: "Drone PATH",
        description: "The system directories at the end of a Drone's PATH, in order. Your own directories and Fleet's PATH come before them.",
        kind: Kind::TextList,
        shipped: Shipped::TextList(&[
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ]),
        applies: Applies::AtRestart,
        row: None,
        env: None,
        tip: None,
    },
];

pub(super) const FEATURES: &[Entry] = &[
    // Enabled: resolved once like every other Machine setting — `#943`.
    Entry {
        key: HELM_CAN_ACT.0,
        section: Section::Features,
        title: "Helm can act",
        description: "Helm may change things when you ask it to, such as dispatching a Job or answering a call. Off, Helm only reads.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: Some("helm-action-authority-tier-1-redirect-enabled-vs-read-only"),
        env: None,
        tip: None,
    },
    Entry {
        key: DRAFT_PULL_REQUESTS.0,
        section: Section::Features,
        title: "Open pull requests as drafts",
        description: "A Job's pull request opens as a draft unless the workflow, armada.yml or the Job says otherwise.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(false),
        applies: Applies::Live,
        row: Some("pull-request-mode-ready-draft"),
        env: None,
        tip: None,
    },
    Entry {
        key: WORKFLOWS.0,
        section: Section::Features,
        title: "Workflows for Jobs",
        description: "A Job can follow a workflow: the proposer picks one, and the thread shows its steps. Off, every Job is a plain one.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: Some("session-workflows"),
        env: None,
        tip: None,
    },
    Entry {
        key: DISPUTE_REVIEWS.0,
        section: Section::Features,
        title: "Dispute reviews",
        description: "A job that disputes a Fleet notice starts a review job in Armada, and Fleet reads replies for pushback. Off, the dispute is kept and nothing starts.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(false),
        applies: Applies::Live,
        row: Some("dispute-reviews"),
        env: None,
        tip: None,
    },
    Entry {
        key: OPEN_GUIDES_FIRST_TIME.0,
        section: Section::Features,
        title: "Open guides the first time",
        description: "A guide opens by itself the first time you meet what it explains. Off, guides open only when you ask.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: NOTIFICATIONS.0,
        section: Section::Features,
        title: "Notifications",
        description: "A macOS notification when a job needs you and Bridge is not in front. The dock count shows either way.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: Some("notification-routing-in-app-vs-os-per-event-type"),
        env: None,
        tip: None,
    },
    Entry {
        key: UI_SOUNDS.0,
        section: Section::Features,
        title: "Interface sounds",
        description: "A quiet tick when you move through a list or press a control.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: REACH_FOLDERS.0,
        section: Section::Features,
        title: "Folders jobs reach",
        description: "Folders outside its slots a job may read and write with no card, and folders it may not.",
        kind: Kind::Json,
        shipped: Shipped::Nothing,
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
];

pub(super) const EDITOR: &[Entry] = &[Entry {
    key: EDITOR_COMMAND.0,
    section: Section::Editor,
    title: "Editor command",
    description: "The command Bridge opens a file with, {file} and {line} filled in, as `code -g {file}:{line}`. Blank uses $VISUAL, then $EDITOR, then the system's default.",
    kind: Kind::Text,
    shipped: Shipped::Text(""),
    applies: Applies::Live,
    row: None,
    env: None,
    tip: None,
}];

pub(super) const TERMINAL: &[Entry] = &[
    Entry {
        key: TERMINAL_APP.0,
        section: Section::Terminal,
        title: "Terminal",
        description:
            "The terminal Bridge opens a Job's worktree in. Custom runs the command below.",
        kind: Kind::Choice(Options::Fixed(TERMINALS)),
        shipped: Shipped::Text("Terminal"),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: TERMINAL_COMMAND.0,
        section: Section::Terminal,
        title: "Terminal command",
        description:
            "The command Bridge runs to open a folder when the terminal is Custom, {dir} filled in.",
        kind: Kind::Text,
        shipped: Shipped::Text(""),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
];

pub(super) const LINKS: &[Entry] = &[Entry {
    key: OPEN_PULL_REQUESTS_IN.0,
    section: Section::Links,
    title: "Open pull requests in",
    description: "Where every pull request link in Bridge opens.",
    kind: Kind::Choice(Options::Fixed(PULL_REQUEST_PLACES)),
    shipped: Shipped::Text("armada"),
    applies: Applies::Live,
    row: None,
    env: None,
    tip: None,
}];

pub(super) const MACHINE: &[Entry] = &[Entry {
    key: MACHINE_NAME.0,
    section: Section::Machine,
    title: "Machine name",
    description: "How the other machines signed in to your account name this one. Blank uses the host's name.",
    kind: Kind::Text,
    shipped: Shipped::Text(""),
    applies: Applies::Live,
    row: Some("machine-name"),
    env: None,
    tip: None,
}];

pub(super) const ACCOUNT: &[Entry] = &[Entry {
    key: ACCOUNT_CLIENT_ID.0,
    section: Section::Account,
    title: "Sign-in app",
    description: "The client id of the OAuth app Armada signs in with. Blank leaves sign-in unavailable until an app is registered.",
    kind: Kind::Text,
    shipped: Shipped::Text(""),
    applies: Applies::Live,
    row: Some("account-client-id"),
    env: None,
    tip: None,
}];

pub(super) const STORE: &[Entry] = &[Entry {
    key: STORE_URL.0,
    section: Section::Store,
    title: "Store address",
    description: "The address of the central store your machines share. Blank leaves sign-in unavailable until a store is deployed.",
    kind: Kind::Text,
    shipped: Shipped::Text(""),
    applies: Applies::Live,
    row: Some("store-url"),
    env: None,
    tip: None,
}];
