// The wire vocabulary, as TypeScript sees it.
//
// **These are hand-written, and that is a gap rather than a design.**
// `crates/ipc/src/lib.rs` says a codegen step emits TypeScript from the Rust
// DTOs and that both generated outputs are checked in, so a cross-language
// breaking change is a build failure. That codegen does not exist yet, so the
// shapes below are a second statement of the ones in `crates/ipc/src/` and they
// drift the day a field moves. Nothing else in Bridge restates them.
//
// Every closed set is left as `string`. The Rust side refuses an arriving
// spelling the registry does not have; a union spelled here would be a third
// copy of a roster that already has two, and an unknown status renders as
// itself rather than as a guess.
//
// What is left here is the smaller neighbours no module of their own has yet
// earned, and the re-exports that keep this the one import for the wire
// vocabulary.

import type { Requester } from "./requester";

/** One Check a step declares. `crates/ipc/src/checks.rs`. */
export type DeclaredCheck = {
  /** `manifest_check` or `diff_nonempty`, as the WorkflowDef schema spells it. */
  kind: string;
  /** The Manifest Check's name. Absent on `diff_nonempty`, which names none. */
  name?: string;
  /** The command the Check resolved to, as the Job's frozen workflow holds it.
   * **Absent on `diff_nonempty`**, which runs nothing. Always the frozen
   * workflow's and never the live Manifest's — editing `armada.yml` mid-Job
   * must not change what a finished step says it ran. */
  run?: string;
  /** The exit code the step expects, where there is a command to return one. */
  expect_exit_code?: number;
  /**
   * Which paths this Check covers, as the Job's frozen workflow holds them.
   * A step that changes none of them does not run it.
   *
   * **Absent means always, and it is never `[]`.** Always and never are
   * opposite answers and Fleet sends no key at all for the first, so a client
   * has nothing to disambiguate. Draw it *before* the Check runs — that is the
   * only moment it says anything the `skipped` row will not say later.
   */
  when?: string[];
  /**
   * Where the Manifest says this Check runs. `gate` is never in a Drone's own
   * run; `handoff` runs last, and only on the step before handoff. **Absent
   * means everywhere.** Since 13.48, #849.
   */
  runs_at?: "gate" | "handoff";
};

/**
 * One `judge_checks[]` entry a step declares, counted rather than quoted.
 * `crates/ipc/src/checks.rs`. **The declaration, never the answer** — what the
 * Judge said is `Judged`, one row per criterion. No question crosses: a
 * question is a prompt in a screenshot.
 */
export type DeclaredJudge = {
  /** How many yes/no questions this entry asks. */
  criteria: number;
  /** How many judges answer each one. **Absent at one**, so present means a
   * panel — a client comparing against `1` would restate the domain's default. */
  panel_size?: number;
};

/**
 * One declared Check, as the gate found it.
 *
 * **`produced` is absent on a pass because a pass measured nothing** — the
 * outcome is the whole sentence. The other five each say something different
 * about why a step did not pass, and none of them is `failed`.
 *
 * **`skipped` is the one that did not stop the step.** The Check declares which
 * paths it covers and this step changed none of them, so it was not run — it is
 * not a pass and it is not a failure, and a surface that drew it as either
 * would be reporting a verification that never happened. `produced` names the
 * paths it covers.
 */
export type CheckRun = {
  /** Which run of the step produced it, counted from one. Since 7.0. Joins
   * to `StepDetail.attempts`. */
  attempt: number;
  /** The Manifest Check's name, or the built-in's kind. Joins to `checks`. */
  name: string;
  /** `check-outcomes.toml`: `passed`, `failed`, `signalled`, `timed_out`, `never_ran`, `skipped`. */
  outcome: string;
  /** What the Check was measured against. Absent on a pass, and on a skip. */
  expected?: string;
  /**
   * The exit code, the signal, the budget it outran, what is not installed —
   * or, on a skip, the paths the Check covers and this step did not touch.
   */
  produced?: string;
  /**
   * Where the Check's stdout and stderr were written, relative to the
   * repository root. **A reference, never the content**, and absent where there
   * is no file — a built-in assertion runs no command, and a Check that never
   * started printed nothing.
   *
   * **Main opens it; the renderer never composes it into a path.** See
   * `artifacts.ts`, which owns that rule for all three kept records.
   */
  output_path?: string;
  /**
   * When a Drone's own asked run answered this Check, rather than the gate
   * running it again. Absent on every Check the gate ran itself. Since
   * protocol 13.39.
   */
  reused_from_dry_run?: string;
  /**
   * Who asked for this run. Present from 23.40; **absent reads as `outside`**
   * (`requesterOf`).
   */
  requester?: Requester;
};

/**
 * A Check's own output, as `GET /sessions/checks/:run/output` answers.
 *
 * **The other half of `CheckRun.output_path`.** The row says where the output
 * was written; this is the read that brings the lines into the app, asked for
 * once by the person who opened one Check. Output is up to two 64 KiB streams,
 * so the cheap fact rides the row and the bytes are fetched. **Never on the
 * event stream.**
 *
 * **A window, and every number on it describes the file.** `from_line` is the
 * file's own numbering, not the window's, so a reader citing line 1,940 means
 * the file's 1,940th line. Since protocol 9.1.
 */
export type CheckOutput = {
  /** Which run of the step wrote it, counted from one. Joins to `CheckRun.attempt`. */
  attempt: number;
  /** The Check whose output this is. The same word `CheckRun.name` carries. */
  name: string;
  /**
   * Where it was read from, relative to the repository root. **What
   * `CheckRun.output_path` already said**, echoed so a surface drawing the path
   * beside the reading takes it off the answer rather than composing it.
   */
  path: string;
  /**
   * The window, oldest line first, verbatim. **Never rewritten** — the marker
   * lines Fleet writes between the two streams are lines of the file like any
   * other, and a reader that stripped them would show stdout and stderr with
   * nothing saying which is which.
   */
  lines: string[];
  /** The number of `lines[0]` in the whole file, counted from one. */
  from_line: number;
  /** How many lines the file has. Exact, because the file is counted as it is read. */
  total_lines: number;
  /** What the file weighs, in bytes. */
  bytes: number;
  /**
   * Whether `lines` is all of it. **Stated rather than inferred from
   * `from_line === 1`**, because a surface that derived it would call a
   * partial answer whole the first time that field's meaning moved.
   */
  whole: boolean;
};
// Every event shape, re-exported so `protocol.ts` stays the one import for the
// wire vocabulary. They live in `events.ts` because this file reached the 900
// lines the gate refuses; the cut is the socket seam `crates/ipc/src/event.rs`
// already draws.
export type { Delivered, Event, Missed, Resync, StreamMessage } from "./events";

// Fleet's own reading of its Manifest, re-exported so `protocol.ts` stays the
// one import for the wire vocabulary. It lives in `reading.ts` because that is
// the cut `crates/ipc/src/reading.rs` makes.
export type {
  ManifestFault,
  ManifestMoved,
  ManifestReading,
  ManifestRefused,
} from "./reading";

/** A failure, flattened for the wire. `docs/contracts/error-contract.md`. */
export type WireError = {
  /** Opaque to Bridge: looked up, never parsed. */
  code: string;
  /** What renders when the lookup misses. */
  message: string;
  run_id: string;
  fields: Record<string, unknown>;
  chain: string[];
  job_id?: string;
  drone_id?: string;
  step_id?: string;
};
