// What a person pointed at while walking a Job's work in Bridge's own window,
// kept on the Job by Fleet so it outlives the worktree, and handed to the Drone
// when the Job is sent back. Hand-mirrored from `crates/ipc/src/walk_notes.rs`.
// Since protocol 23.18.

import type { CaptureServed, StagedFrame, StudioCapture } from "./studio";

/** `capture_walk_note`'s body: `capture_studio_note`'s, with no place on a board. */
export type CaptureWalkNote = { said: string; capture: StudioCapture; frame?: StagedFrame };

export type WalkNote = {
  id: string;
  /** What the person said, verbatim. */
  said: string;
  at: string;
  /** What was pointed at, in one line a person reads — `button “Save”`. */
  element: string;
  selector: string;
  /** The path within the page. */
  location: string;
  served?: CaptureServed;
  /** The kept PNG, where one was taken. */
  frame?: string;
  /** A send-back has already carried it to a Drone. */
  sent?: boolean;
};

export type WalkNotes = { notes: WalkNote[] };

export type RemoveWalkNote = { id: string };
