// What is waiting on a person, in two buckets. `list_alerts`. Hand-mirrored from `crates/ipc/src/alerts.rs`.
// Derived and never stored: Fleet keeps no Alert record, and nothing publishes `alert.raised`.

/** One Job that is not moving without somebody. A row, not a Job: the Trigger is on `JobSummary.alert`. */
export type Alert = {
  job_id: string;
  /** What a person calls this Job. */
  handle: string;
  /** The status it is sitting at, as the wire spells it. */
  status: string;
  /** The stored reason for the transition that brought it here. Absent is a real answer. */
  why?: string;
  /** When it stopped here, where the record holds an instant for it. */
  since?: string;
};

/** Everything waiting on a person: `blocked` is work stopped mid-flight, `waiting` is work resting at a gate. Each oldest first. */
export type AlertList = { blocked: Alert[]; waiting: Alert[] };
