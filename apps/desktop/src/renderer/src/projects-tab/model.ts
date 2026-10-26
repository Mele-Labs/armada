// What the Projects tab draws. **A project is his own list, kept in Fleet on his Mac**: tasks he writes and issues he links, in
// one list in his order, each row naming the Job that holds it. `wire.ts` reads what Fleet lists into these shapes.

/** The Job a row was sent to, by id, and its title as it was sent, so a Job that has ended still reads by name. */
export type Holder = { id: string; title: string };

/** An issue as the tracker reads it: open or closed is the tracker's to say. `url` is empty for one Fleet listed, which keeps the reference only. */
export type Linked = { reference: string; title: string; url: string; state: "open" | "closed" };

/** A pull request as its forge reads it. `url` is empty for one Fleet listed, which keeps the number only. */
export type LinkedPull = { number: number; title: string; url: string; state: "open" | "draft" | "merged" | "closed" };

/**
 * A task he wrote. Done is his to say: by ticking it, or by answering the ask when the Job holding it finishes or a pull
 * request linked to it merges (`asked`). It can carry issues and pull requests of its own, and subtasks one level deep: a
 * subtask holds none. At most one Job holds it, whether Dispatch sent it there or he linked one.
 */
export type TaskRow = {
  kind: "task";
  id: string;
  title: string;
  notes?: string;
  done: boolean;
  /** Fleet asks whether it is done: a Job that held it finished, or a pull request linked to it merged. */
  asked?: boolean;
  job?: Holder;
  issues?: Linked[];
  pulls?: LinkedPull[];
  subtasks?: TaskRow[];
};

/** What a task is written with: the issues, pull requests and Job its tags named. */
export type TaskLinks = Pick<TaskRow, "job" | "issues" | "pulls">;

/** An issue he linked to the project, as a row. It has no tick and no ask. */
export type IssueRow = Linked & { kind: "issue"; id: string; job?: Holder };

export type ProjectRow = TaskRow | IssueRow;

export type Project = {
  id: string;
  /** The repository it belongs to, by Manifest id. */
  manifestId: string;
  name: string;
  rows: ProjectRow[];
  /** When he last added to it or opened it, so the palette lists the last used first. */
  usedAt: number;
};

/**
 * A row's state, as its glyph draws it. A task is open, in a Job, asking (the Job holding it finished, or a pull request linked to it merged, and he has not said
 * whether it is done) or done. An issue is open or closed as its tracker says.
 */
export type RowState = "open" | "held" | "asking" | "done" | "closed";
