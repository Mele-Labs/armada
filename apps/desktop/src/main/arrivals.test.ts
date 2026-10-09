// Whether the dock's own copy of a question and the open Job's detail move together, off one
// event on `/events` — #937. `questions.ts` folds the dock's copy; `refresh` re-reads the open
// Job's detail, `job-focus.ts`'s own province. What this file answers is whether `arrivals.ts`
// calls both from the same arm — not what either does once called, which `questions.test.ts` and
// `job-focus.ts`'s own tests already cover.
//
// A real `Questions` proves the dock's half; `refresh` is a spy standing for the detail's, because
// what re-reads it lives behind Electron's IPC and a socket this file has no reason to open.

import { describe, expect, it, vi } from "vitest";

import { PROTOCOL_ID } from "@armada/protocol";
import type {
  CommandInFlight,
  JobSummary,
  JudgeQuestion,
  ModSummary,
  ProposalSettled,
  QuestionInFlight,
} from "@armada/protocol";
import { NOTHING_YET, type BridgeState } from "../shared/bridge";
import { applyArrival, type ArrivalHost } from "./arrivals";
import type { RehearsalConnection } from "./rehearsal";
import type { RepositoryReads } from "./repositories";
import { Modding } from "./mods";
import { Questions } from "./questions";
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
): { host: ArrivalHost; refresh: ReturnType<typeof vi.fn>; questions: Questions; state: () => BridgeState } {
  let state: BridgeState = { ...NOTHING_YET, jobs, questions: questionsOver };
  const refresh = vi.fn();
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
    overviewAgain: async () => {},
    questions,
    helm: { reconnected: () => {} },
    studios: { again: async () => {}, changed: () => {}, deleted: () => {} },
    sessions: { again: async () => {}, changed: () => {}, row: () => {} },
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
  return { host, refresh, questions, state: () => state };
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
