// What a Job's lanes read from `get_job_lifecycles`, the workflow the record names and the sentences `workflow_waiting_on` words.

import { expect, it } from "vitest";
import type { AppliedRule, HeldInstance, InstanceMove, JobLifecycles, LifecycleDefinition, LifecycleRule, RuleActReading } from "@armada/protocol";
import { inFlight, jobLanes } from "@armada/components/src/compositions/LaneCanvas/build";
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

const deploy: LifecycleDefinition = {
  id: "deploy",
  name: "Deploy",
  scope: { kind: "everywhere" },
  states: [
    { id: "rolling_out", label: "Rolling out", read: { source: "watch", state: "rolling_out" } },
    { id: "healthy", label: "Healthy", read: { source: "watch", state: "healthy" } },
  ],
  transitions: [{ from: "rolling_out", to: "healthy" }],
  ends: ["healthy"],
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

it("names where each lane's states are read from, and Fleet for what Fleet's own events say", () => {
  const job: LifecycleDefinition = { id: "job", name: "Job", scope: { kind: "everywhere" }, states: [{ id: "started", label: "Started", read: { source: "fleet", fact: "started" } }], transitions: [], ends: [] };
  const lanes = lanesOfJob({ held: { ...WITH_PULL, lifecycles: [ticket, pull, deploy, job], instances: [...WITH_PULL.instances, held("deploy", "web", "Deploy", "rolling_out"), held("job", "j1", "Fix it", "started")] }, working: false })!;
  expect(lanes.job.copy.lifecycles.map((one) => one.from)).toEqual(["Tracker", "Forge", "Watch", "Fleet"]);
});

it("keeps a key with a slash in one lane id", () => {
  expect(instanceId("ticket", "web/ARM-41")).not.toContain("/");
});

it("leaves no state lit for an instance whose source said something no state claims, and says so on the lane", () => {
  const unmapped: JobLifecycles = { ...WITH_TICKET, instances: [{ ...held("ticket", "ARM-41", "ARM-41", "todo"), standing: { kind: "unmapped", said: "Blocked" } }], moves: [] };
  const [one] = lanesOfJob({ held: unmapped, working: false })!.job.instances;
  expect(one).toMatchObject({ state: "", visited: [], title: "Unmapped: Blocked" });
});

it("lists an instance that stands in no state as in flight, and one ending the Job ended as not", () => {
  const reviewed: LifecycleDefinition = { ...ticket, states: [...ticket.states, { id: "in_review", label: "In review", read: { source: "tracker", words: ["In Review"] } }] };
  const instances: HeldInstance[] = [
    held("deploy", "web", "Deploy", "rolling_out"),
    held("ticket", "ARM-42", "ARM-42", "in_review"),
    held("ticket", "ARM-43", "ARM-43", "done"),
    { ...held("ticket", "ARM-44", "ARM-44", "todo"), standing: { kind: "unmapped", said: "Awaiting QA" } },
    { ...held("ticket", "ARM-45", "ARM-45", "todo"), standing: { kind: "unread" } },
  ];
  const lanes = (all: HeldInstance[]) => lanesOfJob({ held: { session_id: "job", lifecycles: [reviewed, deploy], instances: all, moves: [] }, working: false })!;
  expect(inFlight(lanes(instances).job).map((one) => [one.label, one.state])).toEqual([
    ["Deploy", "Rolling out"],
    ["ARM-42", "In review"],
    ["ARM-44", "Awaiting QA"],
    ["ARM-45", ""],
  ]);

  const ended = lanes(instances.map((one) => ({ ...one, ended_at: AT })));
  expect(inFlight(ended.job)).toEqual([]);
});

it("draws an instance that ending the Job ended where it stood, as ended and not as now", () => {
  const stopped: JobLifecycles = { ...WITH_TICKET, instances: [{ ...held("ticket", "ARM-41", "ARM-41", "in_progress"), ended_at: AT }] };
  const lanes = lanesOfJob({ held: stopped, working: false })!;
  const [lane] = jobLanes(lanes.job, lanes).lanes;
  expect(lane).toMatchObject({ title: "ARM-41", ended: true });
  const stood = lane!.nodes.find((node) => node.label === "In progress");
  expect(stood).toMatchObject({ ended: true });
  expect(stood?.here).toBeUndefined();

  const live = lanesOfJob({ held: WITH_TICKET, working: false })!;
  expect(jobLanes(live.job, live).lanes[0]).not.toHaveProperty("ended");
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

/** When the PR enters Draft, the ticket moves to In progress. */
const MOVES_TICKET = applied({
  id: "move-ticket",
  name: "Move to In progress",
  when: { kind: "enters", lifecycle: "pull_request", state: "draft" },
  does: [{ kind: "tracker_write", autonomy: "acts", what: { act: "tracker_move", to: "in_progress", target: { among: { kind: "held", lifecycle: "ticket" } } } }],
});
/** When the PR is approved, ask for review: it hands to the step. */
const ASKS = applied({
  id: "ask-review",
  name: "Ask for review",
  when: { kind: "enters", lifecycle: "pull_request", state: "approved" },
  does: [{ kind: "draft", autonomy: "asks", what: { act: "draft", template: "review", target: { among: { kind: "entered" } } } }],
});
/** When the PR is approved and the Job is at Land, ask for review: the condition names the step it hands to. */
const ASKS_AT_LAND = applied({ ...ASKS.rule, id: "ask-at-land", conditions: [{ kind: "at_step", step: "land" }] });
/** When Plan ends, tell the agent: nothing names a place for it to land. */
const TELLS = applied({ id: "tell-on-plan", name: "Tell on plan", when: { kind: "step_ends", step: "plan" }, does: [{ kind: "tell_agent", autonomy: "acts", what: { act: "tell", message: "x" } }] });
/** When the PR enters Draft, start a watch on the deploy `web`. */
const WATCHES = applied({
  id: "watch-deploy",
  name: "Watch the deploy",
  when: { kind: "enters", lifecycle: "pull_request", state: "draft" },
  does: [{ kind: "start_watch", autonomy: "acts", what: { act: "start_watch", recipe: "rollout", values: {}, deploy: "web" } }],
});
const BY_JOB = applied({ id: "on-landed", name: "On landed", when: { kind: "job", event: "landed" }, does: [{ kind: "tell_agent", autonomy: "acts", what: { act: "wake" } }] });

const act = (rule: string, on: RuleActReading["on"], standing: string, over: Partial<RuleActReading> = {}): RuleActReading => ({ rule, text: "Move ARM-41 to In progress", course: "acts", standing, at: AT, ...(on === undefined ? {} : { on }), ...over });

const WITH_PULL: JobLifecycles = { ...WITH_TICKET, lifecycles: [ticket, pull], instances: [...WITH_TICKET.instances, held("pull_request", "812", "PR #812", "draft")] };
const ON_TICKET = { lifecycle: "ticket", key: "ARM-41" };
const ENTERED = { lifecycle: "pull_request", key: "812" };

it("lands a rule where it already says: the state it moves, the first state of the deploy it takes on, the step its condition names, and else the step the Job is on", () => {
  expect(ruleOfWire(MOVES_TICKET)).toMatchObject({ id: "move-ticket", label: "Move to In progress", from: { lane: "pull_request", item: "draft" }, to: { lane: "ticket", item: "in_progress" }, at: "enters", autonomy: "acts", kept: "armada" });
  expect(ruleOfWire(ASKS_AT_LAND)).toMatchObject({ from: { lane: "pull_request", item: "approved" }, to: { lane: "workflow", item: "land" }, autonomy: "asks" });
  expect(ruleOfWire(ASKS)!.to).toEqual({ lane: "workflow", item: "$step" });
  expect(ruleOfWire(WATCHES, [deploy])!.to).toEqual({ lane: "deploy", item: "rolling_out" });
  expect(ruleOfWire(WATCHES)!.to.lane).toBe("deploy");
});

it("leaves from the step a step sets a rule off for, and does not draw one a Job event sets off, which has no node to leave from", () => {
  expect(ruleOfWire(TELLS)).toMatchObject({ from: { lane: "workflow", item: "plan" }, at: "ends", to: { lane: "workflow", item: "$step" } });
  expect(ruleOfWire(BY_JOB)).toBeUndefined();
});

it("moves a state over a step its condition names, and over a deploy it takes on", () => {
  const both = applied({ ...MOVES_TICKET.rule, id: "both", conditions: [{ kind: "at_step", step: "land" }], does: [...MOVES_TICKET.rule.does, ...WATCHES.rule.does] });
  expect(ruleOfWire(both, [deploy])!.to).toEqual({ lane: "ticket", item: "in_progress" });
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
  expect(rules[0]).toMatchObject({ rule: "move-ticket", from: nodeId(instanceId("pull_request", "812"), "draft"), to: nodeId(instanceId("ticket", "ARM-41"), "in_progress"), status: "acted", did: ["Move ARM-41 to In progress"] });
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
  expect(one!.did).toEqual(["Move ARM-41 to In progress", "Assign ARM-41 to you · Stuck: Not allowed", "Close ARM-41 · Planned"]);
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

it("hands a rule that names no place to the step the Job was on when it was planned, and keys it by the instance that set it off", () => {
  const visits = [
    { step: 0, since: "2026-10-29T11:00:00.000Z" },
    { step: 1, since: "2026-10-29T11:30:00.000Z" },
  ];
  const lanes = lanesOfJob({
    held: { ...WITH_PULL, acts: [act("ask-review", undefined, "asked", { entered: ENTERED, text: "Ask Maya and Jun for review", course: "asks", at: "2026-10-29T11:10:00.000Z" })] },
    rules: [ASKS],
    working: false,
    workflow: { name: "feature", steps: STEPS, at: 1, began: true, visits },
  })!;
  expect(lanes.job.fired[0]).toMatchObject({ rule: "ask-review", instance: instanceId("pull_request", "812"), step: "plan", outcome: "asked" });
  const { rules } = jobLanes(lanes.job, lanes);
  expect(rules[0]).toMatchObject({ rule: "ask-review", from: nodeId(instanceId("pull_request", "812"), "approved"), to: nodeId("workflow", "plan"), status: "asked" });
});

it("lands a rule on the step its condition names before it fires and after, wherever the Job is", () => {
  const draw = (acts: RuleActReading[]) => {
    const lanes = lanesOfJob({ held: { ...WITH_PULL, acts }, rules: [ASKS_AT_LAND], working: false, workflow: { name: "feature", steps: STEPS, at: 0, began: true } })!;
    return jobLanes(lanes.job, lanes).rules;
  };
  const to = nodeId("workflow", "land");
  expect(draw([])[0]).toMatchObject({ to, status: "waiting" });
  expect(draw([act("ask-at-land", undefined, "done", { entered: ENTERED, text: "Ask Maya and Jun for review", course: "asks" })])[0]).toMatchObject({ to, status: "acted" });
});

it("lands a rule that takes a deploy on in that deploy's lane, keyed by the deploy though the act is done to none", () => {
  const withDeploy: JobLifecycles = {
    ...WITH_PULL,
    lifecycles: [ticket, pull, deploy],
    instances: [...WITH_PULL.instances, held("deploy", "web", "Deploy", "rolling_out")],
    acts: [act("watch-deploy", undefined, "done", { entered: ENTERED, text: "Start the rollout watch" })],
  };
  const lanes = lanesOfJob({ held: withDeploy, rules: [WATCHES], working: false })!;
  expect(lanes.job.fired[0]).toMatchObject({ rule: "watch-deploy", instance: instanceId("deploy", "web"), outcome: "acted" });
  expect(jobLanes(lanes.job, lanes).rules[0]).toMatchObject({ from: nodeId(instanceId("pull_request", "812"), "draft"), to: nodeId(instanceId("deploy", "web"), "rolling_out"), status: "acted" });
});

it("draws nothing for a rule that would leave a step and land on the same one", () => {
  const lanes = lanesOfJob({
    held: { ...WITH_PULL, acts: [act("tell-on-plan", undefined, "done", { text: "Tell the agent", at: "2026-10-29T11:10:00.000Z" })] },
    rules: [TELLS],
    working: false,
    workflow: { name: "feature", steps: STEPS, at: 1, began: true, visits: [{ step: 0, since: "2026-10-29T11:00:00.000Z" }, { step: 1, since: "2026-10-29T11:30:00.000Z" }] },
  })!;
  expect(lanes.job.fired[0]).toMatchObject({ rule: "tell-on-plan", instance: "", step: "plan" });
  expect(jobLanes(lanes.job, lanes).rules).toEqual([]);
});

it("draws no edge for a rule whose node is not drawn", () => {
  const lanes = lanesOfJob({ held: { ...WITH_TICKET, acts: [act("move-ticket", ON_TICKET, "done")] }, rules: [MOVES_TICKET, ASKS], working: false })!;
  expect(jobLanes(lanes.job, lanes).rules).toEqual([]);
});

it("draws no act where Fleet sends none, and keeps the lanes as they were", () => {
  const lanes = lanesOfJob({ held: WITH_TICKET, rules: [MOVES_TICKET], working: false })!;
  expect(lanes.job.fired).toEqual([]);
});

// ---- The Job's own copy -----------------------------------------------------------------------------------------------------

it("draws the rules the Job's copy holds in place of the repository's, and marks what it changed", () => {
  const own = { ...WITH_PULL, rules: [ASKS], changes: [{ lane: "rule", item: "ask-review", text: "Added by you", by: "you", at: AT }] };
  const lanes = lanesOfJob({ held: own, rules: [MOVES_TICKET], working: false })!;
  expect(lanes.job.copy.rules.map((one) => one.id)).toEqual(["ask-review"]);
  const lone = lanesOfJob({ held: WITH_PULL, rules: [MOVES_TICKET], working: false })!;
  expect(lone.job.copy.rules.map((one) => one.id)).toEqual(["move-ticket"]);
  expect(lanes.job.changes).toEqual([{ id: `rule:ask-review:${AT}`, lane: "rule", item: "ask-review", text: "Added by you", at: AT }]);
});

it("carries which change took a rule out of the copy, so the lanes offer nothing to keep for it", () => {
  const own: JobLifecycles = {
    ...WITH_PULL,
    changes: [
      { lane: "rule", item: "ask-at-land", text: "Dropped by you", by: "you", at: AT, dropped: true },
      { lane: "rule", item: "ask-review", text: "Added by you", by: "you", at: AT },
    ],
  };
  const lanes = lanesOfJob({ held: own, working: false, workflow: { name: "feature", steps: STEPS, at: 0, began: true } })!;
  expect(lanes.job.changes.map((one) => [one.item, one.dropped])).toEqual([["ask-at-land", true], ["ask-review", undefined]]);
});

it("marks a step, a rule and a lifecycle lane that the Job's copy changed, each with who made the change", () => {
  const own: JobLifecycles = {
    ...WITH_PULL,
    rules: [ASKS_AT_LAND],
    changes: [
      { lane: "workflow", item: "plan", text: "Added after Propose by the Job", by: "job", at: AT },
      { lane: "rule", item: "ask-at-land", text: "Added by you", by: "you", at: AT },
      { lane: "ticket", item: "", text: "Changed by the Job", by: "job", at: AT },
    ],
  };
  const lanes = lanesOfJob({ held: own, working: false, workflow: { name: "feature", steps: STEPS, at: 0, began: true } })!;
  const drawn = jobLanes(lanes.job, lanes);
  expect(drawn.lanes[0]!.nodes.find((node) => node.label === "Plan")).toMatchObject({ changed: "Added after Propose by the Job" });
  expect(drawn.lanes[0]!.nodes.find((node) => node.label === "Land")?.changed).toBeUndefined();
  expect(drawn.lanes.find((lane) => lane.title === "ARM-41")).toMatchObject({ changed: "Changed by the Job" });
  expect(drawn.lanes.find((lane) => lane.title.startsWith("PR"))?.changed).toBeUndefined();
  expect(drawn.rules.find((rule) => rule.rule === "ask-at-land")).toMatchObject({ changed: "Added by you" });
});
