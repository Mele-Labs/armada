// A Job's plan and its tasks. `crates/ipc/src/work_plan.rs`. Since 13.21.
//
// **`work_plan`, not `plan`**: `DeclaredPlan` in `work.ts` is where a step said
// its work would be, and this is what the work is. The header rules in
// `protocol.ts` hold: hand-written, and every closed set left as `string`.

import type { Verdict } from "./detail";

/** An approach and its tasks, in plan order, with the history folded away. */
export type WorkPlan = {
  approach: string;
  /** Who recorded the plan as it stands. A later whole recording replaces it. */
  recorded_by: ChangedBy;
  recorded_at: string;
  tasks: PlanTask[];
  /**
   * The groups, in the order they run, each naming its tasks and its runs.
   * Absent from a Fleet before 23.4, which ran a plan as one group.
   */
  groups?: PlanGroup[];
};

/**
 * One group of the plan: the tasks the step's gate runs at the end of, and how
 * each of its runs went. Since 23.4.
 */
export type PlanGroup = {
  /** `G1`, `G2`, … — minted by Fleet at the recording, never renumbered. */
  id: string;
  /** Its tasks' ids, in plan order. Empty where a move took every one out. */
  tasks: string[];
  /**
   * `pending`, `running`, `joining`, `checking`, `passed`, `failed`,
   * `retrying` or `landed`. Fleet writes `pending`, `running`, `retrying`,
   * `passed` and `failed` since 23.4.
   */
  state: string;
  /** When its first run began. */
  started_at?: string;
  /** When its last run was answered, with none open since. */
  ended_at?: string;
  /** Every run, oldest first. Absent where it has not run. */
  attempts?: PlanGroupRun[];
};

/** One run of a group. Since 23.4. */
export type PlanGroupRun = {
  /** Which run of the group, from one: a `CheckRun`'s `group_attempt`. */
  attempt: number;
  step_id: string;
  /** The step's run it was filed under: a `CheckRun`'s `attempt`. */
  step_attempt: number;
  started_at: string;
  ended_at?: string;
  /** What its gate came to. Absent while it is open. */
  verdict?: Verdict;
  /** The commit a green run made, where it made one. */
  commit?: string;
};

/** A run of a step, or a person. */
export type ChangedBy =
  | { by: "step"; step_id: string; attempt: number }
  | { by: "person" };

/** One line of the plan. */
export type PlanTask = {
  /** `T1`, `T2`, … — stable for the life of the plan, never renumbered. */
  id: string;
  title: string;
  /**
   * One line for what the other fields cannot hold — the exact new wording, a
   * gotcha the planner found. Absent where the task has none. **Was `detail`
   * before 15.0**, and not the place paths go any more.
   */
  note?: string;
  /**
   * The repository-relative paths the planner said this task touches, in the
   * order given. Absent where the task names none. Since 15.0.
   */
  scope?: string[];
  /**
   * What the planner said should prove this task, written with the plan.
   * Absent where none was named. Since 15.0.
   */
  expects?: string;
  /**
   * What the work said actually proved it, written by whoever did it. **Read
   * beside `expects` rather than in place of it**: the two disagreeing is the
   * fact worth drawing. Since 15.0.
   */
  shown?: string;
  /**
   * `open`, `working`, `handed_in`, `done`, `failed` or `dropped`. A claim, and
   * it gates nothing. `handed_in` and `failed` since 22.0, written by Fleet
   * alone: the first at a task Drone's hand-in, the second when Checks go red.
   */
  state: string;
  /** Present on a dropped task and on nothing else. */
  reason?: string;
  /** The group it runs in, `G1` and on. Since 23.4. */
  group?: string;
  /**
   * Present on a failed task and on nothing else: which group's Checks were
   * still red on which run. Since 23.4.
   */
  failed_reason?: string;
  /**
   * `difficult`, `medium` or `easy`: how hard the planner thought it was, which
   * picks its model off the Job's `tiers`. **Absent is the planner leaving it
   * to Armada**, never a fourth word. Since 23.6.
   */
  tier?: string;
  /**
   * The model a person picked for this task with Edit this task, which beats
   * the Job's tier map. **Absent is nobody having picked**; the model a Drone
   * actually ran is on its `JobDrone.model`. Since 23.6.
   */
  model?: string;
  /**
   * Each stretch the task was marked `working`, oldest first. Since 14.5.
   * Absent on a task nobody marked working — and on a Fleet before 14.5.
   */
  working_windows?: WorkingWindow[];
};

/**
 * From the change that marked a task `working` to the change that moved it
 * out. **A turn belongs to the task whose window holds its instant** — and an
 * Edit no window holds, to the task whose `scope` names the file (`#1498`).
 * Never to one whose words match its path. `left` is absent while it is still
 * working.
 */
export type WorkingWindow = {
  entered: string;
  left?: string;
};

/**
 * How many tasks stand where. `done` over every count but `dropped` is the
 * figure a person reads: a handed-in task and a failed one join the total and
 * neither joins `done`.
 */
export type TaskCounts = {
  done: number;
  working: number;
  open: number;
  dropped: number;
  /** Since 22.0, and absent at zero. */
  handed_in?: number;
  /** Since 22.0, and absent at zero. */
  failed?: number;
};

/**
 * A Job's plan changed. **The counts ride along and the plan does not**: a row
 * redraws from these, and an open Job reads `get_job` for `work_plan`.
 */
export type JobPlanChanged = {
  job_id: string;
  tasks: TaskCounts;
  /** The task this change moved, `T1` and on. Absent on a whole recording. Since 23.1. */
  task?: string;
  /** That task's state after the change, a `PlanTask.state` word. Present exactly where `task` is. Since 23.1. */
  state?: string;
  /** The group a gate's verdict moved, on that change alone. Since 23.4. */
  group?: string;
  actor: string;
  at: string;
};

/**
 * A person adds a task to a Job's plan — `add_task`. Since 13.30, and carrying
 * `scope` and `expects` since 15.0.
 *
 * Only `title` has to say anything; `after` is the id it comes after, or `""`
 * for the end.
 */
export type AddTask = {
  title: string;
  note: string;
  scope: string[];
  expects: string;
  after: string;
};

/**
 * Restart this task — `restart_task`, `POST /jobs/{job_id}/tasks/{task_id}/restart`.
 * Since 23.4. **No body is valid**, and is the plain restart; `note` is what
 * the new Drone reads first, and is never blank.
 */
export type RestartTask = {
  note?: string;
};

/**
 * A person moves a task or a group — `move_plan`, `POST /jobs/{job_id}/plan/move`.
 * Since 23.4. With `task`, that task goes into `group` after the task `after`
 * names, or first where `after` is absent; without, `group` goes after the
 * group `after` names, or first. **By `after`, never by index.**
 */
export type MovePlan = {
  group: string;
  task?: string;
  after?: string;
};

/**
 * Edit this task — `edit_task`, `POST /jobs/{job_id}/tasks/{task_id}/edit`.
 * Since 23.6 (#1657). **Only the fields a person changed**: one left out is
 * unchanged, and `note` or `expects` sent empty, or `scope` sent as `[]`,
 * clears it. Taken on an open or a failed task; `model` is refused unless
 * `list_models` offers it.
 */
export type EditTask = {
  title?: string;
  note?: string;
  scope?: string[];
  expects?: string;
  model?: string;
};

/**
 * Which model each tier of a Job's tasks runs on — `JobDetail.tiers`. Since
 * 23.6. **A tier left out is Armada picking**: the Drone runs as its step, or
 * the Job, would, and its `JobDrone.model` says which. Never `null`.
 */
export type TierModels = {
  difficult?: string;
  medium?: string;
  easy?: string;
};

/**
 * A person sets a Job's whole tier map — `set_tiers`,
 * `POST /jobs/{job_id}/set_tiers`. Since 23.6. Refused, and nothing kept,
 * where any model is one `list_models` does not offer.
 */
export type SetTiers = {
  tiers: TierModels;
};

/**
 * A person drops a task from a Job's plan, with a reason — `drop_task`.
 * Since 13.30. `reason` is never blank.
 */
export type DropTask = {
  task: string;
  reason: string;
};
