// The session ledger: every agent session a person runs, and what each holds.
// `crates/ipc/src/sessions.rs`, `docs/concepts/session.md`. Since protocol 23.43.
//
// The header rules in `protocol.ts` hold here. `kind` on an attachment is open
// text, so it is a `string`; the sets below that Fleet closes are written out.

import type { HelmCallInFlight } from "./helm-calls";
import type { DeliveryChoice, HostedFacts, SessionMode } from "./hosted-sessions";
import type { ContextBreakdown } from "./context-breakdown";
import type { LoadoutItems, LoadoutSummary, LoadoutUse } from "./loadout";
import type { TurnUsage } from "./turn-usage";
import type { SessionActivity } from "./session-activity";
import type { ReportedTask, SessionBuild } from "./session-build";
import type { ProtocolId } from "./connection";
import type { PlanWindow } from "./plan-usage";
import type { WorkflowSummary } from "./setup";
import type { FileTouch, SessionPlan } from "./session-plan";
import type { JudgeConcern, JudgeReading, WaitingCite } from "./session-rail";
import type { SessionWait, StartAfter } from "./session-waits";
import type { WireError } from "./protocol";

/** What started a session. */
export type SessionOrigin = "terminal" | "bridge";

export type SessionState = "live" | "ended";

/**
 * Where an attachment stands, for every kind of one: a slot let go and a need
 * whose branch landed are different words and the same question.
 */
export type AttachmentState = "standing" | "spent" | "given_back";

export type HolderKind = "session";

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
 * One attachment as it stood from `at`, until its next version or for good. `get_attachment_history`
 * answers them, oldest first; a `SessionRecord` carries only the latest, in `Attachment`.
 */
export type AttachmentVersion = {
  kind: string;
  manifest_id?: string;
  target: string;
  state: AttachmentState;
  detail?: Record<string, string>;
  at: string;
  /** The thread row id of the person's latest message when it was written. */
  from_message?: string;
  /** The plan task in progress when it was written. */
  from_task?: string;
};

/** `get_attachment_history`: every version of everything one session holds, oldest first. */
export type AttachmentHistory = { versions: AttachmentVersion[] };

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
  /** A Mod Job: started from Mod, for modding Armada itself. */
  mod?: true;
  /** What a session Fleet hosts carries beyond a terminal's. Since 23.49. */
  hosted?: HostedFacts;
  /** What a session run in a terminal runs on, once its mod has said. Since 23.53. */
  terminal?: TerminalFacts;
  /** A terminal session whose mod is older than its repository's, or reported no version. Since 23.62. */
  mod_out_of_date?: true;
  /**
   * A hosted session whose agent came up without some of its Armada tools, as Fleet read the CLI's init line. `mod`: the mod did not load; `door`: Fleet's MCP door did not
   * connect; `task_tools`: the CLI served no task list tools. Absent when nothing is missing.
   */
  tools_missing?: { harness: string; update: string; harness_version: string; missing: Array<"mod" | "door" | "task_tools"> };
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
   * The Loadout in brief: counts and the latest turn's uses. Absent where no `equipped` was ever
   * reported. The whole of it is `get_session_loadout`.
   */
  loadout_summary?: LoadoutSummary;
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
  /**
   * The workflow id the Session runs under. Absent is a custom Session, which is not a default workflow.
   * Hosted Sessions only. Since 23.91.
   */
  workflow?: string;
  /** What the proposer did for this Session. Absent where it was never asked. Since 23.91. */
  proposal?: SessionProposal;
  /** Auto land is on: Fleet turns on the forge's auto-merge for each pull request the Session holds, as it attaches. Absent is off. Hosted only. */
  auto_land?: boolean;
  /** The plan the Session recorded. Absent until it records one. See `session-plan.ts`. */
  plan?: SessionPlan;
  /** The files it and its subagents last touched, oldest first, a bounded window. Absent while there are none. */
  touches?: FileTouch[];
  /**
   * The index of the workflow step the Session is on, read by Fleet from what it observed and never from the
   * agent's word. Absent for a custom Session, once the delivering step's pull request merged with no step
   * after it, and while `features.workflows` is off. Since 16 Oct 2026.
   */
  workflow_step?: number;
  /** A judge is reading the work now. Held in memory in Fleet. Absent when none is. Since 16 Oct 2026. */
  judging?: boolean;
  /** The last reading's outcome. Absent until a judge has read. `concerns` 0 is "No concerns". Since 16 Oct 2026. */
  judge?: JudgeReading;
  /**
   * Concerns the person has settled, oldest first, a bounded window: each a `judge` item with
   * `concern.settled` set, for the thread to draw as one line. They are not in `waiting_for`. Since 16 Oct 2026.
   */
  settled?: WaitingItem[];
  /**
   * The items the person closed from `waiting_for`, newest first, a bounded window: each kept whole at the moment he closed it.
   * `reopen_waiting` lets one wait again. Absent when none. Since 29 Oct 2026.
   */
  closed?: ClosedWaiting[];
  /**
   * Dispatched to wait on another Session and not started: it holds its brief, no slot and no process until the condition is
   * met. Absent once it has started. Since 16 Oct 2026.
   */
  waits?: SessionWait;
  /**
   * The follow-ups the agent said it keeps for itself, each open until it is settled, oldest first. `all_done` is refused
   * while any is open. Absent or empty is none. Since 21 Oct 2026.
   */
  later?: SessionLater[];
  /**
   * The name of the live watch that is all that keeps the Session open: its turn has ended and nothing is in `waiting_for`.
   * Absent otherwise.
   */
  held_open_by?: string;
  /**
   * How the Job was started, as it stood when its process first ran: a later change of model, effort or mode does not rewrite it.
   * Absent for a Job started before Fleet kept it and for a terminal session.
   */
  started?: SessionStarted;
  /** When each step of the workflow began, oldest first. A step the Job went back to appears again. Absent when none was entered. */
  workflow_visits?: StepVisit[];
  /** The steps the person skipped, by the same index as `workflow_step`. The Job passes over each one. Absent where none is. */
  skipped_steps?: number[];
  /**
   * What the step the Job is on waits for, as the rail words it: `Waiting on PR #812: Approved`. One sentence for each lifecycle end
   * the step lists that has not come, in the order the step lists them. Absent where the step waits on no lifecycle, and once an end has come.
   */
  workflow_waiting_on?: string[];
  /**
   * The Job's own copy of its workflow, once it changed a step of it (`change_session_steps`): what the rail and the lanes read
   * in place of the repository's. Absent until then. Since 30 Oct 2026.
   */
  workflow_copy?: WorkflowSummary;
};

/** How a Job was started. Every part is absent where it was not set then. `workflow` is the workflow's name as it read then. */
export type SessionStarted = {
  workflow?: string;
  /** The landing chosen by then. A Job that is asked later has none here. */
  delivery?: DeliveryChoice;
  /** The model it ran on, the machine's own where none was picked. */
  model?: string;
  effort?: string;
  mode?: SessionMode;
  /** The Job that started it. */
  by?: { id: string; title: string };
};

/** The time a workflow step began. `step` is its index in the workflow. */
export type StepVisit = { step: number; since: string };

/** A follow-up the agent kept for itself, read from the close of a reply. `id` is what `settle_later` names it by. */
export type SessionLater = { id: string; sentence: string; said_at: string; /** Thumbs up on it (`confirm_later`). Absent where not. */ confirmed?: boolean };

/** How a follow-up was settled: the work is done, it is an issue now (`issue`), or it will not be done (`reason`). */
export type LaterHow = "done" | "filed" | "dropped";

/**
 * `POST /sessions/settle_later`: one open follow-up is settled, by the agent or by the person. `issue` is the forge's number or a
 * tracker's key for a `filed`. `reason` is required of the agent's `dropped`; the person's Drop sends none and `session_id` names the Session.
 * Fleet refuses 422 `fleet.later_not_settleable` for a `filed` with no issue it can read or an agent's drop with no reason, and
 * 409 `fleet.later_not_open` for an id that names no open follow-up.
 */
export type SettleLater = { id: string; how: LaterHow; issue?: string; reason?: string; session_id?: string; /** Thumbs down, Not deferred: the person's `dropped` only; keeps the sentence as no follow-up. */ misread?: boolean };

/** `POST /sessions/later/confirm`: thumbs up on an open follow-up; answered with the `SessionRecord`. */
export type ConfirmLater = { id: string; session_id?: string };

/**
 * `POST /sessions/defer_later`: the agent keeps one follow-up for itself, as a sentence. `session_id` names the Session only where the
 * connection places none. Fleet refuses 422 `fleet.later_not_deferrable` for an empty sentence and 409 `fleet.later_not_open` for one settled already.
 */
export type DeferLater = { sentence: string; session_id?: string };

/** What `defer_later` answers: the follow-up kept and the ones open with it. */
export type LaterDeferred = { id: string; sentence: string; open: SessionLater[] };

/** What `settle_later` answers: the follow-up settled and the ones still open. */
export type LaterSettled = { id: string; sentence: string; open: SessionLater[] };

/** A value of a Session the proposer can pick. */
export type ProposedField = "title" | "workflow" | "model" | "effort";

/**
 * The proposer's part in a Session, per field. A field in `picked` is one the proposer chose and nobody has
 * changed since, so it is marked "Picked by the proposer"; the person changing it in a panel takes it out.
 * `reading` is the call being out: the Session exists and its process starts on the answer. In memory in
 * Fleet, so a Fleet that restarts reads nothing as reading. Since 23.91.
 */
export type SessionProposal = {
  reading?: boolean;
  picked?: ProposedField[];
};

/** What a press on a waiting item does. `target` is an address, a call, a pull request number or a command. */
export type WaitingAct = { kind: "walk" | "answer" | "approve_pr" | "run" | "message"; target: string };

/**
 * Who put an item on the list: the agent, or Fleet from an open card, a permission, an unapproved walk
 * window, a question the session asked in its walk window (`ask_in_window`), or `first_message`: the
 * Session waits on its start card until the person presses Start (`POST /sessions/begin`). A `first_message` item
 * has no reply and its `act` is a `message` on the Session, so a press opens it. Fleet also derives
 * `done` (id `done:<done_since>`, text `Done`) from the turn, so a Session that said it was done is on
 * the list whether or not the agent called a tool. `plan` (id `plan:<version>`) is a recorded plan the
 * person has not approved; it is approved, or sent changes, from its own card. `judge` (id `judge:<n>`) is a
 * concern a judge raised, with `concern` and `cites`; it is sent to the agent or dismissed from its own card.
 * `slot_choice` (id `slot-choice`, text `Every slot is in use`, act `answer`) is Fleet asking what to do for a
 * Session that found every worktree slot held: its two options are `Wait for a slot` (choice 0) and `Add a slot`
 * (choice 1), answered with `answer_waiting` and a `choice`. No message goes to the agent. `delivery` (id `delivery`, text
 * `How should this work be delivered?`, act `answer`) is Fleet asking a bug or feature Job's owner how the work leaves its
 * branch: `Auto land` (choice 0), `Open PR only` (choice 1), `Keep local` (choice 2), each with a description. The agent was
 * told to stop and is sent the choice. `words_address` (id `words:<mod>:<manifest id>`, text `Where does <name> open in <manifest id>?`,
 * act `answer`) is Fleet asking where words a person taught open in a repository when it found no address: an option for each address
 * any Words mod holds, then `Other`, answered with `answer_waiting` and a `choice` or `text` that holds `{match}`. No message goes to the agent. `fleet_call`
 * (id `call:<n>`, text the words the Session was told, act `message`) is a notice a Fleet mod's rule makes a call: the Session was told it and the person is
 * asked to look, and the press opens the Session. Dismissing it settles it and tells the agent nothing. `replied` (id `replied:<turn ended at>`) is a reply whose close
 * asks the person something and that the agent did not ask through a tool; a reply that only reports is the
 * Session's `standing`, not an item. `task_done` (id `task_done:<project>`, text `Mark <title> done?` for one task or
 * `Mark the <project> tasks done?` for several, act `answer`) is Fleet asking whether the written project tasks a
 * finished Job was started with are done: `Mark done` (choice 0) or `Keep open` (choice 1), answered with `answer_waiting` and a
 * `choice`. No message goes to the agent and it cannot be dismissed. It is listed while the Job is live; the ask stands on the
 * tasks (`asked`) when the Job ends, and the Projects tab's tick settles it. `draft` (id `draft:<row>`) is a draft the
 * session proposed with `propose_draft`, read off its row and settled by `answer_waiting`. `watch_not_ours` (id `watch_not_ours:<watch id>`,
 * text `<watch>: <parts> failed`, act `answer` whose `target` is the watch id) is Fleet asking about a watch that ended `not_ours`:
 * `Ignore` (choice 0) settles it and tells the agent nothing, `Fix it` (choice 1) sends the agent the part, its sentence and its link.
 * Words alone go to the agent as a message. Each settles it, as dismissing does. It is listed while the Job is live.
 * `held_child` (id `start:<child id>`, text `Press Start on "<child title>"`, act `message` whose `target` is the child's id) is a Job
 * this one's agent started (`started_to`) that is still held on its start card: its press opens the child, and `begin_session` on the
 * child starts it from the row. Fleet derives it from the child and drops it when the child starts or ends.
 * `preview_offer` (text `<branch> moved since the preview left it out: <why>`, act `run` whose `target` is `scripts/preview --restart`)
 * is Fleet offering a refresh on the Job holding a branch the preview left out, once that branch moves past the tip it was left out at.
 * Dismissing it is Skip, and tells the agent nothing.
 * `rule_act` (id `rule_act:<row>`, text the act in his words, such as `Move ARM-41 to In progress`, act `answer` whose `target` is the id) is
 * an act a lifecycle rule of the Job planned that he is asked about first, or one that failed past the rule's retries where the rule says to
 * ask, when the text says what failed. Exactly two options: `Do it` (choice 0, `Try again` on a failed act) carries the act out and `Skip`
 * (choice 1) records it skipped. Neither sends the agent a word, and dismissing it is Skip. It is listed while the Job is live.
 * A draft act is never one: it is proposed at once and the `draft` item it raises is the ask.
 * `sketch` (id `sketch:<title>`, the sketch attachment's id, text the title, act `message` whose `target` is the Job's own id) is a sketch the
 * agent showed with `show_sketch` that he has not approved or sent back on its card. Its press opens the sketch. Dismissing it settles it.
 * `document` (id `document:<path>`, the page's path, text its title, act `message` whose `target` is the Job's own id) is the newest revision of a
 * page the Job wrote under `.armada/documents/` that he has not approved or sent back: a document artifact with `detail.asked` and no
 * `detail.settled`. Its press opens the document. `answer_document` answers it.
 */
export type WaitingSource = "agent" | "ask_card" | "walk" | "permission" | "window_ask" | "draft" | "first_message" | "replied" | "done" | "plan" | "judge" | "slot_choice" | "delivery" | "words_address" | "fleet_call" | "task_done" | "watch_not_ours" | "held_child" | "preview_offer" | "rule_act" | "sketch" | "document";

/**
 * What a Session is doing, read by Fleet from what happened and never from what the agent said:
 * `working` a turn runs, `stuck` a turn runs and has been quiet, `replied` the turn ended on a
 * message of the agent's that only reports and the person has not answered, `done` the agent said nothing is left, `idle`
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
 * `POST /sessions/draft`, the agent's: a draft put to the person for approval. `title` names what it is and where it
 * goes, `text` is the draft in markdown, `before` the text it replaces. Answered at once with the `SessionRecord`; the
 * person's answer comes back as their message.
 */
export type ProposeDraft = {
  title: string;
  text: string;
  before?: string;
  session_id?: string;
};

/**
 * One thing a session waits on the person for. `id` is stable across updates: the agent's own, or
 * `ask:<call>`, `perm:<call>`, `walk:<url>`, `window_ask:<row>`, `draft:<row>`, `unsent:<session>` where Fleet derived it. `options` is
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
  /** Present only on a `judge` item: what produced it, and how it was settled once it is. */
  concern?: JudgeConcern;
  /** Where a `judge` item points: the files its quotation lands in. Absent when it lands in none. */
  cites?: WaitingCite[];
  /** A `replied` item whose sentence he said is a question (`confirm_waiting`). Absent where he has not. */
  confirmed?: boolean;
};

/**
 * `POST /sessions/waiting/answer`: settle one item. A `choice` (an index into its `options`), `text` of
 * the person's own, or a `mode` that leaves it to the agent: `best` thinks it through, `quick` takes the
 * reasonable path and keeps moving. A walk item needs none of the three, and approving is its default.
 * `fixed` is the person's corrected text for a `draft` item, which approves it with their words; on any other item it is a 422.
 */
export type AnswerWaiting = {
  session_id: string;
  item_id: string;
  choice?: number;
  text?: string;
  mode?: "best" | "quick";
  fixed?: string;
};

/**
 * `POST /sessions/start_another`, the agent's: a new Session in `manifest_id`'s repository, held on its start card
 * with `message` as its first message (`hosted.start.message`); nothing runs until the person presses Start. The
 * caller is placed by its connection.
 */
export type StartAnotherSession = {
  manifest_id: string;
  title: string;
  message: string;
  /**
   * The id of a group of the caller's recorded plan, which makes the new Session a linked Session the caller follows.
   * It is titled with the group's title, its held message is the group's tasks word for word and the caller's handoff
   * with `message` after them, and both ledgers keep the group as `detail.group` on the `started_by` and `started_to` rows.
   * Refused 422 `fleet.group_unknown` for a group the plan lacks, and 409 `fleet.group_handed` for one handed already.
   */
  group?: string;
  /**
   * A Session this one waits on, and when it starts. The new Session starts when the condition is met, and its held first
   * message is then the other Session's handoff followed by `message`. The Session waited on is told `after.needs`.
   */
  after?: StartAfter;
};

/**
 * `POST /sessions/hand_to_session`, the person's: hand one group of a Session's plan to a linked Session. The same start as
 * `StartAnotherSession` with a `group`, except the first message is sent, since the press is his. Answers `SessionHeld`.
 */
export type HandToSession = { session_id: string; group: string };

/**
 * `POST /sessions/fork_this`, the agent's: a new Session that starts as a copy of the caller's own
 * conversation, ending at the row `at` (included) or, absent, at its latest message of its own that the
 * transcript holds. It is held with `message` as `StartAnotherSession`'s is.
 */
export type ForkThisSession = { title: string; message: string; at?: string };

/** What both answer: the Session exists and is listed, and no process has started. */
export type SessionHeld = { session_id: string; title: string; fork: boolean };

/**
 * `POST /sessions/start_another`'s whole body: one Job's fields at the top, or several Jobs in `jobs` and nothing else.
 * Each in `jobs` starts as a call of its own would, in order. One Job answers `SessionHeld`, several `SessionsHeld`.
 */
export type StartAnotherSessions = Partial<StartAnotherSession> & { jobs?: StartAnotherSession[] };

/** What a call with `jobs` answers: one entry per Job in the order given. A refused one leaves the others started. */
export type SessionsHeld = { jobs: HeldOrRefused[] };

/** One Job of `SessionsHeld`: `held` where it started, `refused` with its own call's refusal where it did not. */
export type HeldOrRefused = { held?: SessionHeld; refused?: WireError };

/**
 * `POST /sessions/claim_pull_request`: the caller takes an open pull request no live Session holds.
 * `session_id` names the claimant where the connection places none, which is Bridge's case. Fleet refuses
 * 422 `fleet.pull_request_not_claimable`, naming the holder, where a live one has it or it is not open.
 */
export type ClaimPullRequest = { number: number; session_id?: string };

/** What `claim_pull_request` answers: the pull request now held, and by which Session. */
export type PullRequestClaimed = {
  number: number;
  branch: string;
  url: string;
  holder_id: string;
  /** Auto land is on for the Session and the forge would not turn on auto-merge: its words. The claim stands. Absent where Auto land is off or the forge took it. */
  auto_merge_refused?: string;
};

/**
 * `POST /sessions/claim_issue`: the caller works an issue, named by the forge's number (`7`, `#7`) or a
 * tracker's key (`ABC-12`), held as an `issue` row on its ledger. `pull_requests` are the numbers of the
 * pull requests that work it, which the row's detail carries as `pull_requests`, comma-joined ascending.
 * Fleet refuses 422 `fleet.issue_not_claimable` for a reference that is neither, or a call that places no Session.
 * In a repository whose tracker Fleet writes to, it claims there first and refuses 409 `fleet.issue_held`, naming
 * the holder, where the synced copy gives the issue to someone else.
 */
export type ClaimIssue = { issue: string; session_id?: string; pull_requests?: number[] };

/** What `claim_issue` answers: the issue as held, by which Session, and every pull request its row now names. */
export type IssueClaimed = { issue: string; holder_id: string; pull_requests: number[] };

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
 * `misread` is Not a question on an item Fleet read from a reply: its sentence is kept as one that asks nothing.
 */
export type DismissWaiting = { session_id: string; item_id: string; reason?: string; misread?: boolean };

/**
 * `POST /sessions/waiting/confirm`: thumbs up on an item Fleet read from a reply. Its sentence is kept as one
 * that asks, the item stays, and it reads `confirmed` from then on. Refused on an item of any other source.
 */
export type ConfirmWaiting = { session_id: string; item_id: string };

/**
 * `POST /sessions/waiting/reopen`: the person lets an item he closed wait again, by the `id` in `SessionRecord.closed`. An agent's own item
 * goes back on its list as it was stated, and the agent is told so (`The owner reopened "<text>"; ...`); any other item tells it nothing. Fleet refuses 409 `fleet.session_waiting_unheld` for an id nothing
 * closed holds, and 422 `fleet.session_waiting_unreopenable` where `ended` is true.
 */
export type ReopenWaiting = { session_id: string; item_id: string };

/**
 * An item the person closed, as `SessionRecord.closed` lists it: `item` is the `WaitingItem` as Needs you drew it when he closed it
 * (`id`, `text`, `since`, `source`, `act`, `options`, `steps`; `reopen_waiting` takes `item.id`), and `at` when he closed it. `ended` is sent
 * only as `true`: what the item stood for has ended, so it cannot be reopened (a permission whose call ended, a walk that was withdrawn, any
 * item of a Job that has ended). The agent's own items and those whose subject still stands are reopenable.
 */
export type ClosedWaiting = { item: WaitingItem; at: string; ended?: true };

/** `GET /sessions/waiting/closed`, an agent's read: the Job's `closed` items, newest first. */
export type ClosedWaitingList = { session_id: string; closed: ClosedWaiting[] };

/** One take of a change a session shows in its window. `url` is `http` or `https`. */
export type WindowTake = { name: string; url: string };

/**
 * Something on a page a walk's step aims at, found by role and accessible name or by the words it
 * draws. `role` is `text` for the words an element draws, and then `name` is those words. `name` is a
 * string inside the name in any case, or the whole of it with `exact`. `within` looks only inside another.
 */
export type WalkTarget = { role: string; name?: string; exact?: boolean; within?: WalkTarget };

/**
 * One step of a scripted walk: a `say` caption and exactly one verb, with the key that goes with it.
 * The mock walks' own shape (`mock/walk.ts`), as JSON. Fleet refuses any other combination.
 */
export type WalkStep = { say: string } & (
  | { press: WalkTarget }
  | { look: WalkTarget }
  | { hover: WalkTarget }
  | { later: WalkTarget }
  | { type: string; into: WalkTarget }
  | { key: string; on: WalkTarget }
  | { select: string; in: WalkTarget }
  | { paste: WalkTarget }
  | { drag: WalkTarget; by: { x: number; y: number } }
);

/**
 * `POST /sessions/window`, the agent's: a page to show. `variations` offers several takes of one
 * change in the one window, each a page of its own, and `url` is the first's. Absent: one page.
 * `steps` is a scripted walk of the page that Bridge plays from outside it, on any page. Absent: none.
 */
export type ShowWindow = {
  url: string;
  title?: string;
  session_id?: string;
  variations?: WindowTake[];
  /** A scripted walk of the page, played by Bridge from outside it. */
  steps?: WalkStep[];
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

/** How the person answered a sketch put to him, as `detail.settled` holds it. */
export type SketchHow = "approved" | "sent_back";

/**
 * `POST /sessions/sketch/answer`, Bridge's: Approve, or Send back with a note, on a sketch's card. `sketch` is
 * the ledger id, `sketch:<title>`, and `drawn` its `detail.drawn` as he read it; a sketch drawn again since is a
 * 409, `fleet.sketch_redrawn`. `words` is required for `sent_back`. Every sketch shown carries `detail.asked`
 * `"true"`, and once answered `detail.settled`, `detail.settled_at` and his `detail.words`.
 */
export type AnswerSketch = { session_id: string; sketch: string; drawn: string; how: SketchHow; words?: string };

/** How the person answered a document put to him, as `detail.settled` holds it. */
export type DocumentHow = "approved" | "sent_back";

/**
 * `POST /sessions/document/answer`, Bridge's: Approve, or Send back with a note, on a document's card. `document`
 * is the ledger id, `document:<path>`, and `revision` its `detail.revision` as he read it; a revision kept since
 * is a 409, `fleet.document_revised`. `words` is required for `sent_back`. Every newly kept revision carries
 * `detail.asked` `"true"` and `detail.revision`, and once answered `detail.settled`, `detail.settled_at` and his
 * `detail.words`; a new revision drops those three, so it asks again.
 */
export type AnswerDocument = { session_id: string; document: string; revision: string; how: DocumentHow; words?: string };

/**
 * `POST /sessions/show_file`, the agent's: a file to open in the side panel. `path` is absolute or relative
 * to the folder the job runs in; Fleet keeps the canonical path as an artifact of form `file`, `detail.by`
 * `agent` and `detail.shown` the time of the call, so each call is a new show.
 */
export type ShowFile = { path: string; session_id?: string };

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

/** What `window_failed` answers: whether the walk's owner was told now. */
export type WalkTold = { told: boolean };

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

/** `name_session`: Armada names a hosted session from its first message, as the start names one. `POST /sessions/name`. Not kept as the person's rename. */
export type NameSession = { session_id: string };

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
 * character. `code` is null where the process group was stopped or killed by a signal, and `stopped` is
 * true where it was Stop that ended it: a null `code` with `stopped` false is a signal's.
 */
export type SessionRunMessage =
  | { kind: "opened"; protocol_id: string; run_id: string }
  | { kind: "output"; text: string }
  | { kind: "missed"; dropped: number }
  | { kind: "exited"; code: number | null; stopped: boolean };

/** `open_terminal`: the person opens the Job's shell in Bridge. `POST /sessions/terminal/open`. Answered 200 with `SessionRunUnderway`: the shell already open, or one opened now. `cols` and `rows` are the size in cells where it opens now; absent is 80 by 24. */
export type OpenTerminal = { session_id: string; cols?: number; rows?: number };

/** `start_terminal_run`: the person runs `command` in `dir` with no Job behind it. `POST /sessions/terminal/run`. Answered 200 with `SessionRunUnderway`; what it prints is `observe_session_run`'s. `dir` must exist; `cols` and `rows` are the size in cells, absent is 80 by 24. */
export type StartTerminalRun = { command: string; dir: string; cols?: number; rows?: number };

/** `type_in_terminal`: the Job's agent types into the Job's shell, opening it where it has none. `POST /sessions/terminal/type`. Answered 200 with `TerminalTyped`. `text` is written as it is; `enter` absent is true; `clear` absent is true, which writes Ctrl-U before `text`; `wait_seconds` absent is 10, at most 60; `session_id` is read only where the connection places no hosted Job. */
export type TypeInTerminal = { text: string; enter?: boolean; clear?: boolean; wait_seconds?: number; session_id?: string };

/** `type_in_terminal`'s answer: what the shell printed after the text was typed, as plain lines, the last 200 at most. `still_printing` is whether output was still arriving when the wait ran out. `echoed` is false where the text typed never appeared in what was printed. */
export type TerminalTyped = { run_id: string; printed: string; still_printing: boolean; echoed: boolean };

/** `read_terminal`: the Job's agent reads the Job's shell. `GET /sessions/terminal/read`, these as its query. `lines` absent is 60, at most 500; `until` is waited for in what the shell printed since the agent last typed, as text or, with `regex`, a regular expression; `wait_seconds` absent is 10, at most 60. */
export type ReadTerminal = { session_id?: string; lines?: number; until?: string; regex?: boolean; wait_seconds?: number };

/** `read_terminal`'s answer: the last lines of the Job's shell, as its screen drew them. `GET /sessions/terminal/read`. `matched` is whether `until` was printed before the wait ran out, absent where the read named none. */
export type TerminalScreen = { run_id: string; screen: string; matched?: boolean };

/** `terminal.opened` and `terminal.exited` on `/events`: a Job's shell opened or ended, so a pane can follow it. What it prints is `observe_session_run`'s, on `run_id`. */
export type JobTerminal = { session_id: string; run_id: string };

// The Job browser: `crates/ipc/src/job_browser.rs`, and `docs/practices/protocol/the-job-browser.md` for the contract.

/** `browser_open`: the agent loads an `http` or `https` page. `POST /sessions/browser/open`. Answered 200 with `BrowserAnswer` `page`. `session_id` is read only where the connection places no hosted Job. */
export type BrowserOpen = { url: string; session_id?: string };

/** `browser_click`: the agent presses `target`. `POST /sessions/browser/click`. Answered 200 with `BrowserAnswer` `page`. */
export type BrowserClick = { target: WalkTarget; session_id?: string };

/** `browser_type`: the agent types `text` into `target`, and Enter after it with `submit` (absent is false). `POST /sessions/browser/type`. Answered 200 with `BrowserAnswer` `page`. */
export type BrowserType = { target: WalkTarget; text: string; submit?: boolean; session_id?: string };

/** `browser_snapshot` and `browser_screenshot`: nothing but the Job. `POST /sessions/browser/snapshot` answers `page`; `POST /sessions/browser/screenshot` answers `screenshot`. */
export type BrowserLook = { session_id?: string };

/** `browser_wait_for`: exactly one of `text` and `target`. `timeout_ms` absent or 0 is 5000, at most 30000. `POST /sessions/browser/wait_for`. Answered 200 with `BrowserAnswer` `page`. */
export type BrowserWaitFor = { text?: string; target?: WalkTarget; timeout_ms?: number; session_id?: string };

/** `browser_evaluate`: an expression run in the page. `POST /sessions/browser/evaluate`. Answered 200 with `BrowserAnswer` `evaluated`. */
export type BrowserEvaluate = { expression: string; session_id?: string };

/**
 * One act Bridge is asked to do. `screenshot`'s `path` is the PNG to write, in the Job's scratch folder, which
 * Fleet made. `wait_for` carries one of `text` and `target`, and `timeout_ms` as Fleet held it.
 */
export type BrowserAct =
  | { kind: "open"; url: string }
  | { kind: "click"; target: WalkTarget }
  | { kind: "type"; target: WalkTarget; text: string; submit?: boolean }
  | { kind: "snapshot" }
  | { kind: "screenshot"; path: string }
  | { kind: "wait_for"; text?: string; target?: WalkTarget; timeout_ms: number }
  | { kind: "evaluate"; expression: string };

/** `browser.request` on `/events`: an agent's act, held by Fleet until `answer_browser_request` names `request_id`. Not replayed. */
export type BrowserRequest = { session_id: string; request_id: string; act: BrowserAct };

/**
 * What a done act answers. `page` after `open`, `click`, `type`, `snapshot` and `wait_for`, `snapshot` being the
 * page's accessibility tree as text; `screenshot` after `screenshot`, once the PNG is at the act's `path`; `evaluated`
 * after `evaluate`, `result` being `JSON.stringify`'s text of the value, absent where it has none. Bridge sends no
 * `path` and no `truncated`: Fleet sets them on the agent's answer.
 */
export type BrowserAnswer =
  | { kind: "page"; url: string; title: string; snapshot: string; truncated?: boolean }
  | { kind: "screenshot"; url: string; title: string; path?: string }
  | { kind: "evaluated"; url: string; title: string; result?: string; truncated?: boolean };

/** Why Bridge could not do an act. Fleet words each for the agent; `failed`'s `why` reaches it after "The Job browser could not do that:". */
export type BrowserError =
  | { kind: "owner_driving" }
  | { kind: "not_found" }
  | { kind: "timed_out" }
  | { kind: "failed"; why: string };

/** `answer_browser_request`: Bridge answers one `browser.request` with exactly one of `answer` and `error`. `POST /sessions/browser/answer`. */
export type AnswerBrowserRequest = { request_id: string; answer?: BrowserAnswer; error?: BrowserError };

/** `answer_browser_request`'s answer: the held call it answered. */
export type BrowserRequestAnswered = { request_id: string; session_id: string };

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
  /** `said` is the close of the last thing the agent said to the person: its last 1,500 characters, from a line start. Since 23.86. */
  | { kind: "turn_completed"; said?: string }
  /** A turn began. Since 23.86. */
  | { kind: "turn_started" }
  /** A turn ended on the plan's limit, which resets at `resets_at`. Fleet wakes the session then. */
  | { kind: "limited"; resets_at: string }
  /** Files written, relative to the session's `cwd`. Since 23.81. */
  | { kind: "touched"; paths: string[] }
  | { kind: "wrote" }
  /** The agent reached for a browser outside Bridge: `tool` as the person reads it, and the address it opened. Fleet writes a `browser_reminder` row. */
  | { kind: "browser_outside"; tool: string; address?: string }
  /** The agent's task list, whole, as its last task tool left it. An empty list clears it. */
  | { kind: "tasks"; tasks: ReportedTask[] }
  /** The agent left plan mode. */
  | { kind: "plan_left" }
  /** A `git commit` ran and its output showed a commit made. `branch` is absent on a detached head. */
  | { kind: "committed"; branch?: string }
  /** A `git push` ran and its output showed a ref updated. */
  | { kind: "pushed"; branch?: string }
  /** A turn answered: model, tokens and the tools it called. One row a message id. */
  | { kind: "turn_measured"; turn: TurnUsage }
  /** What the agent knows and was told, whole, replacing the last. */
  | { kind: "equipped"; loadout: LoadoutItems }
  /** A skill, tool or subagent used. Fleet keeps the last fifty turns. */
  | { kind: "used"; use: LoadoutUse }
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
  act: "start" | "chunk" | "end";
  cwd: string;
  name: string;
  /** What the run was narrowed to: the test filter, or `--changed ` and the packages reached. `end` may tell it when `start` could not. */
  target?: string;
  run?: number;
  outcome?: "passed" | "failed";
  took_ms?: number;
  /** All of it on `end`; what is new since the last one on `chunk`. */
  log?: string;
};

/**
 * `ask_for_slot` (`POST /sessions/slot_wanted`): a Session's `armada worktree lease` found every slot
 * held (`ask`, repeated about once a second as its heartbeat) or is done waiting (`done`). Bridge never
 * sends it; it is here so the wire reads whole. The call it raises is the Session's `slot_choice` item.
 */
export type AskForSlot = { act: "ask" | "done"; session_id: string };

/**
 * What it answers: `asking` the call is on the list, `waiting` he chose to wait, `added` a slot was
 * made and is held for the command for about thirty seconds, `unasked` Fleet does not host the Session.
 */
export type SlotAsked = { standing: "asking" | "waiting" | "added" | "unasked" };

/** How a person asked a running Check to stop: `SIGTERM` to its group, or `SIGKILL`. */
export type SessionCheckStop = "graceful" | "force";

/**
 * What it answers. `run` is absent where no Session holds the slot, which is a person's own run.
 * `stop` is a stop a person asked for, answered to `chunk` and to the two stop routes.
 */
export type SessionCheckAnswer = { run?: number; stop?: SessionCheckStop };

/**
 * One message on `GET /sessions/checks/:run/observe`, a Session run's log as it is
 * written. An ended run opens, sends what it holds and closes `finished`.
 */
export type SessionCheckOutputMessage =
  | ({ message: "opened" } & SessionCheckOutputOpened)
  | ({ message: "lines" } & { lines: string[] })
  | ({ message: "closed" } & { because: string });

export type SessionCheckOutputOpened = {
  protocol_id: ProtocolId;
  run: number;
  name: string;
  /** Older lines the first read left out, because the window is bounded. */
  skipped: number;
};
