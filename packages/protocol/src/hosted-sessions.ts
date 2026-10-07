// A session Fleet hosts for Bridge: the commands that drive one, its thread and
// the event that streams it. `crates/ipc/src/hosted_sessions.rs`,
// `docs/concepts/session.md`. Since protocol 23.49.
//
// The header rules in `protocol.ts` hold here. A hosted session is a
// `SessionRecord` with `origin: "bridge"` and `hosted` set; its thread is not on
// the row. Read `get_session` once and follow `session.row`.

import type { HelmCallAnswer, HelmCallInFlight } from "./helm-calls";
import type { SessionRecord } from "./sessions";

/** The permission mode a session runs in. `auto` is the default. */
export type SessionMode = "ask" | "auto" | "accept_edits" | "plan";

/** `POST /sessions/start`. The row holds no slot and no branch until the agent writes. */
export type StartSession = {
  manifest_id: string;
  title?: string;
  /** Absent is the machine's own, as Dispatch's `Auto`. */
  model?: string;
  effort?: string;
  /** Absent is `auto`. */
  mode?: SessionMode;
};

export type TagKind = "session" | "job" | "pull_request" | "branch";

/** What a message names with `@`. Fleet writes the line that tells the agent. */
export type SessionTag = {
  kind: TagKind;
  /** A session's id, a Job's id, a pull request's number or a branch's name. */
  id: string;
  title: string;
  job?: { number: number; branch: string; slot: number; state: string };
};

/** A picture or file sent with a message. `data` is base64. */
export type SessionUpload = { name: string; media_type: string; data: string };

/** `POST /sessions/message`. Answered 202 with the `SessionRecord`. */
export type SendSessionMessage = {
  session_id: string;
  text: string;
  attachments?: SessionUpload[];
  mentions?: SessionTag[];
};

/** `POST /sessions/ask/answer`. `call` is the ask's own `HelmCallInFlight.call`. */
export type AnswerSessionAsk = {
  session_id: string;
  call: string;
  answer: HelmCallAnswer;
  note?: string;
};

/** `POST /sessions/tune`. A model or effort left out is the machine's own. */
export type TuneSession = {
  session_id: string;
  model?: string;
  effort?: string;
  mode: SessionMode;
};

/** `POST /sessions/close`. */
export type CloseSession = { session_id: string };

/** A turn running, or none. A message from another session starts one, so `working` may have no author. */
export type SessionTurn =
  | { state: "idle" }
  | { state: "working"; woken_by?: { id: string; title: string } };

/** What a hosted session carries beyond a terminal's row. */
export type HostedFacts = {
  turn: SessionTurn;
  /** What the agent is held on, while it is. */
  asked?: HelmCallInFlight;
  model?: string;
  effort?: string;
  mode: SessionMode;
  /** False after a quiet timeout: the next message resumes the session. */
  running: boolean;
};

/** Who said a row. Another session that wrote to this one is named. */
export type SessionVoice =
  | { kind: "you" }
  | { kind: "agent" }
  | { kind: "session"; id: string; title: string };

/** A file or picture a message carried, kept by Fleet. `get_session_file` serves it. */
export type SentFile = { id: string; name: string; media_type: string };

export type SessionAskState =
  | "waiting"
  | "ran_unasked"
  | "allowed_once"
  | "allowed_and_remembered"
  | "refused"
  | "unanswered"
  | "session_gone";

/** One row of a hosted session's thread. A row is replaced by its `id`. */
export type SessionRow =
  | {
      kind: "message";
      id: string;
      at: string;
      from: SessionVoice;
      text: string;
      files?: SentFile[];
      tags?: SessionTag[];
    }
  /** A tool call, one line. */
  | { kind: "tool"; id: string; at: string; text: string }
  /** The first write: the slot leased and the branch cut. */
  | { kind: "lease"; id: string; at: string; slot: number; branch: string }
  /** A call put to the person. Replaced as it is answered. */
  | { kind: "ask"; id: string; at: string; ask: HelmCallInFlight; state: SessionAskState };

/** `get_session`: the row and its thread, oldest first. */
export type SessionThread = { session: SessionRecord; rows: SessionRow[] };

/** `session.row`: one row appended, or replaced where `row.id` is held. */
export type SessionRowChanged = { session_id: string; row: SessionRow };
