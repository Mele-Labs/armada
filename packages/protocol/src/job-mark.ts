// The state a Jobs-list row draws, read by Fleet so a client does not work it out. States only: the words a row
// says beside its mark stay with the client. Carried on `SessionRecord.mark`. Mirrors `crates/ipc/src/job_mark.rs`.

/** What a Job is doing, as a row's mark draws it. `SessionState` in `@armada/components`, value for value. */
export type JobState =
  | "blank"
  | "working"
  | "running"
  | "subagent"
  | "stuck"
  | "stopped"
  | "stopped_asks"
  | "waiting"
  | "replied"
  | "done"
  | "failing"
  | "idle"
  | "quiet"
  | "ended"
  | "limited"
  | "after"
  | "draft"
  | "held"
  | "paused"
  | "unstarted";

/** The row's lamp: needs him, failed, running, done; anything else is idle. */
export type JobLamp = "needs" | "failed" | "running" | "done" | "idle";

/** The group a list files the row under. */
export type JobGroup = "needs" | "working" | "held" | "done";

/** One reading of a row: its state, lamp and group. */
export type JobReading = {
  state: JobState;
  lamp: JobLamp;
  group: JobGroup;
};

/**
 * How a Job's row reads. `once_read` is set only where `state` is `replied`: what the row reads as once the person has read the
 * reply, which only the client knows, so both are sent.
 */
export type JobMark = JobReading & {
  once_read?: JobReading;
};
