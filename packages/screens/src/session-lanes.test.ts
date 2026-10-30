// What a Job's lanes read from `get_job_lifecycles`, the workflow the record names and the sentences `workflow_waiting_on` words.

import { expect, it } from "vitest";
import type { AppliedRule, HeldInstance, InstanceMove, JobLifecycles, LifecycleDefinition, LifecycleRule, RuleActReading } from "@armada/protocol";
import { jobLanes } from "@armada/components/src/compositions/LaneCanvas/build";
import { nodeId } from "@armada/components/src/compositions/LaneCanvas/model";

import { columnsOf, instanceId, lanesOfJob, ruleOfWire } from "./session-lanes";

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

// ---- Rules and what they did -------------------------------------------------------------------------------------------------

const KEPT = { rank: { reach: "everywhere", source: "armada" } };
const applied = (rule: Partial<LifecycleRule> & Pick<LifecycleRule, "id" | "name" | "when" | "does">): AppliedRule => ({
  rule: { scope: { kind: "everywhere" }, conditions: [], on_failure: { retries: 0, then: "ask_owner" }, ...rule },
  kept: KEPT,
});
const linked = { among: { kind: "linked_from", lifecycle: "pull_request" } } as const;

/** When the PR enters Draft, the ticket moves to In progress. */
const MOVES_TICKET = applied({
  id: "move-ticket",
  name: "Move to In progress",
  when: { kind: "enters", lifecycle: "pull_request", state: "draft" },
  does: [{ kind: "tracker_write", autonomy: "acts", what: { act: "tracker_move", to: "in_progress", target: { among: { kind: "held", lifecycle: "ticket" } } } }],
});
/** When Plan ends, ask for review: it hands to the step. */
const ASKS = applied({
  id: "ask-review",
  name: "Ask for review",
  when: { kind: "step_ends", step: "plan" },
  does: [{ kind: "draft", autonomy: "asks", what: { act: "draft", template: "review", target: linked } }],
});
const BY_JOB = applied({ id: "on-landed", name: "On landed", when: { kind: "job", event: "landed" }, does: [{ kind: "tell_agent", autonomy: "acts", what: { act: "wake" } }] });

const act = (rule: string, on: RuleActReading["on"], standing: string, over: Partial<RuleActReading> = {}): RuleActReading => ({ rule, text: "Move ARM-41 to In progress", course: "acts", standing, at: AT, ...(on === undefined ? {} : { on }), ...over });

const WITH_PULL: JobLifecycles = { ...WITH_TICKET, lifecycles: [ticket, pull], instances: [...WITH_TICKET.instances, held("pull_request", "812", "PR #812", "draft")] };
const ON_TICKET = { lifecycle: "ticket", key: "ARM-41" };
const ENTERED = { lifecycle: "pull_request", key: "812" };

it("reads a rule that moves a state as an edge that lands on that state, and any other as one that lands on the step", () => {
  expect(ruleOfWire(MOVES_TICKET)).toMatchObject({ id: "move-ticket", label: "Move to In progress", from: { lane: "pull_request", item: "draft" }, to: { lane: "ticket", item: "in_progress" }, at: "enters", autonomy: "acts", kept: "armada" });
  expect(ruleOfWire(ASKS)).toMatchObject({ from: { lane: "workflow", item: "plan" }, at: "ends", autonomy: "asks" });
  expect(ruleOfWire(ASKS)!.to).toEqual({ lane: "workflow", item: "$step" });
});

it("leaves a rule a Job event sets off undrawn, since it has no node to leave from", () => {
  expect(ruleOfWire(BY_JOB)).toBeUndefined();
});

it("names a rule's kept place as the person's, the repository's or Armada's", () => {
  const kept = (source: string) => ruleOfWire({ ...MOVES_TICKET, kept: { rank: { reach: "repository", source } } })!.kept;
  expect([kept("armada"), kept("kit"), kept("repository")]).toEqual(["armada", "yours", "repository"]);
});

it("draws a rule that moved a state as an edge between the two lanes, with what it did", () => {
  const lanes = lanesOfJob({
    held: { ...WITH_PULL, acts: [act("move-ticket", ON_TICKET, "done", { entered: ENTERED, settled_at: "2026-10-29T12:05:00.000Z" })] },
    rules: [MOVES_TICKET, BY_JOB],
    working: false,
  })!;
  const { rules } = jobLanes(lanes.job, lanes);
  expect(rules).toHaveLength(1);
  expect(rules[0]).toMatchObject({ rule: "move-ticket", from: nodeId(instanceId("pull_request", "812"), "draft"), to: nodeId(instanceId("ticket", "ARM-41"), "in_progress"), status: "acted", did: "Move ARM-41 to In progress" });
});

it("keeps what each act did and where it stands, and the rule asks while any act waits on the person", () => {
  const lanes = lanesOfJob({
    held: {
      ...WITH_PULL,
      acts: [act("move-ticket", ON_TICKET, "done"), act("move-ticket", ON_TICKET, "stuck", { text: "Assign ARM-41 to you", said: "Not allowed" }), act("move-ticket", ON_TICKET, "planned", { text: "Close ARM-41", course: "asks" })],
    },
    rules: [MOVES_TICKET],
    working: false,
  })!;
  const [one, ...more] = lanes.job.fired;
  expect(more).toEqual([]);
  expect(one).toMatchObject({ rule: "move-ticket", instance: instanceId("ticket", "ARM-41"), outcome: "asked" });
  expect(one!.acts!.map((each) => [each.text, each.standing, each.course, each.said])).toEqual([
    ["Move ARM-41 to In progress", "done", "acts", undefined],
    ["Assign ARM-41 to you", "stuck", "acts", "Not allowed"],
    ["Close ARM-41", "planned", "asks", undefined],
  ]);
  expect(one!.did).toBe("Move ARM-41 to In progress; Assign ARM-41 to you · Stuck: Not allowed; Close ARM-41 · Planned");
});

it("reads the outcome of a rule from where its acts stand", () => {
  const outcome = (standing: string[]) =>
    lanesOfJob({ held: { ...WITH_PULL, acts: standing.map((one) => act("move-ticket", ON_TICKET, one)) }, rules: [MOVES_TICKET], working: false })!.job.fired[0]!.outcome;
  expect([outcome(["done"]), outcome(["done", "told"]), outcome(["planned"]), outcome(["not_carried"]), outcome(["asked"]), outcome(["skipped", "skipped"]), outcome(["skipped", "done"])]).toEqual([
    "acted",
    "acted",
    "waiting",
    "waiting",
    "asked",
    "skipped",
    "acted",
  ]);
});

it("keeps one line for each instance a rule was done to, and none for a rule that is not drawn", () => {
  const other = { lifecycle: "ticket", key: "ARM-42" };
  const two: JobLifecycles = { ...WITH_PULL, instances: [...WITH_PULL.instances, held("ticket", "ARM-42", "ARM-42", "todo")], acts: [act("move-ticket", ON_TICKET, "done"), act("move-ticket", other, "planned"), act("on-landed", undefined, "done")] };
  const fired = lanesOfJob({ held: two, rules: [MOVES_TICKET, BY_JOB], working: false })!.job.fired;
  expect(fired.map((one) => [one.rule, one.instance, one.outcome])).toEqual([
    ["move-ticket", instanceId("ticket", "ARM-41"), "acted"],
    ["move-ticket", instanceId("ticket", "ARM-42"), "waiting"],
  ]);
});

it("hands a step rule to the step the Job was on when it was planned, and keys it by no instance", () => {
  const visits = [
    { step: 0, since: "2026-10-29T11:00:00.000Z" },
    { step: 1, since: "2026-10-29T11:30:00.000Z" },
  ];
  const lanes = lanesOfJob({
    held: { ...WITH_PULL, acts: [act("ask-review", undefined, "asked", { text: "Ask Maya and Jun for review", course: "asks", at: "2026-10-29T11:10:00.000Z" })] },
    rules: [ASKS],
    working: false,
    workflow: { name: "feature", steps: STEPS, at: 1, began: true, visits },
  })!;
  expect(lanes.job.fired[0]).toMatchObject({ rule: "ask-review", instance: "", step: "plan", outcome: "asked" });
  const { rules } = jobLanes(lanes.job, lanes);
  expect(rules[0]).toMatchObject({ rule: "ask-review", from: nodeId("workflow", "plan"), to: nodeId("workflow", "plan"), status: "asked" });
});

it("keys a rule that hands to a step from a state by the instance whose entering set it off", () => {
  const FROM_STATE = applied({ id: "tell-on-draft", name: "Tell on draft", when: { kind: "enters", lifecycle: "pull_request", state: "draft" }, does: [{ kind: "tell_agent", autonomy: "acts", what: { act: "tell", message: "x" } }] });
  const fired = lanesOfJob({ held: { ...WITH_PULL, acts: [act("tell-on-draft", undefined, "told", { entered: ENTERED, text: "Tell the agent" })] }, rules: [FROM_STATE], working: false })!.job.fired;
  expect(fired.map((one) => one.instance)).toEqual([instanceId("pull_request", "812")]);
});

it("draws no edge for a rule whose node is not drawn", () => {
  const lanes = lanesOfJob({ held: { ...WITH_TICKET, acts: [act("move-ticket", ON_TICKET, "done")] }, rules: [MOVES_TICKET, ASKS], working: false })!;
  expect(jobLanes(lanes.job, lanes).rules).toEqual([]);
});

it("draws no act where Fleet sends none, and keeps the lanes as they were", () => {
  const lanes = lanesOfJob({ held: WITH_TICKET, rules: [MOVES_TICKET], working: false })!;
  expect(lanes.job.fired).toEqual([]);
});
