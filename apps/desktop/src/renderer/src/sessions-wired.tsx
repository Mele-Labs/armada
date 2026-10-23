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
// the draft and the screen leaves it off.** Nothing here shows fixture data. The `/` list is what the agent's own
// init line named.

import { useEffect, useMemo, useSyncExternalStore } from "react";
import type { ReactNode } from "react";

import type { ProposedField, Notice, NoticeScope, SessionHoldsReleased, StartAfter, SessionRunMessage, SettingsList, TimelineEvent, WorkflowSummary } from "@armada/protocol";
import { choicesOf, wireMarksOf } from "@armada/screens/src/session-work";
import type { QueuedIdea, SessionCommand, SessionWait, StartWhen, WaitReading, WorkflowChoice } from "@armada/screens/src/draft/sessions";
import { refusalWords } from "@armada/screens/src/refusal-words";
import { askToTell, NOT_REACHABLE, type RefusedAct } from "./tell";
import { taggedIn } from "./dispatch-panel/waits";
import { hearQueuedIdeas, newIdeaId, readQueuedIdeas, writeQueuedIdeas } from "./queued-ideas";
import { moveRefusalWords } from "./session-moved";
import { sessionsOfPicked } from "@armada/screens/src/board";
import { SessionMemo, rowsOfThread, sessionsOfRecords } from "@armada/screens/src/sessions-wire";
import { conditionOfWhen, defaultBase, planPartsOf, whenOfCondition } from "@armada/screens/src/session-waits";
import { share } from "@armada/screens/src/share";
import type { Beside } from "@armada/screens/src/sessions-wire";
import type {
  CheckoutDiffRead,
  HoldsReleased,
  PullRequestFileRead,
  PullRequestPageRead,
  Session,
  SessionAnswer,
  SessionAttachment,
  SessionQuestionAnswer,
  SessionsDraft,
  SessionTag,
} from "@armada/screens/src/draft/sessions";

import type { BridgeApi } from "../../shared/api";
import type { BridgeState } from "../../shared/bridge";
import { buildChangeOf, type BuildChange } from "./build-change";
import { clashFileRead } from "./clash-file-read";
import { fileChangeRead } from "./file-change-read";
import { catchingUp } from "./checkout-catch-up";
import { uploadOfFile, uploadOfSketch, wireTagOf } from "./session-uploads";
import { readHistory } from "./session-history";
import { SessionsFrom } from "./sessions-draft";
import type { TimelineSource } from "./fleet-timeline";
import { markViewed } from "./cockpit/viewed";
import { stateOf } from "./state-feed";
import { manifestLabel } from "@armada/shell";

const NONE: readonly never[] = [];

/** A page Fleet answered, or the words of its refusal. */
const pageRead = (answer: Awaited<ReturnType<BridgeApi["readPullRequestPage"]>>): PullRequestPageRead =>
  answer.ok ? { ok: true, page: answer.value } : { ok: false, said: refusalWords(answer.outcome) };
/** A file Fleet answered, or the words of its refusal. */
const fileRead = (answer: Awaited<ReturnType<BridgeApi["readPullRequestFile"]>>): PullRequestFileRead =>
  answer.ok ? { ok: true, file: answer.value } : { ok: false, said: refusalWords(answer.outcome) };
/** A checkout's diff Fleet answered, or the words of its refusal. */
const checkoutDiffRead = (answer: Awaited<ReturnType<BridgeApi["readCheckoutDiff"]>>): CheckoutDiffRead =>
  answer.ok ? { ok: true, diff: answer.value.diff, ...(answer.value.cut === true ? { cut: true as const } : {}) } : { ok: false, said: refusalWords(answer.outcome) };
const HELD_WITHIN_MS = 3000;
/** How long the words stand still before the proposer is asked what wait they name. */
const WAIT_READ_AFTER_MS = 400;
/** Briefs read for a wait that are kept; the oldest go first. */
const WAIT_READS_KEPT = 24;

/** One brief that tags a Session, and what the proposer read of it. Until it has answered the defaults stand: it ends, from main. */
type WaitRead = { sessionId: string; words: string; reading: WaitReading | undefined; timer: ReturnType<typeof setTimeout> | undefined; out: Promise<void> | undefined; answered: boolean };

type Sketch = Extract<SessionAttachment, { kind: "sketch" }>;

type Inputs = Pick<BridgeState, "sessions" | "sessionThreads" | "sessionWatches" | "sessionThreadsReading" | "holds" | "repository" | "helm"> & { workflowsOff: boolean };

/** `features.workflows` in settings.json, which the owner turns off to dispatch plain Sessions. On until Fleet says otherwise. */
export const workflowsOffIn = (settings: SettingsList | null): boolean => settings?.settings.find((one) => one.key === "features.workflows")?.value === false;

/** The store behind one window's Sessions. Exported for a test that holds it without a window. */
export class WiredStore {
  private readonly api: BridgeApi;
  private inputs: Inputs | undefined;
  private mapped: readonly Session[] = NONE;
  /** The live Helm Session of the repository the dock answers for, held whatever the rail picked. */
  private helm: Session | undefined;
  private refusal: string | undefined;
  private readonly listeners = new Set<() => void>();
  private readonly pending = new Map<string, readonly SessionTag[]>();
  private readonly sketches = new Map<string, readonly Sketch[]>();
  private readonly pictures = new Map<string, string>();
  private readonly fetching = new Set<string>();
  /** What changed a Session's workflow, model or effort, from the timeline. */
  private events: readonly TimelineEvent[] = NONE;
  /** Picks of the proposer he changed here, until the record says so too. */
  private readonly unpicked = new Map<string, ReadonlySet<ProposedField>>();
  private choices: { from: readonly WorkflowSummary[]; are: readonly WorkflowChoice[] } = { from: NONE, are: NONE };

  /** The ideas saved to start later, replaced and never changed so a reader can tell when they moved. */
  private queuedNow: readonly QueuedIdea[] = readQueuedIdeas();
  private readonly starts = new Set<string>();

  /** What the proposer read of each brief that tags a Session, by Session and words. */
  private readonly waitReads = new Map<string, WaitRead>();
  /** The brief last typed: an older one is not asked about, since the words moved on. */
  private waitLatest: WaitRead | undefined;

  constructor(api: BridgeApi) {
    this.api = api;
  }

  /** Read the state and follow it. Returns the way to stop. */
  attach(): () => void {
    let live = true;
    const source = stateOf(this.api);
    void source.state().then((state) => live && this.take(state));
    const off = source.subscribe((state) => this.take(state));
    // Another window saved or started an idea: this one reads the queue again.
    const heard = hearQueuedIdeas(() => this.keepQueued(readQueuedIdeas()));
    return () => {
      live = false;
      off();
      heard();
    };
  }

  private take(state: BridgeState): void {
    const was = this.inputs;
    // Every message brings the whole state as new objects. What did not change is kept as it was, so a message about
    // one Session finds every other one the same and reads only that one.
    const next = {
      // A failed re-read after a good one is not a list with no Sessions: what was held stays, as main's does.
      sessions: state.sessions.state === "failed" && was?.sessions.state === "read" ? was.sessions : share(was?.sessions, state.sessions),
      sessionThreads: share(was?.sessionThreads, state.sessionThreads),
      sessionWatches: share(was?.sessionWatches, state.sessionWatches),
      sessionThreadsReading: share(was?.sessionThreadsReading, state.sessionThreadsReading),
      holds: share(was?.holds, state.holds),
      repository: state.repository,
      helm: share(was?.helm, state.helm),
      workflowsOff: workflowsOffIn(state.settings),
    };
    if (was !== undefined && was.sessions === next.sessions && was.sessionThreads === next.sessionThreads && was.sessionWatches === next.sessionWatches && was.sessionThreadsReading === next.sessionThreadsReading && was.holds === next.holds && was.repository === next.repository && was.helm === next.helm && was.workflowsOff === next.workflowsOff) return;
    this.inputs = next;
    this.recompute();
  }

  private recompute(): void {
    const read = this.inputs?.sessions;
    // The window holds the picked repository's Sessions,: every list here reads this one.
    const picked = this.inputs?.holds.repositories?.find((one) => one.root === this.inputs?.repository) ?? null;
    const mapped = read === undefined || read.state !== "read" ? NONE : sessionsOfRecords(sessionsOfPicked(read.sessions, picked), this.inputs!.sessionThreads, this.beside, undefined, this.memo, this.inputs!.sessionThreadsReading);
    // Nothing drawn changed: the same list, so what reads it does not draw again.
    if (mapped.length !== this.mapped.length || mapped.some((one, at) => one !== this.mapped[at])) this.mapped = mapped;
    // Helm answers for the repository the dock points at, which the rail's pick need not be.
    const aimed = this.inputs?.helm.state === "pointed" ? this.inputs.helm.manifestId : undefined;
    const record = aimed === undefined || read === undefined || read.state !== "read" ? undefined : read.sessions.find((one) => one.helm === true && one.state !== "ended" && one.manifest_id === aimed);
    this.helm = record === undefined ? undefined : sessionsOfRecords([record], this.inputs!.sessionThreads, this.beside, undefined, this.memo, this.inputs!.sessionThreadsReading)[0];
    this.memo.keep(new Set(read?.state === "read" ? read.sessions.map((one) => one.id) : []));
    this.listeners.forEach((listener) => listener());
  }

  private readonly memo = new SessionMemo();
  /** Bumped when a picture arrives, which a Session's rows draw without any record changing. */
  private epoch = 0;
  private repositoryOf: { from: unknown; read: (manifestId: string | undefined) => string } | undefined;

  private readonly pictureOf = (sessionId: string, fileId: string): string | undefined => this.picture(sessionId, fileId);

  /** One function while the repositories are the same ones, so a Session's rows are not read again for a new closure. */
  private repository(): (manifestId: string | undefined) => string {
    const from = this.inputs?.holds.repositories;
    const held = this.repositoryOf;
    if (held !== undefined && held.from === from) return held.read;
    const read = (manifestId: string | undefined): string => manifestId === undefined ? "" : manifestLabel(manifestId, from ?? []);
    this.repositoryOf = { from, read };
    return read;
  }

  private readonly beside = (id: string): Beside => ({
    picture: this.pictureOf,
    sketches: this.sketches.get(id) ?? NONE,
    pending: this.pending.get(id) ?? NONE,
    repository: this.repository(),
    epoch: this.epoch,
    workflows: this.inputs?.holds.workflows ?? NONE,
    events: this.events,
    ...(this.inputs?.sessionWatches[id] === undefined ? {} : { watches: this.inputs.sessionWatches[id] }),
    ...(this.inputs?.workflowsOff === true ? { workflowsOff: true } : {}),
    ...(this.unpicked.has(id) ? { unpicked: this.unpicked.get(id)! } : {}),
  });

  /** The timeline's entries, of which only a value he changed in a panel is a row of a thread. Drawn again only when one of those changed. */
  readonly setEvents = (events: readonly TimelineEvent[]): void => {
    const changes = events.filter((one) => one.kind === "session.workflow" || one.kind === "session.model" || one.kind === "session.effort");
    if (changes.length === this.events.length && changes.every((one, at) => one.id === this.events[at]?.id)) return;
    this.events = changes;
    this.recompute();
  };

  /** The workflows Fleet holds as the picker lists them, the same array until Fleet's list changes. */
  private workflowChoices(): readonly WorkflowChoice[] {
    const held = this.inputs?.holds.workflows ?? NONE;
    if (held !== this.choices.from) this.choices = { from: held, are: choicesOf(held) };
    return this.choices.are;
  }

  /** The proposer's pick of a value is no longer his once the person has changed it: drawn so at once, and Fleet says so on the record. */
  private unpick(id: string, value: ProposedField): void {
    this.unpicked.set(id, new Set([...(this.unpicked.get(id) ?? []), value]));
    this.recompute();
  }

  /** A picture a message carried, once read. Read on first ask and drawn from then on. */
  private picture(sessionId: string, fileId: string): string | undefined {
    const key = `${sessionId}\n${fileId}`;
    const held = this.pictures.get(key);
    if (held !== undefined || this.fetching.has(key)) return held;
    this.fetching.add(key);
    void this.api.readSessionFile(sessionId, fileId).then((read) => {
      if (!read.ok) return;
      this.pictures.set(key, URL.createObjectURL(new Blob([read.bytes as BlobPart], { type: read.type })));
      this.epoch += 1;
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

  /** A refused act on one Session is a toast, as a failed annotation send is. A terminal that is not listening says what to run. `act` is what was sent, for a Session started from the toast. */
  private tell(outcome: Parameters<typeof refusalWords>[0], act?: RefusedAct): void {
    const unreachable = !outcome.ok && outcome.why === "refused" && outcome.error.code === "fleet.terminal_session_unreachable";
    askToTell(unreachable ? NOT_REACHABLE : (moveRefusalWords(outcome) ?? refusalWords(outcome)), act);
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

  /** The queue as it now stands, in this window and in storage, and whoever reads it told. */
  private keepQueued(next: readonly QueuedIdea[]): void {
    this.queuedNow = next;
    this.listeners.forEach((listener) => listener());
  }

  /** Read from storage first, since another window may have changed the queue since this one last did. */
  private changeQueued(change: (ideas: readonly QueuedIdea[]) => readonly QueuedIdea[]): void {
    const next = change(readQueuedIdeas());
    writeQueuedIdeas(next);
    this.keepQueued(next);
  }

  /** Starts the Session an idea describes. The idea leaves the queue once the Session has an id, and a start Fleet refused keeps it. */
  private async startingQueued(id: string): Promise<string | undefined> {
    const idea = readQueuedIdeas().find((one) => one.id === id);
    // A second press while the first is out would start the idea twice.
    if (idea === undefined || this.starts.has(id)) return undefined;
    this.starts.add(id);
    const tuning = { ...(idea.model === undefined ? {} : { model: idea.model }), ...(idea.effort === undefined ? {} : { effort: idea.effort }) };
    const started = await this.starting(undefined, idea.root, idea.text, tuning).finally(() => this.starts.delete(id));
    if (started === undefined) return undefined;
    this.changeQueued((ideas) => ideas.filter((one) => one.id !== id));
    void this.sending(started, { text: idea.text, files: [], sketches: [], tags: [] });
    return started;
  }

  /** The Session the words tag and the read held for them, made on first sight. */
  private waitReadOf(words: string): { read: WaitRead; tagged: Session } | undefined {
    const tag = taggedIn(words, this.mapped);
    if (tag === undefined) return undefined;
    const key = `${tag.session.id}\n${words}`;
    let read = this.waitReads.get(key);
    if (read === undefined) {
      read = { sessionId: tag.session.id, words, reading: undefined, timer: undefined, out: undefined, answered: false };
      this.waitReads.set(key, read);
      for (const old of [...this.waitReads.keys()].slice(0, Math.max(0, this.waitReads.size - WAIT_READS_KEPT))) this.waitReads.delete(old);
    }
    return { read, tagged: tag.session };
  }

  /** The reading as the draft carries it: the Session tagged, the condition with its plan part titled, and where it starts from. */
  private waitReading(read: WaitRead, tagged: Session): WaitReading {
    const on = { id: tagged.id, title: tagged.title ?? tagged.id };
    if (read.reading !== undefined) return read.reading;
    return { on, when: "ends", base: defaultBase("ends"), needs: "" };
  }

  /** Asks Fleet's proposer once, and tells whoever reads when it has answered. A reading it could not make leaves the defaults. */
  private asking(read: WaitRead, tagged: Session): Promise<void> {
    if (read.out !== undefined) return read.out;
    if (read.timer !== undefined) clearTimeout(read.timer);
    read.timer = undefined;
    read.out = this.api.readSessionWait(read.sessionId, read.words).then((done) => {
      if (done.ok) {
        const { when, part } = whenOfCondition(done.value.condition, planPartsOf(tagged));
        read.reading = { on: { id: tagged.id, title: tagged.title ?? tagged.id }, when, ...(part === undefined ? {} : { part }), base: done.value.base, needs: done.value.needs ?? "", ...(done.value.plain === true ? { plain: true } : {}) };
      }
      read.answered = true;
      this.listeners.forEach((listener) => listener());
    });
    return read.out;
  }

  /** What the words name once the proposer has answered, with it asked once the words pause. Nothing until then: a tag may name no wait, and a sentence drawn on the defaults would be taken back. */
  private readingWait(words: string): WaitReading | undefined {
    const found = this.waitReadOf(words);
    if (found === undefined) return undefined;
    const { read, tagged } = found;
    this.waitLatest = read;
    if (read.out === undefined && read.timer === undefined) {
      read.timer = setTimeout(() => {
        read.timer = undefined;
        if (this.waitLatest === read) void this.asking(read, tagged);
      }, WAIT_READ_AFTER_MS);
    }
    return read.answered ? this.waitReading(read, tagged) : undefined;
  }

  /** What the words name once the proposer has answered, for the send to start on. */
  private async settledWait(words: string): Promise<WaitReading | undefined> {
    const found = this.waitReadOf(words);
    if (found === undefined) return undefined;
    await this.asking(found.read, found.tagged);
    return this.waitReading(found.read, found.tagged);
  }

  /** The wire's `after` for a wait the screens drew: a plan part is a group or a task by what the plan of the Session waited on says. */
  private afterOf(after: Pick<SessionWait, "on" | "when" | "part" | "base" | "needs">): StartAfter {
    const parts = planPartsOf(this.mapped.find((one) => one.id === after.on.id));
    return { session: after.on.id, condition: conditionOfWhen(after.when, after.part, parts), base: after.base, ...(after.needs === "" ? {} : { needs: after.needs }) };
  }

  /** Changes when a waiting Session starts. Fleet tells the Session it waits on and writes a row in both threads; a refusal is a toast. */
  private async changingWait(id: string, when: StartWhen, part?: { id: string; title: string }): Promise<void> {
    const waiting = this.mapped.find((one) => one.id === id);
    if (waiting?.waits === undefined) return;
    const parts = planPartsOf(this.mapped.find((one) => one.id === waiting.waits!.on.id));
    const done = await this.api.changeWaiting({ waiting_session: id, condition: conditionOfWhen(when, part, parts) });
    if (!done.ok) this.tell(done.outcome);
  }

  private async startingNow(id: string): Promise<void> {
    const done = await this.api.startWaitingNow(id);
    if (!done.ok) this.tell(done.outcome);
  }

  private async starting(tag: SessionTag | undefined, root: string | undefined, request?: string, tuning?: { model?: string; effort?: string }, after?: Pick<SessionWait, "on" | "when" | "part" | "base" | "needs">): Promise<string | undefined> {
    this.say(undefined);
    // A start that waits on nothing is the call it always was.
    const done = after === undefined ? await this.api.startSession(undefined, root, request, tuning) : await this.api.startSession(undefined, root, request, tuning, this.afterOf(after));
    if (!done.ok) return void this.say(done.outcome);
    const id = done.value.id;
    if (tag !== undefined) this.pending.set(id, [tag]);
    await this.held(id);
    this.recompute();
    return id;
  }

  private async sending(id: string, sent: Parameters<SessionsDraft["send"]>[1]): Promise<boolean> {
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
      this.tell(done.outcome);
      return false;
    }
    markViewed(id);
    // The wire holds a sketch only as the picture it went as, so the ledger keeps what was drawn here.
    if (sent.sketches.length > 0) {
      this.sketches.set(id, [...(this.sketches.get(id) ?? NONE), ...sent.sketches.map((one): Sketch => ({ kind: "sketch", id: one.id, title: one.title, by: "you", drawing: one.drawing }))]);
      this.recompute();
    }
    return true;
  }

  /** One call: a new Session starts as a copy of a dead one's conversation. Resolves to it once the window holds it. */
  private async forking(id: string): Promise<string | undefined> {
    this.say(undefined);
    const done = await this.api.forkSession(id);
    if (!done.ok) return void this.say(done.outcome);
    await this.held(done.value.id);
    return done.value.id;
  }

  /** One call: a group of the plan goes to a Session of its own, which Fleet starts and briefs. Resolves to it once the window holds it; a refusal is a toast. */
  private async handing(id: string, group: string): Promise<string | undefined> {
    this.say(undefined);
    const done = await this.api.handToSession(id, group);
    if (!done.ok) return void this.tell(done.outcome);
    await this.held(done.value.session_id);
    return done.value.session_id;
  }

  /** One call: the Session ends and starts again in another repository. Resolves to the new one once the window holds it; a refusal is a toast. */
  private async moving(id: string, manifestId: string): Promise<string | undefined> {
    this.say(undefined);
    const done = await this.api.moveSession(id, manifestId);
    if (!done.ok) return void this.tell(done.outcome);
    await this.held(done.value.id);
    return done.value.id;
  }

  /** One call: Fleet stops sending the notice for the scope. A refusal is a toast. */
  private async mutingNotice(notice: Notice, scope: NoticeScope): Promise<void> {
    const done = await this.api.muteNotice({ notice, scope, muted: true });
    if (!done.ok) this.tell(done.outcome);
  }

  /** One call; a refusal is the toast `stop`'s is. Both Sessions' rows change by `session.changed`. */
  private async releasing(id: string, to: string | undefined, rows: readonly { kind: string; target: string }[] | undefined): Promise<HoldsReleased | undefined> {
    this.say(undefined);
    const done = await this.api.releaseSessionHolds(id, to, rows === undefined ? undefined : rows.map(({ kind, target }) => ({ kind, target })));
    if (!done.ok) return void this.tell(done.outcome);
    return holdsReleasedOf(done.value);
  }

  /** As `forking`, from one message of a Session that may still be running. */
  private async forkingAt(id: string, at: string): Promise<string | undefined> {
    this.say(undefined);
    const done = await this.api.forkSessionAt(id, at);
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

  private async acting(id: string, number: number, act: "ready" | "merge" | "auto_merge"): Promise<boolean> {
    this.say(undefined);
    const done = await this.api.pressPullRequest(id, number, act);
    if (!done.ok) this.say(done.outcome);
    // Whatever the answer, the sheet shows the pull request as the forge has it now; the press is not held for it.
    void this.api.pressPullRequest(id, number, "read");
    return done.ok;
  }

  /** A refusal is a toast, since the Manifest form and the deck have no Sessions view to show it in. */
  private async actingInRepository(root: string, number: number, act: "ready" | "merge" | "auto_merge"): Promise<boolean> {
    const manifestId = this.inputs?.holds.repositories?.find((one) => one.root === root)?.manifest?.id;
    if (manifestId === undefined) {
      this.tell({ ok: false, why: "no_manifest" });
      return false;
    }
    const done = await this.api.pressRepositoryPullRequest(manifestId, number, act);
    if (!done.ok) this.tell(done.outcome);
    return done.ok;
  }

  /** One call. Success is the chip, its card and the ledger row changing by Fleet's events; a refusal is a toast. */
  private async refreshing(manifestId: string | undefined, number: number): Promise<void> {
    if (manifestId === undefined) return this.tell({ ok: false, why: "no_manifest" });
    const done = await this.api.refreshPullRequest(manifestId, number);
    if (!done.ok) this.tell(done.outcome);
  }

  private async plain(act: Promise<{ ok: true } | { ok: false; outcome: Parameters<typeof refusalWords>[0] }>, sent?: RefusedAct): Promise<void> {
    this.say(undefined);
    const done = await act;
    if (!done.ok) this.tell(done.outcome, sent);
  }

  /** As `plain`, resolving to whether Fleet took it. */
  private async took(act: Promise<{ ok: true } | { ok: false; outcome: Parameters<typeof refusalWords>[0] }>): Promise<boolean> {
    this.say(undefined);
    const done = await act;
    if (!done.ok) this.tell(done.outcome);
    return done.ok;
  }

  /** The forge's refusal comes after Fleet kept the flag, so it is told and still counts as changed. */
  private async landing(id: string, on: boolean): Promise<boolean> {
    this.say(undefined);
    const done = await this.api.setSessionAutoLand(id, on);
    if (done.ok) return true;
    this.tell(done.outcome);
    const { outcome } = done;
    return !outcome.ok && outcome.why === "refused" && outcome.error.code === "fleet.auto_merge_refused";
  }

  private readonly lookup = (id: string): Session | undefined => this.mapped.find((one) => one.id === id);

  readonly draft: SessionsDraft = this.build();

  /**
   * A shell block run, on a terminal. Messages are listened for before the run starts and held until its id
   * is known, so output that beat the answer is not lost; so are the keys and the size the pane gave in the
   * meantime. The terminal's bytes go on as they came, escapes and all: the pane draws them. A refusal ends
   * the run with code 1 and its words as one line; a stop ends it with `null`, which is how Fleet reports a
   * stopped process group.
   */
  private running(
    id: string,
    command: string,
    on: (event: { kind: "out"; text: string } | { kind: "end"; code: number | null }) => void,
    size?: { cols: number; rows: number },
  ): { stop: () => void; write: (data: string) => void; resize: (cols: number, rows: number) => void } {
    const change = buildChangeOf(command);
    if (change !== undefined) return { ...this.switching(change, on), write: () => undefined, resize: () => undefined };
    let runId: string | null = null;
    let over = false;
    const held: { runId: string; message: SessionRunMessage }[] = [];
    let typed = "";
    let resized: { cols: number; rows: number } | null = null;
    const finish = (code: number | null): void => {
      if (over) return;
      over = true;
      off();
      on({ kind: "end", code });
    };
    const take = (from: string, message: SessionRunMessage): void => {
      if (over || from !== runId) return;
      if (message.kind === "output") {
        if (message.text !== "") on({ kind: "out", text: message.text });
      } else if (message.kind === "exited") finish(message.code);
    };
    const off = this.api.onSessionRunMessage((from, message) => {
      if (runId === null) held.push({ runId: from, message });
      else take(from, message);
    });
    void this.api.startSessionRun(id, command, size?.cols, size?.rows).then((started) => {
      if (over) return;
      if (!started.ok) {
        on({ kind: "out", text: `${refusalWords(started.outcome)}\r\n` });
        finish(1);
        return;
      }
      runId = started.value.run_id;
      for (const one of held.splice(0)) take(one.runId, one.message);
      if (resized !== null) void this.api.resizeSessionRun(runId, resized.cols, resized.rows);
      if (typed !== "") void this.api.writeSessionRun(runId, typed);
    });
    return {
      stop: () => {
        if (runId === null) return finish(null);
        void this.api.stopSessionRun(runId);
      },
      write: (data) => {
        if (over) return;
        if (runId === null) typed += data;
        else void this.api.writeSessionRun(runId, data);
      },
      resize: (cols, rows) => {
        if (over) return;
        if (runId === null) resized = { cols, rows };
        else void this.api.resizeSessionRun(runId, cols, rows);
      },
    };
  }

  /** A restart onto main or the preview, pressed as the Fleet panel presses it; where it stands is the panel's to show. */
  private switching(change: BuildChange, on: (event: { kind: "out"; text: string } | { kind: "end"; code: number | null }) => void): { stop: () => void } {
    void this.api.changeFleetBuild(change.build, change.adopt).then((done) => {
      on({ kind: "out", text: `${done.ok ? change.label : refusalWords(done)}\r\n` });
      on({ kind: "end", code: done.ok ? 0 : 1 });
    });
    return { stop: () => undefined };
  }

  private build(): SessionsDraft {
    // A getter's `this` is the object it is on, so the models read through the store.
    const store = this;
    return {
    get: () => this.mapped,
    helmOf: (manifestId) => (this.helm?.manifestId === manifestId ? this.helm : undefined),
    subscribe: this.subscribe,
    start: (tag, root, request, tuning, after) => this.starting(tag, root, request, tuning, after),
    readsWait: (words) => this.readingWait(words),
    settlesWait: (words) => this.settledWait(words),
    startWhen: (id, when, part) => void this.changingWait(id, when, part),
    startNow: (id) => void this.startingNow(id),
    queued: () => this.queuedNow,
    queue: (idea) => this.changeQueued((ideas) => [...ideas, { id: newIdeaId(), ...idea }]),
    startQueued: (id) => this.startingQueued(id),
    dropQueued: (id) => this.changeQueued((ideas) => ideas.filter((one) => one.id !== id)),
    fork: (id) => this.forking(id),
    handOff: (id, group) => this.handing(id, group),
    forkAt: (id, at) => this.forkingAt(id, at),
    move: (id, manifestId) => this.moving(id, manifestId),
    muteNotice: (notice, scope) => this.mutingNotice(notice, scope),
    retro: (id) => this.retroing(id),
    watch: (id) => void this.api.watchSession(id),
    subagent: async (id, subagentId) => {
      const read = await this.api.readSessionSubagent(id, subagentId);
      if (!read.ok) return undefined;
      const { rows, finished, report, model, context_tokens, context_window } = read.value;
      return {
        rows: rowsOfThread(id, rows, () => undefined),
        finished,
        ...(report === undefined ? {} : { report }),
        ...(model === undefined ? {} : { model }),
        ...(context_tokens === undefined || context_window === undefined ? {} : { context: { tokens: context_tokens, window: context_window } }),
      };
    },
    loadout: async (id) => {
      const read = await this.api.readSessionLoadout(id);
      return read.ok ? read.value : undefined;
    },
    history: (id) => readHistory(this.api, id),
    loadoutItem: async (id, item) => {
      const read = await this.api.readLoadoutItem(id, item);
      return read.ok ? read.value : undefined;
    },
    mcp: async (id) => {
      const read = await this.api.readSessionMcp(id);
      return read.ok ? { ok: true, servers: read.value.servers } : { ok: false, said: refusalWords(read.outcome) };
    },
    actOnMcp: async (id, server, act) => {
      const done = await this.api.actOnSessionMcp(id, server, act);
      if (!done.ok) return { ok: false, said: refusalWords(done.outcome) };
      return { ok: true, servers: done.value.servers, ...(done.value.sign_in === undefined ? {} : { signIn: done.value.sign_in }) };
    },
    close: (id) => void this.plain(this.api.closeSession(id)),
    stop: (id) => void this.plain(this.api.stopSession(id)),
    // The person's Drop: Fleet settles it as dropped and publishes the Session without it.
    dropLater: (id, later) => void this.plain(this.api.settleLater({ session_id: id, id: later, how: "dropped" })),
    stopWatch: (id, watch) => void this.plain(this.api.stopWatch({ session_id: id, watch })),
    retryHandoff: (id) => void this.plain(this.api.retryHandoff(id)),
    pause: (id) => this.took(this.api.pauseSession(id)),
    resume: (id) => this.took(this.api.resumeSession(id)),
    autoLand: (id, on) => this.landing(id, on),
    releaseHolds: (id, to, rows) => this.releasing(id, to, rows),
    rename: (id, title) => void this.plain(this.api.renameSession({ session_id: id, title })),
    refresh: (id, number) => void this.api.pressPullRequest(id, number, "read"),
    pullRequestPage: async (id, number) => pageRead(await this.api.readPullRequestPage(id, number)),
    pullRequestFile: async (id, number, path) => fileRead(await this.api.readPullRequestFile(id, number, path)),
    clashFile: async (id, other, path) => clashFileRead(await this.api.readClashFile(id, other, path)),
    fileChange: async (id, path, seen) => fileChangeRead(await this.api.readSessionFileChange(id, path, seen)),
    writePullRequest: async (id, number, write) => pageRead(await this.api.writePullRequest(id, number, write)),
    openFile: (id, path) => void this.api.openSessionFile(id, path),
    openWindow: (id, url) => void this.api.openSessionWindow(id, url),
    readArtifact: (id, path) => this.api.readSessionArtifact(id, path),
    page: {
      show: (id, address, bounds) => void this.api.showSessionPage(id, address, bounds),
      move: (bounds) => void this.api.moveSessionPage(bounds),
      hide: () => void this.api.hideSessionPage(),
      onEscape: (on) => this.api.onSessionPageEscape(on),
      snapshot: {
        watch: (id, address) => void this.api.watchSessionSnapshot(id, address),
        stop: () => void this.api.stopSessionSnapshot(),
        onShot: (on) => this.api.onSessionSnapshot(on),
      },
    },
    run: (id, command, on, size) => this.running(id, command, on, size),
    terminalRuns: true,
    // The card's command is Fleet's to run: the repository is named by its Manifest and the string is not sent.
    runInRepository: (root, _command, on) =>
      catchingUp((manifestId) => this.api.catchUpCheckout(manifestId), this.inputs?.holds.repositories?.find((one) => one.root === root)?.manifest?.id, on),
    // The files go up as a pull request by the repository's Manifest alone; Fleet reads the hold again and sends what it lists.
    sendCheckoutAsPullRequest: async (root) => {
      const manifestId = this.inputs?.holds.repositories?.find((one) => one.root === root)?.manifest?.id;
      if (manifestId === undefined) return void askToTell("Fleet serves no Manifest for that repository.");
      this.say(undefined);
      const done = await this.api.sendCheckoutAsPullRequest(manifestId);
      if (!done.ok) return void this.tell(done.outcome);
      return {};
    },
    // The file is read by its Manifest and path; Fleet refuses one the hold does not list.
    diffInRepository: async (root, path) => {
      const manifestId = this.inputs?.holds.repositories?.find((one) => one.root === root)?.manifest?.id;
      if (manifestId === undefined) return { ok: false, said: "Fleet serves no Manifest for that repository." };
      return checkoutDiffRead(await this.api.readCheckoutDiff(manifestId, path));
    },
    // A pull request no Session holds is named by its repository's Manifest; Fleet reads the hub again after a merge.
    actInRepository: (root, number, act) => this.actingInRepository(root, number, act),
    refreshPullRequest: (manifestId, number) => this.refreshing(manifestId, number),
    said: () => this.refusal,
    setTags: (id, tags) => this.tagged(id, tags),
    send: (id, sent) => this.sending(id, sent),
    tune: (id, tuning) =>
      void this.plain(
        this.api.tuneSession({
          session_id: id,
          ...(tuning.model === null ? {} : { model: tuning.model }),
          ...(tuning.effort === null ? {} : { effort: tuning.effort }),
          mode: tuning.mode,
        }),
      ),
    command: (id, text) => void this.plain(this.api.runSessionCommand(id, text)),
    act: (id, number, act) => this.acting(id, number, act),
    get models() {
      return store.inputs?.holds.models?.models ?? NONE;
    },
    get defaultModel() {
      return store.inputs?.holds.models?.session;
    },
    efforts: ["low", "medium", "high", "xhigh", "max"],
    work: {
      get workflows() {
        return store.workflowChoices();
      },
      choose: (id, workflow) => {
        store.unpick(id, "workflow");
        void this.plain(this.api.setSessionWorkflow(id, workflow ?? undefined));
      },
      unpick: (id, value) => this.unpick(id, value),
      // Answers whether Fleet took it, so a refused Approve reverts the plan drawn as approved (`pressed.ts`).
      approve: (id) => {
        const version = this.lookup(id)?.work?.plan?.version;
        if (version !== undefined) return this.took(this.api.approvePlan({ session_id: id, version }));
      },
      // Not yet is the Needs you row's own dismissal of the plan's item, `plan:<version>`; Fleet then carries `set_aside_at` on the plan.
      setAside: (id) => {
        const version = this.lookup(id)?.work?.plan?.version;
        if (version !== undefined) void this.plain(this.api.dismissWaiting({ session_id: id, item_id: `plan:${version}` }));
      },
      suggest: (id, marks, approving) => {
        const version = this.lookup(id)?.work?.plan?.version;
        if (version === undefined) return;
        void this.plain(this.api.sendPlanChanges({ session_id: id, version, marks: wireMarksOf(marks), ...(approving === true ? { approve: true } : {}) }));
      },
      sendConcern: (id, concern) => void this.plain(this.api.sendConcern({ session_id: id, item_id: concern })),
      dismissConcern: (id, concern) => void this.plain(this.api.dismissConcern({ session_id: id, item_id: concern })),
      // The answer is the record with `judging` set, which the state carries; the reading lands on it later.
      judge: async (id) => {
        this.say(undefined);
        const done = await this.api.judgeSession({ session_id: id });
        if (!done.ok) this.tell(done.outcome);
      },
    },
    get commands() {
      return store.commandsRead();
    },
    answerWaiting: (id: string, itemId: string, given: { choice?: number; text?: string; mode?: "best" | "quick" }) => {
      const request = { session_id: id, item_id: itemId, ...given };
      return this.plain(this.api.answerWaiting(request), { act: "answer_waiting", request });
    },
    answer: (id: string, answer?: SessionAnswer, answers?: SessionQuestionAnswer[], folder?: string) => {
      const asked = this.lookup(id)?.asked;
      const call = asked?.call;
      if (call === undefined) return;
      const request = {
        session_id: id,
        call,
        answer: answer ?? "allow_once",
        ...(answers === undefined ? {} : { answers }),
        ...(folder === undefined ? {} : { folder }),
      };
      // What the card put to him, so a refusal can be read against it.
      const card = { ask: asked?.command, ask_reaches: asked?.reaches, ask_reaches_to: asked?.reachesTo, ask_offers: asked?.offers?.join(",") };
      void this.plain(this.api.answerSessionAsk(request), { act: "answer_session_ask", request, card });
    },
    };
  }
}

/** Sessions over a real Fleet, or over a fake one answering the same capabilities. */
export function WiredSessions({ events, children }: { events?: TimelineSource; children: ReactNode }) {
  const store = useMemo(() => new WiredStore(window.armada), []);
  useEffect(() => store.attach(), [store]);
  useEffect(() => {
    if (events === undefined) return;
    store.setEvents(events.get());
    return events.subscribe(() => store.setEvents(events.get()));
  }, [store, events]);
  const served = useSyncExternalStore(store.subscribe, store.served);
  const { runInRepository, diffInRepository, actInRepository, refreshPullRequest, sendCheckoutAsPullRequest } = store.draft;
  const acts = useMemo(
    () => ({ runInRepository, diffInRepository, actInRepository, refreshPullRequest, sendCheckoutAsPullRequest }),
    [runInRepository, diffInRepository, actInRepository, refreshPullRequest, sendCheckoutAsPullRequest],
  );
  return (
    <SessionsFrom held={served ? store.draft : undefined} acts={acts}>
      {children}
    </SessionsFrom>
  );
}

/** Fleet's answer, as the deck draws it. */
export function holdsReleasedOf(wire: SessionHoldsReleased): HoldsReleased {
  const row = (one: SessionHoldsReleased["moved"][number]) => ({
    kind: one.kind,
    target: one.target,
    ...(one.released === undefined || one.released === null
      ? {}
      : { saved: { branch: one.released.branch, files: one.released.saved?.files ?? [], ...(one.released.saved?.commit === undefined ? {} : { commit: one.released.saved.commit }) } }),
  });
  return {
    ...(wire.to_session_id === undefined ? {} : { to: wire.to_session_id }),
    moved: wire.moved.map(row),
    kept: wire.kept.map((one) => ({ kind: one.kind, target: one.target, why: one.why })),
  };
}
