// The Sessions Fleet hosts and the ledger of every agent session: reads, and the acts a person takes.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type {
  AnswerSessionAsk,
  Followed,
  FrameRead,
  Outcome,
  PilotOutcome,
  PullRequestState,
  RenameSession,
  ReviewDispatched,
  SendSessionMessage,
  SessionRecord,
  SessionRow,
  SessionSubagent,
  TuneSession,
} from "@armada/protocol";
import type { ArtifactRead, PageBounds } from "@armada/screens/src/draft/sessions";

/** What a session act came to: the thing Fleet answered with, or the refusal to word. */
export type SessionActed<T = SessionRecord> = { ok: true; value: T } | { ok: false; outcome: Outcome };

/** The ways out of a pilot, as Fleet names the acts: submit for verification, attest complete, close as superseded. */
export type PilotExit = "submit" | "attest" | "supersede";

/** What a person can do to a pull request a Session holds, without leaving Bridge. */
export type PullRequestPress = "read" | "ready" | "merge" | "auto_merge" | "review";

export type SessionsApi = {
  /**
   * A blank session on the repository this window picked. **Main reads the pick**, so the renderer
   * never names a repository: the one repository Fleet serves where the window is on All, and a
   * refusal in words where there are several. **`root` names the repository instead** of the pick: the
   * annotation layer starts on the notes' own, whatever the window has picked.
   */
  startSession: (title?: string, root?: string) => Promise<SessionActed>;
  /**
   * Takes a Job over and starts a Session on its worktree, in one call. **Main names the repository from
   * the Job**, so the renderer sends a Job and what is to happen to it. A refusal is Fleet's own, with
   * nothing changed.
   */
  pilotJob: (jobId: string, outcome: PilotOutcome) => Promise<SessionActed>;
  /**
   * Starts a new Session as a copy of an ended or dead one's conversation. **Main names the repository
   * from the Session**, so the renderer sends the Session and nothing else.
   */
  forkSession: (sessionId: string) => Promise<SessionActed>;
  /** One of the three ways out of a pilot. `note` is the person's words on an attestation or a supersede. */
  exitPilot: (jobId: string, exit: PilotExit, note?: string) => Promise<Outcome>;
  /** A message, with the pictures and files sent beside it and what it names with `@`. */
  sendSessionMessage: (send: SendSessionMessage) => Promise<SessionActed>;
  /** One of the offers on the ask the agent is held on. */
  answerSessionAsk: (answer: AnswerSessionAsk) => Promise<SessionActed>;
  /** The model, effort and permission mode the next turn runs on. */
  tuneSession: (tune: TuneSession) => Promise<SessionActed>;
  /** A name the person gave a session, hosted or in a terminal. It stands until the next `/rename` in a terminal. */
  renameSession: (rename: RenameSession) => Promise<SessionActed>;
  /** Ends the process, parks the slot and ends the row. */
  closeSession: (sessionId: string) => Promise<SessionActed>;
  /**
   * Hold a session's thread: read once, then followed by `session.row`. Held threads are kept for the
   * window's life, so reopening one draws at once.
   */
  watchSession: (sessionId: string) => Promise<void>;
  /** A picture or file a message carried, by the id its row gave. */
  readSessionFile: (sessionId: string, file: string) => Promise<FrameRead>;
  /** A subagent's own thread as its transcript stands. Read again while it runs. */
  readSessionSubagent: (sessionId: string, subagentId: string) => Promise<SessionActed<SessionSubagent>>;
  /** Opens a file a Session wrote, in whatever the machine opens it with. Main opens only a path the Session's own ledger names. */
  openSessionFile: (sessionId: string, path: string) => Promise<Followed>;
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
  /** An act on one of a Session's pull requests. `read` brings its state current, `review` answers the Code Review Job it dispatched. */
  pressPullRequest: (
    sessionId: string,
    number: number,
    press: PullRequestPress,
  ) => Promise<SessionActed<PullRequestState | ReviewDispatched>>;
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
};

export const SESSIONS_NOTHING_YET: SessionsState = {
  sessions: { state: "none" },
  sessionThreads: {},
};

export const SESSIONS_CHANNELS = {
  startSession: "bridge:start-session",
  pilotJob: "bridge:pilot-job",
  forkSession: "bridge:fork-session",
  exitPilot: "bridge:exit-pilot",
  sendSessionMessage: "bridge:send-session-message",
  answerSessionAsk: "bridge:answer-session-ask",
  tuneSession: "bridge:tune-session",
  renameSession: "bridge:rename-session",
  closeSession: "bridge:close-session",
  watchSession: "bridge:watch-session",
  readSessionFile: "bridge:read-session-file",
  readSessionSubagent: "bridge:read-session-subagent",
  openSessionFile: "bridge:open-session-file",
  readSessionArtifact: "bridge:read-session-artifact",
  showSessionPage: "bridge:show-session-page",
  moveSessionPage: "bridge:move-session-page",
  hideSessionPage: "bridge:hide-session-page",
  sessionPageEscape: "bridge:session-page-escape",
  pressPullRequest: "bridge:press-pull-request",
} as const;
