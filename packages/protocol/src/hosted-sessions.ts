// A session Fleet hosts for Bridge: the commands that drive one, its thread and
// the event that streams it. `crates/ipc/src/hosted_sessions.rs`,
// `docs/concepts/session.md`. Since protocol 23.49.
//
// The header rules in `protocol.ts` hold here. A hosted session is a
// `SessionRecord` with `origin: "bridge"` and `hosted` set; its thread is not on
// the row. Read `get_session` once and follow `session.row`.

import type { HelmCallAnswer, HelmCallInFlight, QuestionAnswer } from "./helm-calls";
import type { DroneNarrative, PilotOutcome } from "./piloting";
import type { LandsIn, Whose } from "./retro";
import type { SessionRecord, WaitingOption, WindowPlace } from "./sessions";

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
  /**
   * Take a Job over and start the session on its worktree, in one call. Since 23.51. The session's
   * repository is the Job's, so `manifest_id` is not read. Refused with the 409 `take_over` gives, and
   * no session is made.
   */
  pilot?: { job_id: string; outcome: PilotOutcome };
  /**
   * Start as a copy of an ended or dead session's conversation. Since 23.69. A new session with its own
   * id and ledger, holding none of the old one's. Refused 409 `fleet.session_fork_live` for a live one.
   * `at`, since 23.79, is the thread row the copy ends at, included; with it the old session may be live and
   * is left as it was. Refused 422 `fleet.fork_point_unwritten` where no message has that id.
   */
  fork?: { session_id: string; at?: string };
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
  /** One entry per question, for an ask that carries some. Since protocol 23.71. */
  answers?: QuestionAnswer[];
};

/** `POST /sessions/tune`. A model or effort left out is the machine's own. */
export type TuneSession = {
  session_id: string;
  model?: string;
  effort?: string;
  mode: SessionMode;
};

/**
 * `POST /sessions/held`. The mod in a terminal session asks what a person sent it. Since 23.53.
 * `wait_ms` lets Fleet hold the ask while nothing is held, up to 25 seconds; left out, it is answered at once.
 */
export type TakeHeld = { session_id: string; wait_ms?: number };

/**
 * `POST /sessions/ask/terminal`. The mod in a terminal session puts its `AskUserQuestion` to Bridge (`asks`,
 * answered at once with the call), polls for the answer (`wait`, held a bounded time), or says the terminal's own
 * prompt ended first (`settled`).
 */
export type TerminalAsk =
  | { kind: "asks"; session_id: string; input: unknown }
  | { kind: "settled"; session_id: string; answered: boolean }
  | { kind: "wait"; session_id: string; call: string };

/** What Fleet answers it with: the call, no answer yet, or how the question ended. */
export type TerminalAsked =
  | { outcome: "asked"; call: string }
  | { outcome: "waiting" }
  | { outcome: "answered"; updated_input: unknown }
  | { outcome: "refused"; message: string }
  | { outcome: "gone" };

/** What a person sent a terminal session, oldest first, handed over once. Since 23.53. */
/** A command a person chose in Bridge for a terminal session, run there as typed. Since 23.53. */
export type HeldCommand = { command: string; args: string };

/** `end`: Close pressed in Bridge; the mod runs `/exit` after the messages. Absent when not asked. */
export type MessagesHeld = { messages: string[]; commands?: HeldCommand[]; end?: true };

/** `POST /sessions/command`: a slash command written whole, run in a terminal session as typed. */
export type RunSessionCommand = { session_id: string; command: string };

/** `POST /sessions/close`. A terminal session is ended by its mod, and the row ends when the terminal says so. */
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
  /**
   * The slash commands and skills the agent said it has, by name, for `/` to offer. Since 23.51.
   * A session whose agent has not started is given the last one any session read; empty before any
   * has, and after a Fleet restart until one starts.
   */
  commands?: string[];
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

/**
 * Where the writing of a Session's retro has got to. `asking_agent` is absent from a retro written where
 * no live, idle agent was asked. Since 23.78.
 */
export type RetroStage = "asking_agent" | "reading_record" | "writing" | "written" | "failed";

/**
 * One item of a retro as it settles while the model writes: a field is present once its value has ended,
 * in the order the model writes them. The model's items before Fleet holds them to the record, so the
 * kept retro can have fewer. Since 23.78.
 */
export type PartialRetroItem = {
  title?: string;
  what?: string;
  fix?: string;
  who?: Whose;
  lands_in?: LandsIn;
};

/** The `retro` row of a hosted Session's thread. Since 23.78. */
export type RetroRow = Extract<SessionRow, { kind: "retro" }>;

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
  /** A command the person ran in the terminal, as typed, with no output. Since 23.56. */
  | { kind: "command"; id: string; at: string; text: string }
  /** The summary the CLI wrote where it compacted the conversation. Not the person's words. Since 23.56. */
  | { kind: "compaction"; id: string; at: string; text: string }
  /** The first write: the slot leased and the branch cut. */
  | { kind: "lease"; id: string; at: string; slot: number; branch: string }
  /** An `armada check` the agent ran in its slot, replaced by id as it ends. `run` is the id `GET /sessions/checks/:run/output` reads. Since 23.75. */
  | { kind: "check"; id: string; at: string; name: string; run: number; state: "running" | "passed" | "failed" }
  /**
   * A Session's retro being written, replaced by id as it goes and kept when done. `items` settle while
   * the model writes (`stage` `writing`) and are empty otherwise; `retro` is the kept retro's number for
   * `get_session_retro n`, on `written`; `why` is Fleet's words, on `failed`. Since 23.78.
   */
  | {
      kind: "retro";
      id: string;
      /** When the press was accepted. */
      at: string;
      stage: RetroStage;
      /** When it reached `stage`. */
      stage_since: string;
      /** How much of the model's answer has arrived. */
      characters: number;
      items: PartialRetroItem[];
      retro?: number;
      why?: string;
    }
  /**
   * The session showed the person a page in a window. `ended_at` is when the walk ended, by an
   * approval or the window closing; absent while it is open.
   */
  | { kind: "window"; id: string; at: string; title: string; url: string; ended_at?: string }
  /**
   * This session's agent started a session, or forked its own conversation into one, when the person
   * asked it to. Written in the starting session's thread; `session` is the new one.
   */
  | { kind: "started"; id: string; at: string; session: { id: string; title: string }; fork: boolean }
  /**
   * A question the session asked in its walk window, `ask_in_window`. `window` is the address it was
   * asked in. **Replaced by id once answered**, with `answer` set to what the person chose and said.
   */
  | {
      kind: "window_ask";
      id: string;
      at: string;
      question: string;
      options?: WaitingOption[];
      place: WindowPlace;
      window: string;
      answer?: string;
    }
  /**
   * What a piloted session starts with, first in its thread: the Job's worktree handed over and what
   * Fleet knew when its Drone stopped, as the bundle's structured fields. Since 23.51.
   */
  | {
      kind: "handoff";
      id: string;
      at: string;
      job_id: string;
      number: number;
      title: string;
      /** `take_over` or `restart_step`. */
      reason: string;
      slot?: number;
      branch: string;
      /** The step the Job stopped on. Absent where none did. */
      step?: { id: string; label: string };
      attempts: number;
      /** What the Judge refused on that step, in its own words. */
      refusals?: string[];
      plan: {
        /** False is no plan, and `outside` is then empty rather than everything. */
        declared: boolean;
        /** Files the worktree holds changed that the plan did not cover. */
        outside?: string[];
        /** Paths the plan named that nothing changed under. */
        unwritten?: string[];
      };
      /** Absent where no Drone said what it was stuck on. */
      narrative?: DroneNarrative;
    }
  /** A call put to the person. Replaced as it is answered. */
  | { kind: "ask"; id: string; at: string; ask: HelmCallInFlight; state: SessionAskState; via?: string };

/** `get_session`: the row and its thread, oldest first. */
export type SessionThread = { session: SessionRecord; rows: SessionRow[] };

/** `get_session_subagent`: one subagent's own thread. A client reads it again while the subagent runs. */
export type SessionSubagent = { rows: SessionRow[]; finished: boolean; report?: string };

/** `session.row`: one row appended, or replaced where `row.id` is held. */
export type SessionRowChanged = { session_id: string; row: SessionRow };
