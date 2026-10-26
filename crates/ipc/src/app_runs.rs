//! A repository's app, started in development from Bridge's ▶ — `list_app_runs`,
//! `get_app_run_draft`, `start_app_run`, `stop_app_run`, `restart_app_run`, `save_app_run`, and
//! `app_run.changed`. `docs/concepts/manifest.md`, *Commands that keep running*.
//!
//! **A run is several servers and nothing more**, so a process here is a [`ServerState`] read
//! through: `server_id` is what `observe_server` takes, and its output is that socket's, never a
//! second one for runs.
//!
//! [`ServerState`]: crate::ServerState

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};
use crate::run_memory::RunPlace;
use crate::servers::ServerLink;

/// Whether a run is opened at an address or is a window of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppRunOpensIn {
    /// Some process listens on a leased port and offers a link.
    Browser,
    /// Nothing offers an address: it opens a window of its own.
    App,
}

/// Where one process of a run stands. **A server that ends on its own has `failed`**, whatever its
/// code, and `exited` is only one somebody stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppRunState {
    /// Nothing of it has been started on this checkout.
    NotStarted,
    Starting,
    Ready,
    Exited,
    Failed,
}

/// The checkout a run is on. **A main checkout and a Job's worktree, and no other**: a worktree
/// somebody cut by hand is a place `start_server` reaches and a run does not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppRunCheckout {
    Main { path: String },
    Job {
        path: String,
        /// The Job whose worktree it is.
        job_id: String,
        /// The branch the Job's bay is on. Absent where Fleet holds none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
        /// The bay's number.
        bay: u32,
    },
}

/// One server of a run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRunProcess {
    /// The server's name in the Manifest.
    pub name: String,
    pub state: AppRunState,
    /// The port leased to it on this checkout: main and a Job's bay hold different ones. Absent
    /// before it has started once, and for a process that listens on none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// The number it is pinned to, so it is on `port` every time and a sibling that hardcodes it
    /// finds it. Absent: Fleet leases it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_port: Option<u16>,
    /// Where it is opened, resolved against its port. **Only while it is `ready`**: a link pressed
    /// while it starts opens a port nothing answers on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<ServerLink>,
    /// The instance behind it, live or the last that ended: what `observe_server` takes, and the
    /// way to its output. **A restart is a new instance**, so a viewer reads the id again from the
    /// next `app_run.changed`. Absent until it has started once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    /// How it ended, in a sentence. Present on `failed` and `exited`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<String>,
}

/// One process of a run expecting another on a port it does not get: a frontend whose dev proxy
/// points at `127.0.0.1:8731` while the backend is leased another number. **Found in a file the
/// process's folder holds, or in its output**, and kept, so the next draft and start show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRunLink {
    /// The process that expects it: its name in the run.
    pub from: String,
    /// The process it expects on `port`.
    pub to: String,
    pub port: u16,
    /// The file, relative to the repository's root, or `log` where `from`'s output showed a
    /// refused connection to `port`.
    pub found_in: String,
    /// What to hand a Job to mend it. **Bridge opens Dispatch prefilled with it**; Fleet starts no Job.
    pub brief: String,
}

/// One run on one checkout, as it stands: a row of `list_app_runs`, what `start_app_run`,
/// `stop_app_run` and `restart_app_run` answer, and `app_run.changed` whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRun {
    /// Its key under `runs:` in the Manifest. What every act on it names.
    pub name: String,
    /// What a person is shown: its `label`, else its name.
    pub label: String,
    /// **Not called `kind`**: `app_run.changed` carries this row flat beside the event's own `kind`.
    pub opens_in: AppRunOpensIn,
    /// The repository it runs in, by the id a Job there carries.
    pub manifest_id: ManifestId,
    pub checkout: AppRunCheckout,
    /// The servers it starts, in the order the file wrote them.
    pub processes: Vec<AppRunProcess>,
    /// When a process of it was last started on this checkout. **Only what this Fleet has seen
    /// since it started**: absent is not "never".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used: Option<Instant>,
    /// Where one process expects another on a port it was not given. **At most one for a pair of
    /// processes**, and absent where there is none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<AppRunLink>,
}

/// `list_app_runs`: every run the repository declares, as each stands on one checkout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRunList {
    pub runs: Vec<AppRun>,
}

/// The body of `start_app_run`, `stop_app_run` and `restart_app_run`. **A name, never a command and
/// never a path**: what runs is what the Manifest declares, on the checkout this says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedAppRun {
    /// A key `list_app_runs` named.
    pub name: String,
    /// The Job whose worktree it runs on. Absent: the repository's main checkout, which
    /// `?manifest_id=` names. Fleet reads the Job's checkout from its own record of the lease.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

/// One process of a run, as it is proposed and as a person confirms it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRunDraftProcess {
    /// The server's name. Letters, digits and `_`.
    pub name: String,
    /// The folder under the repository's root it starts in. Absent: the root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    /// What starts it. **`${port}` stands for the port Fleet leases the checkout**, so no number
    /// is edited: a command with it listens, and one without listens on none.
    pub command: String,
    /// The variable Armada sets to the port, for a program that reads it from the environment.
    /// **Handed back as it was read**: it is not the person's to edit, and dropping it leaves the
    /// program on a port of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_env: Option<String>,
}

/// A Command the checkout runs before the run's processes can start: an install, a `.venv` made.
/// Saved as a Command and named in `setup.requires`, so it runs once on each checkout and again when
/// what it was traced from changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRunDraftSetup {
    /// The Command's name. Letters, digits and `_`.
    pub name: String,
    /// What it runs, from the checkout's root.
    pub command: String,
}

/// `get_app_run_draft`: the run Fleet proposes for a repository that declares none. **It saves
/// nothing.** With no process the repository's files and Jobs suggested none, and a person writes
/// their own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRunDraft {
    /// The repository's root, absolute: what a process's `dir` is read from.
    pub root: String,
    /// Where saving it puts it: the place the repository's Kit is kept.
    pub place: RunPlace,
    /// **Only the processes not already in a saved run**, which is none where the repository's runs
    /// cover everything its files and Jobs suggest.
    pub processes: Vec<AppRunDraftProcess>,
    /// Where one of these expects another process, drafted or already saved, on a port it would
    /// not get. Absent where none was found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<AppRunLink>,
    /// What the checkout runs first, in order: an install for each lockfile beside a
    /// `package.json`, a `.venv` for each `requirements.txt` or `pyproject.toml`. **Only what no
    /// Command declared already runs.** Absent where there is nothing to install.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub setup: Vec<AppRunDraftSetup>,
}

/// The body of `pin_app_run_port`: a process, and the number its siblings expect it on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinAppRunPort {
    /// The process, by its name in the run. It must listen on a leased port.
    pub process: String,
    /// The number: from 1024 to 65535.
    pub port: u16,
    /// The run to answer, by its key. Absent: the first run that lists `process`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The Job whose worktree is read. Absent: the repository's main checkout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

/// The body of `unpin_app_run_port`: a process given back to leasing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnpinAppRunPort {
    pub process: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

/// `save_app_run`'s body: a confirmed draft and the name its person gave it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveAppRun {
    /// What the person calls it. Its key under `runs:` is this written as letters, digits and `_`.
    pub name: String,
    pub processes: Vec<AppRunDraftProcess>,
    /// The setup it needs, as confirmed: each written as a Command and added to `setup.requires`
    /// after what is there. Absent: none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub setup: Vec<AppRunDraftSetup>,
    /// The Job whose worktree it then starts on. Absent: the main checkout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

/// What `save_app_run` answers: where the run was written, and the run as it stands once started.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedAppRun {
    pub place: RunPlace,
    /// The file it was written to, as that place names it.
    pub file: String,
    pub run: AppRun,
}
