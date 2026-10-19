// A Fleet that serves Sessions, for a window to talk to as it talks to the real one.
//
// **Not the mock's Sessions** (`sessions/script.ts`), which are fixtures behind the draft seam. This
// stands where main stands: it holds the sessions and the threads, answers the capabilities the
// preload carries, and publishes what main would publish. The window above it is the real one, so a
// test through `App` reaches the same code a person's press does. What Fleet does on its own clock —
// an agent's reply, a row, a pull request's Checks moving — is a method the test calls.

import type {
  AnswerSessionAsk,
  AnswerWaiting,
  ChangeWaiting,
  DismissWaiting,
  Attachment,
  ClaimPullRequest,
  AttachmentState,
  HelmCallInFlight,
  PullRequestState,
  SendSessionMessage,
  SessionRecord,
  SessionRow,
  SessionAskState,
  PartialRetroItem,
  RenameSession,
  SessionSubagent,
  SessionLoadout,
  StartAfter,
  TuneSession,
  WaitReading,
} from "@armada/protocol";

import { withThreadRow } from "@armada/screens/src/retro-row";
import type { LoadoutItemBody } from "@armada/screens/src/draft/sessions";
import { loadoutKeyOf } from "@armada/components";
import { REACH_FOLDERS_KEY, reachFoldersOf } from "@armada/protocol";
import { savedInto } from "@armada/settings/fake";

import type { SessionActed } from "../../../shared/api/sessions";
import { retroFrames } from "./session-retro-script";
import { removedSaid } from "./sessions/removed";
import { onTimePassing } from "./time-passes";
import type { Scenario } from "./moment";
import type { FleetHandle } from "./moment";

const AT = "2026-10-07T13:48:02.000Z";

/** A picture that decodes. A sent message draws what it carried, and a broken one raises an error the walks hear. */
const ONE_PIXEL = Uint8Array.from(atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="), (c) => c.charCodeAt(0));

/** An attachment as Fleet writes it. */
export function held(kind: string, target: string, detail: Record<string, string> = {}, state: AttachmentState = "standing"): Attachment {
  return { kind, manifest_id: "armada", target, state, detail, since: AT, changed_at: AT };
}

/** A session Fleet hosts, idle, with nothing leased. */
export function hosted(id: string, change: Partial<SessionRecord> = {}): SessionRecord {
  return {
    id,
    harness: "a_harness",
    origin: "bridge",
    manifest_id: "armada",
    cwd: "/Users/user/code/armada",
    state: "live",
    started_at: AT,
    last_seen_at: AT,
    usage: {},
    attachments: [],
    hosted: { turn: { state: "idle" }, mode: "auto", running: false },
    ...change,
  };
}

/** A session from a person's terminal: on the ledger, and with no thread. */
export function terminal(id: string, change: Partial<SessionRecord> = {}): SessionRecord {
  const { hosted: _hosted, ...rest } = hosted(id, change);
  // A terminal whose mod is asking, as one a person is at: it takes a message, and offers no Fork.
  return { terminal: { listening: true }, ...rest, origin: "terminal" };
}

/** A call the agent is held on, with the three offers. */
export function asking(command: string, call = "call-1"): HelmCallInFlight {
  return {
    call,
    manifest_id: "armada",
    asked_at: AT,
    tool: "Bash",
    detail: command,
    truncated: false,
    rule: `Bash(${command}:*)`,
    offers: ["allow_once", "allow_and_remember", "refuse"],
    holding_for_seconds: 600,
  };
}

/** The pull request as the forge shows it, in the words the ledger's `detail` carries. */
export const pullRequest = (number: number, detail: Partial<Record<string, string>> = {}): Attachment =>
  held("pr", String(number), {
    state: "open",
    auto_merge: "false",
    checks: "passed",
    failing: "",
    title: "Fix the flaky store test",
    branch: "fix/flaky-store",
    address: `https://forge.example/pull/${number}`,
    ...(detail as Record<string, string>),
  });

export type Calls = {
  started: number;
  /** The repository root each start named, in order; absent where the window named none. */
  startedIn: (string | undefined)[];
  /** The wait each start carried, in order: `after` as the window sent it. */
  startedAfter: StartAfter[];
  /** The briefs read for a wait, and the Session each tagged. */
  readWaits: { sessionId: string; request: string }[];
  changedWaits: ChangeWaiting[];
  /** The waiting Sessions started now. */
  startedNow: string[];
  sent: SendSessionMessage[];
  answered: AnswerSessionAsk[];
  waited: AnswerWaiting[];
  claimed: ClaimPullRequest[];
  dismissed: DismissWaiting[];
  tuned: TuneSession[];
  renamed: RenameSession[];
  closed: string[];
  /** The Sessions whose running turn was stopped. */
  stopped: string[];
  forked: string[];
  /** The plan groups handed to a Session of their own, and the Session that handed each. */
  handed: { sessionId: string; group: string }[];
  /** The Sessions forked from one message, and the message. */
  forkedAt: { sessionId: string; at: string }[];
  /** The Sessions moved to another repository, and where. */
  moved: { sessionId: string; manifestId: string }[];
  /** The Sessions the Retro press was pressed on. */
  retroed: string[];
  pressed: { sessionId: string; number: number; press: string }[];
  watched: string[];
  /** The addresses a page view was shown on, and how many times it was taken away. */
  pages: string[];
  pagesHidden: number;
};

export class FakeSessionsFleet {
  /** Who hears Esc pressed in the page's view. */
  private readonly escape = new Set<() => void>();
  /** Esc, pressed while the page's view has focus. */
  pageEscape(): void {
    this.escape.forEach((on) => on());
  }
  readonly calls: Calls = { started: 0, startedIn: [], startedAfter: [], readWaits: [], changedWaits: [], startedNow: [], sent: [], answered: [], waited: [], claimed: [], dismissed: [], tuned: [], renamed: [], closed: [], stopped: [], forked: [], handed: [], forkedAt: [], moved: [], retroed: [], pressed: [], watched: [], pages: [], pagesHidden: 0 };
  /** What `readSessionWait` answers: the defaults, unless a test says otherwise. */
  reading: WaitReading = { condition: { kind: "ends" }, base: "main" };
  private records: SessionRecord[];
  private threads: Record<string, SessionRow[]>;
  private fleet: FleetHandle | undefined;
  private repositories: readonly { root: string; manifest?: { id: string } }[] = [];
  /** Set to have the next message refused, the way Fleet refuses one: the error says why and nothing is sent. */
  refusesSend: { code: string; message: string } | undefined;
  /** Set to have the next End session refused, the way Fleet refuses one: nothing ends and nothing is parked. */
  refusesClose: { code: string; message: string } | undefined;
  /** Set to have the next move refused, the way Fleet refuses one: nothing ends and nothing starts. */
  refusesMove: { code: string; message: string } | undefined;
  /** Set to have the next retro refused the way Fleet refuses one: the error says why and nothing is written. */
  refusesRetro: { code: string; message: string } | undefined;
  /** How long each frame of a retro's writing takes, so a person can see the row move. */
  retroStepMs = 400;
  /** Set to hold a retro at each frame a walk stops on until time passes (a walk's `later`), the others following at once. */
  retroHeld = false;
  /** The items the model writes into a Session's retro, as they settle. None: the stages go by and the retro is kept. */
  retroItems: readonly PartialRetroItem[] = [];
  /** Hears a retro being written, to put it where the Retros page reads from. Answers the number it was kept under. */
  onRetro: ((sessionId: string, title: string | undefined) => number | void) | undefined;
  /** The Sessions a retro is being written for, which Fleet refuses a second press on. */
  /** Set to have a press answered with the Session's last retro and no row, as Fleet does where nothing is new since it. */
  retroNothingNew = false;
  private readonly writingRetro = new Set<string>();
  /** The `retro` rows of Sessions whose thread no window holds, which `get_session` brings when one is opened. */
  private readonly keptRetro: Record<string, SessionRow[]> = {};
  private retroRowed = 0;
  private releaseRetro: (() => void) | undefined;
  /** Set to have the next act on a pull request refused, the way Fleet refuses one: the error says why and nothing changes. */
  refusesPress: { code: string; message: string } | undefined;
  /** Each subagent's thread as its transcript stands, by subagent id. A test changes it to let one run on. */
  subagents: Record<string, SessionSubagent> = {};
  /** Each Session's whole Loadout, by Session id, as `get_session_loadout` answers it. */
  loadouts: Record<string, SessionLoadout> = {};
  /** What each Loadout item says, by Session id and then `loadoutKeyOf`, as `read_loadout_item` answers. An item not here was not found. */
  loadoutItems: Record<string, Record<string, LoadoutItemBody>> = {};
  /** Every Loadout item asked for, in order. */
  readLoadoutItems: string[] = [];
  /** Hears each message the person sends, to script what the agent does with it. */
  onSend: ((send: SendSessionMessage) => void) | undefined;
  /** A Session this Fleet starts or forks says it began now, where the fixtures' fixed instant would age it by days. */
  startsNow = false;
  /** A row for a thread nobody has opened is kept for when it is, as Fleet keeps every thread, and not dropped as main drops it. */
  keepsRows = false;
  /** The threads whose first read is held, by Session id: none, and every read answers at once. `holdRead` sets one. */
  private readonly heldReads = new Map<string, { ms: number; thread: SessionRow[]; cached?: SessionRow[] }>();
  /** The ids whose read has not answered, as main publishes them. */
  private readingIds: string[] = [];
  private minted = 0;
  private rowed = 0;

  constructor(records: SessionRecord[] = [], threads: Record<string, SessionRow[]> = {}) {
    this.records = records;
    this.threads = threads;
  }

  private began(): Partial<SessionRecord> {
    return this.startsNow ? { started_at: new Date().toISOString() } : {};
  }

  /** `base`, with Sessions served: what main publishes once Fleet has answered `list_sessions`, and the capabilities over them. */
  scenario(base: Scenario): Scenario {
    this.repositories = base.state.holds.repositories ?? [];
    return {
      ...base,
      state: { ...base.state, sessions: { state: "read", sessions: this.records }, sessionThreads: {} },
      behaves: (fleet) => {
        this.fleet = fleet;
        // A walk's `later` lets a held retro go on to its next stop. **After the window's fake has registered its own**
        // (it clears the hook of a scenario with no `later`, right after this returns), and the latest wins.
        if (this.retroHeld) queueMicrotask(() => onTimePassing(() => this.releaseRetro?.()));
        return {
          startSession: async (_title, root, request, _tuning, after) => {
            this.calls.started += 1;
            this.calls.startedIn.push(root);
            if (after !== undefined) this.calls.startedAfter.push(after);
            // Fleet records the Manifest of the repository a Session starts in, and the window narrows by it.
            const manifest_id = this.repositories.find((one) => one.root === root)?.manifest?.id;
            const waits =
              after === undefined
                ? undefined
                : { on: { id: after.session, title: this.records.find((one) => one.id === after.session)?.title ?? after.session }, condition: after.condition ?? { kind: "ends" as const }, base: after.base ?? ("main" as const), brief: request ?? "", by: "you" as const, ...(after.needs === undefined ? {} : { needs: after.needs }) };
            const record = hosted(`01SESSION${String((this.minted += 1)).padStart(8, "0")}ABCDEFGH`, { ...this.began(), ...(manifest_id === undefined ? {} : { manifest_id }), ...(waits === undefined ? {} : { waits }) });
            this.set([record, ...this.records]);
            return { ok: true, value: record };
          },
          readSessionWait: async (sessionId, request) => {
            this.calls.readWaits.push({ sessionId, request });
            return { ok: true, value: this.reading };
          },
          changeWaiting: async (change) => {
            this.calls.changedWaits.push(change);
            const found = this.records.find((one) => one.id === change.waiting_session)?.waits;
            if (found === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
            const waits = { ...found, ...(change.condition === undefined ? {} : { condition: change.condition }), ...(change.base === undefined ? {} : { base: change.base }) };
            this.change(change.waiting_session, (one) => ({ ...one, waits }));
            return { ok: true, value: waits };
          },
          startWaitingNow: async (sessionId) => {
            this.calls.startedNow.push(sessionId);
            return this.change(sessionId, ({ waits: _waits, ...one }) => one);
          },
          sendSessionMessage: async (send) => {
            if (this.refusesSend !== undefined) return { ok: false, outcome: { ok: false, why: "refused", error: this.refusesSend } as never };
            this.calls.sent.push(send);
            this.row(send.session_id, {
              kind: "message",
              id: this.rowId(),
              at: AT,
              from: { kind: "you" },
              text: send.text,
              ...(send.attachments === undefined || send.attachments.length === 0
                ? {}
                : { files: send.attachments.map((one, at) => ({ id: `${this.rowed}-${at}`, name: one.name, media_type: one.media_type })) }),
            });
            // The first message of a Session an agent started: sent, it is no longer unsent and nothing waits on it.
            const sent = this.change(send.session_id, (one) => ({
              ...one,
              attachments: one.attachments.map((a) => (a.kind === "unsent" ? { ...a, state: "spent" as const } : a)),
              waiting_for: (one.waiting_for ?? []).filter((item) => !item.id.startsWith("unsent:")),
              hosted: { ...one.hosted!, turn: { state: "working" } },
            }));
            this.onSend?.(send);
            return sent;
          },
          answerSessionAsk: async (answer) => {
            this.calls.answered.push(answer);
            this.settled(answer);
            return this.change(answer.session_id, (one) => {
              const { asked: _asked, ...rest } = one.hosted!;
              return { ...one, hosted: rest };
            });
          },
          answerWaiting: async (answer) => {
            this.calls.waited.push(answer);
            return this.change(answer.session_id, (one) => ({
              ...one,
              waiting_for: (one.waiting_for ?? []).filter((item) => item.id !== answer.item_id),
            }));
          },
          claimPullRequest: async (claim) => {
            this.calls.claimed.push(claim);
            return { ok: true, value: { number: claim.number, branch: "", url: "", holder_id: claim.session_id ?? "" } };
          },
          dismissWaiting: async (dismiss) => {
            this.calls.dismissed.push(dismiss);
            const removed = this.records.find((one) => one.id === dismiss.session_id)?.waiting_for?.find((item) => item.id === dismiss.item_id);
            // Fleet tells the agent of an item it stated, in the person's words; a reply or a done is only dropped.
            if (removed?.source === "agent") {
              this.row(dismiss.session_id, { kind: "message", id: this.rowId(), at: AT, from: { kind: "you" }, text: removedSaid(removed.text, dismiss.reason) });
            }
            return this.change(dismiss.session_id, (one) => ({
              ...one,
              waiting_for: (one.waiting_for ?? []).filter((item) => item.id !== dismiss.item_id),
            }));
          },
          tuneSession: async (tune) => {
            this.calls.tuned.push(tune);
            return this.change(tune.session_id, (one) => ({
              ...one,
              hosted: { ...one.hosted!, mode: tune.mode, ...(tune.model === undefined ? {} : { model: tune.model }), ...(tune.effort === undefined ? {} : { effort: tune.effort }) },
            }));
          },
          renameSession: async (rename) => {
            this.calls.renamed.push(rename);
            return this.change(rename.session_id, (one) => ({ ...one, title: rename.title }));
          },
          forkSession: async (sessionId) => {
            this.calls.forked.push(sessionId);
            const old = this.records.find((one) => one.id === sessionId);
            if (old === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
            // Fleet's own refusal: a session that is still running is not forked.
            if (old.state === "live" && (old.hosted !== undefined || old.terminal?.listening === true)) {
              return { ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.session_fork_live", message: `session ${sessionId} is still running, and only an ended one is forked` } } as never };
            }
            const id = `01FORK${String((this.minted += 1)).padStart(8, "0")}ABCDEFGHIJ`;
            const record = hosted(id, { ...this.began(), ...(old.title === undefined ? {} : { title: old.title }), attachments: [held("forked_from", sessionId, {}, "spent")] });
            this.set([record, ...this.records.map((one) => (one.id === sessionId ? { ...one, attachments: [...one.attachments, held("forked_to", id, {}, "spent")] } : one))]);
            return { ok: true, value: record };
          },
          handToSession: async (sessionId, group) => {
            this.calls.handed.push({ sessionId, group });
            const parent = this.records.find((one) => one.id === sessionId);
            const title = parent?.plan?.groups.find((one) => one.id === group)?.title;
            if (parent === undefined || title === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
            // The press is his, so the brief is sent: nothing is left held.
            const id = this.agentStarts(sessionId, { title, message: title, group });
            this.change(id, (one) => ({ ...one, attachments: one.attachments.filter((a) => a.kind !== "unsent"), waiting_for: [] }));
            return { ok: true, value: { session_id: id, title, fork: false } };
          },
          forkSessionAt: async (sessionId, at) => {
            this.calls.forkedAt.push({ sessionId, at });
            const old = this.records.find((one) => one.id === sessionId);
            if (old === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
            // **Live or not**: the Session it came from is left as it was, and keeps taking messages.
            const id = `01FORK${String((this.minted += 1)).padStart(8, "0")}ABCDEFGHIJ`;
            const record = hosted(id, { ...this.began(), ...(old.title === undefined ? {} : { title: old.title }), attachments: [held("forked_from", sessionId, { at }, "spent")] });
            this.set([record, ...this.records.map((one) => (one.id === sessionId ? { ...one, attachments: [...one.attachments, held("forked_to", id, {}, "spent")] } : one))]);
            return { ok: true, value: record };
          },
          moveSession: async (sessionId, manifestId) => {
            this.calls.moved.push({ sessionId, manifestId });
            const old = this.records.find((one) => one.id === sessionId);
            if (old === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
            const refused = this.refusesMove ?? (old.manifest_id === manifestId ? { code: "fleet.session_move_same", message: "session is already in that repository" } : old.terminal?.listening === true ? { code: "fleet.session_move_listening", message: "session runs in a terminal that is still listening" } : undefined);
            if (refused !== undefined) {
              this.refusesMove = undefined;
              return { ok: false, outcome: { ok: false, why: "refused", error: refused } as never };
            }
            const id = `01MOVE${String((this.minted += 1)).padStart(8, "0")}ABCDEFGHIJ`;
            const branch = old.attachments.filter((one) => one.kind === "branch").map((one) => one.target).join(",");
            const record = hosted(id, { ...this.began(), manifest_id: manifestId, ...(old.title === undefined ? {} : { title: old.title }), attachments: [held("forked_from", sessionId, { ...(old.manifest_id === undefined ? {} : { moved: old.manifest_id }), ...(branch === "" ? {} : { branch }) }, "spent")] });
            const ended = (one: SessionRecord): SessionRecord => ({ ...one, state: "ended", attachments: [...one.attachments, held("forked_to", id, { moved: manifestId }, "spent")] });
            this.set([record, ...this.records.map((one) => (one.id === sessionId ? ended(one) : one))]);
            return { ok: true, value: record };
          },
          retroSession: async (sessionId: string) => {
            this.calls.retroed.push(sessionId);
            if (this.refusesRetro !== undefined) return { ok: false as const, why: "refused", error: this.refusesRetro } as never;
            // **Fleet refuses a second press while one is being written**, and the first goes on.
            if (this.writingRetro.has(sessionId)) {
              return { ok: false as const, why: "refused", error: { code: "fleet.session_retro_being_written", message: `a retro of session ${sessionId} is being written` } } as never;
            }
            this.writingRetro.add(sessionId);
            try {
              await this.writeRetro(sessionId);
            } finally {
              this.writingRetro.delete(sessionId);
            }
            return { ok: true as const };
          },
          closeSession: async (sessionId) => {
            if (this.refusesClose !== undefined) {
              const error = this.refusesClose;
              this.refusesClose = undefined;
              return { ok: false, outcome: { ok: false, why: "refused", error } as never };
            }
            this.calls.closed.push(sessionId);
            // A terminal's end is held for its mod: the row ends when the terminal says it did (`changed`).
            if (this.records.find((one) => one.id === sessionId)?.origin === "terminal") return this.change(sessionId, (one) => one);
            return this.change(sessionId, (one) => ({ ...one, state: "ended" }));
          },
          stopSession: async (sessionId) => {
            this.calls.stopped.push(sessionId);
            return this.change(sessionId, (one) => ({ ...one, hosted: { ...one.hosted!, turn: { state: "idle" } } }));
          },
          watchSession: async (sessionId) => {
            this.calls.watched.push(sessionId);
            if (this.readingIds.includes(sessionId)) return;
            const held = this.heldReads.get(sessionId);
            if (held !== undefined) return this.readHeld(sessionId, held);
            this.threads = { ...this.threads, [sessionId]: this.threads[sessionId] ?? this.keptRetro[sessionId] ?? [] };
            this.publishThreads();
          },
          readSessionSubagent: async (_sessionId, subagentId) => {
            const thread = this.subagents[subagentId];
            return thread === undefined ? { ok: false, outcome: { ok: false, why: "not_connected" } } : { ok: true, value: thread };
          },
          readSessionLoadout: async (sessionId) => {
            const loadout = this.loadouts[sessionId];
            return loadout === undefined ? { ok: false, outcome: { ok: false, why: "not_connected" } } : { ok: true, value: loadout };
          },
          readLoadoutItem: async (sessionId, item) => {
            const key = loadoutKeyOf(item);
            this.readLoadoutItems.push(key);
            return { ok: true, value: this.loadoutItems[sessionId]?.[key] ?? { what: "not_read", why: "No file on this machine names this item." } };
          },
          readSessionFile: async () => ({ ok: true, bytes: ONE_PIXEL, type: "image/png" }),
          openSessionFile: async () => ({ ok: true }),
          readSessionArtifact: async () => ({ ok: true, bytes: new TextEncoder().encode("# Store clock"), type: "text/markdown" }),
          showSessionPage: async (_sessionId, address) => {
            this.calls.pages.push(address);
            return { ok: true };
          },
          moveSessionPage: async () => undefined,
          onSessionPageEscape: (on) => {
            this.escape.add(on);
            return () => void this.escape.delete(on);
          },
          hideSessionPage: async () => {
            this.calls.pagesHidden += 1;
          },
          pressPullRequest: async (sessionId, number, press) => {
            this.calls.pressed.push({ sessionId, number, press });
            if (press !== "read" && this.refusesPress !== undefined) return { ok: false, outcome: { ok: false, why: "refused", error: this.refusesPress } as never };
            if (press === "merge") {
              const refused = this.merge(sessionId, number);
              if (refused !== undefined) return refused;
            }
            const now = this.records.find((one) => one.id === sessionId)?.attachments.find((one) => one.kind === "pr" && one.target === String(number));
            const state: PullRequestState = {
              manifest_id: "armada",
              number,
              state: press === "merge" ? "merged" : "open",
              auto_merge: press === "auto_merge",
              checks: { state: "passed" },
              title: now?.detail?.["title"] ?? "",
              branch: now?.detail?.["branch"] ?? "",
              address: now?.detail?.["address"] ?? "",
            };
            return { ok: true, value: state };
          },
        };
      },
    };
  }

  private rowId(): string {
    return `row${(this.rowed += 1)}`;
  }

  private merge(sessionId: string, number: number): SessionActed<never> | undefined {
    const row = this.records.find((one) => one.id === sessionId)?.attachments.find((one) => one.kind === "pr" && one.target === String(number));
    if (row?.detail?.["checks"] !== "passed") {
      return { ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.merge_checks_not_passed", message: "The checks have not passed." } } as never };
    }
    this.change(sessionId, (one) => ({
      ...one,
      attachments: one.attachments.map((a) => (a === row ? { ...a, state: "spent" as const, detail: { ...a.detail, state: "merged" } } : a)),
    }));
    return undefined;
  }

  /** A turn of the agent has filled more of a Session's context window: the usage and what the window holds move together. */
  fillTo(sessionId: string, usage: SessionRecord["usage"], breakdown: SessionRecord["context_breakdown"]): void {
    this.change(sessionId, (one) => ({ ...one, usage, ...(breakdown === undefined ? {} : { context_breakdown: breakdown }) }));
  }

  private set(records: SessionRecord[]): void {
    this.records = records;
    this.fleet?.publish({ sessions: { state: "read", sessions: records } });
  }

  /**
   * Holds the first read of `sessionId`'s thread for `ms`, as a slow Fleet does, its id among those being read meanwhile. `cached`
   * is an older copy drawn while it waits, as one kept on disk would be; without it the thread has no rows until the read answers.
   */
  holdRead(sessionId: string, ms: number, cached?: SessionRow[]): void {
    const { [sessionId]: thread = [], ...rest } = this.threads;
    // Out of the threads every other read publishes, so no other Job's open lets it in early.
    this.threads = rest;
    this.heldReads.set(sessionId, { ms, thread, ...(cached === undefined ? {} : { cached }) });
  }

  private readHeld(sessionId: string, held: { ms: number; thread: SessionRow[]; cached?: SessionRow[] }): void {
    this.heldReads.delete(sessionId);
    this.readingIds = [...this.readingIds, sessionId];
    if (held.cached !== undefined) this.threads = { ...this.threads, [sessionId]: held.cached };
    this.fleet?.publish({ sessionThreads: this.threads, sessionThreadsReading: this.readingIds });
    setTimeout(() => {
      this.readingIds = this.readingIds.filter((one) => one !== sessionId);
      this.threads = { ...this.threads, [sessionId]: held.thread };
      this.fleet?.publish({ sessionThreads: this.threads, sessionThreadsReading: this.readingIds });
    }, held.ms);
  }

  private publishThreads(): void {
    this.fleet?.publish({ sessionThreads: this.threads });
  }

  private change(sessionId: string, how: (one: SessionRecord) => SessionRecord): SessionActed {
    const found = this.records.find((one) => one.id === sessionId);
    if (found === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const record = how(found);
    this.set(this.records.map((one) => (one.id === sessionId ? record : one)));
    return { ok: true, value: record };
  }

  /**
   * `start_session`, from the agent of `fromId` when the person asked it to: a new Session (`fork` absent), or a fork of
   * the agent's own conversation, whole or at one of its messages. **Fleet holds it for him**: the agent's first
   * message is stored as the Session's unsent message, and nothing runs until he sends it. Answers the new id.
   */
  agentStarts(fromId: string, how: { title: string; message: string; fork?: { at?: string }; group?: string }): string {
    const id = `01AGENT${String((this.minted += 1)).padStart(8, "0")}ABCDEFGHI`;
    const grouped: Record<string, string> = how.group === undefined ? {} : { group: how.group };
    const link = how.fork === undefined ? held("started_by", fromId, grouped, "spent") : held("forked_from", fromId, how.fork.at === undefined ? {} : { at: how.fork.at }, "spent");
    const record = hosted(id, {
      title: how.title,
      attachments: [link, held("unsent", how.message)],
      waiting_for: [{ id: `unsent:${id}`, text: "First message", since: new Date().toISOString(), source: "first_message", act: { kind: "message", target: id } }],
    });
    const back = how.fork === undefined ? held("started_to", id, grouped, "spent") : held("forked_to", id, {}, "spent");
    this.set([record, ...this.records.map((one) => (one.id === fromId ? { ...one, attachments: [...one.attachments, back] } : one))]);
    return id;
  }

  /** The agent takes a turn on its own clock: working, then the rows it writes (a function is made as its turn reaches it), then idle. */
  async agentTurn(sessionId: string, rows: readonly (SessionRow | (() => SessionRow))[]): Promise<void> {
    this.change(sessionId, (one) => ({ ...one, hosted: { ...one.hosted!, turn: { state: "working" } } }));
    for (const row of rows) {
      await new Promise((done) => setTimeout(done, 450));
      this.row(sessionId, typeof row === "function" ? row() : row);
    }
    this.change(sessionId, (one) => ({ ...one, hosted: { ...one.hosted!, turn: { state: "idle" } } }));
  }

  /** `session.changed`: a session, whole, as Fleet publishes it after any fact. */
  changed(record: SessionRecord): void {
    this.set(this.records.some((one) => one.id === record.id) ? this.records.map((one) => (one.id === record.id ? record : one)) : [record, ...this.records]);
  }

  /**
   * A retro written the way Fleet writes it. **A hosted Session's row is appended on the press and
   * replaced as it goes** (`session.row`); a terminal Session's has none, since its thread is a
   * transcript. The press answers when the retro is kept.
   */
  private async writeRetro(sessionId: string): Promise<void> {
    const record = this.records.find((one) => one.id === sessionId);
    const id = `retro-${(this.retroRowed += 1)}`;
    const at = new Date().toISOString();
    if (record?.hosted === undefined || this.retroNothingNew) {
      await new Promise((done) => setTimeout(done, this.retroStepMs * 4));
      this.onRetro?.(sessionId, record?.title);
      return;
    }
    const frames = retroFrames(this.retroItems, 0);
    let since = at;
    let stage: string | undefined;
    for (const [index, frame] of frames.entries()) {
      // **The number is the Retros page's to say**, and it is kept before the row says it is written.
      if (frame.row.stage === "written") {
        const kept = this.onRetro?.(sessionId, record.title);
        frame.row.retro = typeof kept === "number" ? kept : 1;
      }
      if (frame.row.stage !== stage) {
        stage = frame.row.stage;
        since = new Date().toISOString();
      }
      this.row(sessionId, { kind: "retro", id, at, stage_since: since, ...frame.row });
      if (index < frames.length - 1) await this.beat(frame.pause);
    }
  }

  /** Time between frames: a walk holds on a marked frame until it says time passes. */
  private async beat(pause: boolean): Promise<void> {
    if (this.retroHeld && pause) {
      await new Promise<void>((done) => {
        this.releaseRetro = done;
      });
      return;
    }
    await new Promise((done) => setTimeout(done, this.retroStepMs));
  }

  /** `session.row`: one row, appended or replaced by its id, where the thread is held. A retro frame older than the one held is dropped, as main drops it. */
  row(sessionId: string, row: SessionRow): void {
    const held = this.threads[sessionId];
    // Not opened: main holds no thread for it and drops the row. Fleet keeps a retro's, so opening the thread later brings it.
    if (held === undefined) {
      if (row.kind === "retro" || this.keepsRows) this.keptRetro[sessionId] = withThreadRow(this.keptRetro[sessionId] ?? [], row);
      return;
    }
    this.threads = { ...this.threads, [sessionId]: withThreadRow(held, row) };
    this.publishThreads();
  }

  /**
   * An answer settles the ask's row, and an "always" answer puts its folder on the list in settings.json
   * and off the other, as Fleet does. A session answer writes nothing.
   */
  private settled(answer: AnswerSessionAsk): void {
    const STATE: Record<AnswerSessionAsk["answer"], SessionAskState> = {
      allow_once: "allowed_once",
      allow_and_remember: "allowed_and_remembered",
      refuse: "refused",
      always_allow: "always_allowed",
      always_block: "always_blocked",
      allow_for_session: "allowed_for_session",
    };
    const waiting = this.threads[answer.session_id]?.find((row) => row.kind === "ask" && row.ask.call === answer.call && row.state === "waiting");
    if (waiting?.kind === "ask") this.row(answer.session_id, { ...waiting, state: STATE[answer.answer] });
    const settings = this.fleet?.state().settings;
    if (answer.folder === undefined || (answer.answer !== "always_allow" && answer.answer !== "always_block") || settings == null) return;
    const folder = answer.folder;
    const held = reachFoldersOf(settings.settings.find((one) => one.key === REACH_FOLDERS_KEY)?.value);
    const allow = held.allow.filter((one) => one !== folder);
    const block = held.block.filter((one) => one !== folder);
    (answer.answer === "always_allow" ? allow : block).push(folder);
    this.fleet?.publish({ settings: savedInto(settings, { [REACH_FOLDERS_KEY]: { allow, block } }) });
  }

  /** The agent says something in a thread. */
  says(sessionId: string, text: string): void {
    this.row(sessionId, { kind: "message", id: this.rowId(), at: AT, from: { kind: "agent" }, text });
  }
}
