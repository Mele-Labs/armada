// Words typed for a running Job, read for which Job they are for — Bridge's mirror of
// `crates/ipc/src/thought.rs`.
//
// **One operation is Bridge's.** `read_thought` answers whether the words a person typed are a
// question or a thought for other running work, and which. Nothing is kept for them: a Job on your
// words passing words on (`tell_session`) is announced as `operator.words_sent`, which Bridge does
// not read.

import type { Outcome } from "./reads";

/** `read_thought` (`POST /operator/read_thought`). */
export type ReadThought = {
  text: string;
};

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

/**
 * What `read_thought` answers. **A question is the answer whenever Fleet could not say
 * otherwise**, so the words are never taken for a thought about a Job they are not about.
 */
export type ThoughtReading =
  /** Not about a running Job. */
  | {
      kind: "question";
    }
  /** For other work. Without a `target`, nothing running is about it. */
  | {
      kind: "thought";
      target?: ThoughtTarget;
    };

/**
 * What a reading came back as. **Asked for once by the message box and the box's alone**, so it is
 * never part of `BridgeState`. The `outcome` of a read that failed is the box's cue to send the
 * words as a question, since they must never be lost.
 */
export type ThoughtRead =
  | {
      ok: true;
      reading: ThoughtReading;
    }
  | {
      ok: false;
      outcome: Outcome;
    };
