// The Logbook: what Armada learned about each repository, about how the owner works, and about this Mac, as short
// entries every new Job is handed. `docs/concepts/logbook.md`. Fleet serves it: `crates/ipc/src/logbook.rs`.
//
// The wire types are `LogbookEntry`, `Logbook` and the request bodies. `LogbookRead`, `LogbookAnswer` and
// `LogbookForgotten` are Bridge's own envelopes over them, an outcome in place of a body.

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
  /** What the row shows, in a line. Fleet leaves it out of a list: it does not read the record back. */
  says?: string;
  at: string;
};

/** Text the entry held before, and who replaced it: Distil on a contradiction or a trip, Tidy when it merged a near-duplicate in, or the person. */
export type LogbookEarlier = { text: string; until: string; by: "distil" | "person" | "merged" };

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
  locked: boolean;
  edited: boolean;
  /** Earlier text, newest first. */
  history: LogbookEarlier[];
};

/** `get_logbook` (`GET /logbook`): the entries asked for, forgotten ones left out. */
export type Logbook = { entries: LogbookEntry[] };

/**
 * `?repository=` (a root `list_repositories` names), `?section=` and `?state=` on `get_logbook`. With no repository the
 * answer is the habits and this Mac's alone; with no section or state it narrows nothing.
 */
export type LogbookQuery = { repository?: string; section?: LogbookSection; state?: LogbookState };

/** The body of `edit_logbook_entry` (`POST /logbook/entries/:entry_id/edit`). Answers the entry. */
export type EditLogbookEntry = { text: string };

/** The body of `lock_logbook_entry` (`POST /logbook/entries/:entry_id/lock`). Answers the entry. */
export type LockLogbookEntry = { locked: boolean };

/** What `forget_logbook_entry` (`POST /logbook/entries/:entry_id/forget`) answers. */
export type ForgottenLogbookEntry = { id: string };

/**
 * The body of `tidy_logbook` (`POST /logbook/tidy`). `repository` is a served repository's root; absent tidies every
 * repository, with the habits and this Mac's.
 */
export type TidyLogbook = { repository?: string };

/** What `tidy_logbook` answers: the entries folded into another and archived. */
export type TidiedLogbook = { merged: number };

/**
 * The body of `teach_logbook_entry` (`POST /logbook/entries`): an entry a person wrote, which stands at once and
 * answers as the entry. `repository` is a root, required for `repo` and ignored for the others.
 */
export type TeachLogbookEntry = {
  section: LogbookSection;
  repository?: string;
  kind: LogbookKind;
  text: string;
  paths?: string[];
  /** What the person pressed Teach on: a message, notice or Check. It becomes the entry's one evidence row. */
  source?: { session_id: string; cite: string };
};

/** Bridge's envelope over `Logbook`: the entries, or why they could not be read. */
export type LogbookRead = { ok: true; entries: LogbookEntry[] } | { ok: false; outcome: Outcome };

/** What an edit or a lock came back as: the entry as it now stands, or the refusal. */
export type LogbookAnswer = { ok: true; entry: LogbookEntry } | { ok: false; outcome: Outcome };

/** What Forget came back as. The entry is on the deny list and gone from every read. */
export type LogbookForgotten = { ok: true } | { ok: false; outcome: Outcome };

/**
 * `recall`: `GET /logbook/recall?query=&section=&manifest_id=`. The Logbook entries whose text holds the
 * words asked for, best first, at most 20. Faded and candidate entries are in it.
 */
export type Recalled = {
  entries: RecalledEntry[];
};

export type RecalledEntry = {
  section: "repo" | "habits" | "machine";
  kind: "command" | "gotcha" | "convention" | "environment" | "habit";
  state: "candidate" | "standing" | "faded";
  text: string;
  /** How many distinct Sessions bore it out. */
  confirmed_by: number;
};

/** `get_logbook_for_subagent`'s body: `POST /logbook/subagent`. The mod's hook sends it. */
export type SubagentAsk = {
  session_id: string;
  prompt: string;
};

/** What a subagent is handed. Empty where the Logbook holds nothing for it. */
export type ForSubagent = {
  text: string;
};
