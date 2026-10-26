// What the Projects tab draws. **A project is his own list, kept on his Mac**: tasks he writes and issues he links, in one
// list in his order, each row naming the Job that holds it. Nothing serves it yet; the mock's `projects` scenario stands in
// (`mock/slices/projects.ts`).

/** The Job a row was sent to, by id, and its title as it was sent, so a Job that has ended still reads by name. */
export type Holder = { id: string; title: string };

/** A task he wrote. Done is his to say: by ticking it, or by answering the ask when the Job holding it finishes. */
export type TaskRow = { kind: "task"; id: string; title: string; done: boolean; job?: Holder };

/** An issue he linked. Open or closed is the tracker's to say, so it has no tick and no ask. */
export type IssueRow = { kind: "issue"; id: string; reference: string; title: string; url: string; state: "open" | "closed"; job?: Holder };

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
 * A row's state, as its glyph draws it. A task is open, in a Job, asking (the Job holding it finished and he has not said
 * whether it is done) or done. An issue is open or closed as its tracker says.
 */
export type RowState = "open" | "held" | "asking" | "done" | "closed";
