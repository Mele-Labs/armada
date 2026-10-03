// A Job that dispatched a wave of Jobs, and which of them waits on which.
// Draft, for `crates/ipc/src/detail.rs`.
//
// Source of truth: `JobSummary.dispatched_by`, the parent's id on every row it
// dispatched (protocol 14.2), and `dispatched_pass`, the pass of its plan that
// proposed it (23.11, #1692). **The pass is what says a row is a wave's**: a
// member of a landing order carries none.
//
// **The order is on each child's `JobDetail.dependencies`**, which the Board
// row does not carry, so a wave read off the rows alone draws one column.
// Every other fact a wave draws is served — each Job's status, its delivery,
// its held command and its Judge question.
//
// **No kind name.** A Job is a Job (#1530, 22 Sep).

import type { Criterion, JobDetail, JobSummary, Settled } from "@armada/protocol";

/**
 * One Job of the wave.
 *
 * **`waits_on` points at what this Job waits for**, never at what waits for
 * it. Drawn from it, the Job waited on comes first and the one waiting sits
 * behind it — the reverse reads as a roll-up feeding the work it rolls up.
 */
export type WaveJobView = {
  /** The Job's id, which is what a press opens. */
  job: string;
  title: string;
  /** A registered `job_status`. Never a word this file invents. */
  status: string;
  /** What a person calls it. Absent where the Board row has not arrived. */
  handle?: string;
  /**
   * Which pass of the loop dispatched it, counted from one. A loop return
   * replaces `plan.md` whole, so an earlier pass is history rather than a
   * second live split — `WaveView.rounds` carries that difference.
   */
  round: number;
  /** The Jobs this one waits on, by id. Empty is a Job that may start at once. */
  waits_on: readonly string[];
  /** Where its pull request settled, where it has. `Settled` on the wire. */
  landed?: Settled;
  /**
   * What its Drone is handed, and what the split expects of it — the plan's
   * own brief for this piece, as `JobDetail.facts` and `acceptance_criteria`
   * carry them. The Board carries neither, so today they are the mock's.
   *
   * **Criteria and not lines**, because Edit this Job sends them back to
   * `edit_job` (since 23.8): a line has no id, and a list of lines would mint
   * new criteria at every save and drop each one's source and origin.
   */
  facts?: string;
  criteria?: readonly Criterion[];
  /** What it has cost so far, in millionths of a dollar. Absent until it has spent. */
  cost_micros?: number;
  /** How far through its own plan it is. Absent for a Job with no tasks yet. */
  tasks?: { done: number; of: number };
};

/** One pass of plan and roll up, and what its split was for. */
export type WaveRoundView = {
  round: number;
  /**
   * What this pass split the work into, a few words — the strip's label.
   * **Absent off the wire**: Fleet stamps no per-pass line yet, and the strip
   * then names the wave alone rather than a sentence standing in for one.
   */
  says?: string;
  /**
   * Whether this is the plan the Job is running now. Every earlier round is
   * history and says so, rather than reading as a second live split.
   */
  live: boolean;
};

/** A Job and the wave it dispatched. */
export type WaveView = {
  /** The parent Job's id. */
  job: string;
  title: string;
  /** Every pass of the loop, oldest first. */
  rounds: readonly WaveRoundView[];
  /** Every Job the wave dispatched, in the order the plan listed them. */
  jobs: readonly WaveJobView[];
  /**
   * The model that read the live split for its Judge. `StepDetail.judged`
   * carries each criterion's verdict and not who gave it, so this is the
   * mock's until Fleet stamps it.
   */
  judged_by?: string;
};

/**
 * The wave a Job proposed, from the Board's own rows: each one it dispatched
 * that carries the pass that proposed it.
 *
 * **A Job with no such row proposed no wave**, and `undefined` is that answer —
 * never an empty graph, which reads as Jobs that failed to load. A row with
 * no pass is a landing order's member, which is the members band's.
 *
 * **`waits_on` comes back empty and that is honest.** The order is on each
 * child's `JobDetail.dependencies`, which no row carries, so every Job reads
 * as one that may start at once and the graph draws one column.
 */
export function waveOf(detail: JobDetail, board: readonly JobSummary[]): WaveView | undefined {
  const jobs = board.flatMap((row): WaveJobView[] =>
    row.dispatched_by !== detail.job.id || row.dispatched_pass === undefined
      ? []
      : [
          {
            job: row.id,
            title: row.title,
            status: row.status,
            handle: row.handle,
            round: row.dispatched_pass,
            waits_on: [],
            ...(row.landed === undefined ? {} : { landed: row.landed }),
          },
        ],
  );
  if (jobs.length === 0) return undefined;
  const passes = [...new Set(jobs.map((one) => one.round))].sort((one, two) => one - two);
  const latest = passes.at(-1);
  return {
    job: detail.job.id,
    title: detail.job.title,
    rounds: passes.map((round) => ({ round, live: round === latest })),
    jobs,
  };
}
