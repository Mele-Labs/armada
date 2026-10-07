// The session ledger: every agent session a person runs, and what each holds.
// `crates/ipc/src/sessions.rs`, `docs/concepts/session.md`. Since protocol 23.43.
//
// The header rules in `protocol.ts` hold here. `kind` on an attachment is open
// text, so it is a `string`; the sets below that Fleet closes are written out.

import type { HostedFacts, SessionMode } from "./hosted-sessions";

/** What started a session. */
export type SessionOrigin = "terminal" | "bridge";

export type SessionState = "live" | "ended";

/**
 * Where an attachment stands, for every kind of one: a slot let go and a need
 * whose branch landed are different words and the same question.
 */
export type AttachmentState = "standing" | "spent" | "given_back";

export type HolderKind = "session" | "job";

export type Holder = { kind: HolderKind; id: string };

/** The figures a session last reported. Each is absent until reported. */
export type SessionUsage = {
  context_tokens?: number;
  context_window?: number;
  cost_micros?: number;
};

/**
 * One thing a session took or did. `kind` is open: `slot`, `branch`, `pr`,
 * `job`, `studio`, `subagent` and `message` today, and a word Fleet has not met
 * is kept as it arrived. `target` is what the kind is of: a slot number, a
 * branch, a pull request number, a Job id.
 */
export type Attachment = {
  kind: string;
  /** The repository a slot or a branch is of. Absent where it is of none Fleet serves. */
  manifest_id?: string;
  target: string;
  state: AttachmentState;
  detail?: Record<string, string>;
  since: string;
  changed_at: string;
};

/**
 * A session and everything it holds. `report_session` answers it, `list_sessions`
 * lists it and `session.changed` publishes it whole, so a client replaces the
 * row it has.
 */
export type SessionRecord = {
  id: string;
  harness: string;
  origin: SessionOrigin;
  manifest_id?: string;
  cwd: string;
  title?: string;
  state: SessionState;
  started_at: string;
  last_seen_at: string;
  last_turn_at?: string;
  ended_at?: string;
  end_reason?: string;
  usage: SessionUsage;
  attachments: Attachment[];
  /** What a session Fleet hosts carries beyond a terminal's. Since 23.49. */
  hosted?: HostedFacts;
  /** What a session run in a terminal runs on, once its mod has said. Since 23.53. */
  terminal?: TerminalFacts;
  /** A terminal session whose mod is older than its repository's, or reported no version. Since 23.59. */
  mod_out_of_date?: true;
};

/** A command a terminal session lists, for `/` to offer. Since 23.53. */
export type TerminalCommand = { name: string; says: string };

/**
 * What a session run in a terminal runs on, as its mod last said. **The mode is read-only**: the mods API
 * cannot switch a live session's. Since 23.53.
 */
export type TerminalFacts = {
  model?: string;
  effort?: string;
  mode?: SessionMode;
  commands?: TerminalCommand[];
};

/** `list_sessions`, the most recently seen first. */
export type SessionList = { sessions: SessionRecord[] };

/** `rename_session`: a person's name for a session, hosted or in a terminal. `POST /sessions/rename`. Since 23.52. */
export type RenameSession = { session_id: string; title: string };

/** One holder of what `who_owns` was asked about, with its row. */
export type Ownership = {
  holder: Holder;
  attachment: Attachment;
  /** The holder's title, where it is a session that has one. */
  title?: string;
};

/** `who_owns`: every holder, the ones still holding it first. Empty is an answer. */
export type Owners = { holders: Ownership[] };

/** One fact a harness reports. `POST /sessions/report`. */
export type SessionFact =
  | { kind: "started"; cwd: string; title?: string; origin?: SessionOrigin; mod_version?: string }
  | { kind: "titled"; title: string; named?: boolean }
  | { kind: "moved"; cwd: string }
  | {
      kind: "attached";
      attachment: { kind: string; target: string; detail?: Record<string, string> };
    }
  | {
      kind: "settled";
      attachment: { kind: string; target: string };
      state: AttachmentState;
    }
  | { kind: "measured"; usage: SessionUsage }
  | { kind: "turn_completed" }
  | { kind: "ended"; reason: string };

export type SessionReport = {
  harness: string;
  session_id: string;
  fact: SessionFact;
};
