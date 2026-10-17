// Whether the dock's own copy of a question and the open Job's detail move together, off one
// event on `/events` — #937. `questions.ts` folds the dock's copy; `refresh` re-reads the open
// Job's detail, `job-focus.ts`'s own province. What this file answers is whether `arrivals.ts`
// calls both from the same arm — not what either does once called, which `questions.test.ts` and
// `job-focus.ts`'s own tests already cover.
//
// A real `Questions` proves the dock's half; `refresh` is a spy standing for the detail's, because
// what re-reads it lives behind Electron's IPC and a socket this file has no reason to open.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it, vi } from "vitest";

import { PROTOCOL_ID } from "@armada/protocol";
import type {
  CommandInFlight,
  JobSummary,
  JudgeQuestion,
  HeldWorktrees,
  ModelChoices,
  ModSummary,
  ProposalSettled,
  QuestionInFlight,
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
import type { ReviewMaterial } from "./review";

const JOB_ID = "01M1HQZAKN001AJ5MT3PT09KKY";
const FLEET = { protocolId: PROTOCOL_ID, pid: 1, port: 1, startedAt: "" };

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: JOB_ID,
    handle: "12-a-job",
    title: "A job",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "armada",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-13T09:00:00Z",
    current_step_id: "implement",
    ...over,
  };
}

const ASKING: QuestionInFlight = {
  question_id: "q1",
  step_id: "implement",
  asked_at: "2026-09-13T10:00:00Z",
  question: "Which?",
  options: [{ label: "This", consequence: "Does this." }],
};

const JUDGED: JudgeQuestion = {
  step_id: "review",
  criterion_id: "cause",
  question: "Cause fixed?",
  expected: "Fixed.",
  produced: "Hidden.",
  consequence: "Still broken.",
  asked_at: "2026-09-13T10:01:00Z",
};

/**
 * A host reaching only what these arms touch: `questions`, real off its own wiring so the dock's
 * clearing is genuine rather than asserted by a spy; `refresh`, a spy standing in for the open
 * Job's detail. Every other region is never called by `job.asking`, `job.command_waiting` or
 * `job.state_changed`, so a cast stands in rather than a working fake of five other classes.
 */
function fakeHost(
  jobs: JobSummary[],
  questionsOver: BridgeState["questions"] = [],
): { host: ArrivalHost; refresh: ReturnType<typeof vi.fn>; questions: Questions; state: () => BridgeState; autopilots: ReturnType<typeof vi.fn> } {
  let state: BridgeState = { ...NOTHING_YET, jobs, questions: questionsOver };
  const refresh = vi.fn();
  const autopilots = vi.fn();
  const questions = new Questions({
    current: () => state,
    publish: (change) => (state = { ...state, ...change }),
  });
  const mods = new Modding((change) => (state = { ...state, ...change }), () => null);
  const host: ArrivalHost = {
    current: () => state,
    now: () => 0,
    greeted: () => true,
    setGreeted: () => {},
    proposalRef: () => null,
    setProposalRef: () => {},
    setProposalJob: () => {},
    watchedJobId: () => null,
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
    sessions: { again: async () => {}, changed: () => {}, row: () => {} },
    held: { warm: async () => {}, again: async () => {} },
    fleetBuild: { watch: () => {} },
    mods,
    material: {} as unknown as ReviewMaterial,
    socket: { close: () => {}, resetUnreachable: () => {} },
    publish: (change) => (state = { ...state, ...change }),
    fold: (row) => (state = { ...state, jobs: state.jobs.map((one) => (one.id === row.id ? row : one)) }),
    forget: () => {},
    settle: () => {},
    refresh,
    takeAgain: async () => {},
  };
  return { host, refresh, questions, state: () => state, autopilots };
}

describe("a Drone's question, on the dock and on the open Job's detail together", () => {
  it("arrives on both from the one event", () => {
    const { host, refresh, state } = fakeHost([job()]);
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.asking", job_id: JOB_ID, step_id: "implement", asking: ASKING, actor: "drone", at: ASKING.asked_at },
      }),
      FLEET,
    );
    expect(state().questions).toEqual([{ kind: "drone", job_id: JOB_ID, asking: ASKING }]);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });

  it("leaves both on the answer, whichever surface sent it", () => {
    const { host, refresh, state } = fakeHost([job()], [{ kind: "drone", job_id: JOB_ID, asking: ASKING }]);
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.asking", job_id: JOB_ID, step_id: "implement", actor: "human", at: "2026-09-13T10:05:00Z" },
      }),
      FLEET,
    );
    expect(state().questions).toEqual([]);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });
});

describe("a held command, the same pairing", () => {
  const WAITING: CommandInFlight = {
    call: "call_1",
    step_id: "implement",
    asked_at: "2026-09-13T10:02:00Z",
    tool: "Bash",
    detail: "ls",
    truncated: false,
    offers: ["reject"],
    rules: [],
  };

  it("arrives on both from the one event", () => {
    const { host, refresh, state } = fakeHost([job()]);
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.command_waiting", job_id: JOB_ID, step_id: "implement", waiting: WAITING, actor: "drone", at: WAITING.asked_at },
      }),
      FLEET,
    );
    expect(state().questions).toEqual([{ kind: "command", job_id: JOB_ID, waiting: WAITING }]);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });

  it("leaves both on the answer", () => {
    const { host, refresh, state } = fakeHost([job()], [{ kind: "command", job_id: JOB_ID, waiting: WAITING }]);
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.command_waiting", job_id: JOB_ID, step_id: "implement", actor: "human", at: "2026-09-13T10:06:00Z" },
      }),
      FLEET,
    );
    expect(state().questions).toEqual([]);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });
});

describe("a Judge refusal, carried by no event, on the status move that opens or closes it", () => {
  it("entering awaiting_review wakes the dock's own read and re-reads the open Job together", () => {
    const { host, refresh, questions } = fakeHost([job({ status: "running" })]);
    const read = vi.spyOn(questions, "read").mockResolvedValue(undefined);
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.state_changed", job_id: JOB_ID, from: "running", to: "awaiting_review", actor: "fleet", at: "2026-09-13T10:03:00Z" },
      }),
      FLEET,
    );
    expect(read).toHaveBeenCalledWith(FLEET.port, JOB_ID);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });

  it("leaving awaiting_review drops the dock's copy and re-reads the open Job together, however it was answered", () => {
    const { host, refresh, state } = fakeHost(
      [job({ status: "awaiting_review" })],
      [{ kind: "judge", job_id: JOB_ID, question: JUDGED }],
    );
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.state_changed", job_id: JOB_ID, from: "awaiting_review", to: "queued", actor: "fleet", at: "2026-09-13T10:07:00Z" },
      }),
      FLEET,
    );
    expect(state().questions).toEqual([]);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });
});

// #1714/#1716: a dispatched request is a Job at `proposing`, and `proposal.moved` names it.
// The owner's decision of 30 Sep 2026 — a proposal fills in as it is written.
describe("a proposal filling in, on the Job its message names", () => {
  const REQUEST = "Say on the Cleared tab whether the branch was kept";
  const proposingJob = (): JobSummary => {
    const { current_step_id: _none, ...row } = job({ status: "proposing", title: REQUEST, workflow_id: "" });
    return row;
  };

  /** `proposal.moved` as Fleet publishes it, mid-answer. */
  function moved(settled: ProposalSettled | undefined, over: Record<string, unknown> = {}): string {
    return JSON.stringify({
      message: "event",
      cursor: 3,
      event: {
        kind: "proposal.moved",
        proposal_id: "01PROPOSAL",
        job_id: JOB_ID,
        client_ref: "someone-else",
        proposing: {
          proposal_id: "01PROPOSAL",
          client_ref: "someone-else",
          model: "sonnet",
          since: "2026-09-13T10:00:00Z",
          budget_ms: 600_000,
          reached: "answering",
          answered_characters: 120,
          ...(settled === undefined ? {} : { settled }),
        },
        actor: "human",
        at: "2026-09-13T10:00:05Z",
        ...over,
      },
    });
  }

  it("puts the workflow on the row first, and leaves the request as its title", () => {
    const { host, state } = fakeHost([proposingJob()]);
    applyArrival(host, moved({ workflow_id: "bug" }), FLEET);
    expect(state().jobs[0]?.workflow_id).toBe("bug");
    expect(state().jobs[0]?.title).toBe(REQUEST);
  });

  it("takes the title next, on a row somebody else dispatched as well as this window's own", () => {
    const { host, state } = fakeHost([proposingJob()]);
    applyArrival(host, moved({ workflow_id: "bug", title: "Name the branch a clear kept" }), FLEET);
    expect(state().jobs[0]?.title).toBe("Name the branch a clear kept");
    expect(state().proposing, "somebody else's call is not this window's wait").toBeNull();
  });

  it("fills the open Job's page too, and the request moves into its brief", () => {
    const { host, state } = fakeHost([proposingJob()]);
    host.publish({
      watched: {
        state: "read",
        jobId: JOB_ID,
        detail: { job: proposingJob(), created_at: "2026-09-13T09:00:00Z", steps: [], acceptance_criteria: [], dependencies: [] },
      },
    });
    applyArrival(
      host,
      moved({ workflow_id: "bug", title: "Name the branch a clear kept", done_when: ["The row names the branch"] }),
      FLEET,
    );
    const watched = state().watched;
    if (watched.state !== "read") throw new Error("the open Job was dropped");
    expect(watched.detail.job.title).toBe("Name the branch a clear kept");
    expect(watched.detail.facts).toBe(REQUEST);
    expect(watched.detail.acceptance_criteria.map((one) => one.text)).toEqual(["The row names the branch"]);
  });

  it("still publishes this window's own wait beside the fold", () => {
    const { host, state } = fakeHost([proposingJob()]);
    const mine = { ...host, proposalRef: () => "someone-else" };
    applyArrival(mine, moved({ workflow_id: "bug" }), FLEET);
    expect(state().proposing?.settled).toEqual({ workflow_id: "bug" });
    expect(state().jobs[0]?.workflow_id).toBe("bug");
  });

  it("carries the Job its message names onto this window's own wait, and keeps it on the messages after", () => {
    const { host, state } = fakeHost([proposingJob()]);
    const mine = { ...host, proposalRef: () => "someone-else" };
    applyArrival(mine, moved(undefined), FLEET);
    expect(state().proposing?.job_id).toBe(JOB_ID);
    applyArrival(mine, moved({ workflow_id: "bug" }, { job_id: undefined }), FLEET);
    expect(state().proposing?.job_id).toBe(JOB_ID);
    expect(state().proposing?.settled).toEqual({ workflow_id: "bug" });
  });

  it("does not hand one call's Job to the next call, and clears with the call", () => {
    const { host, state } = fakeHost([proposingJob()]);
    const mine = { ...host, proposalRef: () => "someone-else" };
    applyArrival(mine, moved(undefined), FLEET);
    const next = JSON.parse(moved(undefined, { proposal_id: "01OTHER", job_id: undefined })) as { event: { proposing: object } };
    next.event.proposing = { ...next.event.proposing, proposal_id: "01OTHER" };
    applyArrival(mine, JSON.stringify(next), FLEET);
    expect(state().proposing?.job_id).toBeUndefined();
    applyArrival(mine, moved(undefined, { proposing: undefined }), FLEET);
    expect(state().proposing).toBeNull();
  });

  it("moves nothing on a message that names no Job, which is a Fleet older than 21.6", () => {
    const { host, state } = fakeHost([proposingJob()]);
    applyArrival(host, moved({ workflow_id: "bug" }, { job_id: undefined }), FLEET);
    expect(state().jobs[0]?.workflow_id).toBe("");
  });
});

describe("the mods on this machine, off `mods.changed`", () => {
  const row = (name: string, over: Partial<ModSummary> = {}): ModSummary => ({ name, kind: "theme", enabled: true, valid: true, ...over });
  const changed = (mods: ModSummary[]) => JSON.stringify({ message: "event", cursor: 3, event: { kind: "mods.changed", mods } });

  it("holds nothing until the list has been read, and replaces it whole after", () => {
    const { host, state } = fakeHost([]);
    expect(state().mods).toBeNull();
    applyArrival(host, changed([row("calm"), row("warm")]), FLEET);
    expect(state().mods?.mods.map((one) => one.name)).toEqual(["calm", "warm"]);
    // Whole, not folded: a mod that left the folder leaves the list, and a row's new state replaces the old.
    applyArrival(host, changed([row("warm", { enabled: false, valid: false, reason: "line 3: not a token" })]), FLEET);
    expect(state().mods?.mods).toEqual([row("warm", { enabled: false, valid: false, reason: "line 3: not a token" })]);
  });

  it("is not a Job event: the Board is untouched", () => {
    const { host, state } = fakeHost([job()]);
    applyArrival(host, changed([row("calm")]), FLEET);
    expect(state().jobs).toEqual([job()]);
  });
});

describe("a repository's Autopilot, changed in Fleet", () => {
  it("is told on whole, without its kind, and moves no Job", () => {
    const { host, refresh, state, autopilots } = fakeHost([job()]);
    const autopilot = { manifest_id: "armada", instruction: "work the backlog", at_once: 2, state: "paused", started_at: "2026-10-12T03:00:00Z", spent_micros: 0, jobs_started: 0, at_work: 0, picks: [] };
    applyArrival(host, JSON.stringify({ message: "event", cursor: 3, event: { kind: "autopilot.changed", ...autopilot } }), FLEET);
    expect(autopilots).toHaveBeenCalledWith(autopilot);
    expect(refresh).not.toHaveBeenCalled();
    expect(state().jobs).toEqual([job()]);
    expect(state().connection).toMatchObject({ state: "connected" });
  });
});

describe("a card the Job's Commander put to the owner", () => {
  const CARD = {
    card_id: "01CARD",
    job_id: JOB_ID,
    by: "commander",
    raised_at: "2026-10-13T10:00:00Z",
    text: "desktop_test is red on main as well",
    reach: "A Check is skipped or edited by you",
    question: "What should Group 3 do?",
    options: [
      { id: "1", label: "Skip desktop_test" },
      { id: "2", label: "Leave it" },
    ],
  };
  const event = (card?: typeof CARD) =>
    JSON.stringify({ message: "event", cursor: 3, event: { kind: "commander.card", job_id: JOB_ID, ...(card === undefined ? {} : { card }) } });

  it("puts the Job's row among those waiting on him, and re-reads the open Job", () => {
    const { host, refresh, state } = fakeHost([job()]);
    applyArrival(host, event(CARD), FLEET);
    expect(state().jobs[0]?.asking).toBe(true);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });

  it("takes it off the row when it is answered", () => {
    const { host, state } = fakeHost([job({ asking: true })]);
    applyArrival(host, event(), FLEET);
    expect(state().jobs[0]?.asking).toBe(false);
  });

  it("leaves the row waiting where a Drone's own question is still out", () => {
    const { host, state } = fakeHost([job({ asking: true })], [{ kind: "drone", job_id: JOB_ID, asking: ASKING }]);
    applyArrival(host, event(), FLEET);
    expect(state().jobs[0]?.asking).toBe(true);
  });
});

describe("the default model's step down, carried whole by its event", () => {
  it("is passed on as the state and nothing of the event's kind", () => {
    const { host } = fakeHost([job()]);
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
    const { host } = fakeHost([job()]);
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
  it("is told to the windows whole, and moves nothing on the Board", () => {
    const { host, state } = fakeHost([job()]);
    const appended = vi.fn();
    const entry = { id: 7, at: "2026-10-13T10:00:00Z", kind: "job.started", subject: { kind: "job", id: JOB_ID }, tone: "run", says: "Started", live: true };
    applyArrival(
      { ...host, timeline: { appended } },
      JSON.stringify({ message: "event", cursor: 9, event: { kind: "timeline.appended", event: entry } }),
      FLEET,
    );
    expect(appended).toHaveBeenCalledWith(entry);
    expect(state().jobs).toEqual([job()]);
  });
});

describe("a review run, off `review_run.changed`", () => {
  it("is told to the windows whole, and moves nothing on the Board", () => {
    const { host, state } = fakeHost([job()]);
    const changed = vi.fn();
    const run = { manifest_id: "armada", number: 7, kind: "tool", name: "risks", head: "abc1234", state: "running", started_at: "2026-10-16T10:00:00Z" };
    applyArrival({ ...host, reviews: { changed, walkChanged: () => {} } }, JSON.stringify({ message: "event", cursor: 9, event: { kind: "review_run.changed", run } }), FLEET);
    expect(changed).toHaveBeenCalledWith(run);
    expect(state().jobs).toEqual([job()]);
  });
});

describe("a held review walk, off `review_walk.changed`", () => {
  it("is told to the windows whole, and moves nothing on the Board", () => {
    const { host, state } = fakeHost([job()]);
    const walkChanged = vi.fn();
    const walk = { id: "w1", manifest_id: "armada", number: 7, state: "starting", slot: 3, address: "http://localhost:5173" };
    applyArrival({ ...host, reviews: { changed: () => {}, walkChanged } }, JSON.stringify({ message: "event", cursor: 10, event: { kind: "review_walk.changed", walk } }), FLEET);
    expect(walkChanged).toHaveBeenCalledWith(walk);
    expect(state().jobs).toEqual([job()]);
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
    const { host, state } = fakeHost([]);
    host.publish({ holds: { ...state().holds, models: OPUS } });
    answer.body = SONNET;
    applyArrival(host, event(kind), { ...FLEET, port });
    await vi.waitFor(() => expect(state().holds.models?.session).toBe("sonnet"));
    // Only the models moved: the rest of what is held stays.
    expect(state().holds.workflows).toEqual([]);
  });

  it("keeps what was held where the read fails", async () => {
    const port = await fleetAnswering({ status: 500, body: SONNET });
    const { host, state } = fakeHost([]);
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
  const pool = (...slots: WorktreeSlot[]): WorktreesHeld => ({ worktrees: [], slots });

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

  const slotEvent = (kind: "slot.leased" | "slot.released" | "job.started") =>
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
    const { host } = fakeHost([]);
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
    const { host } = fakeHost([]);
    const told = { ...host, held: { warm: async () => {}, again } };
    applyArrival(told, slotEvent("job.started"), FLEET);
    expect(again).not.toHaveBeenCalled();
    applyArrival(told, slotEvent("slot.leased"), FLEET);
    applyArrival(told, slotEvent("slot.released"), FLEET);
    expect(again).toHaveBeenCalledTimes(2);
    expect(again).toHaveBeenCalledWith(FLEET.port);
  });
});
