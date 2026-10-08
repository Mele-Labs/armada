// Every session on the machine, and the threads of the ones a window opened.
//
// **Kept current by the stream, not by polling**: `session.changed` carries a session whole after any
// fact about it and `session.row` one row of a thread, so both fold the one way, and an act's own
// answer is the same record. Held beside `studios.ts` for its reason, and unlike it nothing here is
// scoped to a surface: the ledger answers who owns a branch or a pull request for a chip anywhere in
// the window, so the list is read on every connection and never dropped.
//
// **Threads are held for the window's life once opened.** A row that arrives before the read of its
// thread has answered is kept and merged after it, by id, so the gap between asking and hearing loses
// none; a row for a thread nobody opened is dropped, since the read that opens it brings every row.

import type {
  AnswerSessionAsk,
  Followed,
  FrameRead,
  Outcome,
  PullRequestState,
  RenameSession,
  ReviewDispatched,
  SendSessionMessage,
  SessionList,
  SessionRecord,
  SessionRow,
  SessionRowChanged,
  SessionThread,
  StartSession,
  TuneSession,
} from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import type { ArtifactRead } from "@armada/screens/src/draft/sessions";
import type { PullRequestPress, SessionActed } from "../shared/api/sessions";
import { openSessionFile, readSessionArtifact } from "./session-file";
import { ask, sessionFileOf } from "./request";

type Publish = (change: Partial<BridgeState>) => void;

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** Where each press on a pull request is served. `crates/ipc/operations/` is the authority. */
const PRESS_ROUTE: Readonly<Record<"ready" | "merge" | "auto_merge", string>> = {
  ready: "ready",
  merge: "merge",
  auto_merge: "auto_merge",
};

/** A row replaced where its id is held, appended where it is not. */
export function withRow(rows: readonly SessionRow[], row: SessionRow): SessionRow[] {
  return rows.some((one) => one.id === row.id) ? rows.map((one) => (one.id === row.id ? row : one)) : [...rows, row];
}

/** A session replaced where its id is held, put first where it is not: the list is most recently seen first. */
export function withSession(sessions: readonly SessionRecord[], record: SessionRecord): SessionRecord[] {
  return sessions.some((one) => one.id === record.id)
    ? sessions.map((one) => (one.id === record.id ? record : one))
    : [record, ...sessions];
}

export class SessionsHost {
  private readonly publish: Publish;
  private readonly port: () => number | null;
  private sessions: SessionRecord[] | null = null;
  private threads: Record<string, SessionRow[]> = {};
  /** Threads whose read has not answered, with the rows that arrived meanwhile. */
  private reading = new Map<string, SessionRow[]>();

  constructor(publish: Publish, port: () => number | null) {
    this.publish = publish;
    this.port = port;
  }

  /** Read every session, and every thread a window holds again. Once per connection, and on a refresh. */
  async again(port: number): Promise<void> {
    const answer = await ask(port, "GET", "/sessions");
    if (answer.ok !== true) {
      this.sessions = null;
      this.publish({ sessions: { state: "failed", outcome: answer.outcome } });
      return;
    }
    this.sessions = (answer.body as SessionList).sessions;
    this.publish({ sessions: { state: "read", sessions: this.sessions } });
    await Promise.all(Object.keys(this.threads).map((id) => this.read(port, id)));
  }

  /** `session.changed`: the session, whole. */
  changed(record: SessionRecord): void {
    this.fold(record);
  }

  /** `session.row`: one row of a thread, appended or replaced by its id. */
  row(change: SessionRowChanged): void {
    const waiting = this.reading.get(change.session_id);
    if (waiting !== undefined) {
      this.reading.set(change.session_id, withRow(waiting, change.row));
      return;
    }
    const held = this.threads[change.session_id];
    if (held === undefined) return;
    this.threads = { ...this.threads, [change.session_id]: withRow(held, change.row) };
    this.publish({ sessionThreads: this.threads });
  }

  /** Open a thread: read it once, then it follows the stream. */
  async watch(sessionId: string): Promise<void> {
    const port = this.port();
    if (port === null || this.threads[sessionId] !== undefined || this.reading.has(sessionId)) return;
    await this.read(port, sessionId);
  }

  private async read(port: number, sessionId: string): Promise<void> {
    this.reading.set(sessionId, []);
    const answer = await ask(port, "GET", `/sessions/one?session_id=${encodeURIComponent(sessionId)}`);
    const meanwhile = this.reading.get(sessionId) ?? [];
    this.reading.delete(sessionId);
    if (answer.ok !== true) return;
    const thread = answer.body as SessionThread;
    this.threads = { ...this.threads, [sessionId]: meanwhile.reduce(withRow, thread.rows) };
    this.fold(thread.session);
    this.publish({ sessionThreads: this.threads });
  }

  private fold(record: SessionRecord): void {
    this.sessions = withSession(this.sessions ?? [], record);
    this.publish({ sessions: { state: "read", sessions: this.sessions } });
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

  async answer(answer: AnswerSessionAsk): Promise<SessionActed> {
    return await this.act("POST", "/sessions/ask/answer", answer);
  }

  async tune(tune: TuneSession): Promise<SessionActed> {
    return await this.act("POST", "/sessions/tune", tune);
  }

  async rename(rename: RenameSession): Promise<SessionActed> {
    return await this.act("POST", "/sessions/rename", rename);
  }

  /** A new session as a copy of an ended or dead one. **The repository is the old session's own**, read off the record main holds. */
  async fork(sessionId: string): Promise<SessionActed> {
    const manifestId = this.sessions?.find((one) => one.id === sessionId)?.manifest_id;
    if (manifestId === undefined) return { ok: false, outcome: { ok: false, why: "no_manifest" } };
    return await this.start({ manifest_id: manifestId, fork: { session_id: sessionId } });
  }

  async end(sessionId: string): Promise<SessionActed> {
    return await this.act("POST", "/sessions/close", { session_id: sessionId });
  }

  async file(sessionId: string, file: string): Promise<FrameRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    return await sessionFileOf(port, sessionId, file);
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
  async press(sessionId: string, number: number, press: PullRequestPress): Promise<SessionActed<PullRequestState | ReviewDispatched>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const manifestId = this.sessions?.find((one) => one.id === sessionId)?.manifest_id;
    if (manifestId === undefined) return { ok: false, outcome: { ok: false, why: "no_manifest" } };
    const repository = encodeURIComponent(manifestId);
    const answer =
      press === "review"
        ? await ask(port, "POST", `/pull_request_reviews/${repository}`, { pull_request: String(number), session_id: sessionId })
        : press === "read"
          ? await ask(port, "GET", `/pull_requests/${repository}/${number}`)
          : await ask(port, "POST", `/pull_requests/${repository}/${number}/${PRESS_ROUTE[press]}`);
    return answer.ok === true
      ? { ok: true, value: answer.body as PullRequestState | ReviewDispatched }
      : { ok: false, outcome: answer.outcome };
  }

  /** The window is going: nothing is read for it any more. */
  close(): void {
    this.sessions = null;
    this.threads = {};
    this.reading.clear();
  }
}
