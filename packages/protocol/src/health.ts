// What Fleet can say about its own health, and what it cannot — `GET /health`.
// `crates/ipc/src/health.rs`.
//
// **Not Doctor.** Doctor's grid is modules probed from `adapters`, `config` and
// `store`, and it is not built — `docs/concepts/doctor.md`. This answers the
// rows Fleet itself holds, and names the rest as unprobed.
//
// The header rules in `protocol.ts` hold here: these are hand-written, and they
// drift the day a field moves.

/** One module, asked. */
export type Probe = {
  /** The module name as Doctor's grid spells it — `Fleet`, `SQLite`, `Manifest`, `System stats`. */
  module: string;
  /** `pass`, `warn` or `fail`, Doctor's own three words. A string: nothing matches on it. */
  outcome: string;
  /** What the probe read, in one line. Never empty. */
  detail: string;
};

/**
 * A set of Doctor probes Fleet cannot run, grouped by who owns them rather than
 * listed by name.
 */
export type Unprobed = {
  /** Which crate or surface owns the probes. */
  owner: string;
  /** Why Fleet cannot run them. Read by a person, never matched on. */
  because: string;
};

/**
 * How far this machine lets a job on your words act, as `crates/fleet/src/on_your_words.rs`'s
 * `Authority` spells it on the wire.
 */
export type ActionAuthority = "acting" | "read_only";

/**
 * Every probe Fleet ran, and everything it could not run one for. **`not_probed`
 * is the honest half**: four passing rows with no mention of the modules nobody
 * asked read as a healthy machine.
 */
export type FleetHealth = {
  probes: Probe[];
  not_probed: Unprobed[];
  /**
   * `settings.action-authority-tier-1-redirect-enabled-vs-read-only`,
   * resolved once when Fleet started. `#1127`.
   */
  action_authority: ActionAuthority;
  /**
   * Whether any served repository left a workflow definition out, so a rail can
   * warn without counting. A boolean and never a number: the list is
   * `GET /workflows/left_out`. Absent from a Fleet older than 23.33.
   */
  workflows_left_out?: boolean;
  /**
   * Free space where Jobs are built, against the two floors Fleet holds it to.
   * Absent from a Fleet older than the reclaim, and from one that has not yet
   * read the disk.
   */
  disk?: DiskHealth;
};

/** The disk under the first served repository. */
export type DiskHealth = {
  /** Free on the volume of the first served repository. */
  free_bytes: number;
  /** `limits.diskReclaimFloorGib` in bytes. Under it Fleet drops build output; 0 is off. */
  reclaim_floor_bytes: number;
  /** `limits.diskFloorGib` in bytes. Under it no new Job starts. */
  hold_floor_bytes: number;
  /** The last pass that dropped anything. Absent where none has since Fleet started. */
  last_pass?: ReclaimPass;
  /** Build folder paths in the order a pass would drop them next, at most ten. Absent over the reclaim floor. */
  next_to_drop?: string[];
};

/** One pass under the reclaim floor. */
export type ReclaimPass = {
  /** When it ended, RFC 3339. */
  at: string;
  /** What the volume gained over the pass. */
  freed_bytes: number;
};
