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

/** The fixed set of topics the sorter places an entry under. An entry it could not place has none, and Bridge draws it under Other. */
export type LogbookTopic = "git" | "slots" | "testing" | "tools" | "mac" | "other";

export type LogbookEntry = {
  id: string;
  section: LogbookSection;
  /** The repository's root, on a `repo` entry only. */
  repository?: string;
  kind: LogbookKind;
  /** The topic the sorter placed it under. Absent where it could not place it. */
  topic?: LogbookTopic;
  /** A few words naming it, as its line in the log reads. Absent on an entry written before titles were. */
  title?: string;
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
export type Logbook = { entries: LogbookEntry[]; tidy: LogbookTidy };

/** Whether a Tidy pass is running, and what the last one did. `last` is kept in memory, so a Fleet restart forgets it; a failed pass is kept with `failed: true`. */
export type LogbookTidy = { running: boolean; last?: LogbookTidyLast };

/** What a Tidy pass did: the entries it folded into another and archived, when it ended, and whether a repository's call failed (`merged` then counts what was archived before it did). */
export type LogbookTidyLast = { merged: number; at: string; failed: boolean };

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

/** What `tidy_logbook` answers at once: a pass is running. What it merged shows in `Logbook.tidy` once it finishes. */
export type TidiedLogbook = { running: boolean };

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

/** The three looks Organize takes: entries that say one thing, entries nobody has confirmed in 45 days, and entries that read as workarounds for a fault in Armada. */
export type OrganizeAction = "merge" | "stale" | "bugs";

/** The body of `organize_logbook` (`POST /logbook/organize`). `repository` is a served repository's root; absent covers every repository, with the habits and this Mac's. */
export type OrganizeLogbook = { action: OrganizeAction; repository?: string };

/**
 * One thing Organize proposes; nothing has changed when it is read. `id` names it (the entry kept for a merge, the
 * entry otherwise). `entry_ids` lists the entries it concerns, the one a merge keeps first. `text` is the kept entry as
 * it would read for a merge, the entry's own text otherwise. `note` is how long since a stale entry was confirmed, or a
 * bug's one-line fault, and empty for a merge.
 */
export type OrganizeProposal = { id: string; entry_ids: string[]; text: string; note: string };

/** What `organize_logbook` answers once the look is done. `failed`: a repository's model call failed or answered wrongly, so its proposals are missing. */
export type OrganizedLogbook = { action: OrganizeAction; proposals: OrganizeProposal[]; failed: boolean };

/** The body of `apply_organize_logbook` (`POST /logbook/organize/apply`): the proposals a person kept, as `organize_logbook` answered them. */
export type ApplyOrganizeLogbook = { action: OrganizeAction; proposals: OrganizeProposal[] };

/**
 * What `apply_organize_logbook` answers. `applied` counts entries folded (merge) or archived (stale); `bugs` writes
 * nothing and answers the proposals back in `reported`, which is all reporting does for now.
 */
export type AppliedOrganizeLogbook = { action: OrganizeAction; applied: number; reported: OrganizeProposal[] };

/** Bridge's envelope over `Logbook`: the entries, or why they could not be read. */
export type LogbookRead = { ok: true; entries: LogbookEntry[]; tidy: LogbookTidy } | { ok: false; outcome: Outcome };

/** What an edit or a lock came back as: the entry as it now stands, or the refusal. */
export type LogbookAnswer = { ok: true; entry: LogbookEntry } | { ok: false; outcome: Outcome };

/** What Forget came back as. The entry is on the deny list and gone from every read. */
export type LogbookForgotten = { ok: true } | { ok: false; outcome: Outcome };

/** What pressing Tidy came back as: a pass is running, or the refusal. */
export type LogbookTidied = { ok: true; running: boolean } | { ok: false; outcome: Outcome };

/** Bridge's envelope over `OrganizedLogbook`: what Organize proposes, or why it could not look. */
export type LogbookOrganized = ({ ok: true } & OrganizedLogbook) | { ok: false; outcome: Outcome };

/** Bridge's envelope over `AppliedOrganizeLogbook`. */
export type LogbookOrganizeApplied = ({ ok: true } & AppliedOrganizeLogbook) | { ok: false; outcome: Outcome };

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
