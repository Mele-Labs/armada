// What `arrivals.ts` does with a stream message: each event reaches the one host it belongs to, and
// moves nothing else. A real `Questions` holds the dock's half; every other region is a spy or a
// cast, because what each does once called is its own tests' business.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it, vi } from "vitest";

import { PROTOCOL_ID } from "@armada/protocol";
import type {
  HeldWorktrees,
  ModelChoices,
  ModSummary,
  WorktreesHeld,
  WorktreeSlot,
} from "@armada/protocol";
import { NOTHING_YET, type BridgeState } from "../shared/bridge";
import { applyArrival, type ArrivalHost } from "./arrivals";
import type { RehearsalConnection } from "./rehearsal";
import type { RepositoryReads } from "./repositories";
import { Modding } from "./mods";
import { Questions } from "./questions";
import { HeldReader } from "./holding";

const FLEET = { protocolId: PROTOCOL_ID, pid: 1, port: 1, startedAt: "" };

/**
 * A host reaching only what these arms touch: `questions`, real off its own wiring; every other
 * region is never called by the events below, so a cast stands in rather than a working fake of
 * five other classes.
 */
function fakeHost(
  questionsOver: BridgeState["questions"] = [],
): { host: ArrivalHost; questions: Questions; state: () => BridgeState; autopilots: ReturnType<typeof vi.fn> } {
  let state: BridgeState = { ...NOTHING_YET, questions: questionsOver };
  const autopilots = vi.fn();
  const questions = new Questions({
    current: () => state,
    publish: (change) => (state = { ...state, ...change }),
  });
  const mods = new Modding((change) => (state = { ...state, ...change }), () => null);
  const host: ArrivalHost = {
    current: () => state,
    now: () => 0,
    repositories: {} as unknown as RepositoryReads,
    rehearsal: {} as unknown as RehearsalConnection,
    sleep: { changed: () => {} },
    planUsage: { changed: () => {} },
    modelDownshift: { changed: () => {}, seed: async () => {} },
    timeline: { appended: () => {} },
    reviews: { changed: () => {}, walkChanged: () => {} },
    issues: { autopilotChanged: autopilots, autopilotsAgain: async () => {} },
    overviewAgain: async () => {},
    questions,
    helm: { reconnected: () => {} },
    studios: { again: async () => {}, changed: () => {}, deleted: () => {} },
    sessions: { again: async () => {}, changed: () => {}, row: () => {}, terminal: () => {}, watchesChanged: () => {} },
    held: { warm: async () => {}, again: async () => {} },
    fleetBuild: { watch: () => {} },
    mods,
    socket: { close: () => {}, resetUnreachable: () => {} },
    publish: (change) => (state = { ...state, ...change }),
    settle: () => {},
  };
  return { host, questions, state: () => state, autopilots };
}

describe("the mods on this machine, off `mods.changed`", () => {
  const row = (name: string, over: Partial<ModSummary> = {}): ModSummary => ({ name, kind: "theme", enabled: true, valid: true, ...over });
  const changed = (mods: ModSummary[]) => JSON.stringify({ message: "event", cursor: 3, event: { kind: "mods.changed", mods } });

  it("holds nothing until the list has been read, and replaces it whole after", () => {
    const { host, state } = fakeHost();
    expect(state().mods).toBeNull();
    applyArrival(host, changed([row("calm"), row("warm")]), FLEET);
    expect(state().mods?.mods.map((one) => one.name)).toEqual(["calm", "warm"]);
    // Whole, not folded: a mod that left the folder leaves the list, and a row's new state replaces the old.
    applyArrival(host, changed([row("warm", { enabled: false, valid: false, reason: "line 3: not a token" })]), FLEET);
    expect(state().mods?.mods).toEqual([row("warm", { enabled: false, valid: false, reason: "line 3: not a token" })]);
  });
});

describe("a repository's Autopilot, changed in Fleet", () => {
  it("is told on whole, without its kind", () => {
    const { host, state, autopilots } = fakeHost();
    const autopilot = { manifest_id: "armada", instruction: "work the backlog", at_once: 2, state: "paused", started_at: "2026-10-12T03:00:00Z", spent_micros: 0, sessions_started: 0, at_work: 0, picks: [] };
    applyArrival(host, JSON.stringify({ message: "event", cursor: 3, event: { kind: "autopilot.changed", ...autopilot } }), FLEET);
    expect(autopilots).toHaveBeenCalledWith(autopilot);
    expect(state().connection).toMatchObject({ state: "connected" });
  });
});

describe("a Job's watches, carried whole by their event", () => {
  it("is told on as the Job's list, without its kind", () => {
    const { host } = fakeHost();
    const watchesChanged = vi.fn();
    const change = { session_id: "s1", watches: [{ id: "w1", session_id: "s1", name: "Staging deploy", state: "running", live: true }] };
    applyArrival({ ...host, sessions: { ...host.sessions, watchesChanged } }, JSON.stringify({ message: "event", cursor: 4, event: { kind: "watches.changed", ...change } }), FLEET);
    expect(watchesChanged).toHaveBeenCalledWith(change);
  });
});

describe("the default model's step down, carried whole by its event", () => {
  it("is passed on as the state and nothing of the event's kind", () => {
    const { host } = fakeHost();
    const changed = vi.fn();
    const state = { downshift: { phase: "offered", percent: 83, was: "opus", to: "sonnet" } };
    applyArrival(
      { ...host, modelDownshift: { changed, seed: async () => {} } },
      JSON.stringify({ message: "event", cursor: 4, event: { kind: "model_downshift.changed", ...state } }),
      FLEET,
    );
    expect(changed).toHaveBeenCalledWith(state);
  });

  it("passes on none when the event carries none", () => {
    const { host } = fakeHost();
    const changed = vi.fn();
    applyArrival(
      { ...host, modelDownshift: { changed, seed: async () => {} } },
      JSON.stringify({ message: "event", cursor: 4, event: { kind: "model_downshift.changed" } }),
      FLEET,
    );
    expect(changed).toHaveBeenCalledWith({});
  });
});

describe("an entry on the Cockpit's timeline, off `timeline.appended`", () => {
  it("is told to the windows whole", () => {
    const { host } = fakeHost();
    const appended = vi.fn();
    const entry = { id: 7, at: "2026-10-13T10:00:00Z", kind: "session.started", subject: { kind: "session", id: "s1" }, tone: "run", says: "Started", live: true };
    applyArrival(
      { ...host, timeline: { appended } },
      JSON.stringify({ message: "event", cursor: 9, event: { kind: "timeline.appended", event: entry } }),
      FLEET,
    );
    expect(appended).toHaveBeenCalledWith(entry);
  });
});

describe("a review run, off `review_run.changed`", () => {
  it("is told to the windows whole", () => {
    const { host } = fakeHost();
    const changed = vi.fn();
    const run = { manifest_id: "armada", number: 7, kind: "tool", name: "risks", head: "abc1234", state: "running", started_at: "2026-10-16T10:00:00Z" };
    applyArrival({ ...host, reviews: { changed, walkChanged: () => {} } }, JSON.stringify({ message: "event", cursor: 9, event: { kind: "review_run.changed", run } }), FLEET);
    expect(changed).toHaveBeenCalledWith(run);
  });
});

describe("a held review walk, off `review_walk.changed`", () => {
  it("is told to the windows whole", () => {
    const { host } = fakeHost();
    const walkChanged = vi.fn();
    const walk = { id: "w1", manifest_id: "armada", number: 7, state: "starting", slot: 3, address: "http://localhost:5173" };
    applyArrival({ ...host, reviews: { changed: () => {}, walkChanged } }, JSON.stringify({ message: "event", cursor: 10, event: { kind: "review_walk.changed", walk } }), FLEET);
    expect(walkChanged).toHaveBeenCalledWith(walk);
  });
});

describe("the models Fleet holds, read again when what sets them moves", () => {
  let server: Server | null = null;
  afterEach(async () => {
    const held = server;
    server = null;
    if (held === null) return;
    held.closeAllConnections();
    await new Promise<void>((done) => held.close(() => done()));
  });

  /** A Fleet that answers `/models` with whatever `answer` holds now, and nothing else. */
  async function fleetAnswering(answer: { status: number; body: ModelChoices }): Promise<number> {
    server = createServer((request, response) => {
      const models = request.url === "/models";
      response.writeHead(models ? answer.status : 404, { "content-type": "application/json" });
      response.end(JSON.stringify(models ? answer.body : {}));
      request.resume();
    });
    const listening = server;
    await new Promise<void>((done) => listening.listen(0, "127.0.0.1", () => done()));
    return (listening.address() as AddressInfo).port;
  }

  const OPUS: ModelChoices = { models: ["opus", "sonnet"], default: "sonnet", session: "opus" };
  const SONNET: ModelChoices = { ...OPUS, session: "sonnet" };
  const event = (kind: "settings.changed" | "model_downshift.changed") =>
    JSON.stringify({ message: "event", cursor: 5, event: kind === "settings.changed" ? { kind, settings: [] } : { kind } });

  it.each(["settings.changed", "model_downshift.changed"] as const)("follows `models.session` after %s", async (kind) => {
    const answer = { status: 200, body: OPUS };
    const port = await fleetAnswering(answer);
    const { host, state } = fakeHost();
    host.publish({ holds: { ...state().holds, models: OPUS } });
    answer.body = SONNET;
    applyArrival(host, event(kind), { ...FLEET, port });
    await vi.waitFor(() => expect(state().holds.models?.session).toBe("sonnet"));
    // Only the models moved: the rest of what is held stays.
    expect(state().holds.workflows).toEqual([]);
  });

  it("keeps what was held where the read fails", async () => {
    const port = await fleetAnswering({ status: 500, body: SONNET });
    const { host, state } = fakeHost();
    host.publish({ holds: { ...state().holds, models: OPUS } });
    applyArrival(host, event("settings.changed"), { ...FLEET, port });
    // The other reads in the same arm settle on the same server; give them the same moment.
    await new Promise((done) => setTimeout(done, 50));
    expect(state().holds.models).toEqual(OPUS);
  });
});

describe("the pool's bays, read again when Fleet says a slot was leased or given back", () => {
  let server: Server | null = null;
  afterEach(async () => {
    const held = server;
    server = null;
    if (held === null) return;
    held.closeAllConnections();
    await new Promise<void>((done) => held.close(() => done()));
  });

  /** `list_worktrees` as Fleet answers it for a Session that took slot 4 on its first write. */
  const taken: WorktreeSlot = {
    manifest_id: "armada",
    slot: 4,
    path: "/p/slot-4",
    held: { state: "hosted_session", session_id: "d02bc3e3-94ce-4972-896c-9df4056ecf28" },
    branch: "armada/session-d02bc3e3",
    since: "2026-10-15T12:05:12Z",
    base: "main",
    warm: true,
  };
  const pool = (...slots: WorktreeSlot[]): WorktreesHeld => ({ slots });

  async function poolAnswering(answer: { body: WorktreesHeld }): Promise<number> {
    server = createServer((request, response) => {
      response.writeHead(request.url === "/worktrees" ? 200 : 404, { "content-type": "application/json" });
      response.end(JSON.stringify(request.url === "/worktrees" ? answer.body : {}));
      request.resume();
    });
    const listening = server;
    await new Promise<void>((done) => listening.listen(0, "127.0.0.1", () => done()));
    return (listening.address() as AddressInfo).port;
  }

  const slotEvent = (kind: "slot.leased" | "slot.released" | "session.started") =>
    JSON.stringify({
      message: "event",
      cursor: 11,
      event: {
        kind: "timeline.appended",
        event: { id: 12, at: "2026-10-15T12:05:12Z", kind, subject: { kind: "session", id: "d02bc3e3-94ce-4972-896c-9df4056ecf28" }, tone: "idle", says: kind, detail: "slot-4", manifest_id: "armada", live: true },
      },
    });

  const slotsOf = (one: HeldWorktrees | undefined) => (one?.state === "read" ? one.held.slots : undefined);

  it("draws a slot a Session took after the surface opened, and draws it gone once it is given back", async () => {
    const answer = { body: pool() };
    const port = await poolAnswering(answer);
    const drawn: HeldWorktrees[] = [];
    const reader = new HeldReader((one) => drawn.push(one));
    const { host } = fakeHost();
    await reader.want(port, true);
    expect(slotsOf(drawn.at(-1))).toEqual([]);

    answer.body = pool(taken);
    applyArrival({ ...host, held: reader }, slotEvent("slot.leased"), { ...FLEET, port });
    await vi.waitFor(() => expect(slotsOf(drawn.at(-1))).toEqual([taken]));

    answer.body = pool({ ...taken, held: { state: "free" } });
    applyArrival({ ...host, held: reader }, slotEvent("slot.released"), { ...FLEET, port });
    await vi.waitFor(() => expect(slotsOf(drawn.at(-1))?.[0]?.held).toEqual({ state: "free" }));
  });

  it("reads the pool for a lease or a release and for nothing else on the timeline", () => {
    const again = vi.fn(async () => {});
    const { host } = fakeHost();
    const told = { ...host, held: { warm: async () => {}, again } };
    applyArrival(told, slotEvent("session.started"), FLEET);
    expect(again).not.toHaveBeenCalled();
    applyArrival(told, slotEvent("slot.leased"), FLEET);
    applyArrival(told, slotEvent("slot.released"), FLEET);
    expect(again).toHaveBeenCalledTimes(2);
    expect(again).toHaveBeenCalledWith(FLEET.port);
  });
});
