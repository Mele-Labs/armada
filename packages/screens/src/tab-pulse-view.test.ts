// What the Pulse board says a Job is taking, and the answers it must not round.
//
// The board is built on `PulseView`, which is derived from today's wire. These
// are the decisions between that shape and the rows the panel draws: which
// checkout a look's finding is about, and what each figure reads when its
// reading is nought or is a fault.

import { describe, expect, it } from "vitest";

import type { JobExamined, Look, StepDetail } from "@armada/protocol";
import type { PulseView } from "./draft/pulse";
import { detail, job, spend } from "./fixtures/build/base";
import { checksRunning, judgesRunning, pulseFiguresOf, pulseReadingOf } from "./resources";

const BRANCH = "armada/1538-pulse";
const JOB_LOG = ".armada/logs/1538-pulse.jsonl";
const TRANSCRIPT = ".armada/transcripts/1538-pulse/01DRONE.jsonl";

function view(over: Partial<PulseView> = {}): PulseView {
  return {
    job: "01J",
    read_at: "2026-09-22T10:30:00.000Z",
    held: "running",
    processes: [
      {
        pid: 4121,
        command: "node",
        cpu_percent: 12.5,
        memory_bytes: 184_000_000,
        running_for: "04:12",
        recorded: true,
        owner: BRANCH,
      },
    ],
    worktrees: [{ path: "/repo/.armada/worktrees/1538", branch: BRANCH, bytes: 1_020_054_016 }],
    logs: [{ kind: "job", path: JOB_LOG, owner: null, writing: false }],
    ...over,
  };
}

function looked(found: Look["found"]): JobExamined {
  return {
    job_id: "01J",
    looked_at: "2026-09-22T10:30:00.000Z",
    found: "cannot_tell",
    looks: [{ asked: "worktree", found, said: "" }],
    resources: { job_id: "01J", read_at: "2026-09-22T10:30:00.000Z", held: "running", processes: [] },
  };
}

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "implement",
    label: "Implement",
    ordinal: 1,
    state: "running",
    check_runs: [],
    judged: [],
    flagged: [],
    overridden: false,
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-22T09:22:00Z",
    updated_at: "2026-09-22T10:30:00Z",
    ...over,
  };
}

describe("the rows the board draws", () => {
  it("carries the owner through, so a process names the checkout it is in", () => {
    expect(pulseReadingOf(view(), null).processes[0]?.owner).toBe(BRANCH);
  });

  it("says a process nothing placed is not placed, rather than dropping the row", () => {
    const one = view({ processes: [{ ...view().processes[0]!, owner: null }] });

    expect(pulseReadingOf(one, null).processes).toHaveLength(1);
    expect(pulseReadingOf(one, null).processes[0]?.owner).toBeNull();
  });

  it("keeps an unmeasured worktree unmeasured, never a zero", () => {
    const one = view({ worktrees: [{ path: "/repo/.armada/worktrees/1538", branch: BRANCH }] });

    expect(pulseReadingOf(one, null).worktrees[0]?.bytes).toBeUndefined();
  });

  // Fleet keeps a size for 30 s, so the reading's own age would understate it.
  it("ages a worktree's size from its measured_at, not the reading's read_at", () => {
    const kept = view({
      worktrees: [{ ...view().worktrees[0]!, measured_at: "2026-09-22T10:29:35.000Z" }],
    });
    const now = Date.parse("2026-09-22T10:30:05.000Z");

    expect(pulseReadingOf(kept, null, null, now).worktrees[0]?.age).toBe("30s");
  });

  it("puts a look's finding on the one checkout it asked about", () => {
    const [row] = pulseReadingOf(view(), looked("not_working")).worktrees;

    expect(row?.state).toBe("gone");
    expect(row?.wrong).toBe(true);
  });

  it("leaves several checkouts alone, because the look asked about one", () => {
    const two = view({
      worktrees: [
        { path: "/a", branch: "armada/a" },
        { path: "/b", branch: "armada/b" },
      ],
    });

    expect(pulseReadingOf(two, looked("not_working")).worktrees.map((one) => one.wrong)).toEqual([
      undefined,
      undefined,
    ]);
  });

  it("says whether the Job's one checkout has its Drone in it, and offers to open it", () => {
    const [working] = pulseReadingOf(view(), null).worktrees;
    const [idle] = pulseReadingOf(view({ held: "none" }), null).worktrees;

    expect([working?.state, working?.working, working?.open]).toEqual(["1 drone working", true, "worktree"]);
    expect(idle?.state).toBe("no drone working");
  });

  it("offers Open on the Job's own log, and on no transcript", () => {
    const logs = pulseReadingOf(
      view({
        logs: [
          { kind: "job", path: JOB_LOG, owner: null, writing: false },
          { kind: "transcript", path: TRANSCRIPT, owner: null, writing: false },
        ],
      }),
      null,
    ).logs;

    expect(logs.map((one) => one.open)).toEqual(["log", undefined]);
  });

  it("names a transcript by its Drone, so two of them are two rows", () => {
    const one = view({ logs: [{ kind: "transcript", path: TRANSCRIPT, owner: null, writing: true }] });

    expect(pulseReadingOf(one, null).logs[0]?.about).toBe("01DRONE");
  });

  it("draws a brief on both reads once: named by the Job, weighed by the reading", () => {
    const judged = { attempt: 1, criterion_id: "no_drift", verdict: "met", brief_path: "briefs/no_drift-1.md" };
    const whole = detail(job("running"), [step({ judged: [judged] })]);
    const read = view({
      logs: [
        { kind: "job", path: JOB_LOG, owner: null, bytes: 18_204, writing: true },
        { kind: "brief", path: "briefs/no_drift-1.md", owner: null, bytes: 6_204, writing: false },
      ],
    });

    expect(pulseReadingOf(read, null, whole).logs).toEqual([
      { kind: "job", owner: null, bytes: 18_204, writing: true, open: "log" },
      {
        kind: "brief",
        owner: null,
        about: "implement · no_drift",
        bytes: 6_204,
        writing: false,
        open: { kept: "briefs/no_drift-1.md", what: "brief" },
      },
    ]);
  });

  it("draws a brief only the reading lists by its file, with no Open main would refuse", () => {
    const read = view({
      logs: [{ kind: "brief", path: "briefs/regression_verify.1.gaming.md", owner: null, bytes: 5_377, writing: false }],
    });

    expect(pulseReadingOf(read, null, null).logs).toEqual([
      { kind: "brief", owner: null, about: "regression_verify.1.gaming", bytes: 5_377, writing: false },
    ]);
  });

  it("lists each brief the Judge was asked with once, off the Job rather than the reading", () => {
    const judged = { attempt: 1, criterion_id: "no_drift", verdict: "met", brief_path: "briefs/no_drift-1.md" };
    const whole = detail(job("running"), [step({ judged: [judged, { ...judged, member: 2 }] })]);

    const briefs = pulseReadingOf(view(), null, whole).logs.filter((one) => one.kind === "brief");

    expect(briefs).toEqual([
      {
        kind: "brief",
        owner: null,
        about: "implement · no_drift",
        writing: false,
        open: { kept: "briefs/no_drift-1.md", what: "brief" },
      },
    ]);
  });

  it("marks a log that is still being written", () => {
    const one = view({ logs: [{ kind: "job", path: JOB_LOG, owner: null, writing: true }] });

    expect(pulseReadingOf(one, null).logs[0]?.writing).toBe(true);
  });
});

describe("the figures over the board", () => {
  it("counts a Drone that is not up as 0, since the label carries the verb", () => {
    const figures = pulseFiguresOf(view({ held: "none", processes: [] }), null);

    expect(figures.find((one) => one.label === "Drones running")?.value).toBe("0");
  });

  it("counts the one Drone Fleet recorded while it is up", () => {
    expect(pulseFiguresOf(view(), null).find((one) => one.label === "Drones running")?.value).toBe("1");
  });

  it("draws a recorded pid nothing holds as a fault, not as an idle Job", () => {
    const drones = pulseFiguresOf(view({ held: "gone" }), null).find(
      (one) => one.label === "Drones running",
    );

    expect(drones?.value).toBe("0");
    expect(drones?.detail).toBe("fleet recorded one");
    expect(drones?.wrong).toBe(true);
  });

  it("does not claim a Drone count where the probe would not run", () => {
    const drones = pulseFiguresOf(view({ held: "unreadable" }), null).find(
      (one) => one.label === "Drones running",
    );

    expect(drones?.value).toBe("could not be read");
    // Words rather than a reading, so the band draws it in sans at body size.
    expect(drones?.words).toBe(true);
  });

  it("leaves the Drones row out entirely where nothing read the machine", () => {
    expect(pulseFiguresOf(null, null).map((one) => one.label)).not.toContain("Drones running");
  });

  it("draws neither cap where Fleet does not count, rather than a zero", () => {
    const labels = pulseFiguresOf(view(), null).map((one) => one.label);

    expect(labels).not.toContain("Spend");
    expect(labels).not.toContain("Turns");
  });

  it("presses Spend and Turns through to their caps only where the caller has somewhere to send them", () => {
    const whole = detail(job("running"), [], { spend: spend() });
    const cost = () => undefined;
    const turns = () => undefined;

    const pressed = pulseFiguresOf(view(), whole, { cost, turns });
    expect(pressed.find((one) => one.label === "Spend")?.onPress).toBe(cost);
    expect(pressed.find((one) => one.label === "Turns")?.onPress).toBe(turns);
    expect(pressed.find((one) => one.label === "Spend")?.pressLabel).toBe("Change the cost cap in Settings");

    expect(pulseFiguresOf(view(), whole).some((one) => one.onPress !== undefined)).toBe(false);
  });

  it("puts the rule on the first figure that is a cost, and on no other", () => {
    const figures = pulseFiguresOf(view(), null);

    // No spend and no turns here, so Processes is what the rule falls on.
    expect(figures.filter((one) => one.apart === true).map((one) => one.label)).toEqual([
      "Processes",
    ]);
  });
});

describe("what is running on the Job's own steps", () => {
  it("counts a Check the gate started and has not finished", () => {
    const running = step({
      checks: [{ kind: "manifest_check", name: "test", when: [], expect_exit_code: 0 }],
      checking: { attempt: 1, checks: [{ name: "test", started_at: "2026-09-22T10:29:00Z" }] },
    });

    expect(checksRunning([running])).toBe(1);
  });

  it("does not count a Check that has finished on this attempt", () => {
    const done = step({
      checks: [{ kind: "manifest_check", name: "test", when: [], expect_exit_code: 0 }],
      check_runs: [{ attempt: 1, name: "test", outcome: "passed" }],
      attempts: [{ attempt: 1, outcome: "advanced", started_at: "2026-09-22T09:22:00Z" }],
    });

    expect(checksRunning([done])).toBe(0);
  });

  it("counts a Judge call that is out, which is not a step state", () => {
    const judging = step({
      judging: {
        look: "criterion",
        model: "sonnet",
        call: 1,
        of: 2,
        since: "2026-09-22T10:29:30Z",
        budget_ms: 120_000,
      },
    });

    expect(judgesRunning([judging, step()])).toBe(1);
  });
});
