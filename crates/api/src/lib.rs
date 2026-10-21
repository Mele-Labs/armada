//! HTTP and WebSocket transport, on one listener.
//!
//! **This crate does not depend on `fleet`**, which is what makes the daemon
//! core drivable in tests with zero network. The two run as one process and
//! talk over an in-process channel; the seam is real even though the process
//! boundary is not. [`Queries`] and [`Commands`] are stated here and
//! implemented in `fleet`, so the dependency points one way and
//! `cargo tree -p api` names no `fleet`; [`Daemon`] is the three together and
//! the bound [`router`] takes. They speak `ipc` DTOs and nothing else: **no
//! `core_model` type appears anywhere in this crate**, so there is no shape
//! here through which a domain field could reach the wire unredacted.
//!
//! **axum, with its built-in WebSocket upgrade as an extractor in the same
//! router**, so there is no second port and no assembly step. gRPC and `tonic`
//! were rejected on measured cost: +43 crates, and a hard build failure on a
//! clean machine with no `protoc`. Queries and commands are request-response
//! and go over HTTP; only unsolicited pushes need the socket, and who initiates
//! is the whole rule.
//!
//! **A subset of the inventory, and a stream.** [`SERVED`] is the operations M1
//! needs, named with `crates/ipc/operations/`'s own keys. The rest of the
//! inventory, the `/v0` lifeboat and version-skew handling belong to Ship and
//! are neither built nor stubbed here.

/// A form's edits to `armada.yml`, as edits rather than as a file.
/// Which door a request came through, named before a handler sees it.
mod acting;
/// The account a machine is signed in to: its profiles, signing in and out.
mod accounting;
mod amending;
mod answers;
/// Who the listener answers at all: a page in a browser is refused before any
/// handler runs. **The seam's one rule about callers**, and `#1460`.
mod callers;
mod commands;
/// One repository's Helm: its Session found, spoken into and started over.
mod conversing;
mod daemon;
/// The agent's door: the HTTP surface, spoken as MCP. **Who may open it is
/// `#698`; what it is scoped to is here.**
mod door;
/// The Manifest file's two routes: read its text, and write a corrected one
/// back. Journey 9, *Editing*.
mod editing;
/// The reads that belong to no Job: the roster, the machine, the spend and
/// what crossed the stream.
mod fleetwide;
/// One Job's own log, followed off the file Fleet already writes.
/// **The third voice in the activity log.**
mod following;
mod hosted_sessions;
mod importing;
mod run_memory;
mod notice_mutes;
/// A repository's open issues. #91.
mod issues;
/// A repository working its own backlog. #2092.
mod autopilots;
/// Kit's MCP servers, and each Manifest's word over one. `#1275`.
mod kit;
/// Share: pieces made into a package, read, added to Kit, and bound.
mod sharing;
/// The build Fleet runs on, read, and the restart onto another.
mod building;
/// Fleet's four changeable limits, read and saved.
mod limiting;
/// Setup's proposals, an edit to one, and its Write.
mod manifest_proposals;
mod mcp;
/// Mods: folders on this machine that change how Bridge looks.
mod mods;
mod clashing;
mod placed;
/// A person's Bridge preferences, read and saved. `limiting`'s shape one
/// table over.
mod preferring;
mod pull_request_panel;
mod pull_requests;
mod queries;
/// Natural Search and the Walk tool's routes, beside a pull request's runs.
mod review_walks;
mod receipts;
mod reconciling;
/// The run sheet's routes: a person's run of one Manifest entry in a Job's
/// worktree, and what the runs left.
mod rehearsing;
/// Rules a person always-allowed for a Manifest's repository, read and taken
/// back — kept by Fleet itself since `#836`, never `armada.yml`.
mod repositories;
mod repository_allow;
/// Requests: an HTTP request filled from a repository's environments, and its collections.
mod requests;
/// A Job's retro and the Lessons listing. Since 23.12.
mod retros;
mod routes;
mod scoped;
/// Everything a handler is given. **Next door to the table**, which is what
/// the gate rule reads.
mod served;
mod servers;
mod sessions;
/// settings.json, read and saved.
mod settling;
mod sockets;
mod stream;
/// A Studio's routes: the list, one Studio, and every act on one. `#1285`.
mod studios;
/// What happened across the Fleet in a window of time. Since 23.82.
mod timeline;
/// One slot's leases and checkouts, for the Cockpit's Bay pane.
mod bay_history;
/// A person's run's output, on a socket of its own per run — never `/events`.
mod watching_run;
mod watching_session_run;

#[cfg(test)]
mod tests;

pub use acting::{asked_by, asking, carrying, via, BRIDGE, CALLER_HEADER, PHONE};
pub use daemon::{
    offerable, Accounts, Admitting, Authoring, Commands, Conversations, Daemon,
    Autopilots, HelmReach, HostedSessions, Issues, Mods, Clashing,
    PullRequestPanel, PullRequests, Queries, Redirector, Refusal, Rehearsing, Requests, Retros, Sessions, Settings, Steerer,
    StoredFile, Studios,
};
pub use door::{door_within, door_without_a_manifest, offered, Scope, DOOR_PATH};
pub use following::{Follow, Followed, LandOutput, SessionCheckLive, FOLLOW};
pub use mcp::Caller;
pub use reconciling::Reconciliation;
pub use routes::{router, Route, SERVED};
pub use served::Served;
pub use stream::{
    tally_every, Broadcaster, Next, Subscription, Tally, BACKLOG, TALLIED, TALLY_EVERY,
};
pub use watching_session_run::{ObservedSessionRun, SessionRunEvent, SESSION_RUN_BACKLOG};
pub use watching_run::{
    ObservedCheckoutRun, ObservedServer, RunChunk, RunFeed, RunSeen, RunWatch,
    RUN_BACKLOG,
};
