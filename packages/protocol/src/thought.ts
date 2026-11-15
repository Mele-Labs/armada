// Words sent on to a running Job — Bridge's mirror of `crates/ipc/src/thought.rs`. A Job on your words passing
// words on (`tell_session`) is announced as `operator.words_sent`, which Bridge does not read.

/** What a thought may go to. */
export type ThoughtKind = "session";

/** The Session a thought goes to, named so a person can tell which. */
export type ThoughtTarget = {
  kind: ThoughtKind;
  /** A Session's id. */
  id: string;
  /** What a person calls it. */
  title: string;
  /** The repository it belongs to. Absent for a Session Fleet places in none it serves. */
  manifest_id?: string;
  /** The slot a Session holds, where it holds one. */
  slot?: number;
};
