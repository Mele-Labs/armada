// His projects on the wire, across repositories, and the tasks in none, mirrored by hand from `crates/ipc/src/projects.rs`.
// `docs/practices/protocol/projects.md`. **A project is his and has no repository**: every row names its own.
//
// The header rules in `protocol.ts` hold here.

/**
 * A written task's own state. `held` is a Job working on it; `asked` is a Job finished, or a pull request linked to the task
 * merged, and whether the task is done is yet to be said, which outlives the Job. More may come: one this build does not
 * know draws as open.
 */
export type ProjectTaskState = "open" | "done" | "held" | "asked";

/** The Job holding a task. */
export type ProjectHolder = {
  session_id: string;
  /** Absent where the Job has no title. */
  title?: string;
};

/** A row a Job starts with, in `StartSession.project_rows`. A row id is unique across projects and the loose rows, so it is the whole address. */
export type ProjectRowRef = {
  row: string;
};

/** One row of a project. The tag is `kind`. */
export type ProjectRow =
  /**
   * A task he wrote. `notes` is empty for none. `held_by` is present while `state` is `held`, and while it is `asked` by a Job;
   * a task asked about by a linked pull request's merge, which no Job held, has none.
   * **A subtask is a task with a `parent`**, the id of a top-level task of the same project, one level deep. Rows are flat:
   * a parent is followed by its subtasks, so a client indents a row that has a `parent`. `issues` are the issues linked to
   * the task, and `pulls` the pull requests, each in the order linked; left out when none.
   */
  | {
      kind: "task";
      id: string;
      /** The repository the task is in, by Manifest id. Moving it between projects keeps it. */
      manifest_id: string;
      title: string;
      notes: string;
      state: ProjectTaskState;
      held_by?: ProjectHolder;
      parent?: string;
      issues?: TaskIssue[];
      pulls?: TaskPull[];
    }
  /**
   * A linked issue: `issue` is the tracker's reference only (`7`, `ABC-12`, `armada-3fz`). `title`
   * and `state` are the tracker's, read when the rows were, and absent where it could not say.
   */
  | { kind: "issue"; id: string; manifest_id: string; issue: string; title?: string; state?: "open" | "closed" };

/**
 * An issue linked to a task: the tracker's reference only is kept. `title` and `state` are the tracker's, read when the rows
 * were, and absent where it could not say.
 */
export type TaskIssue = {
  issue: string;
  title?: string;
  state?: "open" | "closed";
};

/**
 * A pull request linked to a task: the number only is kept. `title` and `state` are the forge's, read when the rows were,
 * and absent where it could not say. A draft is `open`.
 */
export type TaskPull = {
  number: number;
  title?: string;
  state?: "open" | "merged" | "closed";
};

export type Project = {
  id: string;
  name: string;
  /** In order, across repositories: a row's position is its place here. */
  rows: ProjectRow[];
};

/**
 * `GET /projects`, what every project write answers, and `projects.changed` published whole. A client replaces
 * what it holds.
 */
export type Projects = {
  /** In the order he keeps them. */
  projects: Project[];
  /** The tasks in no project, of every repository, in their own order. */
  loose: ProjectRow[];
};

/** `POST /projects` and `POST /projects/project/:project/rename`. */
export type NameProject = {
  name: string;
};

/** `POST /projects/rows/:row/edit` and `POST /projects/rows/:row/subtasks` (a task under the top-level task `row`): title and notes, whole. */
export type ProjectTask = {
  title: string;
  notes?: string;
};

/** `POST /projects/tasks`: a task in the repository `manifest_id`, in the project `project` or, left out, in none. */
export type NewProjectTask = {
  manifest_id: string;
  project?: string;
  title: string;
  notes?: string;
};

/** `POST /projects/issues`: an issue of the repository `manifest_id`, `#7` or `7`, `ABC-12`, `armada-3fz`, a row of `project` or, left out, a loose one. */
export type NewProjectIssue = {
  manifest_id: string;
  project?: string;
  issue: string;
};

/** `POST /projects/rows/:row/issues` puts an issue on a task or subtask, and `.../issues/unlink` takes it off: `#7` or `7`, `ABC-12`, `armada-3fz`. */
export type LinkProjectIssue = {
  issue: string;
};

/**
 * `POST /projects/rows/:row/move`: `position` is counted from 0 among the rows of the parent the row ends up under; past the end
 * is the end. `project` is the project it goes into, `""` for none, and left out to keep the one it is in. `parent` is the
 * top-level task to make it a subtask of, `""` to take it to the top level, and left out to keep the parent it has, or the top
 * level where the project changes. A task with subtasks cannot go under another, and an issue row cannot go under any. The row
 * keeps its repository, and a task's subtasks go with it.
 */
export type MoveProjectRow = {
  project?: string;
  position: number;
  parent?: string;
};

/** `POST /projects/rows/:row/tick`: done, or open again. */
export type TickProjectTask = {
  done: boolean;
};

/**
 * `POST /projects/rows/:row/job`: a Job of the task's repository that has not ended holds the task, and its
 * subtasks, as if it had been started with it; a Job holding it already is replaced. `.../rows/:row/job/unlink` takes no body
 * and lets a held task, and the subtasks the same Job held, go back to open.
 */
export type LinkTaskJob = {
  session_id: string;
};

/**
 * `POST /projects/rows/:row/pulls` puts a pull request of the task's repository, by number, on a task or
 * subtask, and `.../rows/:row/pulls/unlink` takes it off.
 */
export type LinkTaskPull = {
  number: number;
};
