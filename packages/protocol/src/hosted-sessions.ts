// A session Fleet hosts for Bridge: the commands that drive one, its thread and
// the event that streams it. `crates/ipc/src/hosted_sessions.rs`,
// `docs/concepts/session.md`. Since protocol 23.49.
//
// The header rules in `protocol.ts` hold here. A hosted session is a
// `SessionRecord` with `origin: "bridge"` and `hosted` set; its thread is not on
// the row. Read `get_session` once and follow `session.row`.

import type { HelmCallAnswer, HelmCallInFlight, QuestionAnswer } from "./helm-calls";
import type { LandsIn, Whose } from "./retro";
import type { SlotReleased } from "./holding";
import type { PlanChangeLine } from "./session-plan";
import type { RequestBy, RequestHeader } from "./requests";
import type { ThoughtTarget } from "./thought";
import type { SessionRecord, WaitingOption, WalkStep, WindowPlace, WindowTake } from "./sessions";
import type { StartAfter, WaitBy, WaiterEvent, WaiterSide } from "./session-waits";

/** The permission mode a session runs in. `auto` is the default. */
export type SessionMode = "ask" | "auto" | "accept_edits" | "plan";

/** What a Session that is followed did, for its parent's thread. */
export type LinkedEvent = "needs" | "done" | "merged";

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
   * Start as a copy of an ended or dead session's conversation. Since 23.69. A new session with its own
   * id and ledger, holding none of the old one's. Refused 409 `fleet.session_fork_live` for a live one.
   * `at`, since 23.79, is the thread row the copy ends at, included; with it the old session may be live and
   * is left as it was. Refused 422 `fleet.fork_point_unwritten` where no message has that id.
   */
  fork?: { session_id: string; at?: string };
  /**
   * Start Setup's session in a served repository with no `armada.yml` yet. It runs in `root`, carries no
   * Manifest, and `manifest_id` is not read. A root that has one by now starts as a plain start on it.
   */
  root?: string;
  /**
   * The words the person dispatched. Since 23.91. Fleet reads them once with the Job proposer's model and
   * fills in the title, workflow, model and effort this start leaves unset; the answer is not waited on.
   * The session is answered at once with `proposal.reading` true, and its process starts on the answer.
   * Not read with `fork` or `root`.
   */
  request?: string;
  /**
   * A Session this one waits on, and when it starts. With it `request` is the brief and nothing is sent: the Session holds
   * its brief, no slot and no process until the condition is met, and then starts on the other Session's handoff followed
   * by the brief. The Session waited on is told. Not read with `fork` or `root`.
   */
  after?: StartAfter;
  /**
   * Start the session held: it is answered at once, the proposer reads `request`, and no process starts until
   * `POST /sessions/begin`. Its `hosted.start` is present while it waits. Dispatch sends it. Not read with
   * `fork`, `root` or `after`.
   */
  held?: boolean;
};

export type TagKind = "session" | "pull_request" | "branch" | "issue";

/** What a message names with `@`. Fleet writes the line that tells the agent. */
export type SessionTag = {
  kind: TagKind;
  /** A session's id, a pull request's or an issue's number, or a branch's name. */
  id: string;
  /** An issue's or a pull request's reads `#2214 The title`. */
  title: string;
  /** The forge's address for an issue or a pull request. Absent for any other kind. */
  url?: string;
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
  /** The folder an `always_allow`, `always_block` or `allow_for_session` is for. A 422 `fleet.session_answer_not_a_folder` without one that holds `reaches`. */
  folder?: string;
};

/**
 * `POST /sessions/begin`. Starts a held session: its process starts on `request`, and `hosted.start` goes. `delivery`
 * is how the work lands, where the session's workflow has a landing step; left out, the choice `start.picked` names
 * stands. Refused 422 `fleet.session_delivery_unchosen` where the workflow asks and neither names one. Answers the record.
 */
export type BeginSession = { session_id: string; delivery?: DeliveryChoice };

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

/** `POST /sessions/move`: ends the session and continues its conversation in another served repository. Answers the new session's record. */
export type MoveSession = { session_id: string; manifest_id: string };

/** `POST /sessions/stop`: ends the turn that is running. The session stays live and its next message resumes it. Since 23.84. */
export type StopSession = { session_id: string };

/** `POST /sessions/handoff/retry`: runs again the hand off that failed on this Job (a standing `handoff_failed` row). Answers the new Job's record; refused 409 `fleet.handoff_not_failed` where none stands and `fleet.handoff_running` while one runs, and 422 `fleet.handoff_failed` with the reason where it fails again (a fresh row replaces the old). */
export type RetryHandoff = { session_id: string };

/** `POST /sessions/pause`: ends the turn, parks the slot's work on its branch and gives the slot back. Answers the record with `hosted.paused` set. Since 23.92. */
export type PauseSession = { session_id: string };

/** `POST /sessions/resume`: takes a slot again at the branch tip, or waits for one (`hosted.paused.resuming`). Starts no turn. Since 23.92. */
export type ResumeSession = { session_id: string };

/**
 * `POST /sessions/auto_land`: Auto land on or off for a hosted Job. Answers the `SessionRecord`, whose `auto_land` reads the flag.
 * The flag changes first, so a 409 `fleet.auto_merge_refused` (the forge's words) leaves it set; 422 `fleet.session_command_refused` for a terminal or ended Job leaves it as it was.
 */
export type SetSessionAutoLand = { id: string; on: boolean };

/** The marker on a paused hosted Session. Not a state: `state` stays `live`. Since 23.92. */
export type SessionPaused = { at: string; resuming: boolean };

/**
 * `POST /sessions/workflow`: the person picked the Session's workflow in a panel, or cleared it. Since 23.91.
 * Answers the `SessionRecord`. A Session that has run is told at once; one that has not, with its first turn.
 * 422 `fleet.session_workflow_not_held` for an id the Session's repository does not hold.
 */
export type SetSessionWorkflow = {
  session_id: string;
  /** Absent clears it: a custom Session. */
  workflow?: string;
};

/** `POST /sessions/release_holds`: give back the slots, branches, pull requests, Jobs and needs a Session holds, or hand them to `to_session_id`. `rows` absent is every one. */
export type ReleaseSessionHolds = { session_id: string; to_session_id?: string; rows?: ChosenHold[] };

/** One row the person ticked: its kind and target as the ledger names them. */
export type ChosenHold = { kind: string; target: string };

/** What `release_session_holds` moved, and what stayed with why. */
export type SessionHoldsReleased = {
  to_session_id?: string;
  moved: HoldMoved[];
  kept: HoldKept[];
};

/** One row given back or handed over. `released` is a slot's branch and what was committed to it. */
export type HoldMoved = { kind: string; target: string; released?: SlotReleased };

/** One row that stayed the Session's. */
export type HoldKept = { kind: string; target: string; why: string };

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
  /**
   * The model the CLI resolved for the running process, as its `init` line names it: `vendor-opus-5-5` where
   * `model` is `opus` or absent. Absent until the process has started, and again once it is replaced. Since 23.85.
   */
  resolved_model?: string;
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
  /** Set while the person has paused the session. Since 23.92. */
  paused?: SessionPaused;
  /** Present while a held session waits for `POST /sessions/begin`, and gone once it is begun. */
  start?: StartHeld;
};

/**
 * What a held session waits on. `asks_delivery` is whether its workflow has a landing step, so a choice is owed before it begins.
 * `picked` is the choice already standing: the person's `usual` for this kind of workflow in this repository, or one the request
 * `named`. Fleet recomputes it when the workflow changes, and leaves it out where there is none.
 */
export type StartHeld = {
  asks_delivery: boolean;
  picked?: DeliveryPicked;
};

/** Where a standing delivery choice came from: the person's `usual`, or one the request `named`. */
export type DeliveryPicked = { choice: DeliveryChoice; from: "usual" | "named" };

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
  | "always_allowed"
  | "always_blocked"
  | "allowed_for_session"
  | "refused"
  | "unanswered"
  | "kept"
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

/** Where a failed check's log says it failed: a path relative to the slot, and its 1-based line where the log names one. */
export type CheckBlame = { path: string; line?: number };

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
      /** The Fleet notice this row was sent for (`behind_base`, `write_clash`, …: `Notice`), absent on every other row. Mute it by `set_notice_mute`. */
      notice?: string;
    }
  /**
   * Words Helm's dock sent to a Session or a Job instead of asking Helm, from `tell_session` and
   * `record_thought_sent`. Only a Helm Session's thread has them.
   */
  | {
      kind: "sent";
      id: string;
      at: string;
      text: string;
      target: ThoughtTarget;
      files?: SentFile[];
      tags?: SessionTag[];
    }
  /** A tool call, one line. */
  | { kind: "tool"; id: string; at: string; text: string }
  /** A command the person ran in the terminal, as typed, with no output. Since 23.56. */
  | { kind: "command"; id: string; at: string; text: string }
  /** The summary the CLI wrote where it compacted the conversation. Not the person's words. Since 23.56. */
  | { kind: "compaction"; id: string; at: string; text: string }
  /**
   * The agent disputed a notice Fleet sent it. `notice` is the id of Fleet's `message` row; `why` is the agent's reason.
   * `spotted` is `called` where the agent said so itself, `read` where Fleet read it from the agent's replies after
   * 10 minutes. `review` is the review Job's session id; absent where no review was started.
   */
  | { kind: "disputed"; id: string; at: string; notice: string; why: string; spotted: "called" | "read"; review?: string }
  /** The first write: the slot leased and the branch cut. */
  | { kind: "lease"; id: string; at: string; slot: number; branch: string }
  /** The slot given back, with the branch it was on. */
  | { kind: "release"; id: string; at: string; slot: number; branch: string }
  /**
   * An `armada check` the agent ran in its slot, replaced by id as it ends. `run` is the id `GET /sessions/checks/:run/output` reads. Since 23.75.
   * `target` is what the run was narrowed to, the filter it was given or the crates `--changed` narrowed to; absent, it ran whole.
   * `blame` is where a failed run's log says it failed, read by Fleet as the run ends; absent while running, on a pass, and where the log names no file.
   */
  | { kind: "check"; id: string; at: string; name: string; run: number; state: "running" | "passed" | "failed"; target?: string; blame?: CheckBlame }
  /**
   * A command the person ran from the thread or the Needs-you band, ended, and what the Session was told of it, in place
   * of a message from Fleet. `code` is absent where the run was stopped or a signal ended it; `tail` is the last lines it
   * printed as plain text, the same the agent was sent. Since 23.89.
   */
  | { kind: "ran"; id: string; at: string; command: string; code?: number; tail: string }
  /**
   * An HTTP request sent from Requests by the owner with this Job open, or by the Job's agent, and how it
   * was answered. The request half is whole, so a row reopens to edit and send again: `url` and `headers` as sent, filled.
   * `status` is absent where nothing answered and `error` says why; `response` is the answer's body, cut at 16 KiB where
   * `truncated`. **No secret's value is on the row**: a secret reads `{{name}}` in every field. Since Requests.
   */
  | {
      kind: "requested";
      id: string;
      at: string;
      by: RequestBy;
      method: string;
      url: string;
      environment?: string;
      headers?: RequestHeader[];
      body?: string;
      status?: number;
      ms: number;
      error?: string;
      response: string;
      truncated?: boolean;
    }
  /**
   * A request the Job's agent saved into a collection with `save_request_to_collection`. `collection_id` and `request` are
   * the ids Requests opens it by; `name` and `collection` are what the row says. Since Requests.
   */
  | { kind: "saved"; id: string; at: string; name: string; collection: string; collection_id: string; request: string }
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
   * approval or the window closing; absent while it is open. `variations` are the takes it shows
   * when it shows several, `url` the first's; absent for one page. `steps` are the scripted walk it
   * carries, absent where it carries none.
   */
  | {
      kind: "window";
      id: string;
      at: string;
      title: string;
      url: string;
      variations?: WindowTake[];
      /** The scripted walk the window carries, when it carries one. */
      steps?: WalkStep[];
      /** The other origins the window may pass through, such as a sign-in provider. */
      origins?: string[];
      ended_at?: string;
    }
  /**
   * This session's agent started a session, or forked its own conversation into one, when the person
   * asked it to. Written in the starting session's thread; `session` is the new one.
   */
  | { kind: "started"; id: string; at: string; session: { id: string; title: string }; fork: boolean }
  /**
   * A Session this one follows needs the person, ended, or had a pull request merged. Written in the followed
   * Session's parent's thread where it happened, and the parent's agent is told the same moment; `session` is the linked one.
   */
  | { kind: "linked"; id: string; at: string; session: { id: string; title: string }; event: LinkedEvent }
  /**
   * A Session waits on this one, its condition changed, or it started. Written in this Session's thread (`side` absent,
   * `session` the one that waits), and in the waiting Session's thread where the condition changed (`side` `waiting`,
   * `session` the one it waits on). `says` is what the waiting Session needs or where it started from; `from` and `to` are
   * the condition as it was and as it is, and `by` who changed it.
   */
  | { kind: "waiter"; id: string; at: string; session: { id: string; title: string }; event: WaiterEvent; side?: WaiterSide; says?: string; from?: string; to?: string; by?: WaitBy }
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
  /** A call put to the person. Replaced as it is answered. */
  | { kind: "ask"; id: string; at: string; ask: HelmCallInFlight; state: SessionAskState; via?: string }
  /**
   * The agent recorded a plan, or recorded it again. The plan is `SessionRecord.plan`; this row is where
   * in the thread it was recorded.
   */
  | { kind: "plan"; id: string; at: string; version: number }
  /**
   * What the person sent the agent against a version of the plan, as Fleet worded it. The plan does not
   * change until the agent records it again. `approved`: the plan was approved with them.
   */
  | { kind: "plan_changes"; id: string; at: string; version: number; approved?: boolean; lines: PlanChangeLine[] }
  /**
   * How the Job's work is delivered. Replaced as the owner answers; `named` is set where the first message
   * already said and nothing was asked. `choice` is absent while the question stands.
   */
  | { kind: "delivery"; id: string; at: string; choice?: DeliveryChoice; named?: boolean };

/** What happens to a Job's work once it is done: open a pull request and merge it, stop at the pull request, or stop at the local commit. */
export type DeliveryChoice = "auto_land" | "open_pr" | "keep_local";

/** `get_session`: the row and its thread, oldest first. */
export type SessionThread = { session: SessionRecord; rows: SessionRow[] };

/** `get_session_subagent`: one subagent's own thread. A client reads it again while the subagent runs. */
export type SessionSubagent = {
  rows: SessionRow[];
  finished: boolean;
  report?: string;
  /** The model id its last answer was written under. Absent until it has answered. */
  model?: string;
  /** What that answer had in its window, and the window it is measured against. Absent until it has answered. */
  context_tokens?: number;
  context_window?: number;
};

/** `session.row`: one row appended, or replaced where `row.id` is held. */
export type SessionRowChanged = { session_id: string; row: SessionRow };
