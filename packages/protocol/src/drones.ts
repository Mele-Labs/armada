// Every Drone one Job has had, as `GET /jobs/:job_id/drones` answers.
// `crates/ipc/src/drones.rs`, `list_job_drones`.
//
// **Off the Job's record, never the roster.** `list_drones` loses a Drone the
// moment it exits; this keeps every one the history names, running or not.
//
// Hand-written like `resources.ts`, for the same reason: the codegen that would
// emit the DTOs from the Rust source does not exist yet.

/**
 * Where one of a Job's Drones is.
 *
 * **`killed` and `failed` are two answers, because they want two responses.**
 * `killed` is a person ending it, by `kill_drone` or `kill_job`; `failed` is a
 * Drone that left without its step passing, on its own. `done` is one whose
 * step passed its advance gate, or reached a person's gate, while it was on it.
 */
export type DroneState = "running" | "done" | "failed" | "killed";

/**
 * One Drone a Job has had. **There is no task**: Fleet runs one Drone per step,
 * and nothing joins a Drone to a plan task.
 */
export type JobDrone = {
  drone_id: string;
  /** The step it was put on. A slot does not outlive a step boundary, so it has one. */
  step_id: string;
  /**
   * The task it was put on, `T1` and on, on a step that works its tasks a Drone
   * each. **Absent on a Drone that worked its whole step.** Since 23.1.
   */
  task?: string;
  /**
   * The model it was spawned as: a person's pick on its task, the Job's tier
   * map for the task's tier, the step's model, or the Job's, in that order.
   * **Absent on a Drone spawned before 23.5**, which recorded none.
   */
  model?: string;
  state: DroneState;
  /** When it was spawned onto the step, off `drone_spawned`. */
  since: string;
  /** When it left, off `drone_exited`. **Absent while it runs.** */
  ended_at?: string;
  /**
   * Turns taken, summed across its session's terminating lines. **Absent where
   * none has been seen**, never nought: a running Drone in its first invocation
   * has taken turns nothing has counted yet.
   */
  turns?: number;
  /**
   * What it has cost, in millionths of a dollar, as of its last terminating
   * line. **Absent is a Drone that never named a price**, not a price of
   * nothing. A running Drone carries one as of its last finished invocation, so
   * it trails whatever it has done since.
   */
  cost_micros?: number;
};

/** Every Drone a Job has had, in the order they were spawned. */
export type JobDrones = {
  job_id: string;
  drones: JobDrone[];
};
