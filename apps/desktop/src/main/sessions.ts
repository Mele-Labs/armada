// Every session on the machine, and the threads of the ones a window opened.
//
// **Kept current by the stream, not by polling**: `session.changed` carries a session whole after any
// fact about it and `session.row` one row of a thread, so both fold the one way, and an act's own
// answer is the same record. Held beside `studios.ts` for its reason, and unlike it nothing here is
// scoped to a surface: the ledger answers who owns a branch or a pull request for a chip anywhere in
// the window, so the list is read on every connection and never dropped.
//
// **Threads are held once opened, the twenty opened last.** Reopening one draws it at once from here,
// and the stream has kept it current since; only the one opened longest ago is let go. A row that arrives before the read of its
// thread has answered is kept and merged after it, by id, so the gap between asking and hearing loses
// none; a row for a thread nobody opened is dropped, since the read that opens it brings every row.
//
// **Held threads are also kept on disk**, so a Job opened after a restart draws from the last copy while Fleet
// is read. The copy is never the truth: Fleet's answer replaces it, and a failed read leaves it standing.

import { mkdir, readdir, readFile, stat, unlink, writeFile } from "node:fs/promises";
import { join } from "node:path";

import WebSocket from "ws";

import type {
  ReleaseSessionHolds,
  SessionHeld,
  SessionHoldsReleased,
  AnswerSessionAsk,
  AnswerWaiting,
  CatchUpCheckout,
  CheckoutCaughtUp,
  CheckoutDiff,
  CheckoutSentAsPullRequest,
  CheckoutToPullRequest,
  ApprovePlan,
  ChangeWaiting,
  ClaimPullRequest,
  DismissConcern,
  DismissWaiting,
  Followed,
  FrameRead,
  HandToSession,
  Outcome,
  PullRequestClaimed,
  PullRequestFilePatch,
  PullRequestPage,
  PullRequestState,
  ReadSessionWait,
  JudgeSession,
  RenameSession,
  RunSessionCommand,
 
  SendConcern,
  SendPlanChanges,
  SendSessionMessage,
  SendSessionNote,
  NoteSent,
  SendSessionNotes,
  NotesSent,
  SessionList,
  SessionRecord,
  SessionRow,
  SessionRowChanged,
  SessionRunMessage,
  SessionRunUnderway,
  SessionSubagent,
  SessionLoadout,
  SessionThread,
  SessionWait,
  StartSession,
  StartWaitingNow,
  ResizeSessionRun,
  StartSessionRun,
  StopSessionRun,
  TuneSession,
  WaitReading,
  WindowClosed,
  WalkTold,
  WindowFailed,
  WalkStep,
  WindowSignIn,
  WindowTake,
  WriteSessionRun,
} from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import type { ArtifactRead, LoadoutItemAsk, LoadoutItemBody } from "@armada/screens/src/draft/sessions";
import { withThreadRow } from "@armada/screens/src/retro-row";
import type { PullRequestPress, PullRequestWrite, SessionActed } from "../shared/api/sessions";
import { openSessionFile, originsOfWindow, readSessionArtifact, readSteps, stepsOfWindow, takesOfWindow, titleOfWindow } from "./session-file";
import { ask, MODEL_CALL_MS, sessionFileOf } from "./request";
import { HOST } from "./runtime-file";

type Publish = (change: Partial<BridgeState>) => void;

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** A Bridge offline a few minutes still opens the page shown meanwhile, and a walk from before a restart stays closed. */
export const REOPEN_WITHIN_MS = 15 * 60 * 1000;

/** Where each press on a pull request is served. `crates/ipc/operations/` is the authority. */
const PRESS_ROUTE: Readonly<Record<"ready" | "merge" | "auto_merge", string>> = {
  ready: "ready",
  merge: "merge",
  auto_merge: "auto_merge",
};

/**
 * A row replaced where its id is held, appended where it is not. **A retro row is replaced only by
 * a later frame of itself**, since Fleet replaces it many times as it is written.
 */
export const withRow = withThreadRow<SessionRow>;

/** A session replaced where its id is held, put first where it is not: the list is most recently seen first. */
export function withSession(sessions: readonly SessionRecord[], record: SessionRecord): SessionRecord[] {
  return sessions.some((one) => one.id === record.id)
    ? sessions.map((one) => (one.id === record.id ? record : one))
    : [record, ...sessions];
}

/** How many opened threads are held. Each is every row of a conversation, which runs to hundreds. */
export const THREADS_HELD = 20;

/** How long after its first change a thread waits to be written, so a busy one is written once. */
const WRITE_AFTER_MS = 2000;

export class SessionsHost {
  private readonly publish: Publish;
  private readonly port: () => number | null;
  private sessions: SessionRecord[] | null = null;
  private threads: Record<string, SessionRow[]> = {};
  /** The ids of the threads held, least recently opened first. */
  private opening: string[] = [];
  /** Puts a page a Session showed in the walk window. Set once by main, which owns windows. `pressed`: the owner asked for it. */
  private shown: (
    sessionId: string,
    title: string,
    url: string,
    takes: readonly WindowTake[],
    origins: readonly string[],
    signIn: readonly string[],
    steps: readonly WalkStep[],
    pressed: boolean,
  ) => void = () => {};
  /** Shows a question asked in a Session's walk, by its waiting item. Set once by main, as `shown` is. */
  private askedIn: (sessionId: string, title: string, window: string, place: string, itemId: string) => void = () => {};
  /** The window rows already acted on, so a row met again opens nothing. */
  private readonly opened = new Set<string>();
  /** A window row waits on Fleet for its sign-in, so a window and a question about it are acted on in the order their rows came. */
  private turn: Promise<void> = Promise.resolve();
  /** Where a run's messages go: main's, set once, since windows are not a thing this host holds. */
  private ran: (runId: string, message: SessionRunMessage) => void = () => {};
  /** Told every list after it changes, with when the first read answered, or `null` before it has. Set once by main. */
  private listed: (sessions: readonly SessionRecord[], readAt: number | null) => void = () => {};
  private readAt: number | null = null;
  /** The socket each running block is read on, by run id. */
  private readonly runs = new Map<string, WebSocket>();
  /** Threads whose read has not answered, with the rows that arrived meanwhile. */
  private reading = new Map<string, SessionRow[]>();
  /** Where held threads are kept, one file each; none where nothing is kept. */
  private dir: string | null = null;
  /** The threads changed since they were last written. */
  private readonly dirty = new Set<string>();
  private writing: NodeJS.Timeout | undefined;

  private readonly now: () => number;

  constructor(publish: Publish, port: () => number | null, now: () => number = Date.now) {
    this.publish = publish;
    this.port = port;
    this.now = now;
  }

  /** Keep held threads in this directory. Set once by main, which knows `userData`. */
  keepIn(dir: string): void {
    this.dir = dir;
  }

  /** Read every session, and every thread a window holds again. Once per connection, and on a refresh. */
  async again(port: number): Promise<void> {
    const answer = await ask(port, "GET", "/sessions");
    if (answer.ok !== true) {
      // **A list already held stays when the next read fails**, as the pool's does (`holding.ts`): a read that times out
      // on a busy machine must not empty every Session out of the window and put them all back a moment later.
      if (this.sessions !== null) return;
      this.publish({ sessions: { state: "failed", outcome: answer.outcome } });
      return;
    }
    this.sessions = (answer.body as SessionList).sessions;
    this.readAt ??= this.now();
    this.publish({ sessions: { state: "read", sessions: this.sessions } });
    this.listed(this.sessions, this.readAt);
    // Before the threads are read, since a thread read marks its window rows as met.
    await this.reopen(port);
    // Every thread opened, not only those held: one opened before the port was known was touched and read nothing.
    await Promise.all(this.opening.filter((id) => !this.reading.has(id)).map((id) => this.read(port, id)));
  }

  /**
   * Puts in the walk window each live Session's newest shown page that never opened here, because Bridge was
   * not connected when it was shown. **One that has ended, that this process already opened, or that was
   * shown more than `REOPEN_WITHIN_MS` ago is left**. Fleet holds the Session's sign-in for one that
   * opens, and it is taken now. **There is one walk window**: the first opens it, and every walk standing
   * on a Job's waiting items joins it there, these among them.
   */
  private async reopen(port: number): Promise<void> {
    const showing = (this.sessions ?? []).filter(
      (one) => one.state === "live" && one.attachments.some((held) => held.kind === "artifact" && held.detail?.["form"] === "window"),
    );
    await Promise.all(
      showing.map(async (record) => {
        const answer = await ask(port, "GET", `/sessions/one?session_id=${encodeURIComponent(record.id)}`);
        if (answer.ok !== true) return;
        const row = (answer.body as SessionThread).rows.findLast((one) => one.kind === "window");
        if (row === undefined || row.kind !== "window" || row.ended_at !== undefined || this.opened.has(row.id)) return;
        if (!(this.now() - Date.parse(row.at) <= REOPEN_WITHIN_MS)) return;
        this.opened.add(row.id);
        this.showWindow(record.id, row);
      }),
    );
  }

  /** Opens the window a `window` row names, with the sign-in Fleet holds for it, in the order the rows came. */
  private showWindow(sessionId: string, row: Extract<SessionRow, { kind: "window" }>): void {
    const { title, url, variations, steps, origins } = row;
    this.inOrder(async () => {
      const signIn = await this.signInFor(sessionId, url);
      this.shown(sessionId, title, url, variations ?? [], origins ?? [], signIn, readSteps(steps), false);
    });
  }

  /** `session.changed`: the session, whole. */
  changed(record: SessionRecord): void {
    this.fold(record);
  }

  /** Say who is told each Session list after it changes: main's, as `onWindow` is. */
  onRead(listed: (sessions: readonly SessionRecord[], readAt: number | null) => void): void {
    this.listed = listed;
  }

  /** Say where a run's messages go: main's, as `onWindow` is. */
  onRun(ran: (runId: string, message: SessionRunMessage) => void): void {
    this.ran = ran;
  }

  /**
   * A shell block run from a Session's thread. Fleet starts it and answers 202 with a run id; its
   * output is then read on a socket of its own, opened here so no line is missed between the answer
   * and the renderer hearing the id. Nothing is folded: a run is not part of the Session's record.
   */
  async startRun(start: StartSessionRun): Promise<SessionActed<SessionRunUnderway>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/sessions/run/start", start);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    const underway = answer.body as SessionRunUnderway;
    this.observeRun(port, underway.run_id);
    return { ok: true, value: underway };
  }

  /** The one pull a held checkout is offered. Fleet answers 200 with the code and words, even for a pull git refused. */
  async catchUpCheckout(catchUp: CatchUpCheckout): Promise<SessionActed<CheckoutCaughtUp>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/merge_lines/catch_up", catchUp);
    return answer.ok === true ? { ok: true, value: answer.body as CheckoutCaughtUp } : { ok: false, outcome: answer.outcome };
  }

  /** A held checkout's uncommitted files as a pull request, committed by Fleet without touching the checkout. */
  async sendCheckoutAsPullRequest(send: CheckoutToPullRequest): Promise<SessionActed<CheckoutSentAsPullRequest>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/merge_lines/checkout_to_pull_request", send);
    return answer.ok === true ? { ok: true, value: answer.body as CheckoutSentAsPullRequest } : { ok: false, outcome: answer.outcome };
  }

  /** One uncommitted file of a held checkout, read by Fleet from the served checkout. */
  async checkoutDiff(manifestId: string, path: string): Promise<SessionActed<CheckoutDiff>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", `/merge_lines/checkout_diff?manifest_id=${encodeURIComponent(manifestId)}&path=${encodeURIComponent(path)}`);
    return answer.ok === true ? { ok: true, value: answer.body as CheckoutDiff } : { ok: false, outcome: answer.outcome };
  }

  async stopRun(stop: StopSessionRun): Promise<SessionActed<SessionRunUnderway>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/sessions/run/stop", stop);
    return answer.ok === true ? { ok: true, value: answer.body as SessionRunUnderway } : { ok: false, outcome: answer.outcome };
  }

  /** Keys the person typed into a run's terminal. */
  async writeRun(write: WriteSessionRun): Promise<SessionActed<SessionRunUnderway>> {
    return await this.actOnRun("/sessions/run/write", write);
  }

  async resizeRun(resize: ResizeSessionRun): Promise<SessionActed<SessionRunUnderway>> {
    return await this.actOnRun("/sessions/run/resize", resize);
  }

  private async actOnRun(path: string, body: WriteSessionRun | ResizeSessionRun): Promise<SessionActed<SessionRunUnderway>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", path, body);
    return answer.ok === true ? { ok: true, value: answer.body as SessionRunUnderway } : { ok: false, outcome: answer.outcome };
  }

  private observeRun(port: number, runId: string): void {
    const socket = new WebSocket(`ws://${HOST}:${port}/sessions/run/${encodeURIComponent(runId)}/observe`);
    this.runs.set(runId, socket);
    let exited = false;
    // A socket that goes without `exited` ends the run for the reader all the same, as a stop does.
    const finish = (): void => {
      if (this.runs.get(runId) === socket) this.runs.delete(runId);
      if (!exited) this.ran(runId, { kind: "exited", code: null });
      exited = true;
    };
    socket.on("message", (data: WebSocket.RawData) => {
      let message: SessionRunMessage;
      try {
        message = JSON.parse(String(data)) as SessionRunMessage;
      } catch {
        return;
      }
      if (message.kind === "exited") exited = true;
      this.ran(runId, message);
    });
    socket.on("error", () => {});
    socket.on("close", finish);
  }

  /** Say what opens a window: main's, since a window is not a thing this host holds. */
  onWindow(
    shown: (
      sessionId: string,
      title: string,
      url: string,
      takes: readonly WindowTake[],
      origins: readonly string[],
      signIn: readonly string[],
      steps: readonly WalkStep[],
      pressed: boolean,
    ) => void,
  ): void {
    this.shown = shown;
  }

  /** Say what brings a window forward for a question the Session asked in it: main's, as `onWindow` is. */
  onAsked(asked: (sessionId: string, title: string, window: string, place: string, itemId: string) => void): void {
    this.askedIn = asked;
  }

  /**
   * A press on a window the Session showed, or on its row. **Only an address the Session's ledger shows
   * as a window**, so the renderer names a session and an address and main loads nothing else.
   */
  openWindow(sessionId: string, url: string): Outcome {
    const record = this.sessions?.find((one) => one.id === sessionId);
    const title = titleOfWindow(record, url);
    if (title === null) return { ok: false, why: "no_manifest" };
    this.shown(sessionId, title, url, takesOfWindow(record, url), originsOfWindow(record, url), [], stepsOfWindow(record, url), true);
    return { ok: true };
  }

  /**
   * The addresses Fleet held for a window to sign in with, once, or none. **A secret**: it goes to the
   * window that loads it and is neither kept nor logged here, and a Fleet that does not answer, or has
   * none, gives none.
   */
  private async signInFor(sessionId: string, url: string): Promise<string[]> {
    const port = this.port();
    if (port === null) return [];
    const answer = await ask(port, "POST", "/sessions/window/sign_in", { session_id: sessionId, url });
    if (answer.ok !== true) return [];
    const held = (answer.body as Partial<WindowSignIn> | null)?.sign_in;
    return Array.isArray(held) ? held.filter((one) => typeof one === "string") : [];
  }

  private inOrder(act: () => void | Promise<void>): void {
    this.turn = this.turn.then(act).catch(() => {});
  }

  /** `session.row`: one row of a thread, appended or replaced by its id. */
  row(change: SessionRowChanged): void {
    // **Opens when the Session shows it, and only then**: a thread read brings the rows already
    // there, and none of those opens a window.
    if (change.row.kind === "window" && !this.opened.has(change.row.id)) {
      this.opened.add(change.row.id);
      this.showWindow(change.session_id, change.row);
    }
    // A question asked in the walk takes the person there, once, when it is asked.
    if (change.row.kind === "window_ask" && change.row.answer === undefined && !this.opened.has(change.row.id)) {
      this.opened.add(change.row.id);
      const record = this.sessions?.find((one) => one.id === change.session_id);
      const title = titleOfWindow(record, change.row.window) ?? change.row.window;
      const { id, window, place } = change.row;
      // Fleet keys the question's waiting item by its row: `window_ask:<row id>`.
      this.inOrder(() => this.askedIn(change.session_id, title, window, place.url, `window_ask:${id}`));
    }
    const waiting = this.reading.get(change.session_id);
    if (waiting !== undefined) this.reading.set(change.session_id, withRow(waiting, change.row));
    const held = this.threads[change.session_id];
    // A copy drawn from disk while Fleet is read follows the stream too; Fleet's answer then replaces it with these rows merged.
    if (held !== undefined) {
      this.threads = { ...this.threads, [change.session_id]: withRow(held, change.row) };
      this.publish({ sessionThreads: this.threads });
      if (waiting === undefined) this.markDirty(change.session_id);
    }
  }

  /** Open a thread: read it once, then it follows the stream. */
  async watch(sessionId: string): Promise<void> {
    this.touch(sessionId);
    const port = this.port();
    if (port === null || this.threads[sessionId] !== undefined || this.reading.has(sessionId)) return;
    await this.read(port, sessionId);
  }

  /** Note a thread as the one opened last, and let the one opened longest ago go where more than `THREADS_HELD` are held. */
  private touch(sessionId: string): void {
    this.opening = [...this.opening.filter((one) => one !== sessionId), sessionId];
    const gone = this.opening.slice(0, -THREADS_HELD);
    if (gone.length === 0) return;
    this.opening = this.opening.slice(-THREADS_HELD);
    for (const id of gone) this.dirty.delete(id);
    const held = Object.fromEntries(Object.entries(this.threads).filter(([id]) => !gone.includes(id)));
    if (Object.keys(held).length === Object.keys(this.threads).length) return;
    this.threads = held;
    this.publish({ sessionThreads: this.threads });
  }

  private async read(port: number, sessionId: string): Promise<void> {
    this.reading.set(sessionId, []);
    this.publish({ sessionThreadsReading: [...this.reading.keys()] });
    const drawn = this.threads[sessionId] === undefined ? this.drawFromDisk(sessionId) : Promise.resolve();
    const answer = await ask(port, "GET", `/sessions/one?session_id=${encodeURIComponent(sessionId)}`);
    // The disk copy is drawn or skipped before Fleet's answer, so it can never land over it.
    await drawn;
    const meanwhile = this.reading.get(sessionId) ?? [];
    this.reading.delete(sessionId);
    const reading = [...this.reading.keys()];
    // A thread let go while it was being read is not put back: that would hold more than `THREADS_HELD`.
    if (answer.ok !== true || !this.opening.includes(sessionId)) {
      this.publish({ sessionThreadsReading: reading });
      return;
    }
    const thread = answer.body as SessionThread;
    for (const row of thread.rows) if (row.kind === "window" || row.kind === "window_ask") this.opened.add(row.id);
    this.threads = { ...this.threads, [sessionId]: meanwhile.reduce(withRow, thread.rows) };
    this.fold(thread.session);
    this.publish({ sessionThreads: this.threads, sessionThreadsReading: reading });
    this.markDirty(sessionId);
  }

  /** The last copy of a thread from disk, drawn if it is still being read and nothing has drawn it. Rows met on the way in are merged. */
  private async drawFromDisk(sessionId: string): Promise<void> {
    const rows = await this.cached(sessionId);
    if (rows === null || !this.reading.has(sessionId) || !this.opening.includes(sessionId) || this.threads[sessionId] !== undefined) return;
    // **Not marked in `opened`**: only Fleet's read says which windows were met, so a cached window row opens or hides nothing.
    this.threads = { ...this.threads, [sessionId]: (this.reading.get(sessionId) ?? []).reduce(withRow, rows) };
    this.publish({ sessionThreads: this.threads });
  }

  private async cached(sessionId: string): Promise<SessionRow[] | null> {
    if (this.dir === null) return null;
    try {
      const rows: unknown = JSON.parse(await readFile(join(this.dir, `${encodeURIComponent(sessionId)}.json`), "utf8"));
      return Array.isArray(rows) ? (rows as SessionRow[]) : null;
    } catch {
      return null;
    }
  }

  /** A held thread changed: it is written `WRITE_AFTER_MS` after the first change since the last write, with whatever changed by then. */
  private markDirty(sessionId: string): void {
    if (this.dir === null) return;
    this.dirty.add(sessionId);
    this.writing ??= setTimeout(() => void this.write(), WRITE_AFTER_MS);
  }

  /** Writes the changed threads still held, then lets the files of all but the `THREADS_HELD` newest go. A cache that fails to write is just not there. */
  private async write(): Promise<void> {
    this.writing = undefined;
    const dir = this.dir;
    const changed = [...this.dirty].flatMap((id) => (this.threads[id] === undefined ? [] : [[id, this.threads[id]] as const]));
    this.dirty.clear();
    if (dir === null || changed.length === 0) return;
    try {
      await mkdir(dir, { recursive: true });
      await Promise.all(changed.map(([id, rows]) => writeFile(join(dir, `${encodeURIComponent(id)}.json`), JSON.stringify(rows))));
      const files = await Promise.all(
        (await readdir(dir)).filter((name) => name.endsWith(".json")).map(async (name) => ({ name, at: (await stat(join(dir, name))).mtimeMs })),
      );
      const stale = files.sort((a, b) => b.at - a.at).slice(THREADS_HELD);
      await Promise.all(stale.map((one) => unlink(join(dir, one.name))));
    } catch {}
  }

  private fold(record: SessionRecord): void {
    this.sessions = withSession(this.sessions ?? [], record);
    this.publish({ sessions: { state: "read", sessions: this.sessions } });
    this.listed(this.sessions, this.readAt);
  }

  /** A session act: the route, the body, and the record Fleet answered with folded into what is held. */
  private async act(method: "POST", path: string, body: unknown): Promise<SessionActed> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    const record = answer.body as SessionRecord;
    this.fold(record);
    return { ok: true, value: record };
  }

  async start(start: StartSession): Promise<SessionActed> {
    return await this.act("POST", "/sessions/start", start);
  }

  async send(send: SendSessionMessage): Promise<SessionActed> {
    return await this.act("POST", "/sessions/message", send);
  }

  /** A pinned note. Fleet answers with the record and the candidate lines, so the record is folded like a message's and the lines go to the caller. */
  async sendNote(send: SendSessionNote): Promise<SessionActed<NoteSent>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/sessions/note", send);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    const sent = answer.body as NoteSent;
    this.fold(sent.session);
    return { ok: true, value: sent };
  }

  /** A walk's notes as one turn, folded as one note is; the lines come back one list per note, in order. */
  async sendNotes(send: SendSessionNotes): Promise<SessionActed<NotesSent>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/sessions/notes", send);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    const sent = answer.body as NotesSent;
    this.fold(sent.session);
    return { ok: true, value: sent };
  }

  async answer(answer: AnswerSessionAsk): Promise<SessionActed> {
    return await this.act("POST", "/sessions/ask/answer", answer);
  }

  async answerWaiting(answer: AnswerWaiting): Promise<SessionActed> {
    return await this.act("POST", "/sessions/waiting/answer", answer);
  }

  /** A pull request nobody holds, taken for a session. Fleet shows the new row in the ledger, so nothing is folded here. */
  async claimPullRequest(claim: ClaimPullRequest): Promise<SessionActed<PullRequestClaimed>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/sessions/claim_pull_request", claim);
    return answer.ok === true ? { ok: true, value: answer.body as PullRequestClaimed } : { ok: false, outcome: answer.outcome };
  }

  /** The proposer's reading of a brief that tags a Session. Nothing is held from it. */
  async readWait(read: ReadSessionWait): Promise<SessionActed<WaitReading>> {
    return await this.asked("/sessions/read_wait", read);
  }

  /** A waiting Session's condition or base, changed. Fleet writes a row in both threads, so nothing is folded here. */
  async changeWaiting(change: ChangeWaiting): Promise<SessionActed<SessionWait>> {
    return await this.asked("/sessions/change_waiting", change);
  }

  async startWaitingNow(start: StartWaitingNow): Promise<SessionActed> {
    return await this.act("POST", "/sessions/start_waiting", start);
  }

  private async asked<T>(path: string, body: unknown): Promise<SessionActed<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", path, body);
    return answer.ok === true ? { ok: true, value: answer.body as T } : { ok: false, outcome: answer.outcome };
  }

  async dismissWaiting(dismiss: DismissWaiting): Promise<SessionActed> {
    return await this.act("POST", "/sessions/waiting/dismiss", dismiss);
  }

  /** The person approved a plan. Fleet tells the agent and answers the record, which is folded. */
  async approvePlan(approve: ApprovePlan): Promise<SessionActed> {
    return await this.act("POST", "/sessions/plan/approve", approve);
  }

  /** The marks he made on a plan, sent to the agent as one message. */
  async sendPlanChanges(changes: SendPlanChanges): Promise<SessionActed> {
    return await this.act("POST", "/sessions/plan/changes", changes);
  }

  /** The person asked for a judge to read the work now. Fleet answers the record with `judging` set; the reading lands on it. */
  async judgeSession(judge: JudgeSession): Promise<SessionActed> {
    return await this.act("POST", "/sessions/judge", judge);
  }

  /** A judge's concern, sent to the agent as the person's message and settled. */
  async sendConcern(send: SendConcern): Promise<SessionActed> {
    return await this.act("POST", "/sessions/judge/send", send);
  }

  /** A judge's concern, settled with the agent told nothing. */
  async dismissConcern(dismiss: DismissConcern): Promise<SessionActed> {
    return await this.act("POST", "/sessions/judge/dismiss", dismiss);
  }

  /** The person closed a window a Session showed: its walk is over, and its waiting item stands. */
  async windowClosed(closed: WindowClosed): Promise<SessionActed> {
    return await this.act("POST", "/sessions/window/closed", closed);
  }

  /** A window a Session showed could not show its page: Fleet tells the Session. Fleet answers `WalkTold`, not the record, so nothing is folded. */
  async windowFailed(failed: WindowFailed): Promise<SessionActed<WalkTold>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/sessions/window/failed", failed);
    return answer.ok === true ? { ok: true, value: answer.body as WalkTold } : { ok: false, outcome: answer.outcome };
  }

  async tune(tune: TuneSession): Promise<SessionActed> {
    return await this.act("POST", "/sessions/tune", tune);
  }

  async runCommand(run: RunSessionCommand): Promise<SessionActed> {
    return await this.act("POST", "/sessions/command", run);
  }

  async rename(rename: RenameSession): Promise<SessionActed> {
    return await this.act("POST", "/sessions/rename", rename);
  }

  /** A new session as a copy of an ended or dead one, or of any one up to the row `at`. **The repository is the old session's own**, read off the record main holds. */
  async fork(sessionId: string, at?: string): Promise<SessionActed> {
    const manifestId = this.sessions?.find((one) => one.id === sessionId)?.manifest_id;
    if (manifestId === undefined) return { ok: false, outcome: { ok: false, why: "no_manifest" } };
    return await this.start({ manifest_id: manifestId, fork: at === undefined ? { session_id: sessionId } : { session_id: sessionId, at } });
  }

  async stop(sessionId: string): Promise<SessionActed> {
    return await this.act("POST", "/sessions/stop", { session_id: sessionId });
  }

  /** Runs again the hand off that failed on Job `sessionId`. Answers the new Job's record; where it fails again Fleet writes a fresh `handoff_failed` row and the Job changes by `session.changed`. */
  async retryHandoff(sessionId: string): Promise<SessionActed> {
    return await this.act("POST", "/sessions/handoff/retry", { session_id: sessionId });
  }

  /** Stops the turn, commits the work to the branch and gives the slot back. Answers the record with `hosted.paused` set. */
  async pause(sessionId: string): Promise<SessionActed> {
    return await this.act("POST", "/sessions/pause", { session_id: sessionId });
  }

  /** Auto land on or off. A forge's refusal is a 409 after the flag changed, and the record follows by `session.changed`; a 200 answers the record. */
  async setAutoLand(sessionId: string, on: boolean): Promise<SessionActed> {
    return await this.act("POST", "/sessions/auto_land", { id: sessionId, on });
  }

  /** Takes a slot again at the branch tip, or waits for one. Starts no turn. */
  async resume(sessionId: string): Promise<SessionActed> {
    return await this.act("POST", "/sessions/resume", { session_id: sessionId });
  }

  /** The Session's workflow, or none with no `workflow`. Fleet tells the agent; the record it answers is folded in. */
  async setWorkflow(sessionId: string, workflow?: string): Promise<SessionActed> {
    return await this.act("POST", "/sessions/workflow", workflow === undefined ? { session_id: sessionId } : { session_id: sessionId, workflow });
  }

  /** Every slot, branch, pull request, Job and need the Session holds, given back or handed to `release.to_session_id`. Both rows change by `session.changed`. */
  /** One group of a Session's plan handed to a Session of its own. The new Session and the parent's row change by `session.changed`. */
  async handToSession(hand: HandToSession): Promise<SessionActed<SessionHeld>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/sessions/hand_to_session", hand);
    return answer.ok === true ? { ok: true, value: answer.body as SessionHeld } : { ok: false, outcome: answer.outcome };
  }

  async releaseHolds(release: ReleaseSessionHolds): Promise<SessionActed<SessionHoldsReleased>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/sessions/release_holds", release);
    return answer.ok === true ? { ok: true, value: answer.body as SessionHoldsReleased } : { ok: false, outcome: answer.outcome };
  }

  /** The record Fleet answers is the new Session's, which is folded in; the old one's change arrives as `session.changed`. */
  async move(sessionId: string, manifestId: string): Promise<SessionActed> {
    return await this.act("POST", "/sessions/move", { session_id: sessionId, manifest_id: manifestId });
  }

  async end(sessionId: string): Promise<SessionActed> {
    return await this.act("POST", "/sessions/close", { session_id: sessionId });
  }

  /**
   * Writes the Session's retro: Fleet asks the agent what got in its way and answers when the retro
   * is written, so the wait is `MODEL_CALL_MS`. The retro is read afterwards on `GET /jobs/:id/retro`.
   */
  async retro(sessionId: string): Promise<Outcome> {
    const port = this.port();
    if (port === null) return NOT_CONNECTED;
    const answer = await ask(port, "POST", `/sessions/${encodeURIComponent(sessionId)}/retro`, undefined, MODEL_CALL_MS);
    return answer.ok === true ? { ok: true } : answer.outcome;
  }

  async file(sessionId: string, file: string): Promise<FrameRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    return await sessionFileOf(port, sessionId, file);
  }

  /** A subagent's own thread, read whole each time it is asked: the window asks again while it runs. */
  async subagent(sessionId: string, subagentId: string): Promise<SessionActed<SessionSubagent>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", `/sessions/subagent?session_id=${encodeURIComponent(sessionId)}&subagent_id=${encodeURIComponent(subagentId)}`);
    return answer.ok === true ? { ok: true, value: answer.body as SessionSubagent } : { ok: false, outcome: answer.outcome };
  }

  /** What a session's agent knows, was told and used each turn, read whole each time the window asks. */
  async loadout(sessionId: string): Promise<SessionActed<SessionLoadout>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", `/sessions/loadout?session_id=${encodeURIComponent(sessionId)}`);
    return answer.ok === true ? { ok: true, value: answer.body as SessionLoadout } : { ok: false, outcome: answer.outcome };
  }

  /** What one item of a session's Loadout says, as Fleet read it from disk. */
  async loadoutItem(sessionId: string, item: LoadoutItemAsk): Promise<SessionActed<LoadoutItemBody>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const query = new URLSearchParams({ session_id: sessionId, kind: item.kind, name: item.name, source: item.source, ...(item.from === undefined ? {} : { from: item.from }) });
    const answer = await ask(port, "GET", `/sessions/loadout/item?${query.toString()}`);
    return answer.ok === true ? { ok: true, value: answer.body as LoadoutItemBody } : { ok: false, outcome: answer.outcome };
  }

  /** A file the session wrote, opened where the machine opens it. Only a path the session's ledger names. */
  async openFile(sessionId: string, path: string): Promise<Followed> {
    return await openSessionFile(this.sessions?.find((one) => one.id === sessionId), path);
  }

  /** The record main holds for a session, which is what a path or an address has to be named by. */
  recordOf(sessionId: string): SessionRecord | undefined {
    return this.sessions?.find((one) => one.id === sessionId);
  }

  /** A file the session's ledger names, read for the panel. */
  async readArtifact(sessionId: string, path: string): Promise<ArtifactRead> {
    return await readSessionArtifact(this.recordOf(sessionId), path);
  }

  /**
   * A press on a pull request one of the sessions holds. **The repository is the session's own**,
   * read off the record main holds, so the renderer names a session and a number and never a path.
   * Fleet refreshes every `pr` row on the answer and `session.changed` carries them, so nothing is
   * folded here.
   */
  async press(sessionId: string, number: number, press: PullRequestPress): Promise<SessionActed<PullRequestState>> {
    const manifestId = this.sessions?.find((one) => one.id === sessionId)?.manifest_id;
    if (manifestId === undefined) return this.port() === null ? { ok: false, outcome: NOT_CONNECTED } : { ok: false, outcome: { ok: false, why: "no_manifest" } };
    return await this.pressOn(manifestId, number, press);
  }

  /**
   * The same press on a pull request no Session holds, such as a form edit's: **the repository is named by its Manifest**,
   * as `catchUpCheckout` names it.
   */
  async pressInRepository(manifestId: string, number: number, press: PullRequestPress): Promise<SessionActed<PullRequestState>> {
    return await this.pressOn(manifestId, number, press);
  }

  private async pressOn(manifestId: string, number: number, press: PullRequestPress): Promise<SessionActed<PullRequestState>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const repository = encodeURIComponent(manifestId);
    const answer =
      press === "read"
        ? await ask(port, "GET", `/pull_requests/${repository}/${number}`)
        : await ask(port, "POST", `/pull_requests/${repository}/${number}/${PRESS_ROUTE[press]}`);
    return answer.ok === true
      ? { ok: true, value: answer.body as PullRequestState }
      : { ok: false, outcome: answer.outcome };
  }

  /**
   * What the panel's tabs draw, read from the forge by Fleet. **The repository is the session's own**,
   * as `press` names it.
   */
  async pullRequestPage(sessionId: string, number: number): Promise<SessionActed<PullRequestPage>> {
    return await this.onPage(sessionId, (port, repository) => ask(port, "GET", `/pull_request_details/${repository}/${number}`));
  }

  /** One file's patch, read from the forge by Fleet when a person opens it. The repository is the session's own. */
  async pullRequestFile(sessionId: string, number: number, path: string): Promise<SessionActed<PullRequestFilePatch>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const manifestId = this.sessions?.find((one) => one.id === sessionId)?.manifest_id;
    if (manifestId === undefined) return { ok: false, outcome: { ok: false, why: "no_manifest" } };
    const answer = await ask(port, "GET", `/pull_request_files/${encodeURIComponent(manifestId)}/${number}?path=${encodeURIComponent(path)}`);
    return answer.ok === true ? { ok: true, value: answer.body as PullRequestFilePatch } : { ok: false, outcome: answer.outcome };
  }

  /** A comment, an approval or a request for changes. Fleet answers the page read again after it; a refusal carries the forge's words. */
  async writePullRequest(sessionId: string, number: number, write: PullRequestWrite): Promise<SessionActed<PullRequestPage>> {
    return await this.onPage(sessionId, (port, repository) =>
      ask(port, "POST", `/pull_request_writes/${repository}`, {
        pull_request: number,
        kind: write.kind,
        ...(write.body === undefined ? {} : { body: write.body }),
        session_id: sessionId,
      }),
    );
  }

  private async onPage(sessionId: string, go: (port: number, repository: string) => ReturnType<typeof ask>): Promise<SessionActed<PullRequestPage>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const manifestId = this.sessions?.find((one) => one.id === sessionId)?.manifest_id;
    if (manifestId === undefined) return { ok: false, outcome: { ok: false, why: "no_manifest" } };
    const answer = await go(port, encodeURIComponent(manifestId));
    return answer.ok === true ? { ok: true, value: answer.body as PullRequestPage } : { ok: false, outcome: answer.outcome };
  }

  /** The window is going: nothing is read for it any more. */
  close(): void {
    this.sessions = null;
    this.readAt = null;
    this.threads = {};
    this.opening = [];
    this.reading.clear();
    clearTimeout(this.writing);
    this.writing = undefined;
    this.dirty.clear();
    for (const socket of this.runs.values()) socket.close();
    this.runs.clear();
  }
}
