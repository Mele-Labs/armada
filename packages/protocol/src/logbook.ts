// The Logbook: what Armada learned about each repository, about how the owner works, and about this Mac, as short
// entries every new Job is handed. `docs/concepts/logbook.md`.
//
// **No Fleet route serves it yet.** These are the shapes the mock answers, written from the concept, so the wiring
// swaps the source and nothing above it. The Rust side mirrors them in `crates/ipc` when the routes land.

import type { Outcome } from "./reads";

/** Whose entry it is: one repository's, the owner's across every repository, or this Mac's. */
export type LogbookSection = "repo" | "habits" | "machine";

export type LogbookKind = "command" | "gotcha" | "convention" | "environment" | "habit";

/** A candidate is not handed to agents, a standing entry is, and a faded one is kept and searchable only. */
export type LogbookState = "candidate" | "standing" | "faded";

/** One record row an entry cites, and the Job whose record holds it. */
export type LogbookEvidence = {
  job_id: string;
  job_title: string;
  /** The row's own cite in that record, as Distil named it. */
  cite: string;
  /** What the row shows, in a line. */
  says: string;
  at: string;
};

/** Text the entry held before, and who replaced it: Distil on a contradiction or a trip, or the person. */
export type LogbookEarlier = { text: string; until: string; by: "distil" | "person" };

export type LogbookEntry = {
  id: string;
  section: LogbookSection;
  /** The repository's root, on a `repo` entry only. */
  repository?: string;
  kind: LogbookKind;
  /** One or two lines saying what to do. */
  text: string;
  /** Where it applies, so a subagent working there is handed it. */
  paths?: string[];
  evidence: LogbookEvidence[];
  /** How many distinct Jobs confirmed it. */
  confirmed_by: number;
  first_seen: string;
  last_confirmed: string;
  last_served?: string;
  /** When a Job last met its warning anyway. Absent where none has. */
  last_tripped?: string;
  state: LogbookState;
  pinned: boolean;
  edited: boolean;
  /** Earlier text, newest first. */
  history: LogbookEarlier[];
};

/** `GET /logbook`: every section, forgotten entries left out. */
export type LogbookRead = { ok: true; entries: LogbookEntry[] } | { ok: false; outcome: Outcome };

/** What an edit or a pin came back as: the entry as it now stands, or the refusal. */
export type LogbookAnswer = { ok: true; entry: LogbookEntry } | { ok: false; outcome: Outcome };

/** What Forget came back as. The entry is on the deny list and gone from every read. */
export type LogbookForgotten = { ok: true } | { ok: false; outcome: Outcome };
