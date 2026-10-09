// Sessions as a real Fleet serves them: the draft seam (`sessions-draft.tsx`) implemented over what
// main publishes and the capabilities the preload carries.
//
// **The seam stays the seam.** The screens read `SessionsDraft`; the mock fills it from fixtures and
// this fills it from `BridgeState.sessions`, so a surface cannot tell which it is drawing and a walk
// on the mock still plays. What this holds beside what main publishes is only what Fleet has no
// field for: tags chosen and not yet sent, the sketches this window sent, the addresses of pictures
// it has read, and the last refusal in words.
//
// **Where Fleet serves something this build has no screen for, or none yet, the act is absent from
// the draft and the screen leaves it off.** Nothing here shows fixture data. Pilot is one call, which
// takes the Job over and starts the Session on its worktree, and the `/` list is what the agent's own
// init line named.

import { useEffect, useMemo, useSyncExternalStore } from "react";
import type { ReactNode } from "react";

import { EFFORTS } from "@armada/jobs/draft/tuning";
import type { SessionTag as WireTag } from "@armada/protocol";
import type { SessionCommand } from "@armada/screens/src/draft/sessions";
import { refusalWords } from "@armada/screens/src/refusal-words";
import { askToTell, NOT_REACHABLE } from "./tell";
import { rowsOfThread, sessionsOfRecords } from "@armada/screens/src/sessions-wire";
import type { Beside } from "@armada/screens/src/sessions-wire";
import type {
  Session,
  SessionAnswer,
  SessionAttachment,
  SessionQuestionAnswer,
  SessionsDraft,
  SessionTag,
} from "@armada/screens/src/draft/sessions";

import type { BridgeApi } from "../../shared/api";
import type { BridgeState } from "../../shared/bridge";
import { uploadOfFile, uploadOfSketch } from "./session-uploads";
import { SessionsFrom } from "./sessions-draft";

const NONE: readonly never[] = [];
const HELD_WITHIN_MS = 3000;

type Sketch = Extract<SessionAttachment, { kind: "sketch" }>;

type Inputs = Pick<BridgeState, "sessions" | "sessionThreads" | "jobs" | "holds">;

const wireTagOf = (tag: SessionTag): WireTag => ({
  kind: tag.kind,
  id: tag.id,
  title: tag.title,
  ...(tag.job === undefined || tag.job.slot === undefined ? {} : { job: { number: tag.job.number, branch: tag.job.branch, slot: tag.job.slot, state: tag.job.state } }),
});

/** The store behind one window's Sessions. Exported for a test that holds it without a window. */
export class WiredStore {
  private readonly api: BridgeApi;
  private inputs: Inputs | undefined;
  private mapped: readonly Session[] = NONE;
  private refusal: string | undefined;
  private readonly listeners = new Set<() => void>();
  private readonly pending = new Map<string, readonly SessionTag[]>();
  private readonly sketches = new Map<string, readonly Sketch[]>();
  private readonly pictures = new Map<string, string>();
  private readonly fetching = new Set<string>();

  constructor(api: BridgeApi) {
    this.api = api;
  }

  /** Read the state and follow it. Returns the way to stop. */
  attach(): () => void {
    let live = true;
    void this.api.state().then((state) => live && this.take(state));
    const off = this.api.subscribe((state) => this.take(state));
    return () => {
      live = false;
      off();
    };
  }

  private take(state: BridgeState): void {
    const was = this.inputs;
    if (was !== undefined && was.sessions === state.sessions && was.sessionThreads === state.sessionThreads && was.jobs === state.jobs && was.holds === state.holds) return;
    this.inputs = { sessions: state.sessions, sessionThreads: state.sessionThreads, jobs: state.jobs, holds: state.holds };
    this.recompute();
  }

  private recompute(): void {
    const read = this.inputs?.sessions;
    this.mapped = read === undefined || read.state !== "read" ? NONE : sessionsOfRecords(read.sessions, this.inputs!.sessionThreads, this.beside);
    this.listeners.forEach((listener) => listener());
  }

  private readonly beside = (id: string): Beside => ({
    jobs: this.inputs?.jobs ?? NONE,
    picture: (sessionId, fileId) => this.picture(sessionId, fileId),
    sketches: this.sketches.get(id) ?? NONE,
    pending: this.pending.get(id) ?? NONE,
  });

  /** A picture a message carried, once read. Read on first ask and drawn from then on. */
  private picture(sessionId: string, fileId: string): string | undefined {
    const key = `${sessionId}\n${fileId}`;
    const held = this.pictures.get(key);
    if (held !== undefined || this.fetching.has(key)) return held;
    this.fetching.add(key);
    void this.api.readSessionFile(sessionId, fileId).then((read) => {
      if (!read.ok) return;
      this.pictures.set(key, URL.createObjectURL(new Blob([read.bytes as BlobPart], { type: read.type })));
      this.recompute();
    });
    return undefined;
  }

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };

  /** Whether Fleet has answered for Sessions. Until it has, and on a Fleet that does not serve them, the surface is left off. */
  readonly served = (): boolean => this.inputs?.sessions.state === "read";

  private say(outcome: Parameters<typeof refusalWords>[0] | undefined): void {
    this.refusal = outcome === undefined ? undefined : refusalWords(outcome);
    this.listeners.forEach((listener) => listener());
  }

  /** A refused act on one Session is a toast, as a failed annotation send is. A terminal that is not listening says what to run. */
  private tell(outcome: Parameters<typeof refusalWords>[0]): void {
    const unreachable = !outcome.ok && outcome.why === "refused" && outcome.error.code === "fleet.terminal_session_unreachable";
    askToTell(unreachable ? NOT_REACHABLE : refusalWords(outcome));
  }

  private tagged(id: string, tags: readonly SessionTag[]): void {
    this.pending.set(id, tags);
    this.recompute();
  }

  /** Resolves once the window holds a session, or after a moment: the answer to a start can beat the state that carries it. */
  private held(id: string): Promise<void> {
    return new Promise((done) => {
      const stop = () => {
        off();
        clearTimeout(timer);
        done();
      };
      const off = this.subscribe(() => this.mapped.some((one) => one.id === id) && stop());
      const timer = setTimeout(stop, HELD_WITHIN_MS);
      if (this.mapped.some((one) => one.id === id)) stop();
    });
  }

  private async starting(tag: SessionTag | undefined): Promise<string | undefined> {
    this.say(undefined);
    const done = await this.api.startSession();
    if (!done.ok) return void this.say(done.outcome);
    const id = done.value.id;
    if (tag !== undefined) this.pending.set(id, [tag]);
    await this.held(id);
    this.recompute();
    return id;
  }

  private async sending(id: string, sent: Parameters<SessionsDraft["send"]>[1]): Promise<void> {
    this.say(undefined);
    const tags = this.pending.get(id) ?? NONE;
    this.tagged(id, NONE);
    const files = (await Promise.all(sent.files.map(uploadOfFile))).filter((one) => one !== undefined);
    const drawn = await Promise.all(sent.sketches.map(uploadOfSketch));
    const attachments = [...files, ...drawn];
    const done = await this.api.sendSessionMessage({
      session_id: id,
      text: sent.text,
      ...(attachments.length === 0 ? {} : { attachments }),
      ...(sent.tags.length === 0 ? {} : { mentions: sent.tags.map(wireTagOf) }),
    });
    if (!done.ok) {
      this.tagged(id, tags);
      return this.tell(done.outcome);
    }
    // The wire holds a sketch only as the picture it went as, so the ledger keeps what was drawn here.
    if (sent.sketches.length > 0) {
      this.sketches.set(id, [...(this.sketches.get(id) ?? NONE), ...sent.sketches.map((one): Sketch => ({ kind: "sketch", id: one.id, title: one.title, by: "you", drawing: one.drawing }))]);
      this.recompute();
    }
  }

  /** One call: the Job is taken over and a Session starts on its worktree. Resolves to the Session once the window holds it. */
  private async piloting(jobId: string, outcome: "take_over" | "restart_step"): Promise<string | undefined> {
    this.say(undefined);
    const done = await this.api.pilotJob(jobId, outcome);
    if (!done.ok) return void this.say(done.outcome);
    await this.held(done.value.id);
    return done.value.id;
  }

  /** One call: a new Session starts as a copy of a dead one's conversation. Resolves to it once the window holds it. */
  private async forking(id: string): Promise<string | undefined> {
    this.say(undefined);
    const done = await this.api.forkSession(id);
    if (!done.ok) return void this.say(done.outcome);
    await this.held(done.value.id);
    return done.value.id;
  }

  /** One call: the Session's retro is written. Resolves to whether it was; a refusal says why in `said`. */
  private async retroing(id: string): Promise<boolean> {
    this.say(undefined);
    const done = await this.api.retroSession(id);
    if (!done.ok) {
      this.say(done);
      return false;
    }
    return true;
  }

  private async exiting(jobId: string, exit: "submit" | "attest" | "supersede"): Promise<void> {
    this.say(undefined);
    const done = await this.api.exitPilot(jobId, exit);
    if (!done.ok) this.say(done);
  }

  private commandsFrom: readonly string[] | undefined;
  private commandsAre: readonly SessionCommand[] = NONE;

  /** What the agent said it has, from the first session Fleet gave a list for. It has no sentence for each, so none is drawn. */
  private commandsRead(): readonly SessionCommand[] {
    const read = this.inputs?.sessions;
    const names = read?.state === "read" ? read.sessions.find((one) => (one.hosted?.commands?.length ?? 0) > 0)?.hosted?.commands : undefined;
    if (names === undefined) return NONE;
    if (names !== this.commandsFrom) {
      this.commandsFrom = names;
      this.commandsAre = names.map((name) => ({ name, says: "" }));
    }
    return this.commandsAre;
  }

  private async acting(id: string, number: number, act: "ready" | "merge" | "auto_merge" | "review"): Promise<void> {
    this.say(undefined);
    const done = await this.api.pressPullRequest(id, number, act);
    if (!done.ok) this.say(done.outcome);
    // Whatever the answer, the sheet shows the pull request as the forge has it now.
    if (act !== "review") await this.api.pressPullRequest(id, number, "read");
  }

  private async plain(act: Promise<{ ok: true } | { ok: false; outcome: Parameters<typeof refusalWords>[0] }>): Promise<void> {
    this.say(undefined);
    const done = await act;
    if (!done.ok) this.tell(done.outcome);
  }

  private readonly lookup = (id: string): Session | undefined => this.mapped.find((one) => one.id === id);

  readonly draft: SessionsDraft = this.build();

  private build(): SessionsDraft {
    // A getter's `this` is the object it is on, so the models read through the store.
    const store = this;
    return {
    get: () => this.mapped,
    subscribe: this.subscribe,
    start: (tag) => this.starting(tag),
    pilot: (jobId, outcome) => this.piloting(jobId, outcome),
    fork: (id) => this.forking(id),
    retro: (id) => this.retroing(id),
    exit: (jobId, exit) => void this.exiting(jobId, exit),
    watch: (id) => void this.api.watchSession(id),
    subagent: async (id, subagentId) => {
      const read = await this.api.readSessionSubagent(id, subagentId);
      if (!read.ok) return undefined;
      const { rows, finished, report } = read.value;
      return { rows: rowsOfThread(id, rows, () => undefined), finished, ...(report === undefined ? {} : { report }) };
    },
    close: (id) => void this.plain(this.api.closeSession(id)),
    rename: (id, title) => void this.plain(this.api.renameSession({ session_id: id, title })),
    refresh: (id, number) => void this.api.pressPullRequest(id, number, "read"),
    openFile: (id, path) => void this.api.openSessionFile(id, path),
    openWindow: (id, url) => void this.api.openSessionWindow(id, url),
    readArtifact: (id, path) => this.api.readSessionArtifact(id, path),
    page: {
      show: (id, address, bounds) => void this.api.showSessionPage(id, address, bounds),
      move: (bounds) => void this.api.moveSessionPage(bounds),
      hide: () => void this.api.hideSessionPage(),
      onEscape: (on) => this.api.onSessionPageEscape(on),
    },
    said: () => this.refusal,
    taggable: () => {
      const sessions = this.mapped;
      const jobs = this.inputs?.jobs ?? NONE;
      const prs = new Map<number, SessionTag>();
      const branches = new Set<string>();
      for (const one of sessions) {
        for (const held of one.attachments) {
          if (held.kind === "pull_request") prs.set(held.number, { kind: "pull_request", id: String(held.number), title: `#${held.number} ${held.title}`.trim() });
          if (held.kind === "branch") branches.add(held.name);
        }
      }
      for (const job of jobs) if (job.branch !== undefined) branches.add(job.branch);
      return [
        ...jobs.map((job): SessionTag => ({ kind: "job", id: job.id, title: job.title })),
        ...prs.values(),
        ...[...branches].map((name): SessionTag => ({ kind: "branch", id: name, title: name })),
      ];
    },
    setTags: (id, tags) => this.tagged(id, tags),
    send: (id, sent) => void this.sending(id, sent),
    tune: (id, tuning) =>
      void this.plain(
        this.api.tuneSession({
          session_id: id,
          ...(tuning.model === null ? {} : { model: tuning.model }),
          ...(tuning.effort === null ? {} : { effort: tuning.effort }),
          mode: tuning.mode,
        }),
      ),
    act: (id, number, act) => void this.acting(id, number, act),
    reviews: "fleet",
    get models() {
      return store.inputs?.holds.models?.models ?? NONE;
    },
    efforts: EFFORTS,
    get commands() {
      return store.commandsRead();
    },
    answer: (id: string, answer?: SessionAnswer, answers?: SessionQuestionAnswer[]) => {
      const call = this.lookup(id)?.asked?.call;
      if (call === undefined) return;
      void this.plain(
        this.api.answerSessionAsk({
          session_id: id,
          call,
          answer: answer ?? "allow_once",
          ...(answers === undefined ? {} : { answers }),
        }),
      );
    },
    };
  }
}

/** Sessions over a real Fleet, or over a fake one answering the same capabilities. */
export function WiredSessions({ children }: { children: ReactNode }) {
  const store = useMemo(() => new WiredStore(window.armada), []);
  useEffect(() => store.attach(), [store]);
  const served = useSyncExternalStore(store.subscribe, store.served);
  return <SessionsFrom held={served ? store.draft : undefined}>{children}</SessionsFrom>;
}
