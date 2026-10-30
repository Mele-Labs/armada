// What a Job's lanes read from `get_job_lifecycles`, the workflow the record names and the sentences `workflow_waiting_on` words.

import { expect, it } from "vitest";
import type { HeldInstance, InstanceMove, JobLifecycles, LifecycleDefinition } from "@armada/protocol";
import { jobLanes } from "@armada/components/src/compositions/LaneCanvas/build";

import { columnsOf, instanceId, lanesOfJob } from "./session-lanes";

const AT = "2026-10-29T12:00:00.000Z";

const ticket: LifecycleDefinition = {
  id: "ticket",
  name: "Ticket",
  scope: { kind: "everywhere" },
  states: [
    { id: "todo", label: "To do", read: { source: "tracker", words: ["To Do"] } },
    { id: "ready", label: "Ready", read: { source: "tracker", words: ["Ready"] } },
    { id: "in_progress", label: "In progress", read: { source: "tracker", words: ["In Progress"] } },
    { id: "done", label: "Done", read: { source: "tracker", words: ["Done"] } },
  ],
  transitions: [
    { from: "todo", to: "ready" },
    { from: "todo", to: "in_progress" },
    { from: "ready", to: "in_progress" },
    { from: "in_progress", to: "done" },
  ],
  ends: ["done"],
};

const pull: LifecycleDefinition = {
  id: "pull_request",
  name: "PR",
  scope: { kind: "everywhere" },
  states: [
    { id: "draft", label: "Draft", read: { source: "forge", fact: "draft" } },
    { id: "approved", label: "Approved", read: { source: "forge", fact: "approved" } },
  ],
  transitions: [{ from: "draft", to: "approved" }],
  ends: [],
};

const held = (lifecycle: string, key: string, label: string, state: string): HeldInstance => ({ lifecycle, key, label, standing: { kind: "in", state }, links: [] });
const move = (lifecycle: string, key: string, from: InstanceMove["from"], state: string): InstanceMove => ({ lifecycle, key, from, to: { kind: "in", state }, source: "tracker", at: AT });

const WITH_TICKET: JobLifecycles = {
  session_id: "job",
  lifecycles: [ticket],
  instances: [held("ticket", "ARM-41", "ARM-41", "in_progress")],
  moves: [move("ticket", "ARM-41", { kind: "unread" }, "todo"), move("ticket", "ARM-41", { kind: "in", state: "todo" }, "in_progress")],
};

const STEPS = [
  { id: "plan", label: "Plan" },
  { id: "land", label: "Land" },
];

it("draws no lanes for a Job that holds no instance", () => {
  expect(lanesOfJob({ held: { session_id: "job", lifecycles: [], instances: [], moves: [] }, working: false })).toBeUndefined();
});

it("places each state after the ones that lead to it, and never beside another in its lane", () => {
  const cols = columnsOf(ticket);
  expect([...cols]).toEqual([
    ["todo", 0],
    ["ready", 1],
    ["in_progress", 2],
    ["done", 3],
  ]);
  const branching = columnsOf({ states: ticket.states, transitions: [{ from: "todo", to: "ready" }, { from: "todo", to: "in_progress" }, { from: "in_progress", to: "done" }] });
  expect(new Set(branching.values()).size).toBe(4);
  expect(branching.get("done")!).toBeGreaterThan(branching.get("in_progress")!);
});

it("reads where an instance stands, the states it went through and the lifecycle's ends", () => {
  const lanes = lanesOfJob({ held: WITH_TICKET, working: false })!;
  const [one] = lanes.job.instances;
  expect(one).toMatchObject({ id: instanceId("ticket", "ARM-41"), lifecycle: "ticket", label: "ARM-41", state: "in_progress", visited: ["todo", "in_progress"] });
  const [lifecycle] = lanes.job.copy.lifecycles;
  expect(lifecycle).toMatchObject({ id: "ticket", label: "Ticket", kind: "tracker" });
  expect(lifecycle!.states.find((state) => state.id === "done")).toMatchObject({ end: true });
  expect(lifecycle!.states.find((state) => state.id === "todo")?.end).toBeUndefined();
});

it("keeps a key with a slash in one lane id", () => {
  expect(instanceId("ticket", "web/ARM-41")).not.toContain("/");
});

it("leaves no state lit for an instance whose source said something no state claims, and says so on the lane", () => {
  const unmapped: JobLifecycles = { ...WITH_TICKET, instances: [{ ...held("ticket", "ARM-41", "ARM-41", "todo"), standing: { kind: "unmapped", said: "Blocked" } }], moves: [] };
  const [one] = lanesOfJob({ held: unmapped, working: false })!.job.instances;
  expect(one).toMatchObject({ state: "", visited: [], title: "Unmapped: Blocked" });
});

it("draws the workflow on top with the step the Job is at, and one line for each sentence the step waits with", () => {
  const withPull: JobLifecycles = { ...WITH_TICKET, lifecycles: [ticket, pull], instances: [...WITH_TICKET.instances, held("pull_request", "812", "PR #812", "draft")], moves: WITH_TICKET.moves };
  const lanes = lanesOfJob({
    held: withPull,
    working: true,
    waiting: ["Waiting on PR #812: Approved", "Waiting on ARM-41: Done"],
    workflow: { name: "feature", steps: STEPS, at: 1, began: true },
  })!;
  const drawn = jobLanes(lanes.job, lanes);
  expect(drawn.lanes.map((lane) => lane.title)).toEqual(["feature", "ARM-41", "PR #812"]);
  const steps = drawn.lanes[0]!.nodes.filter((node) => node.role === "step" && node.kind !== "start" && node.kind !== "end");
  expect(steps.map((node) => [node.label, node.passed, node.lit, node.waiting])).toEqual([
    ["Plan", true, undefined, undefined],
    ["Land", undefined, true, ["Waiting on PR #812: Approved", "Waiting on ARM-41: Done"]],
  ]);
});

it("holds the step at nothing before one starts and past the last once every one has ended, where Fleet names no step", () => {
  const before = lanesOfJob({ held: WITH_TICKET, working: false, workflow: { name: "feature", steps: STEPS, began: false } })!;
  const after = lanesOfJob({ held: WITH_TICKET, working: false, workflow: { name: "feature", steps: STEPS, began: true } })!;
  expect([before.at, after.at]).toEqual([-1, STEPS.length]);
  expect(before.working).toBe(false);
});

it("draws no top lane for a Job with no workflow, and says nothing of waiting when the list is empty", () => {
  const lanes = lanesOfJob({ held: WITH_TICKET, working: true, waiting: [] })!;
  expect(lanes.job.waiting).toBeUndefined();
  expect(jobLanes(lanes.job, lanes).lanes.map((lane) => lane.id)).toEqual([instanceId("ticket", "ARM-41")]);
});

it("passes the steps a person skipped on", () => {
  const lanes = lanesOfJob({ held: WITH_TICKET, working: false, workflow: { name: "feature", steps: STEPS, at: 0, began: true, skipped: [1] } })!;
  expect(lanes.skipped).toEqual([1]);
});
