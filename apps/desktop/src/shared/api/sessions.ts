// The Sessions Fleet hosts and the ledger of every agent session: reads, and the acts a person takes.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { AdversarialReview, DismissFinding, ModSessionStarted, ProjectRowRef, SendFinding, StartAdversarialReview, StartModSession, DeleteWatch, ReadWatchLog, StopWatch, WatchList, WatchLog, WatchSummary } from "@armada/protocol";
import type {
  AnswerSessionAsk,
  AnswerWaiting,
  ApprovePlan,
  CheckoutCaughtUp,
  CheckoutDiff,
  ChangeWaiting,
  CheckoutSentAsPullRequest,
  ClaimIssue,
  LaterSettled,
  ConfirmLater,
  SettleLater,
  ClaimPullRequest,
  DismissConcern,
  ConfirmWaiting,
  DismissWaiting,
  Followed,
  FrameRead,
  Outcome,
  IssueClaimed,
  PullRequestClaimed,
  PullRequestFilePatch,
  PullRequestPage,
  PullRequestState,
  JudgeSession,
  RenameSession,
  SendConcern,
  SendPlanChanges,
  SendSessionMessage,
  SendSessionNote,
  NoteSent,
  SessionWait,
  StartAfter,
  WaitReading,
  ChosenHold,
  SessionHeld,
  SetNoticeMute,
  SessionHoldsReleased,
  SessionRecord,
  SessionRow,
  SessionRunMessage,
  SessionRunUnderway,
  JobTerminal,
  SessionSubagent,
  SessionLoadout,
  AttachmentHistory,
  McpServerAct,
  SessionMcp,
  SessionMcpActed,
  TuneSession,
  BeginSession,
  WritePullRequest,
} from "@armada/protocol";
import type { ArtifactRead, LoadoutItemAsk, LoadoutItemBody, PageBounds, PageShot } from "@armada/screens/src/draft/sessions";

/** What a session act came to: the thing Fleet answered with, or the refusal to word. */
export type SessionActed<T = SessionRecord> = { ok: true; value: T } | { ok: false; outcome: Outcome };

/** What a person can do to a pull request a Session holds, without leaving Bridge. */
/** A Job's shell opening or ending, as `terminal.opened` and `terminal.exited` carry it. */
export type JobTerminalChange = JobTerminal & { change: "opened" | "exited" };

/**
 * A shell moved from its own window back into Bridge, docked or in the side panel. `shell` is the Job's last shell main followed,
 * so one that ended in its window is shown ended rather than opened again; absent where main followed none.
 */
export type TerminalMovedBack = { sessionId: string; to: "docked" | "side panel"; shell?: { runId: string; ended: boolean } };

export type PullRequestPress = "read" | "ready" | "merge" | "auto_merge";

/** What the comment box writes to a pull request a Session holds: the act, and its Markdown. The pull request is named apart. */
export type PullRequestWrite = Pick<WritePullRequest, "kind" | "body">;

export type SessionsApi = {
  /**
   * A blank session on the repository this window picked. **Main reads the pick**, so the renderer
   * never names a repository: the one repository Fleet serves where the window is on All, and a
   * refusal in words where there are several. **`root` names the repository instead** of the pick: the
   * annotation layer starts on the notes' own, whatever the window has picked. **`request` is the words the person
   * dispatched**, which Fleet reads once to fill in the title, workflow, model and effort; the answer is not waited on.
   * **`after` makes the Session wait on another**: `request` is then its brief, kept, and nothing is sent until the condition is met.
   * **`held` starts it on a card before any agent moves**: the Session carries `hosted.start` until `beginSession`.
   * **`projectRows` are the rows of the repository's projects it starts with**: each written task among them is held by the Job.
   */
  startSession: (title?: string, root?: string, request?: string, tuning?: { model?: string; effort?: string }, after?: StartAfter, held?: boolean, projectRows?: readonly ProjectRowRef[]) => Promise<SessionActed>;
  /** Begins a held Session: the agent starts, and how the work lands is the choice sent, or the one `hosted.start.picked` names. Answers the record with `start` gone. */
  beginSession: (begin: BeginSession) => Promise<SessionActed>;
  /** Fleet's proposer reads a brief that tags `sessionId` for the wait it names. A reading it could not make is `ends`, from main. */
  readSessionWait: (sessionId: string, request: string) => Promise<SessionActed<WaitReading>>;
  /** Changes when a waiting Session starts, or where from. The Session it waits on is told, and both threads carry a row. */
  changeWaiting: (change: ChangeWaiting) => Promise<SessionActed<SessionWait>>;
  /** Starts a waiting Session now, whatever its condition. */
  startWaitingNow: (sessionId: string) => Promise<SessionActed>;
  /**
   * Starts a new Session as a copy of an ended or dead one's conversation. **Main names the repository
   * from the Session**, so the renderer sends the Session and nothing else.
   */
  forkSession: (sessionId: string) => Promise<SessionActed>;
  /**
   * Starts a new Session as a copy of a Session's conversation up to and including the thread row `at`,
   * live or not; the Session it came from goes on untouched. Like `forkSession`, only the ids are sent:
   * the repository is the Session's own, read in main (23.79).
   */
  forkSessionAt: (sessionId: string, at: string) => Promise<SessionActed>;
  /** A message, with the pictures and files sent beside it and what it names with `@`. */
  sendSessionMessage: (send: SendSessionMessage) => Promise<SessionActed>;
  /** A note pinned on a page, sent whole. Fleet answers with the turn taken and the lines of code the spot was drawn from. */
  sendSessionNote: (send: SendSessionNote) => Promise<SessionActed<NoteSent>>;
  /** A Mod Job on what was pointed at. Fleet sends it the Mod brief and the note itself, and answers with the Job's id, not its record: the record arrives on Fleet's events. */
  startModSession: (start: StartModSession) => Promise<SessionActed<ModSessionStarted>>;
  /** One of the offers on the ask the agent is held on. */
  answerSessionAsk: (answer: AnswerSessionAsk) => Promise<SessionActed>;
  /** Settles one thing a session waits on: a numbered choice, the person's words, or a mode that leaves it to the agent. */
  answerWaiting: (answer: AnswerWaiting) => Promise<SessionActed>;
  /** Takes an open pull request nobody holds for a Session. Fleet refuses, naming the holder, where a live one has it. */
  claimPullRequest: (claim: ClaimPullRequest) => Promise<SessionActed<PullRequestClaimed>>;
  /** Holds an issue, by the forge's number or a tracker's key, on a Session's ledger. Fleet refuses a reference that is neither. */
  claimIssue: (claim: ClaimIssue) => Promise<SessionActed<IssueClaimed>>;
  /** Settles a follow-up a Session kept. Bridge's Drop sends `dropped` with no reason; Fleet takes the item off the Session's `later`. */
  settleLater: (settle: SettleLater) => Promise<SessionActed<LaterSettled>>;
  /** The person's Stop on one watch of a Session. Fleet ends it `stopped` and answers it. */
  stopWatch: (stop: StopWatch) => Promise<SessionActed<WatchSummary>>;
  /** The person's delete on one ended watch of a Session. Fleet forgets it and answers the Job's watches after. */
  deleteWatch: (remove: DeleteWatch) => Promise<SessionActed<WatchList>>;
  /** A failed reading's log, read whole. */
  readWatchLog: (read: ReadWatchLog) => Promise<SessionActed<WatchLog>>;
  /** Drops one thing a session waits on for good. Nothing is sent to the agent. */
  dismissWaiting: (dismiss: DismissWaiting) => Promise<SessionActed>;
  /** Thumbs up on a follow-up Fleet read from a reply: the row stays open and reads `confirmed`. Fleet refuses an id that names no open follow-up. */
  confirmLater: (confirm: ConfirmLater) => Promise<SessionActed>;
  /** Thumbs up on a question Fleet read from a reply: the item stays and reads `confirmed`. Fleet refuses an item of any other source. */
  confirmWaiting: (confirm: ConfirmWaiting) => Promise<SessionActed>;
  /** Approves the plan a Session recorded, the version he read. Fleet tells the agent. A newer version refuses. */
  approvePlan: (approve: ApprovePlan) => Promise<SessionActed>;
  /** Sends the marks he made on a plan to the agent as one message. The plan stays until the agent records it again. */
  sendPlanChanges: (changes: SendPlanChanges) => Promise<SessionActed>;
  /** Runs a judge now on the work so far against the plan. Answers at once with `judging` set; `judge` lands on the record. */
  judgeSession: (judge: JudgeSession) => Promise<SessionActed>;
  /** Sends a judge's concern (`judge:<n>`) to the agent as his message, and settles it. */
  sendConcern: (send: SendConcern) => Promise<SessionActed>;
  /** Settles a judge's concern. The agent is told nothing. */
  dismissConcern: (dismiss: DismissConcern) => Promise<SessionActed>;
  /** Starts an adversarial review of a plan, or answers the one kept for it where `again` is not set. Fleet answers the review, not the record: it lands on `plan.adversarial` by `session.changed`. */
  startAdversarialReview: (start: StartAdversarialReview) => Promise<SessionActed<AdversarialReview>>;
  /** Sends a finding to the agent as his message, and settles it. Answers the review. */
  sendFinding: (send: SendFinding) => Promise<SessionActed<AdversarialReview>>;
  /** Settles a finding. The agent is told nothing. Answers the review. */
  dismissFinding: (dismiss: DismissFinding) => Promise<SessionActed<AdversarialReview>>;
  /** The model, effort and permission mode the next turn runs on. */
  tuneSession: (tune: TuneSession) => Promise<SessionActed>;
  /** A slash command written in a terminal Session's thread, run in its terminal as typed. Nothing comes back. */
  runSessionCommand: (sessionId: string, command: string) => Promise<SessionActed>;
  /** A name the person gave a session, hosted or in a terminal. It stands until the next `/rename` in a terminal. */
  renameSession: (rename: RenameSession) => Promise<SessionActed>;
  /**
   * Writes a retro of what happened in a Session since its last one, once Fleet has asked a live
   * agent what got in its way. Answers when it is written, which can take a minute.
   */
  retroSession: (sessionId: string) => Promise<Outcome>;
  /** Ends the process, parks the slot and ends the row. */
  closeSession: (sessionId: string) => Promise<SessionActed>;
  /** Ends the turn that is running. The Session rests, its row stays live, and its next message resumes it. */
  stopSession: (sessionId: string) => Promise<SessionActed>;
  /** Runs again the hand off that failed on this Job. Refused where none stands or one runs; a hand off that fails again is a fresh failed row on the Job. */
  retryHandoff: (sessionId: string) => Promise<SessionActed>;
  /** Stops the turn, commits the work to the Session's branch and gives its slot back. The row stays, marked paused. */
  pauseSession: (sessionId: string) => Promise<SessionActed>;
  /** Auto land on or off for a hosted Session. Fleet keeps the flag first, so a forge's refusal leaves it set and comes back as a refusal. */
  setSessionAutoLand: (sessionId: string, on: boolean) => Promise<SessionActed>;
  /** Takes a slot again at the branch tip, or waits for one. A message to a paused Session does the same. */
  resumeSession: (sessionId: string) => Promise<SessionActed>;
  /**
   * Sets the workflow a Session follows, or clears it with no `workflow` so it is a custom Session. Fleet tells the
   * agent at once. Answers the Session's record.
   */
  setSessionWorkflow: (sessionId: string, workflow?: string) => Promise<SessionActed>;
  /**
   * Gives back the slots, branches, pull requests, Jobs and needs the Session holds, or hands them to
   * `toSessionId`, a live Session of the same repository: `rows`, or every one where absent. A turn
   * running is stopped first where a slot is among them. Answers what moved and what stayed, with why.
   */
  releaseSessionHolds: (sessionId: string, toSessionId?: string, rows?: ChosenHold[]) => Promise<SessionActed<SessionHoldsReleased>>;
  /**
   * Hands one group of the Session's plan to a linked Session of its own: Fleet starts it in the same repository, titled with the
   * group's, and sends the group's tasks and the Session's handoff as its first message. Answers the new Session's id.
   */
  handToSession: (sessionId: string, group: string) => Promise<SessionActed<SessionHeld>>;
  /**
   * Moves a Session started in the wrong repository: Fleet ends it and starts its whole conversation again in
   * `manifestId`. Answers the new Session's record; the old one's slot, branch and files stay where they were.
   */
  moveSession: (sessionId: string, manifestId: string) => Promise<SessionActed>;
  /** Turns a Fleet notice off for a Job, a repository or an hour, or back on. Answers what Fleet kept. */
  muteNotice: (mute: SetNoticeMute) => Promise<SessionActed<SetNoticeMute>>;
  /**
   * Hold a session's thread: read once, then followed by `session.row`. Held threads are kept for the
   * window's life, so reopening one draws at once.
   */
  watchSession: (sessionId: string) => Promise<void>;
  /** A picture or file a message carried, by the id its row gave. */
  readSessionFile: (sessionId: string, file: string) => Promise<FrameRead>;
  /** A subagent's own thread as its transcript stands. Read again while it runs. */
  readSessionSubagent: (sessionId: string, subagentId: string) => Promise<SessionActed<SessionSubagent>>;
  /** What a Session's agent knows, was told and used each turn: `get_session_loadout`. Read again on a `session.changed`. */
  readSessionLoadout: (sessionId: string) => Promise<SessionActed<SessionLoadout>>;
  /** Every version of everything a Session holds, oldest first: `get_attachment_history`. Read again on a `session.changed`. */
  readAttachmentHistory: (sessionId: string) => Promise<SessionActed<AttachmentHistory>>;
  /** What one item of a Session's Loadout says, read from disk by Fleet: `read_loadout_item`. */
  readLoadoutItem: (sessionId: string, item: LoadoutItemAsk) => Promise<SessionActed<LoadoutItemBody>>;
  /** A hosted Session's MCP servers, asked of its process now: `get_session_mcp`. No event says to read again. */
  readSessionMcp: (sessionId: string) => Promise<SessionActed<SessionMcp>>;
  /**
   * Reconnects, turns on or off, or signs in to one server, and answers the servers after: `act_on_session_mcp`.
   * Waits for the server to settle, up to a minute and a half. A `sign_in` in the answer is a page for `openLink`.
   */
  actOnSessionMcp: (sessionId: string, server: string, act: McpServerAct) => Promise<SessionActed<SessionMcpActed>>;
  /** Opens a file a Session wrote, in whatever the machine opens it with. Main opens only a path the Session's own ledger names. */
  openSessionFile: (sessionId: string, path: string) => Promise<Followed>;
  /** Opens, or raises, the window on a page a Session showed. Main opens only an address the Session's own ledger shows as a window. */
  openSessionWindow: (sessionId: string, url: string) => Promise<Outcome>;
  /** The addresses of the Session windows open now, which main alone knows. */
  getSessionWindows: () => Promise<readonly string[]>;
  /** The addresses open, whole, whenever a Session window opens or closes. */
  onSessionWindows: (onChanged: (addresses: readonly string[]) => void) => () => void;
  /** A file the Session's ledger names, read for the panel: its bytes and kind, or why it was not read. */
  readSessionArtifact: (sessionId: string, path: string) => Promise<ArtifactRead>;
  /** Shows a page or doc the Session's ledger names in a web view main owns, over `bounds` of this window. */
  showSessionPage: (sessionId: string, address: string, bounds: PageBounds) => Promise<Followed>;
  /** Moves the page's view to where the panel's body now is. */
  moveSessionPage: (bounds: PageBounds) => Promise<void>;
  /** Removes the page's view. */
  hideSessionPage: () => Promise<void>;
  /** Esc pressed while the page's view has focus, which the renderer never hears itself. Returns its remover. */
  onSessionPageEscape: (on: () => void) => () => void;
  /** Starts pictures of a page the Session's ledger names, taken in a view main holds out of sight and sent to `onSessionSnapshot`: the live view's page, drawn small. One at a time per window. */
  watchSessionSnapshot: (sessionId: string, address: string) => Promise<Followed>;
  /** Ends the pictures and closes the view they were taken in. */
  stopSessionSnapshot: () => Promise<void>;
  /** Each picture of the page being watched, as PNG bytes, when it first reads so and each time it changes. Returns its remover. */
  onSessionSnapshot: (on: (shot: PageShot) => void) => () => void;
  /** An act on one of a Session's pull requests. `read` brings its state current, `review` answers the Code Review Job it dispatched. */
  pressPullRequest: (
    sessionId: string,
    number: number,
    press: PullRequestPress,
  ) => Promise<SessionActed<PullRequestState>>;
  /**
   * The same acts on a pull request no Session holds, such as a form edit's: the repository is named by its Manifest, and
   * `review` dispatches its Code Review Job under no Session.
   */
  pressRepositoryPullRequest: (
    manifestId: string,
    number: number,
    press: PullRequestPress,
  ) => Promise<SessionActed<PullRequestState>>;
  /** What the panel's tabs and facts draw, read from the forge when the panel opens. Main names the repository from the Session. */
  readPullRequestPage: (sessionId: string, number: number) => Promise<SessionActed<PullRequestPage>>;
  /** One file of a pull request, read when a person opens it from the Files tab. Main names the repository from the Session. */
  readPullRequestFile: (sessionId: string, number: number, path: string) => Promise<SessionActed<PullRequestFilePatch>>;
  /** A comment, an approval or a request for changes on a pull request a Session holds. Answers the page read again after it. */
  writePullRequest: (sessionId: string, number: number, write: PullRequestWrite) => Promise<SessionActed<PullRequestPage>>;
  /** Runs a shell block from a Session's thread, on a terminal of `cols` by `rows` cells (both or neither; 80 by 24 without). Answers with the run's id, or Fleet's refusal; its output arrives on `onSessionRunMessage`. */
  startSessionRun: (sessionId: string, command: string, cols?: number, rows?: number) => Promise<SessionActed<SessionRunUnderway>>;
  /** Opens the Job's shell, or is handed the one already open, on a terminal of `cols` by `rows` cells (both or neither; 80 by 24 without, used only where it opens now). Answers with its run id: what it prints arrives on `onSessionRunMessage`, and typing, resizing and stopping it are the run calls on that id. */
  openTerminal: (sessionId: string, cols?: number, rows?: number) => Promise<SessionActed<SessionRunUnderway>>;
  /** Runs `command` in the directory `dir` with no Job behind it, on a terminal of `cols` by `rows` cells (both or neither; 80 by 24 without). Answers with the run's id, or Fleet's refusal; its output arrives on `onSessionRunMessage`, and typing, resizing and stopping it are the run calls on that id. */
  startTerminalRun: (command: string, dir: string, cols?: number, rows?: number) => Promise<SessionActed<SessionRunUnderway>>;
  /** Ends a run's process group. */
  stopSessionRun: (runId: string) => Promise<SessionActed<SessionRunUnderway>>;
  /** Types into a run's terminal: the keys as a string, written as they are. Refused where the run has ended. */
  writeSessionRun: (runId: string, data: string) => Promise<SessionActed<SessionRunUnderway>>;
  /** Tells a run's terminal its size in cells. */
  resizeSessionRun: (runId: string, cols: number, rows: number) => Promise<SessionActed<SessionRunUnderway>>;
  /**
   * Pulls a held checkout level with its base: the one press "Your checkout" offers. **Names the repository by
   * its Manifest and no command**; Fleet reads the hold again, runs the pull it knows and answers with git's exit
   * code and words, a refused or conflicted pull among them. A refusal in words is Fleet's own.
   */
  catchUpCheckout: (manifestId: string) => Promise<SessionActed<CheckoutCaughtUp>>;
  /**
   * Sends a held checkout's uncommitted files up as a pull request with auto-merge on, without touching the
   * checkout. **Names the repository by its Manifest and no path**; Fleet reads the hold again, refuses what
   * the pull would refuse and answers with the pull request's number. The hold then carries it.
   */
  sendCheckoutAsPullRequest: (manifestId: string) => Promise<SessionActed<CheckoutSentAsPullRequest>>;
  /**
   * One uncommitted file of a held checkout, as a unified diff against HEAD. Names the repository by its Manifest
   * and the path; Fleet refuses a path the hold does not list.
   */
  readCheckoutDiff: (manifestId: string, path: string) => Promise<SessionActed<CheckoutDiff>>;
  /** What every run this window started prints, by run id, ending with `exited`. Returns its remover. */
  onSessionRunMessage: (on: (runId: string, message: SessionRunMessage) => void) => () => void;
  /** A Job's shell opened (the agent's first `type_in_terminal` too) or ended, so a pane can follow it. Output follows on `onSessionRunMessage` by `run_id`. Returns its remover. */
  onSessionTerminal: (on: (terminal: JobTerminalChange) => void) => () => void;
  /** Opens the Job's shell in a window of its own, or brings the one it has to the front. The window opens the shell as a pane does. */
  openTerminalWindow: (sessionId: string) => Promise<void>;
  /** The Jobs whose shell has a window open now, which main alone knows. */
  getTerminalWindows: () => Promise<readonly string[]>;
  /** The Jobs with a terminal window, whole, whenever one opens or closes. Returns its remover. */
  onTerminalWindows: (onChanged: (sessionIds: readonly string[]) => void) => () => void;
  /** What a shell main follows has printed so far, its last 256 KiB, for a window that comes to it late. Null where main holds none. */
  readTerminalSoFar: (runId: string) => Promise<string | null>;
  /** Moves a shell out of its window, which closes, and back into Bridge: every window hears it on `onTerminalMovedBack`. */
  moveTerminalBack: (sessionId: string, to: TerminalMovedBack["to"]) => Promise<void>;
  /** A shell moved back from its window, for the document that draws that Job's dock. Returns its remover. */
  onTerminalMovedBack: (on: (move: TerminalMovedBack) => void) => () => void;
};

/** Every session on the machine, as `list_sessions` answers and `session.changed` keeps current. */
export type SessionsRead =
  | { state: "none" }
  | { state: "read"; sessions: SessionRecord[] }
  | { state: "failed"; outcome: Outcome };

export type SessionsState = {
  /** `none` until Fleet has answered, and on a Fleet that does not serve Sessions: the surface is then left off. */
  sessions: SessionsRead;
  /** The threads a window has opened, oldest row first, by session id. */
  sessionThreads: Record<string, SessionRow[]>;
  /** Each Job's watches, live and ended, the newest first, by session id: `list_watches` on opening it, then `watches.changed`. */
  sessionWatches: Record<string, WatchSummary[]>;
  /** The ids whose read from Fleet has not answered, a thread held or cached being read again among them. */
  sessionThreadsReading: string[];
};

export const SESSIONS_NOTHING_YET: SessionsState = {
  sessions: { state: "none" },
  sessionThreads: {},
  sessionWatches: {},
  sessionThreadsReading: [],
};

export const SESSIONS_CHANNELS = {
  startSession: "bridge:start-session",
  beginSession: "bridge:begin-session",
  forkSession: "bridge:fork-session",
  forkSessionAt: "bridge:fork-session-at",
  sendSessionMessage: "bridge:send-session-message",
  sendSessionNote: "bridge:send-session-note",
  startModSession: "bridge:start-mod-session",
  answerSessionAsk: "bridge:answer-session-ask",
  answerWaiting: "bridge:answer-waiting",
  claimPullRequest: "bridge:claim-pull-request",
  claimIssue: "bridge:claim-issue",
  settleLater: "bridge:settle-later",
  stopWatch: "bridge:stop-watch",
  deleteWatch: "bridge:delete-watch",
  readWatchLog: "bridge:read-watch-log",
  readSessionWait: "bridge:read-session-wait",
  changeWaiting: "bridge:change-waiting",
  startWaitingNow: "bridge:start-waiting-now",
  dismissWaiting: "bridge:dismiss-waiting",
  confirmWaiting: "bridge:confirm-waiting",
  confirmLater: "bridge:confirm-later",
  approvePlan: "bridge:approve-plan",
  sendPlanChanges: "bridge:send-plan-changes",
  judgeSession: "bridge:judge-session",
  sendConcern: "bridge:send-concern",
  dismissConcern: "bridge:dismiss-concern",
  startAdversarialReview: "bridge:start-adversarial-review",
  sendFinding: "bridge:send-finding",
  dismissFinding: "bridge:dismiss-finding",
  tuneSession: "bridge:tune-session",
  runSessionCommand: "bridge:run-session-command",
  renameSession: "bridge:rename-session",
  retroSession: "bridge:retro-session",
  closeSession: "bridge:close-session",
  stopSession: "bridge:stop-session",
  retryHandoff: "bridge:retry-handoff",
  pauseSession: "bridge:pause-session",
  resumeSession: "bridge:resume-session",
  setSessionAutoLand: "bridge:set-session-auto-land",
  setSessionWorkflow: "bridge:set-session-workflow",
  releaseSessionHolds: "bridge:release-session-holds",
  handToSession: "bridge:hand-to-session",
  moveSession: "bridge:move-session",
  muteNotice: "bridge:mute-notice",
  watchSession: "bridge:watch-session",
  readSessionFile: "bridge:read-session-file",
  readSessionSubagent: "bridge:read-session-subagent",
  readSessionLoadout: "bridge:read-session-loadout",
  readAttachmentHistory: "bridge:read-attachment-history",
  readLoadoutItem: "bridge:read-loadout-item",
  readSessionMcp: "bridge:read-session-mcp",
  actOnSessionMcp: "bridge:act-on-session-mcp",
  openSessionFile: "bridge:open-session-file",
  openSessionWindow: "bridge:open-session-window",
  getSessionWindows: "bridge:get-session-windows",
  /** Main to every window, not a request. */
  sessionWindowsChanged: "bridge:session-windows-changed",
  readSessionArtifact: "bridge:read-session-artifact",
  showSessionPage: "bridge:show-session-page",
  moveSessionPage: "bridge:move-session-page",
  hideSessionPage: "bridge:hide-session-page",
  sessionPageEscape: "bridge:session-page-escape",
  watchSessionSnapshot: "bridge:watch-session-snapshot",
  stopSessionSnapshot: "bridge:stop-session-snapshot",
  sessionSnapshot: "bridge:session-snapshot",
  pressPullRequest: "bridge:press-pull-request",
  pressRepositoryPullRequest: "bridge:press-repository-pull-request",
  readPullRequestPage: "bridge:read-pull-request-page",
  readPullRequestFile: "bridge:read-pull-request-file",
  writePullRequest: "bridge:write-pull-request",
  startSessionRun: "bridge:start-session-run",
  openTerminal: "bridge:open-terminal",
  startTerminalRun: "bridge:start-terminal-run",
  stopSessionRun: "bridge:stop-session-run",
  writeSessionRun: "bridge:write-session-run",
  resizeSessionRun: "bridge:resize-session-run",
  sessionRunMessage: "bridge:session-run-message",
  sessionTerminal: "bridge:session-terminal",
  openTerminalWindow: "bridge:open-terminal-window",
  getTerminalWindows: "bridge:get-terminal-windows",
  /** Main to every window, not a request. */
  terminalWindowsChanged: "bridge:terminal-windows-changed",
  readTerminalSoFar: "bridge:read-terminal-so-far",
  moveTerminalBack: "bridge:move-terminal-back",
  /** Main to every window, not a request. */
  terminalMovedBack: "bridge:terminal-moved-back",
  catchUpCheckout: "bridge:catch-up-checkout",
  sendCheckoutAsPullRequest: "bridge:send-checkout-as-pull-request",
  readCheckoutDiff: "bridge:read-checkout-diff",
} as const;
