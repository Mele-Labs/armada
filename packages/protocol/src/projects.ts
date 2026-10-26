// A repository's projects on the wire, mirrored by hand from `crates/ipc/src/projects.rs`.
// `docs/practices/protocol/projects.md`.
//
// The header rules in `protocol.ts` hold here.

/**
 * A written task's own state. `held` is a Job working on it; `asked` is a Job finished and whether the task is done is yet
 * to be said, which outlives the Job. More may come: one this build does not know draws as open.
 */
export type ProjectTaskState = "open" | "done" | "held" | "asked";

/** The Job holding a task. */
export type ProjectHolder = {
  session_id: string;
  /** Absent where the Job has no title. */
  title?: string;
};

/** A row a Job starts with, in `StartSession.project_rows`. */
export type ProjectRowRef = {
  project: string;
  row: string;
};

/** One row of a project. The tag is `kind`. */
export type ProjectRow =
  /**
   * A task he wrote. `notes` is empty for none. `held_by` is present while `state` is `held` or `asked`, and only then.
   * **A subtask is a task with a `parent`**, the id of a top-level task of the same project, one level deep. Rows are flat:
   * a parent is followed by its subtasks, so a client indents a row that has a `parent`. `issues` are the issues linked to
   * the task, in the order linked; left out when none.
   */
  | {
      kind: "task";
      id: string;
      title: string;
      notes: string;
      state: ProjectTaskState;
      held_by?: ProjectHolder;
      parent?: string;
      issues?: TaskIssue[];
    }
  /**
   * A linked issue: `issue` is the tracker's reference only (`7`, `ABC-12`, `armada-3fz`). `title`
   * and `state` are the tracker's, read when the rows were, and absent where it could not say.
   */
  | { kind: "issue"; id: string; issue: string; title?: string; state?: "open" | "closed" };

/**
 * An issue linked to a task: the tracker's reference only is kept. `title` and `state` are the tracker's, read when the rows
 * were, and absent where it could not say.
 */
export type TaskIssue = {
  issue: string;
  title?: string;
  state?: "open" | "closed";
};

export type Project = {
  id: string;
  name: string;
  /** In order: a row's position is its place here. */
  rows: ProjectRow[];
};

/**
 * `GET /projects/:repository`, what every project write answers, and `projects.changed` published
 * whole. A client replaces what it holds for `manifest_id`.
 */
export type Projects = {
  manifest_id: string;
  /** In the order he keeps them. */
  projects: Project[];
};

/** `POST /projects/:repository` and `POST /projects/:repository/:project/rename`. */
export type NameProject = {
  name: string;
};

/**
 * `POST /projects/:repository/:project/tasks`, `.../rows/:row/edit` and `.../rows/:row/subtasks` (a task under the top-level task
 * `row`): title and notes, whole.
 */
export type ProjectTask = {
  title: string;
  notes?: string;
};

/**
 * `POST /projects/:repository/:project/issues`: `#7` or `7`, `ABC-12`, `armada-3fz`. The same body to `.../rows/:row/issues` puts the
 * issue on a task or subtask, and to `.../rows/:row/issues/unlink` takes it off.
 */
export type LinkProjectIssue = {
  issue: string;
};

/**
 * `POST /projects/:repository/:project/rows/:row/move`: `position` is counted from 0 among the rows of the parent the row ends
 * up under; past the end is the end. `parent` is the top-level task to make it a subtask of, `""` to take it to the top level,
 * and left out to keep the parent it has. A task with subtasks cannot go under another, and an issue row cannot go under any.
 */
export type MoveProjectRow = {
  position: number;
  parent?: string;
};

/** `POST /projects/:repository/:project/rows/:row/tick`: done, or open again. */
export type TickProjectTask = {
  done: boolean;
};
