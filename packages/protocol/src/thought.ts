// Helm reading a thought for where it belongs, and recording where it went — Bridge's mirror of
// `crates/ipc/src/thought.rs`. `docs/concepts/helm.md`, *Routing a thought*.
//
// **Two operations, both Bridge's.** `read_thought` answers whether the words a person typed into
// Helm's box are a question for Helm or a thought for other running work, and which.
// `record_thought_sent` is said after Bridge delivered the words, and Fleet keeps a `sent` row in the
// Helm Session's thread for them. Neither sends anything to a Session, a Job or Helm's own session.

import type { SessionTag, SessionUpload } from "./hosted-sessions";
import type { Outcome } from "./reads";

/** `read_thought` (`POST /helm/read_thought`). */
export type ReadThought = {
  text: string;
};

/** What a thought may go to. */
export type ThoughtKind = "session" | "job";

/** The Session or Job a thought goes to, named so a person can tell which. */
export type ThoughtTarget = {
  kind: ThoughtKind;
  /** A Session's id, or a Job's. */
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
 * otherwise**, so the words go to Helm and are never lost.
 */
export type ThoughtReading =
  /** For Helm itself. */
  | { kind: "question" }
  /** For other work. Without a `target`, nothing running is about it. */
  | { kind: "thought"; target?: ThoughtTarget };

/**
 * `record_thought_sent` (`POST /helm/thought_sent?manifest_id=`): the words were delivered. The
 * manifest is the Helm conversation whose thread keeps the row, as `ask_helm`'s is.
 */
export type ThoughtSent = {
  text: string;
  target: ThoughtTarget;
  /** What the thought went with, kept with the Helm Session's own uploads as an ask's are. Left out when empty. */
  attachments?: SessionUpload[];
  mentions?: SessionTag[];
};

/**
 * What a reading came back as. **`HelmDebugRead`'s shape, for its reason**: it is asked for once by
 * the dock and is the dock's alone, so it is never part of `BridgeState`. The `outcome` of a read
 * that failed is the dock's cue to ask Helm, since the words must never be lost.
 */
export type ThoughtRead = { ok: true; reading: ThoughtReading } | { ok: false; outcome: Outcome };
