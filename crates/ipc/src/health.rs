//! What Fleet can say about its own health, and what it cannot.
//!
//! **Not Doctor.** Doctor's grid is ten modules probed from `adapters`,
//! `config` and `store`, and it is not built — `docs/concepts/doctor.md`. This
//! answers the rows Fleet itself holds, and names the rest as unprobed.

use serde::{Deserialize, Serialize};

/// One module, asked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Probe {
    /// The module name as Doctor's grid spells it — `Fleet`, `SQLite`,
    /// `Manifest`, `System stats`.
    pub module: String,
    /// `pass`, `warn` or `fail`, Doctor's own three words. A string rather than
    /// a closed set: the vocabulary is a surface's, and nothing matches on it.
    pub outcome: String,
    /// What the probe read, in one line. Never empty — a `pass` with nothing
    /// beside it is a row a person cannot check.
    pub detail: String,
}

/// Every probe Fleet ran, and everything it could not run one for.
///
/// **`not_probed` is the honest half.** A report of four passing rows with no
/// mention of the modules nobody asked reads as a healthy machine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetHealth {
    pub probes: Vec<Probe>,
    /// What this answer says nothing about, with why beside each.
    pub not_probed: Vec<Unprobed>,
    /// `settings.action-authority-tier-1-redirect-enabled-vs-read-only`,
    /// resolved once when Fleet started. `#1127` — Bridge had no way to read
    /// what Fleet actually decided, only the setting's own description.
    pub action_authority: ActionAuthority,
    /// Whether any served repository left a workflow definition out, so a rail
    /// can warn without counting. **A boolean and never a number**: the list is
    /// `list_left_out_workflows`, and what a person is told is that something
    /// is there to read. `false` from a Fleet older than 23.33.
    #[serde(default)]
    pub workflows_left_out: bool,
    /// Free space where Jobs are built, and what Fleet is doing about it. Absent from a Fleet
    /// older than the reclaim, and from one that has not yet read the disk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk: Option<DiskHealth>,
}

/// The disk under the first served repository, against the two floors Fleet holds it to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskHealth {
    /// Free on the volume of the first served repository.
    pub free_bytes: u64,
    /// `limits.diskReclaimFloorGib` in bytes. Under it Fleet drops build output; zero is off.
    pub reclaim_floor_bytes: u64,
    /// `limits.diskFloorGib` in bytes. Under it no new Job starts.
    pub hold_floor_bytes: u64,
    /// The last pass that dropped anything. Absent where none has since Fleet started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_pass: Option<ReclaimPass>,
    /// Build folder paths in the order a pass would drop them next, at most ten. Empty over the
    /// reclaim floor, where nothing would be.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next_to_drop: Vec<String>,
}

/// One pass under the reclaim floor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimPass {
    /// When it ended, RFC 3339 as every wire time.
    pub at: String,
    /// What the volume gained over the pass.
    pub freed_bytes: u64,
}

/// How far this machine lets a Job on your words act, as the wire spells
/// `fleet::on_your_words::Authority`. Its own type here rather than a shared one: `ipc` names
/// nothing from `fleet`, and a wire enum outlives the Rust type it mirrors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionAuthority {
    Acting,
    ReadOnly,
}

/// A set of Doctor probes Fleet cannot run.
///
/// **Grouped by who owns them, not listed by name.** The modules are named in
/// `docs/concepts/doctor.md`, and the crate boundary that keeps a vendor's
/// spellings inside `adapters` is the reason they are not restated here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unprobed {
    /// Which crate or surface owns the probes.
    pub owner: String,
    /// Why Fleet cannot run them, and where their list is. Read by a person,
    /// never matched on.
    pub because: String,
}
