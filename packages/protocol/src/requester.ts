// Who asked for a Check run, and a Drone's own runs as rows of their own.
// Since 23.40. The Rust half is `crates/ipc/src/requester.rs` and `asked.rs`.

/**
 * Who asked for one run, as a value a surface can follow to the place that
 * asked.
 *
 * **`kind` is an opaque string**, as a Record row's is: a kind this build has
 * never heard of is drawn from the ids it carries, and is never a type error.
 * The ids each kind needs are beside it, absent on every kind that does not use
 * them.
 *
 * | `kind` | Who | Ids |
 * |---|---|---|
 * | `gate` | A Job's step gate | `job_id`, `step` |
 * | `drone_task` | A Drone asking on a plan task | `job_id`, `step`, `task_id`, `drone_id` |
 * | `drone_step` | A Drone asking on a step with no task | `job_id`, `step`, `drone_id` |
 * | `merge_line` | The merge line, for one branch | `branch` |
 * | `outside` | Nothing in Armada: a person's press, a bare `armada check` | none |
 *
 * **`outside` is a value, and an absent requester reads as it** — a Fleet
 * before 23.40 sends none, and a fixture may not.
 */
export type Requester = {
  kind: string;
  job_id?: string;
  step?: string;
  /** `T3` and on, as the plan names a task. */
  task_id?: string;
  drone_id?: string;
  branch?: string;
  /**
   * What a person calls the Job, `1-a-job`. **A Drone's handle**: its transcript
   * is named under it and it has no other name than its id. Present on every kind
   * that names a Job, where Fleet knows it. Since 23.40.
   */
  handle?: string;
};

export const REQUESTER = {
  gate: "gate",
  droneTask: "drone_task",
  droneStep: "drone_step",
  mergeLine: "merge_line",
  outside: "outside",
} as const;

/** What a row that carries no requester reads as. */
export const OUTSIDE: Requester = { kind: REQUESTER.outside };

/** The requester a row carries, or `outside` where it carries none. */
export function requesterOf(row: { requester?: Requester }): Requester {
  return row.requester ?? OUTSIDE;
}

/**
 * Where an asked run stands. `lost` is a run whose task died, or whose Fleet
 * went away while it was `running`. Read as an opaque string by anything that
 * draws it, so a later state is a minor bump.
 */
export type AskedRunState = "running" | "passed" | "failed" | "stopped" | "lost";

/**
 * A Drone's own run of a step's Checks, **its own row and never a `CheckRun`**:
 * a dry result must not be readable as a gate's pass. On `StepDetail.asked_runs`
 * and `RunList.asked_runs`, oldest first.
 */
export type AskedRun = {
  /** Fleet's own number for the row, unique across restarts. */
  id: number;
  /** A `drone_task` or `drone_step` requester. */
  requester: Requester;
  /** Which run of the step it was asked in. */
  attempt: number;
  started_at: string;
  /** Absent while it runs, and on a run nobody saw end. */
  finished_at?: string;
  state: AskedRunState;
  /** The Checks the run was about, in the step's order. */
  checks: string[];
  narrowed: boolean;
  /** The one Check the ask named, where it named one. */
  only_check?: string;
  /** One per entry of `checks`, in its order, empty where that Check kept none. Absent while it runs. */
  logs?: string[];
};
