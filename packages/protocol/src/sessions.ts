// The session ledger: every agent session a person runs, and what each holds.
// `crates/ipc/src/sessions.rs`, `docs/concepts/session.md`. Since protocol 23.43.
//
// The header rules in `protocol.ts` hold here. `kind` on an attachment is open
// text, so it is a `string`; the sets below that Fleet closes are written out.

import type { HelmCallInFlight } from "./helm-calls";
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
  /** A terminal session whose mod is older than its repository's, or reported no version. Since 23.62. */
  mod_out_of_date?: true;
  /**
   * What the session waits on the person for: the agent's list with Fleet's own items merged in, the
   * agent's first. Absent is nothing, and always absent for an ended session.
   */
  waiting_for?: WaitingItem[];
};

/** What a press on a waiting item does. `target` is an address, a call, a pull request number or a command. */
export type WaitingAct = { kind: "walk" | "answer" | "approve_pr" | "run"; target: string };

/** Who put an item on the list: the agent, or Fleet from an open card, a permission or an unapproved walk window. */
export type WaitingSource = "agent" | "ask_card" | "walk" | "permission";

/**
 * One thing a session waits on the person for. `id` is stable across updates: the agent's own, or
 * `ask:<call>`, `perm:<call>`, `walk:<url>` where Fleet derived it. `options` is present only on an
 * `answer` item that has choices, and `answer_waiting`'s `choice` indexes it.
 */
export type WaitingItem = {
  id: string;
  text: string;
  since: string;
  source: WaitingSource;
  act?: WaitingAct;
  options?: { label: string }[];
};

/**
 * `POST /sessions/waiting/answer`: settle one item. A `choice` (an index into its `options`), `text` of
 * the person's own, or a `mode` that leaves it to the agent: `best` thinks it through, `quick` takes the
 * reasonable path and keeps moving. A walk item needs none of the three, and approving is its default.
 */
export type AnswerWaiting = {
  session_id: string;
  item_id: string;
  choice?: number;
  text?: string;
  mode?: "best" | "quick";
};

/**
 * `POST /sessions/claim_pull_request`: the caller takes an open pull request no live Session or Job holds.
 * `session_id` or `job_id` names the claimant where the connection places none, which is Bridge's case:
 * exactly one of the two. Fleet refuses
 * 422 `fleet.pull_request_not_claimable`, naming the holder, where a live one has it or it is not open.
 */
export type ClaimPullRequest = { number: number; session_id?: string; job_id?: string };

/** What `claim_pull_request` answers: the pull request now held, and by whom (`session` or `job`). */
export type PullRequestClaimed = { number: number; branch: string; url: string; holder_kind: string; holder_id: string };

/**
 * `POST /sessions/waiting/dismiss`: drop one item for good. Its id never comes back, whether Fleet derived
 * it (`ask:`, `perm:`, `walk:`) or the agent stated it, and nothing is sent to the agent.
 */
export type DismissWaiting = { session_id: string; item_id: string };

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
  /** Whether its mod asked within the last ten seconds. Absent is not listening. Since 23.69. */
  listening?: boolean;
  /** The question its terminal is showing, while Bridge may still answer it. */
  asked?: HelmCallInFlight;
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
