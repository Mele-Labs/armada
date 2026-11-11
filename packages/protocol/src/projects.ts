// His projects on the wire, across repositories, and the tasks in none, mirrored by hand from `crates/ipc/src/projects.rs`.
// `docs/practices/protocol/projects.md`. **A project is his and has no repository**: every row names its own.
//
// The header rules in `protocol.ts` hold here.

import type { RunCloud, RunPlace } from "./run-memory";
import type { ValuesSet } from "./values";

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
  | {
      kind: "issue";
      id: string;
      manifest_id: string;
      issue: string;
      title?: string;
      state?: "open" | "closed";
    };

/**
 * An issue linked to a task: the tracker's reference only is kept. `title` and `state` are the tracker's, read when the rows
 * were, and absent where it could not say.
 */
export type TaskIssue = {
  issue: string;
  /** The repository it was linked in, by Manifest id: the task's own unless the task moved since. Absent reads as the task's own. */
  manifest_id?: string;
  title?: string;
  state?: "open" | "closed";
};

/**
 * A pull request linked to a task: the number only is kept. `title` and `state` are the forge's, read when the rows were,
 * and absent where it could not say. A draft is `open`.
 */
export type TaskPull = {
  number: number;
  /** The repository it was linked in, by Manifest id: the task's own unless the task moved since. Absent reads as the task's own. */
  manifest_id?: string;
  title?: string;
  state?: "open" | "merged" | "closed";
};

export type Project = {
  id: string;
  name: string;
  /** In order, across repositories: a row's position is its place here. */
  rows: ProjectRow[];
  /** The repositories, whole or by a folder, its file lists, in its order. Empty where `unread`. */
  members: ProjectMember[];
  /** Where its file is kept. None chosen is `this_mac`, and it is never `repository`. */
  place: ProjectPlace;
  /** Present where its file could not be read: the sentence that says why. `name` is then the last the store saw, and `members` is empty. */
  unread?: string;
  /** What the project sets for how work behaves, only the keys it sets. Absent where it sets none. `GET /projects/values` says what each key comes to. */
  values?: ValuesSet;
};

/** A member of a project: a whole repository, or a folder of it that carries its own `armada.yml`. */
export type ProjectMember = {
  /** The repository, by Manifest id. */
  manifest_id: string;
  /** A path inside the repository, `/`-separated, with no leading or trailing `/`. Absent for the whole of it. */
  folder?: string;
};

/**
 * Where a project's file is kept, as `set_run_memory_place` takes a repository's: `synced_folder` is required for `synced` and
 * `cloud` for `cloud`. **`repository` is refused** with `fleet.project_place_refused`: a project is not kept in one.
 */
export type ProjectPlace = {
  place: RunPlace;
  /** The folder `synced` writes under. */
  synced_folder?: string;
  /** Where `cloud` keeps it. */
  cloud?: RunCloud;
};

/** A project file Fleet could not read. */
export type ProjectLeftOut = {
  /** `this_mac`, `synced` or `cloud`. */
  place: RunPlace;
  /** The file, or the object a bucket holds it as. */
  path: string;
  /** The sentence a person reads, naming the key or the id at fault. */
  why: string;
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
  /** The project files Fleet could not read. A file a project of his names is here too, and that project says so as `unread`. */
  left_out: ProjectLeftOut[];
};

/** `POST /projects`: the name, trimmed, and what the project starts with. Absent `members` is none, and absent `place` is this Mac. */
export type NewProject = {
  name: string;
  members?: ProjectMember[];
  place?: ProjectPlace;
  /** The values it starts with. Absent sets none. */
  values?: ValuesSet;
};

/**
 * `POST /projects/project/:project/edit`: what changes, and only that. `members` present replaces the whole list, `[]` clearing it;
 * `values` present replaces the whole block, `{}` taking every value back.
 */
export type EditProject = {
  name?: string;
  members?: ProjectMember[];
  place?: ProjectPlace;
  values?: ValuesSet;
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

/**
 * `POST /projects/rows/:row/repository`: a top-level task, and its subtasks, to the repository `manifest_id`. It keeps its project
 * and its place. Its issues and pull requests stay with the repository each was linked in.
 */
export type MoveProjectTaskRepository = {
  manifest_id: string;
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
