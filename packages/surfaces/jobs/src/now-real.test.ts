// What the Now panel is handed for a Job on a real Fleet, from the shapes Fleet serves: the
// Drones `list_job_drones` names (`crates/api/src/tests/shapes.rs`' `job_drones`), a step's
// `checking` and `judging`, and the detail's three open questions.

import { describe, expect, it } from "vitest";

import type { CheckUnderway, JobDetail, JobDrones, JudgeInFlight, Turn } from "@armada/protocol";
import { sampleDetail, sampleJob, sampleStep } from "@armada/screens/src/draft/sample";

import { droneViewsOf } from "./draft/drone";
import { nowViewOf } from "./now-real";
import { nowPanelOf } from "./now-panel";

const DRONE = "01M3WJ6FGZ003DCX123T7W6YP1";
const OTHER = "01M3WJ6FGZ003DCX123T7W6YP2";
const STEP = { id: "implement", name: "Implement" };

const LISTED: JobDrones = {
  job_id: "01M130Y1380016YK5S0JXBXDQ5",
  drones: [
    { drone_id: "01DRONEDONE", step_id: "plan", state: "done", since: "2026-10-09T09:00:00Z", ended_at: "2026-10-09T09:10:00Z" },
    { drone_id: DRONE, step_id: "implement", state: "running", since: "2026-10-09T09:11:00Z", turns: 4 },
  ],
};

const JUDGING: JudgeInFlight = { look: "criterion", criterion_id: "C1", model: "opus", call: 1, of: 2, since: "2026-10-09T09:30:00Z", budget_ms: 120_000 };

function check(name: string, over: Partial<CheckUnderway> = {}): CheckUnderway {
  return { name, started_at: "2026-10-09T09:20:00Z", output_path: `.armada/logs/${name}.live.log`, ...over };
}

/** A Job on its Implement step, the detail plus the Drones as the host holds them. */
function onImplement(over: Partial<JobDetail> = {}, step: Parameters<typeof sampleStep>[0] = {}, listed: JobDrones | undefined = LISTED, turns?: readonly Turn[]) {
  const whole = sampleDetail({
    job: sampleJob({ current_step_id: "implement", ...(listed === undefined ? { assigned_drone: DRONE } : {}) }),
    steps: [sampleStep({ step_id: "plan", label: "Plan", state: "advanced" }), sampleStep({ step_id: "implement", label: "Implement", ...step })],
    ...over,
  });
  return { whole, drones: droneViewsOf(listed, whole, turns) };
}

function called(seq: number, tool: string, detail: string): Turn {
  return { ts: "2026-10-09T09:12:00Z", seq, by: "drone", drone_id: DRONE, step: "implement", saw: { event: "called", tool, call: `c${seq}`, detail, truncated: false } };
}

describe("the Now panel on a real Job: where the panel is drawn at all", () => {
  it("is absent while the detail is unread", () => {
    expect(nowViewOf({ whole: null, drones: [] })).toBeUndefined();
  });

  it.each(["completed_success", "completed_failed", "killed", "rejected", "superseded"])("is absent on a Job that is over (%s)", (status) => {
    const { whole, drones } = onImplement({ job: sampleJob({ status, current_step_id: "implement" }) });
    expect(nowViewOf({ whole, drones })).toBeUndefined();
  });

  it.each(["awaiting_approval", "proposing"])("is absent before the run (%s)", (status) => {
    const { whole, drones } = onImplement({ job: sampleJob({ status }) });
    expect(nowViewOf({ whole, drones })).toBeUndefined();
  });

  it("is empty, so it says nothing is running, on a live Job with nothing running or waited on", () => {
    const whole = sampleDetail({ job: sampleJob({ status: "awaiting_review" }) });
    expect(nowViewOf({ whole, drones: [] })).toEqual({});
  });
});

describe("the Now panel on a real Job: the Drones", () => {
  it("names each running Drone after its step and opens it by id", () => {
    const { whole, drones } = onImplement();
    expect(nowViewOf({ whole, drones })?.running).toEqual([
      { key: `drone:${DRONE}`, of: "drone", name: "Implement Drone", state: "running", target: DRONE, step: STEP },
    ]);
  });

  it("leaves out a Drone that finished, and one resting at the gate", () => {
    const resting: JobDrones = { ...LISTED, drones: [LISTED.drones[0]!, { ...LISTED.drones[1]!, at_rest_since: "2026-10-09T09:25:00Z" }] };
    const { whole, drones } = onImplement({}, {}, resting);
    expect(nowViewOf({ whole, drones })).toEqual({});
  });

  it("names a task's Drone for its task, so two on one step read as two", () => {
    const two: JobDrones = {
      ...LISTED,
      drones: [
        { ...LISTED.drones[1]!, task: "T1" },
        { drone_id: OTHER, step_id: "implement", task: "T2", state: "running", since: "2026-10-09T09:11:00Z" },
      ],
    };
    const { whole, drones } = onImplement({}, {}, two);
    expect(nowViewOf({ whole, drones })?.running?.map((one) => one.name)).toEqual(["Implement Drone on T1", "Implement Drone on T2"]);
  });

  it("lists none on a Job held at a gate, whose Drone the row still names", () => {
    const { whole, drones } = onImplement({ job: sampleJob({ status: "awaiting_review", current_step_id: "implement", assigned_drone: DRONE }) }, {}, undefined);
    expect(nowViewOf({ whole, drones })).toEqual({});
  });

  it("shows the Job's own Drone from the row before the list is read", () => {
    const { whole, drones } = onImplement({}, {}, undefined);
    expect(nowViewOf({ whole, drones })?.running?.map((one) => one.target)).toEqual([DRONE]);
  });

  it("carries its last call as the line and its last three as the tail, from its own transcript only", () => {
    const rows = [called(1, "Read", "a.rs"), called(2, "Read", "b.rs"), called(3, "Edit", "c.rs"), called(4, "Bash", "cargo test")];
    const { whole, drones } = onImplement({}, {}, LISTED, rows);
    const [row] = nowViewOf({ whole, drones })?.running ?? [];
    expect(row?.line).toBe("Bash cargo test");
    expect(row?.tail).toEqual(["Read b.rs", "Edit c.rs", "Bash cargo test"]);
  });

  it("draws no line where the transcript is not in hand", () => {
    const { whole, drones } = onImplement();
    expect(nowViewOf({ whole, drones })?.running?.[0]).not.toHaveProperty("line");
  });
});

describe("the Now panel on a real Job: the Checks", () => {
  const checking = (checks: CheckUnderway[]) => ({ checking: { attempt: 1, checks } });

  it("reads running until a result lands, then passed or failed, in the gate's order", () => {
    const { whole, drones } = onImplement(
      {},
      checking([
        check("typecheck", { ran: { attempt: 1, name: "typecheck", outcome: "passed" } }),
        check("lint", { ran: { attempt: 1, name: "lint", outcome: "failed", produced: "exit code 1" } }),
        check("store"),
      ]),
      { ...LISTED, drones: [LISTED.drones[0]!] },
    );
    const rows = nowViewOf({ whole, drones })?.running ?? [];
    expect(rows.map((one) => [one.name, one.state])).toEqual([
      ["typecheck", "passed"],
      ["lint", "failed"],
      ["store", "running"],
    ]);
    expect(rows[1]?.tail).toEqual(["exit code 1"]);
  });

  it("counts a timeout or a signal as failed, never as a pass", () => {
    const { whole, drones } = onImplement({}, checking([check("slow", { ran: { attempt: 1, name: "slow", outcome: "timed_out" } })]));
    expect(nowViewOf({ whole, drones })?.running?.find((one) => one.name === "slow")?.state).toBe("failed");
  });

  it("leaves out a Check still waiting for a place, and one that was skipped", () => {
    const { whole, drones } = onImplement(
      {},
      checking([check("queued", { started_at: undefined, output_path: undefined }), check("skipped", { ran: { attempt: 1, name: "skipped", outcome: "skipped" } })]),
      { ...LISTED, drones: [] },
    );
    expect(nowViewOf({ whole, drones })).toEqual({});
  });

  it("opens the gate's live log, by the file's own name", () => {
    const { whole, drones } = onImplement({}, checking([check("store")]), { ...LISTED, drones: [] });
    const [row] = nowViewOf({ whole, drones })?.running ?? [];
    expect(row?.log).toEqual({ name: "store", kept: "store.live.log", live: true });
  });

  it("offers retry and skip on a Check that has not passed, and neither on one that has", () => {
    const { whole, drones } = onImplement({}, checking([check("a", { ran: { attempt: 1, name: "a", outcome: "passed" } }), check("b")]), { ...LISTED, drones: [] });
    const [passed, running] = nowViewOf({ whole, drones })?.running ?? [];
    expect(passed?.acts).toBeUndefined();
    expect(running?.acts?.map((one) => one.glyph)).toEqual(["retry", "skip"]);
  });

  it("presses through to the log sheet", () => {
    const { whole, drones } = onImplement({}, checking([check("store")]), { ...LISTED, drones: [] });
    const opened: unknown[] = [];
    const props = nowPanelOf(nowViewOf({ whole, drones }), { onOpenCheckLog: (log) => opened.push(log), onSaid: () => {} });
    props?.running?.[0]?.onOpen();
    expect(opened).toEqual([{ name: "store", kept: "store.live.log", live: true }]);
  });
});

describe("the Now panel on a real Job: the Judge", () => {
  it("runs on the step whose call is out", () => {
    const { whole, drones } = onImplement({}, { judging: JUDGING }, { ...LISTED, drones: [] });
    expect(nowViewOf({ whole, drones })?.running).toEqual([{ key: "judge:implement", of: "judge", name: "Judge on Implement", state: "running", step: STEP }]);
  });

  it("asks what the open judge question asks", () => {
    const { whole, drones } = onImplement({
      judge_question: { step_id: "implement", criterion_id: "C2", question: "Is the retry cap in scope?", expected: "e", produced: "p", consequence: "c", asked_at: "2026-10-09T09:40:00Z" },
    });
    expect(nowViewOf({ whole, drones })?.asks).toEqual([{ key: "judge:implement:C2", kind: "judge", name: "Judge on Implement", text: "Is the retry cap in scope?" }]);
  });
});

describe("the Now panel on a real Job: what a Drone asks", () => {
  const question = {
    question_id: "Q1",
    step_id: "implement",
    asked_at: "2026-10-09T09:41:00Z",
    question: "Which clock does the fixture pin?",
    options: [
      { label: "Store", consequence: "Pins the store clock" },
      { label: "Wall", consequence: "Pins the wall clock" },
    ],
  };

  it("names the one Drone working that step, and opens it", () => {
    const { whole, drones } = onImplement({ asking: question });
    expect(nowViewOf({ whole, drones })?.asks).toEqual([
      { key: "ask:Q1", kind: "drone", name: "Implement Drone", text: "Which clock does the fixture pin?", target: DRONE },
    ]);
  });

  it("names no Drone where two work the step and the wire does not say which", () => {
    const two: JobDrones = { ...LISTED, drones: [LISTED.drones[1]!, { drone_id: OTHER, step_id: "implement", state: "running", since: "2026-10-09T09:11:00Z" }] };
    const { whole, drones } = onImplement({ asking: question }, {}, two);
    expect(nowViewOf({ whole, drones })?.asks?.[0]).not.toHaveProperty("target");
  });

  it("reads a command a Drone is held on, and opens the Drone the wire names", () => {
    const { whole, drones } = onImplement({
      command_waiting: {
        call: "toolu_1",
        step_id: "implement",
        drone_id: DRONE,
        asked_at: "2026-10-09T09:42:00Z",
        tool: "Bash",
        detail: "pnpm add -D reselect",
        truncated: false,
        offers: ["allow_for_job", "reject"],
        rules: [],
      },
    });
    expect(nowViewOf({ whole, drones })?.asks).toEqual([
      { key: "command:toolu_1", kind: "drone", name: "Implement Drone", text: "Bash pnpm add -D reselect", target: DRONE },
    ]);
  });
});

describe("the Now panel on a real Job: what the Job waits on, only where the row says", () => {
  const queued = (over: Parameters<typeof sampleJob>[0]) => {
    const whole = sampleDetail({ job: sampleJob({ status: "queued", ...over }) });
    return { whole, drones: [] };
  };

  it("reads resources from the queued reason", () => {
    expect(nowViewOf(queued({ queued_reason: "waiting_on_resources" }))?.waiting).toEqual([{ key: "held:waiting_on_resources", kind: "resource", text: "Waiting on resources" }]);
  });

  it("says which ceiling an over-budget Job hit", () => {
    expect(nowViewOf(queued({ queued_reason: "over_budget", budget_hold: "cost_cap" }))?.waiting?.[0]?.text).toBe("Over the cost cap");
  });

  it("reads a pause from the marker, at a gate too", () => {
    const whole = sampleDetail({ job: sampleJob({ status: "awaiting_review", paused: { by: "person", at: "2026-10-09T09:00:00Z", resuming: false } }) });
    expect(nowViewOf({ whole, drones: [] })?.waiting).toEqual([{ key: "held:paused", kind: "resource", text: "Paused" }]);
  });

  it("names the Jobs it waits on by their titles, and drops one that is over", () => {
    const board = [sampleJob({ id: "J1", title: "Pin the store clock" }), sampleJob({ id: "J2", title: "Cache the read", status: "completed_success" })];
    const view = nowViewOf({ ...queued({ queued_reason: "blocked_by_dependency", waits_on: ["J1", "J2", "J3"] }), board });
    expect(view?.waiting).toEqual([
      { key: "job:J1", kind: "job", text: "Pin the store clock", target: "J1" },
      { key: "job:J3", kind: "job", text: "J3", target: "J3" },
    ]);
  });

  it("states nothing on a Job that is queued for no reason Fleet gives", () => {
    expect(nowViewOf(queued({}))).toEqual({});
  });
});
