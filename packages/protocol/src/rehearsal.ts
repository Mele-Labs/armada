// A person's run of one Manifest entry in the main checkout.
// `crates/ipc/src/rehearsal.rs`. Since protocol 10.6.
//
// **A rehearsal, never a verdict.** Nothing here is a Check row or Evidence.
// Bridge draws a run's result from its code and `stopped` — how it looks, not
// what it counts.

import type { Requester } from "./requester";
import type { ProtocolId } from "./connection";
import type { Outcome } from "./reads";
import type { ServerEntry } from "./servers";

/**
 * One file a run changed. **A name and a kind, never bytes.** `change` is one of
 * `added`, `modified`, `deleted`, `renamed`, `copied`, `type_changed`,
 * `conflicted`, `unreadable`, left as `string` like every other closed set.
 */
export type ChangedFile = {
  /** Repository-relative, exactly as git spells it. */
  path: string;
  change: string;
};

/** `setup.seed` as the Manifest declares it, and where its warm-up stands. Since protocol 13.46. */
export type DeclaredSeed = { paths: string[]; warmed_by: string[]; warmth: SeedWarmth };

/** Whether the seed at the current base commit may be cloned. */
export type SeedWarmth =
  | { state: "warm"; commit: string }
  | { state: "warming"; commit: string }
  | { state: "cold"; why: string };

export type RunEntry = {
  name: string;
  /** The `run` line, exactly as declared. */
  run: string;
  /** It declares `narrow`. */
  narrows: boolean;
  /** What a narrowed run resolves to. Absent where it declares no `narrow`. */
  narrow_run?: string;
  /** The Commands that run first, in order. */
  requires: string[];
  expect_exit_code: number;
  destructive: boolean;
  frozen: boolean;
};

/** `GET /manifest/runs/:run_id/output` — `CheckOutput`'s window. */
export type RunOutput = {
  id: string;
  name: string;
  path: string;
  lines: string[];
  from_line: number;
  total_lines: number;
  bytes: number;
  whole: boolean;
};

/** `stop_checkout_run`'s and `undo_checkout_run`'s body. */
export type NamedRun = { id: string };

/** What `get_checkout_run_output` came back as, answered to the caller rather than held as state. */
export type RunOutputRead = { ok: true; output: RunOutput } | { ok: false; outcome: Outcome };

/** `GET /manifest/run_sheet` — what can be run in the main checkout. */
export type CheckoutRunSheet = {
  /** The Commands `setup.requires` names, in its order. */
  setup: RunEntry[];
  /** Every Check, in the order the Manifest declares them. */
  checks: RunEntry[];
  /** Every Command `setup.requires` does not name. */
  commands: RunEntry[];
  /** When the Manifest was last changed by a commit. */
  manifest_edited_at?: string;
  /** The run in flight in the checkout, if one is. */
  running?: CheckoutRunUnderway;
  /** The Commands declaring `serve`, each with the checkout's instance. */
  servers?: ServerEntry[];
  /**
   * The latest Verify, underway or ended and not yet replaced. **Absent where
   * none has run since Fleet started**: it is held in memory, and each step's
   * own record is a run like any other.
   */
  verify?: CheckoutVerify;
  /** Each workspace below the root with its own `armada.yml`, and the Commands it declares. */
  workspaces?: WorkspaceCommands[];
  /** The seed `setup.seed` declares, and how warm it is. Absent where the Manifest declares none. Since protocol 13.46. */
  seed?: DeclaredSeed;
};

/** One workspace's own Commands. `dir` is relative to the repository root. */
export type WorkspaceCommands = { dir: string; commands: RunEntry[] };

/**
 * `POST /manifest/start_run`'s body. A name and nothing else.
 */
export type StartCheckoutRun = { name: string; workspace?: string };

/** A checkout run that has started and not finished. */
export type CheckoutRunUnderway = {
  id: string;
  name: string;
  command: string;
  /** Elapsed time is counted from here; nothing ticks on the wire. */
  started_at: string;
  /** The workspace whose own file declared it. Absent is the root's. */
  workspace?: string;
  /** Who asked for it. Since 23.40; **absent reads as `outside`** (`requesterOf`). */
  requester?: Requester;
};

/** One finished checkout run. `GET /manifest/runs`'s row. */
export type CheckoutRunRecord = {
  id: string;
  name: string;
  /** The workspace whose own file declared it. Absent is the root's. */
  workspace?: string;
  command: string;
  required: string[];
  started_at: string;
  ended_at: string;
  duration_ms: number;
  /** Absent where there was no code: a signal, a budget, a spawn that failed. */
  exit_code?: number;
  expect_exit_code: number;
  /** How it ended, in a sentence. A rehearsal: the hue is Bridge's reading. */
  ended: string;
  stopped: boolean;
  changed: ChangedFile[];
  /** Why the change could not be read. `changed` then means nothing. */
  changed_unreadable?: string;
  /**
   * Whether Undo is there to offer. This tree holds a person's own
   * uncommitted work, so a run with no snapshot behind it offers none.
   */
  undoable: boolean;
  undone_at?: string;
  /** The log, relative to `ManifestSummary.records_root`. */
  log: string;
  /** Who asked for it. Since 23.40; **absent reads as `outside`** (`requesterOf`). */
  requester?: Requester;
};

/**
 * `GET /manifest/runs/:run_id/diff` — what one checkout run changed.
 *
 * **Never against `HEAD`**: this tree holds a person's own uncommitted work,
 * and a patch against `HEAD` would show all of it as the run's. `against` says
 * so on the wire rather than leaving a reader to assume it.
 */
export type CheckoutRunDiff = {
  id: string;
  /** The tree just before the run against the tree just after it. */
  against: "run_snapshot";
  reading: RunDiffReading;
};

/** The patch, or why there is none. No patch stands in for a gone snapshot. */
export type RunDiffReading =
  | {
      state: "read";
      /** `CheckoutRunRecord.changed`, read beside the patch. Empty: nothing changed. */
      files: ChangedFile[];
      /** Git's unified diff. Absent where there is nothing in it. */
      patch?: string;
    }
  | { state: "gone"; why: string };

/** `GET /manifest/runs` — newest first, and what would not read. */
export type CheckoutRunList = {
  runs: CheckoutRunRecord[];
  unreadable: { id: string; why: string }[];
};

/**
 * One message on `GET /manifest/runs/:run_id/observe`: the log so far, then new lines, a count
 * where the bound dropped some, and why it stopped. What ended the run is
 * `checkout_run.finished`'s; this carries only what the run printed.
 */
export type CheckoutRunMessage =
  | ({ message: "opened" } & CheckoutRunOpened)
  | ({ message: "lines" } & { lines: string[] })
  | ({ message: "missed" } & { dropped: number })
  | ({ message: "closed" } & { because: string });

export type CheckoutRunOpened = {
  protocol_id: ProtocolId;
  id: string;
  name: string;
  path: string;
  /** The run was still going when this opened. */
  live: boolean;
  /** Older lines the opening read left out, because the window is bounded. */
  skipped: number;
};

/**
 * `GET /manifest/run_sheet`, in four states. Bridge-only, not on the wire.
 */
export type CheckoutRunSheetRead =
  | { state: "none" }
  | { state: "reading" }
  | { state: "read"; sheet: CheckoutRunSheet }
  | { state: "failed"; outcome: Outcome };

/**
 * The checkout run a window is watching on `observe_checkout_run`, as main
 * holds it, one run at a time. Bridge-only, not on the wire.
 */
export type CheckoutRunFollowed =
  | { state: "none" }
  | { state: "opening"; runId: string }
  | {
      state: "following";
      runId: string;
      name: string;
      path: string;
      /** The file's own line number of `lines[0]`, counted from one. */
      fromLine: number;
      /** The newest lines, oldest first, bounded. */
      lines: string[];
      /** Why the stream ended, or absent while it is still arriving. */
      ended?: string;
    }
  | { state: "failed"; runId: string; detail: string };

/** What `list_checkout_runs` came back as. `RunListRead`'s shape and reason. */
export type CheckoutRunListRead =
  | { ok: true; runs: CheckoutRunList }
  | { ok: false; outcome: Outcome };

/**
 * What `get_checkout_run_diff` came back as. `RunListRead`'s shape, and asked
 * once by whoever pressed *Open the diff*: a finished run's patch does not move.
 */
export type CheckoutRunDiffRead =
  | { ok: true; diff: CheckoutRunDiff }
  | { ok: false; outcome: Outcome };

// ---------------------------------------------------------------------------
// Verify — Journey 9, *Verify*: setup and every Check once, in the checkout.
// ---------------------------------------------------------------------------

/** `POST /manifest/start_verify`'s body, which may be empty: no `workspace` is the root's Manifest. */
export type StartCheckoutVerify = { workspace?: string };

/**
 * `POST /manifest/start_verify`'s answer, and `CheckoutRunSheet.verify`.
 *
 * **A sequence of ordinary checkout runs**, so nothing here restates a run:
 * a step that ran carries its record. A rehearsal of the whole file, never a
 * verdict on it — no state below maps onto a status colour.
 */
export type CheckoutVerify = {
  id: string;
  started_at: string;
  /** Absent while it is underway. */
  ended_at?: string;
  /** The workspace whose own `armada.yml` this ran, relative to the root. Absent is the root's. */
  workspace?: string;
  /** Who asked for it. Since 23.40; **absent reads as `outside`** (`requesterOf`). */
  requester?: Requester;
  /** Setup in `setup.requires` order, then every Check in written order. */
  steps: VerifyStep[];
};

/** One step. `group` is `setup` or `checks`, and is rendered as its label. */
export type VerifyStep = { group: string; name: string; run: string } & VerifyStepState;

/** Where one step has got to. `not_run` says why in a sentence. */
export type VerifyStepState =
  | { state: "waiting" }
  | { state: "running"; run_id: string }
  | { state: "ran"; record: CheckoutRunRecord }
  | { state: "not_run"; why: string };
