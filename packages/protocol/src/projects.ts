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
  /** A task he wrote. `notes` is empty for none. `held_by` is present while `state` is `held` or `asked`, and only then. */
  | { kind: "task"; id: string; title: string; notes: string; state: ProjectTaskState; held_by?: ProjectHolder }
  /**
   * A linked issue: `issue` is the tracker's reference only (`7`, `ABC-12`, `armada-3fz`). `title`
   * and `state` are the tracker's, read when the rows were, and absent where it could not say.
   */
  | { kind: "issue"; id: string; issue: string; title?: string; state?: "open" | "closed" };

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

/** `POST /projects/:repository/:project/tasks` and `.../rows/:row/edit`: title and notes, whole. */
export type ProjectTask = {
  title: string;
  notes?: string;
};

/** `POST /projects/:repository/:project/issues`: `#7` or `7`, `ABC-12`, `armada-3fz`. */
export type LinkProjectIssue = {
  issue: string;
};

/** `POST /projects/:repository/:project/rows/:row/move`: counted from 0; past the end is the end. */
export type MoveProjectRow = {
  position: number;
};

/** `POST /projects/:repository/:project/rows/:row/tick`: done, or open again. */
export type TickProjectTask = {
  done: boolean;
};
