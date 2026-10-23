// A note the person pinned on a page, sent to a Session whole. `crates/ipc/src/session_notes.rs`,
// `docs/practices/protocol.md`, *A pinned note*. Since 17 Oct 2026.
//
// The header rules in `protocol.ts` hold here. Fleet answers with the lines of code the spot was drawn
// from, found by the words on screen, so the Session starts at a file and a line.

import type { SessionRecord } from "./sessions";

/**
 * `POST /sessions/note`. Answered 202 with `NoteSent`. It takes a turn, as `SendSessionMessage` does.
 * **The thread keeps only `said`**; the rest is lines Fleet adds.
 */
export type SendSessionNote = { session_id: string } & SessionNote;

/**
 * `POST /sessions/notes`. Answered 202 with `NotesSent`. The notes of a walk as one turn, numbered in order.
 * **Never empty**: an empty `notes` is a 422 `fleet.session_notes_empty`.
 */
export type SendSessionNotes = {
  session_id: string;
  notes: SessionNote[];
};

/** One note, whichever Session it goes to. */
export type SessionNote = {
  /** The person's words, trimmed. */
  said: string;
  /** The page's url as loaded. Each note has its own: a walk crosses takes and pages. */
  address: string;
  /** The `.armada/annotations/<id>.json` note, where Bridge's own layer took it, so `mark_annotation_done` still finds it. */
  note_id?: string;
  place: NotePlace;
  state: NoteState;
  crop?: NoteCrop;
  /** What Bridge held about the Job open when he pinned the note, as labelled lines. Background, never the task: Fleet puts them after his words. */
  job?: string[];
};

/** Where the person pointed. */
export type NotePlace = {
  /** The component that drew the element, where the page names one. */
  component?: string;
  /** The components around it, innermost first. */
  owners?: string[];
  selector: string;
  element: NoteElement;
  /** At most 8 accessible names of what encloses it, and the nearest heading above, innermost first. */
  nearby?: string[];
  /** `path:line` from the closest `[data-armada-source]`. A claim: Fleet checks it against the text. */
  stamp?: string;
  /** Path, query and hash. */
  location: string;
  screen?: string;
  layer?: string;
};

/** The element itself. */
export type NoteElement = {
  tag: string;
  /** Its text, cut to 200 characters. */
  text: string;
  /** Its accessible name, where it has one. */
  label?: string;
};

/** What the page was showing. */
export type NoteState = {
  /** At most 12, each `role "name"`: `tab "Bash"`, `option "Session 3"`. */
  selected?: string[];
  walk?: NoteWalk;
  take?: NoteTake;
  scenario?: string;
};

/** The walk the page was on, and its step. */
export type NoteWalk = { name: string; step: number; of: number };

/** The take of a walk with variations. */
export type NoteTake = { letter: string; name: string };

/** A picture of the spot. `data` is base64. */
export type NoteCrop = { media_type: string; data: string };

/** `send_session_note`'s answer: the turn taken, and the lines the Session was handed. */
export type NoteSent = {
  session: SessionRecord;
  /** At most 5, best first. */
  candidates: Candidate[];
};

/** `send_session_notes`' answer: the one turn taken, and each note's lines. */
export type NotesSent = {
  session: SessionRecord;
  /** One list per note, in the notes' order; each at most 5, best first. */
  candidates: Candidate[][];
};

/** A line of code the spot may have been drawn from. */
export type Candidate = {
  /** Relative to the checkout searched. */
  file: string;
  /** 1-based. */
  line: number;
  /** Why it matched, in one short line. */
  why: string;
  /** The line, trimmed, at most 160 characters. */
  text: string;
};
