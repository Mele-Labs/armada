// A Record row, from the three shapes the log admits.

import type { AskedRun, JobDetail, Recorded } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { executingSequential } from "../fixtures/build/arc";
import { reviewHeldByPolicy } from "../fixtures/build/policy";
import {
  countsOf,
  familyOf,
  LEDGER_FAMILIES,
  ledgerOf,
  ledgerRowOf,
  ledgerRowsOf,
  unfiledIn,
} from "./ledger";

function moved(over: Partial<Recorded> = {}): Recorded {
  return {
    seq: 7,
    status: "running",
    moved: { kind: "status", to: "awaiting_review" },
    actor: "fleet",
    at: "2026-09-22T10:00:00Z",
    ...over,
  };
}

describe("the door an act came through", () => {
  it("marks a person's act taken on the phone, and no other door", () => {
    expect(ledgerRowOf(moved({ actor: "human", via: "phone" })).fromPhone).toBe(true);
    expect(ledgerRowOf(moved({ actor: "human", via: "phone" })).actor).toBe("person");
    expect(ledgerRowOf(moved({ actor: "human", via: "bridge" })).fromPhone).toBeUndefined();
    expect(ledgerRowOf(moved({ actor: "human" })).fromPhone).toBeUndefined();
  });
});

describe("where a row happened", () => {
  it("names no step for the Job's own machine moving, which is a fact not a gap", () => {
    expect(ledgerRowOf(moved()).coord).toBeNull();
  });

  it("names the step for a step move", () => {
    const row = ledgerRowOf(
      moved({ moved: { kind: "step", step_id: "implement", from: "running", to: "advanced" } }),
    );

    expect(row.coord).toEqual({ step: "implement", step_attempt: 1 });
  });

  it("names no group and no task, because the wire has neither", () => {
    const row = ledgerRowOf(
      moved({ moved: { kind: "drone", step_id: "implement", drone_id: "01D", presence: "drone_spawned" } }),
    );

    expect(row.coord?.group).toBeUndefined();
    expect(row.coord?.task).toBeUndefined();
  });
});

describe("what a row is", () => {
  it("qualifies a status move by where it went, so kinds stay distinguishable", () => {
    expect(ledgerRowOf(moved()).kind).toBe("status_awaiting_review");
  });

  it("takes a Drone row's kind from its presence", () => {
    const row = ledgerRowOf(
      moved({ moved: { kind: "drone", step_id: "plan", drone_id: "01D", presence: "drone_exited" } }),
    );

    expect(row.kind).toBe("drone_exited");
  });

  it("is a plain string, so a new kind is not a major version", () => {
    expect(typeof ledgerRowOf(moved()).kind).toBe("string");
  });
});

describe("who a row is about", () => {
  it("calls the wire's human a person", () => {
    expect(ledgerRowOf(moved({ actor: "human" })).actor).toBe("person");
  });

  it("carries a Drone through", () => {
    expect(ledgerRowOf(moved({ actor: "drone" })).actor).toBe("drone");
  });

  it("reads anything else as Fleet, since judge and check are the draft's own", () => {
    expect(ledgerRowOf(moved({ actor: "fleet" })).actor).toBe("fleet");
    expect(ledgerRowOf(moved({ actor: "something" })).actor).toBe("fleet");
  });
});

describe("what a row came to", () => {
  it("carries a status move's reason where it stored one", () => {
    const row = ledgerRowOf(
      moved({ moved: { kind: "status", to: "escalated", reason: { named: "gate_failure" } } }),
    );

    expect(row.outcome).toBe("gate_failure");
  });

  it("is empty on the destinations that store none, not a placeholder", () => {
    expect(ledgerRowOf(moved()).outcome).toBe("");
  });

  it("carries the why on the one step move that stops a step", () => {
    const row = ledgerRowOf(
      moved({
        moved: { kind: "step", step_id: "tests", from: "running", to: "stopped", why: "gate_failure" },
      }),
    );

    expect(row.outcome).toBe("gate_failure");
  });
});

describe("what a reader pages with", () => {
  it("is the log's own seq and never the instant", () => {
    const row = ledgerRowOf(moved({ seq: 42, at: "2026-09-22T10:00:00Z" }));

    expect(row.cursor).toBe(42);
  });

  it("keeps rows in seq order, including two inside one millisecond", () => {
    const rows = ledgerRowsOf({
      job_id: "01J",
      moves: [
        moved({ seq: 1, at: "2026-09-22T10:00:00Z" }),
        moved({ seq: 2, at: "2026-09-22T10:00:00Z" }),
      ],
    });

    expect(rows.map((row) => row.cursor)).toEqual([1, 2]);
  });

  it("is empty on a Job created and not yet moved, which is a real answer", () => {
    expect(ledgerRowsOf({ job_id: "01J", moves: [] })).toEqual([]);
  });
});

describe("the nine filters", () => {
  it("puts a Check's run and a Judge's answer in different families", () => {
    expect(familyOf("checked")).toBe("checks");
    expect(familyOf("judged")).toBe("judges");
  });

  it("files a task's own done under Tasks and never under Evidence", () => {
    expect(familyOf("task_done")).toBe("tasks");
  });

  it("files a case run under Tests, which is never an Evidence row", () => {
    expect(familyOf("case_run")).toBe("tests");
    expect(familyOf("cases_rerun")).toBe("tests");
  });

  // The owner asked for the Job filter on 28 September 2026, standing on the
  // line that said these rows answered to nothing.
  it("files the Job's own machine moving under Job, whatever it moved to", () => {
    expect(familyOf("created")).toBe("job");
    expect(familyOf("started")).toBe("job");
    expect(familyOf("status_completed_success")).toBe("job");
    expect(familyOf("status_queued")).toBe("job");
  });

  it("gives a kind it has never heard of no family rather than guessing one", () => {
    expect(familyOf("something_the_backend_invented")).toBeNull();
  });

  it("counts no row twice, which is the invariant — never that the eight sum to All", () => {
    const rows = ledgerOf({ detail: arcDetail() });
    const counts = countsOf(rows);
    const filed = LEDGER_FAMILIES.reduce((total, one) => total + counts[one], 0);

    expect(filed).toBe(rows.length - unfiledIn(rows).length);
  });

  it("leaves nothing under All alone once the Job's own rows have a filter", () => {
    const rows = ledgerOf({ detail: arcDetail() });

    expect(rows.some((row) => row.kind === "created")).toBe(true);
    expect(unfiledIn(rows)).toEqual([]);
  });
});

describe("the Record, composed from today's reads", () => {
  it("reads newest first", () => {
    const rows = ledgerOf({ detail: arcDetail() });

    expect(rows.map((row) => row.at)).toEqual([...rows.map((row) => row.at)].sort().reverse());
  });

  it("gives a Check's run its own row, with Check in the who column", () => {
    const rows = ledgerOf({ detail: arcDetail() }).filter((row) => row.kind === "checked");

    expect(rows.map((row) => row.what)).toContain("typecheck");
    expect(rows.every((row) => row.actor === "check")).toBe(true);
  });

  it("says Judge on a criterion answered and Fleet on the plan being recorded", () => {
    const rows = ledgerOf({ detail: arcDetail() });

    expect(rows.find((row) => row.kind === "judged")?.actor).toBe("judge");
    expect(rows.find((row) => row.kind === "plan_recorded")?.actor).toBe("fleet");
  });

  it("places a task row down to its group and its task", () => {
    const rows = ledgerOf({ detail: arcDetail() }).filter((one) => one.kind === "task_done");

    expect(rows.every((row) => row.coord?.group !== undefined)).toBe(true);
    expect(rows.map((row) => row.coord?.task)).toContain("T1");
  });

  it("names no step at all for the Job's own machine moving", () => {
    const row = ledgerOf({ detail: arcDetail() }).find((one) => one.kind === "created");

    expect(row?.coord).toBeNull();
  });

  // **Never "outside the plan".** `docs/concepts/plan.md` forbids that name for
  // a file scope, and the owner asked what it meant on this exact cell.
  it("says a finished task's files were in scope, or names the ones that were not", () => {
    const rows = ledgerOf({ detail: arcDetail() }).filter((row) => row.kind === "task_files");

    expect(rows.some((row) => row.outcome === "In scope")).toBe(true);
    expect(rows.some((row) => /^[^/]+ (was|were) out of scope$/.test(row.outcome))).toBe(true);
    expect(rows.every((row) => !/the plan/.test(row.outcome))).toBe(true);
  });

  it("lets the history own the moves it carries, rather than deriving them twice", () => {
    const detail = arcDetail();
    const history = [
      moved({ seq: 1, moved: { kind: "status", to: "queued" }, actor: "human", at: detail.created_at }),
    ];
    const rows = ledgerOf({ detail, history });

    expect(rows.filter((row) => row.coord === null).map((row) => row.kind)).toEqual([
      "status_queued",
    ]);
  });

  it("draws a Job with no plan and nothing run, rather than nothing at all", () => {
    const rows = ledgerOf({ detail: { ...arcDetail(), steps: [], work_plan: undefined } });

    expect(rows.map((row) => row.kind)).toContain("created");
  });
});

describe("the Checks a Drone asked for", () => {
  const asked = (over: Partial<AskedRun> = {}): AskedRun => ({
    id: 1,
    requester: { kind: "drone_step", job_id: "01JOB", step: "implement", drone_id: "01DRONE" },
    attempt: 1,
    started_at: "2026-10-06T10:00:00.000Z",
    state: "failed",
    checks: ["suite", "lint"],
    narrowed: false,
    ...over,
  });
  const withAsked = (...runs: AskedRun[]): JobDetail => {
    const detail = arcDetail();
    const [first, ...rest] = detail.steps;
    return { ...detail, steps: [{ ...first!, asked_runs: runs }, ...rest] };
  };

  it("gives each asked run its own row, signed by the Drone, under the Checks filter", () => {
    const rows = ledgerOf({ detail: withAsked(asked(), asked({ id: 2, state: "passed" })) }).filter(
      (row) => row.kind === "asked_run",
    );

    expect(rows).toHaveLength(2);
    expect(rows.every((row) => row.actor === "drone")).toBe(true);
    expect(familyOf("asked_run")).toBe("checks");
  });

  it("says how each ended, and names the Checks it was about", () => {
    const rows = ledgerOf({
      detail: withAsked(asked({ state: "lost" }), asked({ id: 2, state: "running" })),
    }).filter((row) => row.kind === "asked_run");

    expect(rows.map((row) => row.outcome).sort()).toEqual(["Lost", "Running"]);
    expect(rows[0]!.what).toContain("suite, lint");
  });

  it("is never a Check's row", () => {
    const rows = ledgerOf({ detail: withAsked(asked()) });

    expect(rows.filter((row) => row.kind === "checked").every((row) => row.actor === "check")).toBe(true);
    expect(rows.filter((row) => row.kind === "asked_run").some((row) => row.actor === "check")).toBe(false);
  });

  it("places the row on the step and run it was asked in", () => {
    const row = ledgerOf({ detail: withAsked(asked({ attempt: 2 })) }).find(
      (one) => one.kind === "asked_run",
    );

    expect(row?.coord?.step_attempt).toBe(2);
  });
});

/** The arc's own Job mid-`implement`, which is the richest Record there is. */
function arcDetail(): JobDetail {
  const moment = executingSequential();
  const read = moment.fixtures[0]!.watched;
  if (read.state !== "read") throw new Error("the arc's implement moment carries no detail");
  return read.detail;
}

// #1683. Regression check defers to the repository's review-gate policy and
// held twice; only the second run carries what the policy resolved to.
describe("what the repository said, on a run its gate asked", () => {
  it("says it on the step move that closed the run, and nothing on the older run", () => {
    const { detail, moves } = heldByPolicy();
    const rows = ledgerOf({ detail, history: moves }).filter(
      (row) => row.kind === "step" && row.what.includes("regression_verify, running to awaiting_human"),
    );

    expect(rows.map((row) => row.outcome)).toEqual(["The repository said a person answers", ""]);
  });

  // The owner, 2 Oct 2026: the rule must not read as what failed the run.
  it("says a run that ended before the gate never reached the rule it names", () => {
    const { detail, moves } = heldByPolicy();
    const row = ledgerOf({ detail, history: moves }).find(
      (one) => one.kind === "step" && one.what.includes("running to retrying"),
    );

    expect(row?.outcome).toBe(
      "gate_failure: the repository said a person answers, but the run ended before that gate",
    );
  });

  it("says it on the run's own row where no history was read", () => {
    const { detail } = heldByPolicy();
    const rows = ledgerOf({ detail }).filter(
      (row) => row.kind === "drone_exited" && row.coord?.step === "regression_verify",
    );

    expect(rows.map((row) => row.outcome)).toEqual([
      "Awaiting_human: the repository said a person answers",
      "Retrying — gate_failure: the repository said a person answers, but the run ended before that gate",
      "Awaiting_human",
    ]);
  });

  it("says nothing where the step's own gate decided, whatever the run recorded", () => {
    const { detail, moves } = heldByPolicy();
    const ownGate = {
      ...detail,
      steps: detail.steps.map((step) => ({ ...step, advance_gate: "human_always" })),
    };
    const rows = ledgerOf({ detail: ownGate, history: moves });

    expect(rows.some((row) => row.outcome.includes("the repository said"))).toBe(false);
  });

  it("reads the auto-merge policy on a step that defers to it", () => {
    const { detail, moves } = heldByPolicy();
    const merging = {
      ...detail,
      steps: detail.steps.map((step) =>
        step.step_id === "regression_verify" ? { ...step, advance_gate: "manifest_rule:auto_merge" } : step,
      ),
    };
    const rows = ledgerOf({ detail: merging, history: moves });

    expect(rows.map((row) => row.outcome)).toContain(
      "The repository said Fleet merges once the forge's checks pass",
    );
  });
});

function heldByPolicy(): { detail: JobDetail; moves: Recorded[] } {
  const fixture = reviewHeldByPolicy();
  if (fixture.watched.state !== "read" || fixture.history?.state !== "read") {
    throw new Error("the policy fixture carries its detail and its history");
  }
  return { detail: fixture.watched.detail, moves: fixture.history.moves };
}
