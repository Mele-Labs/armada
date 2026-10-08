// Every Check run one repository's Jobs asked for or ran, in one read.
// `GET /manifest/checks`, `list_manifest_checks`. Since 23.40.
// The Rust half is `crates/ipc/src/manifest_checks.rs`.

import type { Outcome } from "./reads";
import type { Requester } from "./requester";

/** `source` of a row a gate wrote. */
export const SOURCE_GATE = "gate";
/** `source` of a run a Drone asked for. */
export const SOURCE_ASKED_RUN = "asked_run";

/**
 * The newest rows, at most 200, and whether there were more. **A read across
 * Jobs and no new record.** Merge-line Checks (`get_merge_lines`) and checkout
 * runs (`list_checkout_runs`) are not here.
 */
export type ManifestChecks = {
  /** Newest first. */
  rows: ManifestCheckRow[];
  /** How many there were before the cut. */
  total: number;
  /** True where `total` is more than `rows` holds. Absent otherwise. */
  truncated?: boolean;
};

export type ManifestCheckRow = {
  /** `gate` or `asked_run`. An opaque string. */
  source: string;
  requester: Requester;
  /** Absent on a Session's run, which belongs to no Job. Proposed, not on the wire. */
  job_id?: string;
  /** What a person calls the Job. */
  job_handle?: string;
  job_title?: string;
  step?: string;
  /** The Session whose agent ran it, on a `session_run` row. Proposed, not on the wire. */
  session_id?: string;
  /** Which run of the step. */
  attempt: number;
  /** The group whose gate it ran at, `G1` and on. Absent at a step's own gate. */
  group?: string;
  /** A gate row's Check, or an asked run's Checks joined by `, `. */
  name: string;
  /**
   * A gate row's outcome (`passed`, `failed`, `signalled`, `timed_out`,
   * `never_ran`, `skipped`) or an asked run's state (`running`, `passed`,
   * `failed`, `stopped`, `lost`). **Since 23.64 also `waiting`** on a gate row and an asked run,
   * and `running` on a gate row, which has `started_at` and no `ended_at`. Opaque strings.
   */
  state: string;
  /** **Absent on a gate row**, which records when its ruling was written. */
  started_at?: string;
  /** A gate row's write, an asked run's close. Absent while an asked run goes. */
  ended_at?: string;
  /** Start to end on an asked run that has both. **A gate row has none.** */
  took_ms?: number;
  /** Each openable with `get_check_output` on `job_id` and the log's `kept`. */
  logs?: ManifestCheckLog[];
  /** The asked run's own id, on an `asked_run` row. */
  asked_run_id?: number;
};

/**
 * What `list_manifest_checks` came back as. Answered to the caller rather than held as state,
 * `CheckOutputRead`'s reason: the Checks page asks once on opening and again as a run ends.
 * Bridge-only, not on the wire.
 */
export type ManifestChecksRead = { ok: true; checks: ManifestChecks } | { ok: false; outcome: Outcome };

export type ManifestCheckLog = {
  check: string;
  /** The log file's own name, `get_check_output`'s `:kept`. */
  kept: string;
};
