// The session ledger: every agent session a person runs, and what each holds.
// `crates/ipc/src/sessions.rs`, `docs/concepts/session.md`. Since protocol 23.43.
//
// The header rules in `protocol.ts` hold here. `kind` on an attachment is open
// text, so it is a `string`; the sets below that Fleet closes are written out.

import type { HelmCallInFlight } from "./helm-calls";
import type { HostedFacts, SessionMode } from "./hosted-sessions";
import type { ContextBreakdown } from "./context-breakdown";
import type { SessionActivity } from "./session-activity";
import type { ReportedTask, SessionBuild } from "./session-build";
import type { PlanWindow } from "./plan-usage";

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
  /** This is its repository's Helm Session, which Bridge draws marked. Absent for every other. */
  helm?: true;
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
  /**
   * When the agent said, through `all_done`, that it has nothing left to work on. Absent when it has not,
   * or has been given work since: the person's message, a non-empty `waiting_for`, a hosted turn starting.
   */
  done_since?: string;
  /** What the Session is doing, derived by Fleet on every read. Absent from a Fleet before 23.86. */
  standing?: SessionStanding;
  /**
   * The plan's limit stopped this session, and Fleet wakes it with `Continue` a minute after this
   * instant, when the limit has reset. Absent when it is not held: a turn, a message from the person,
   * the wake and the session ending all let it go.
   */
  held_until?: string;
  /**
   * What fills the context window, as `/context` shows it, from the latest report. Absent until a
   * mod reports one.
   */
  context_breakdown?: ContextBreakdown;
  /**
   * What a hosted session is doing, as Fleet saw its tool calls. Absent before the first call, on a
   * session a terminal runs and once the session ends. Since 14 Oct 2026.
   */
  activity?: SessionActivity;
  /**
   * What the Session has built, derived by Fleet on every read from what it observed. Absent for a Session that has
   * done nothing Fleet observed. See `session-build.ts`.
   */
  build?: SessionBuild;
};

/** What a press on a waiting item does. `target` is an address, a call, a pull request number or a command. */
export type WaitingAct = { kind: "walk" | "answer" | "approve_pr" | "run" | "message"; target: string };

/**
 * Who put an item on the list: the agent, or Fleet from an open card, a permission, an unapproved walk
 * window, a question the session asked in its walk window (`ask_in_window`), or `first_message`: another
 * Session's agent started this one and the person has not sent its first message. A `first_message` item
 * has no reply and its `act` is a `message` on the Session, so a press opens it. Fleet also derives
 * `done` (id `done:<done_since>`, text `Done`) from the turn, so a Session that said it was done is on
 * the list whether or not the agent called a tool. `replied` is no longer served: a reply is the
 * Session's `standing`, not an item.
 */
export type WaitingSource = "agent" | "ask_card" | "walk" | "permission" | "window_ask" | "first_message" | "replied" | "done";

/**
 * What a Session is doing, read by Fleet from what happened and never from what the agent said:
 * `working` a turn runs, `stuck` a turn runs and has been quiet, `replied` the turn ended on the
 * agent's message and the person has not answered, `done` the agent said nothing is left, `idle`
 * nothing is owed. One answer for every surface that draws a Session. Since 23.86.
 */
export type SessionStanding = "working" | "stuck" | "replied" | "done" | "idle";

/** One choice of an item or a question, with what choosing it means where the asker said. */
export type WaitingOption = { label: string; description?: string };

/**
 * Where on a walked page a question points: the address, on the origin of a window the session showed,
 * a CSS selector for the element, and text it shows that picks one where the selector finds several.
 */
export type WindowPlace = { url: string; selector: string; text?: string };

/**
 * `POST /sessions/window/ask`, the agent's: a question about a place on the page its walk window shows.
 * Answered at once with the `SessionRecord`; the person's answer comes back as their message.
 */
export type AskInWindow = {
  question: string;
  options?: WaitingOption[];
  url: string;
  selector: string;
  text?: string;
  session_id?: string;
};

/**
 * One thing a session waits on the person for. `id` is stable across updates: the agent's own, or
 * `ask:<call>`, `perm:<call>`, `walk:<url>`, `window_ask:<row>`, `unsent:<session>` where Fleet derived it. `options` is
 * present only on an item that has choices, and `answer_waiting`'s `choice` indexes it. `place` is
 * present only on a `window_ask` item, whose `act` is a walk of the window it was asked in.
 */
export type WaitingItem = {
  id: string;
  text: string;
  since: string;
  source: WaitingSource;
  act?: WaitingAct;
  options?: WaitingOption[];
  /** The procedure the person follows, in order; its press opens them as steps. Absent when the item has none. */
  steps?: string[];
  place?: WindowPlace;
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
 * `POST /sessions/start_another`, the agent's: a new Session in `manifest_id`'s repository. Its `message` is
 * held unsent and nothing runs until the person sends it. The caller is placed by its connection.
 */
export type StartAnotherSession = { manifest_id: string; title: string; message: string };

/**
 * `POST /sessions/fork_this`, the agent's: a new Session that starts as a copy of the caller's own
 * conversation, ending at the row `at` (included) or, absent, at its latest message of its own that the
 * transcript holds. `message` is held unsent as `StartAnotherSession`'s is.
 */
export type ForkThisSession = { title: string; message: string; at?: string };

/** What both answer: the Session exists and is listed, and no process has started. */
export type SessionHeld = { session_id: string; title: string; fork: boolean };

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
 * `POST /sessions/take_main_red`: a Session takes its repository's red main in the central store, so no Job
 * on another machine is handed it. `session_id` names the Session where the connection places none.
 * `take_over` replaces another machine's claim, and is asked for after a refusal that named the holder.
 * Fleet refuses 409 `fleet.main_taken_elsewhere` naming the holder, and 500 `fleet.main_claim_not_made`
 * where the store would not answer.
 */
export type TakeMainRed = { session_id?: string; take_over?: boolean };

/** What `take_main_red` answers. `claimed` is false where the switch is off or the machine is signed out: nothing was written. */
export type MainRedTaken = { root: string; claimed: boolean };

/**
 * `POST /sessions/waiting/dismiss`: drop one item for good. Its id never comes back, whether Fleet derived
 * it (`ask:`, `perm:`, `walk:`) or the agent stated it. Nothing is sent to the agent, except for an item
 * the agent stated: it is told `Owner removed "<item text>"`, with `reason` after it where the person gave one.
 */
export type DismissWaiting = { session_id: string; item_id: string; reason?: string };

/** One take of a change a session shows in its window. `url` is `http` or `https`. */
export type WindowTake = { name: string; url: string };

/**
 * `POST /sessions/window`, the agent's: a page to show. `variations` offers several takes of one
 * change in the one window, each a page of its own, and `url` is the first's. Absent: one page.
 */
export type ShowWindow = {
  url: string;
  title?: string;
  session_id?: string;
  variations?: WindowTake[];
  /** Other `http` or `https` origins, with no path, the window may pass through: a single sign-on provider. */
  origins?: string[];
  /** Addresses on the shown origin or one of `origins`, loaded in order before the page. A secret: Fleet keeps it in memory and writes it nowhere. */
  sign_in?: string[];
};

/** `POST /sessions/window/sign_in`: Bridge, opening a window, asks for the sign-in addresses held for it. */
export type TakeWindowSignIn = { session_id: string; url: string };

/** What it answers: the addresses, once. Empty where none are held. */
export type WindowSignIn = { sign_in: string[] };

/**
 * `POST /sessions/sketch`, the agent's: a diagram to draw. `scene` is the format `parseScene` reads
 * (`SketchScene/scene.ts`), which Fleet has already checked by the time Bridge sees it. It lands on the
 * ledger as an artifact of form `sketch`, keyed by `title`: a second call with the same title replaces it.
 */
export type ShowSketch = { title: string; scene: unknown; session_id?: string };

/**
 * `POST /sessions/window/closed`: the person closed the window a session showed, by its `url` as
 * `show_window` named it. It ends the session's walk and settles nothing on Waiting on you.
 */
export type WindowClosed = { session_id: string; url: string };

/**
 * `POST /sessions/window/failed`: the window a session showed could not show its page. Fleet tells the
 * session in its own voice; the walk stays open.
 */
export type WindowFailed = { session_id: string; url: string; why: string };

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

/**
 * `start_session_run`: the person ran a shell block from a Session's thread, on a terminal of its own.
 * `POST /sessions/run/start`. `cols` and `rows` are the terminal's size in cells; absent is 80 by 24.
 */
export type StartSessionRun = { session_id: string; command: string; cols?: number; rows?: number };

/** `start_session_run`, answered 202: the run is underway; what it prints is on `observe_session_run`. */
export type SessionRunUnderway = { run_id: string };

/** `stop_session_run`: end a run's process group. `POST /sessions/run/stop`. Answered 200 with the same body. */
export type StopSessionRun = { run_id: string };

/** `write_session_run`: keys typed into a run's terminal, written as they are (Ctrl-C is "\u0003"). `POST /sessions/run/write`. Answered 200 with `SessionRunUnderway`. */
export type WriteSessionRun = { run_id: string; data: string };

/** `resize_session_run`: the run's terminal changed size, in cells. `POST /sessions/run/resize`. Answered 200 with `SessionRunUnderway`. */
export type ResizeSessionRun = { run_id: string; cols: number; rows: number };

/**
 * `observe_session_run`, `GET /sessions/run/:run_id/observe` on a socket of its own. What the run
 * printed so far and then what it prints next as `output`, then `exited` once, then the socket closes.
 * `text` is the terminal's bytes as UTF-8 with its escape sequences intact; no message splits a
 * character. `code` is null where the process group was stopped or killed by a signal.
 */
export type SessionRunMessage =
  | { kind: "opened"; protocol_id: string; run_id: string }
  | { kind: "output"; text: string }
  | { kind: "missed"; dropped: number }
  | { kind: "exited"; code: number | null };

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
  | { kind: "measured"; usage: SessionUsage; plan?: PlanWindow[]; context_breakdown?: ContextBreakdown }
  /** `said` is the first line of the last thing the agent said to the person. Since 23.86. */
  | { kind: "turn_completed"; said?: string }
  /** A turn began. Since 23.86. */
  | { kind: "turn_started" }
  /** A turn ended on the plan's limit, which resets at `resets_at`. Fleet wakes the session then. */
  | { kind: "limited"; resets_at: string }
  /** Files written, relative to the session's `cwd`. Since 23.81. */
  | { kind: "touched"; paths: string[] }
  | { kind: "wrote" }
  /** The agent's task list, whole, as its last task tool left it. An empty list clears it. */
  | { kind: "tasks"; tasks: ReportedTask[] }
  /** The agent left plan mode. */
  | { kind: "plan_left" }
  /** A `git commit` ran and its output showed a commit made. `branch` is absent on a detached head. */
  | { kind: "committed"; branch?: string }
  /** A `git push` ran and its output showed a ref updated. */
  | { kind: "pushed"; branch?: string }
  | { kind: "ended"; reason: string };

export type SessionReport = {
  harness: string;
  session_id: string;
  fact: SessionFact;
};

/**
 * `POST /sessions/checks`: `armada check` tells Fleet a run began or ended in `cwd`. Fleet places the
 * Session by the slot the directory is in. `run`, `outcome` and `log` ride the `end`. Since 23.75.
 * Bridge never sends it; it is here so the wire reads whole.
 */
export type SessionCheckCall = {
  act: "start" | "end";
  cwd: string;
  name: string;
  /** What the run was narrowed to: the test filter, or `--changed ` and the packages reached. `end` may tell it when `start` could not. */
  target?: string;
  run?: number;
  outcome?: "passed" | "failed";
  took_ms?: number;
  log?: string;
};

/** What it answers. `run` is absent where no Session holds the slot, which is a person's own run. */
export type SessionCheckAnswer = { run?: number };
