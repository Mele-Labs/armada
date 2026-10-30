// The mock Jobs' lanes as Fleet would serve them. `workspaces-lanes-data.ts` models what the Jobs hold; this turns it into `get_job_lifecycles` and the sentences
// `workflow_waiting_on` words, and draws the lanes from those by the same function a real Job's are (`lanesOfJob`), so a change to how they are read shows on the walks.
// **What the wire does not carry yet stays the fixtures'**: a ticket's title and assignee, who is on a pull request, the changes a Job made to its own copy, the comments
// under a pull request, the steps' own waits, the card a rule asked, and the rules that start a watch, start a Job or mark a document, which land on a lane of their own that no rule
// names. Each is laid over what the wire gave, and goes when a read carries it. **The rules that move a ticket or hand to a step, and what they did, go over the wire**: as
// `list_lifecycle_rules` and `acts`.

import type { AppliedRule, HeldInstance, InstanceMove, JobLifecycles, LifecycleDefinition, RuleActReading, RuleDoing } from "@armada/protocol";
import type { Instance, JobLanes, Lifecycle, Rule, SessionLanes } from "@armada/components";
import { instanceId, lanesOfJob } from "@armada/screens/src/session-lanes";

const FIRST_AT = Date.parse("2026-10-29T12:00:00.000Z");

/** How the lifecycle's states are read, by what reads the lifecycle. A mock's words: only their source is drawn. */
function readOf(lifecycle: Lifecycle, state: Lifecycle["states"][number]): LifecycleDefinition["states"][number]["read"] {
  switch (lifecycle.kind) {
    case "forge":
      return { source: "forge", fact: state.id };
    case "watch":
      return { source: "watch", state: state.id };
    case "job":
    case "document":
      return { source: "fleet", fact: state.id };
    default:
      return { source: "tracker", words: state.reads.flatMap((one) => one.values) };
  }
}

function definitionOf(lifecycle: Lifecycle): LifecycleDefinition {
  return {
    id: lifecycle.id,
    name: lifecycle.label,
    scope: { kind: "everywhere" },
    states: lifecycle.states.map((state) => ({ id: state.id, label: state.label, read: readOf(lifecycle, state) })),
    transitions: lifecycle.next.map(([from, to]) => ({ from, to })),
    ends: lifecycle.states.filter((state) => state.end === true).map((state) => state.id),
  };
}

/** Whether the wire can say what a rule does: it moves a ticket, or a state sets it off and it hands to a step. */
const carried = (rule: Rule): boolean => rule.does.every((one) => one.act === "tracker") || (rule.from.lane !== "workflow" && rule.to.lane === "workflow");

function doingOf(rule: Rule, one: Rule["does"][number]): RuleDoing {
  const target = { among: { kind: "held", lifecycle: rule.to.lane } } as const;
  const entered = { among: { kind: "entered" } } as const;
  switch (one.act) {
    case "tracker":
      return { kind: "tracker_write", autonomy: rule.autonomy, what: one.text.startsWith("Assign") ? { act: "tracker_assign", to: "you", target } : { act: "tracker_move", to: rule.to.item, target } };
    case "message":
      return { kind: "draft", autonomy: rule.autonomy, what: { act: "draft", template: "review ask", target: entered } };
    case "tell":
      return { kind: "tell_agent", autonomy: rule.autonomy, what: { act: "tell", message: one.text } };
    default:
      return { kind: "run_agent", autonomy: rule.autonomy, what: { act: "run_drone", prompt: one.text } };
  }
}

const PLACE = { armada: "armada", yours: "kit", repository: "repository" } as const;

/** A fixture rule as `list_lifecycle_rules` answers it. */
function ruleWire(rule: Rule): AppliedRule {
  return {
    rule: {
      id: rule.id,
      name: rule.label,
      scope: { kind: "everywhere" },
      when: rule.from.lane === "workflow" ? { kind: rule.at === "ends" ? "step_ends" : "step_starts", step: rule.from.item } : { kind: "enters", lifecycle: rule.from.lane, state: rule.from.item },
      conditions: [],
      does: rule.does.map((one) => doingOf(rule, one)),
      autonomy: rule.autonomy,
      on_failure: { retries: 0, then: "ask_owner" },
    },
    kept: { rank: { reach: "everywhere", source: PLACE[rule.kept] } },
  };
}

/** The mock's one clock, a day of 29 Oct 2026 read as the person's own, so an act drawn as `12:31` was planned at 12:31. */
const atClock = (hhmm: string): string => {
  const [hours, minutes] = hhmm.split(":").map(Number);
  return new Date(2026, 9, 29, hours, minutes).toISOString();
};

/** When each step began on a Job that reached it, in minutes after 12:00. The rules that hand to a step land on the one the Job was on when they fired. */
const STEP_MINUTES = [0, 10, 20, 35, 55, 70];
const stepStart = (step: number): string => new Date(2026, 9, 29, 12, STEP_MINUTES[step] ?? 70 + (step - 5) * 15).toISOString();

/** What a Job's rules did, as `get_job_lifecycles` carries it: an act for each thing the rule does, where the fixture says it fired. */
function actsOf(job: JobLanes): RuleActReading[] {
  return job.fired.flatMap((one) => {
    const rule = job.copy.rules.find((each) => each.id === one.rule);
    const held = job.instances.find((each) => each.id === one.instance);
    if (rule === undefined || held === undefined || !carried(rule)) return [];
    const link = { lifecycle: held.lifecycle, key: held.id };
    const standing = one.outcome === "acted" ? "done" : one.outcome === "asked" ? "asked" : one.outcome === "skipped" ? "skipped" : "planned";
    return rule.does.map(
      (each): RuleActReading => ({ rule: rule.id, text: each.text, course: rule.autonomy === "asks" ? "asks" : "acts", standing, at: atClock(one.at), on: link, ...(rule.to.lane === "workflow" ? { entered: link } : {}) }),
    );
  });
}

/** What a Job holds, as `get_job_lifecycles` answers it: the lifecycles its instances are of, the instances where they stand, and the moves that took them there. */
export function wireOf(sessionId: string, job: JobLanes): JobLifecycles {
  const used = job.copy.lifecycles.filter((lifecycle) => job.instances.some((one) => one.lifecycle === lifecycle.id));
  const instances = job.instances.map((one): HeldInstance => ({ lifecycle: one.lifecycle, key: one.id, label: one.label, standing: { kind: "in", state: one.state }, links: [] }));
  let tick = 0;
  const moves = job.instances.flatMap((one) =>
    one.visited.map((state, at): InstanceMove => {
      tick += 1;
      return {
        lifecycle: one.lifecycle,
        key: one.id,
        from: at === 0 ? { kind: "unread" } : { kind: "in", state: one.visited[at - 1]! },
        to: { kind: "in", state },
        source: "mock",
        at: new Date(FIRST_AT + tick * 60_000).toISOString(),
      };
    }),
  );
  const acts = actsOf(job);
  return { session_id: sessionId, lifecycles: used.map(definitionOf), instances, moves, ...(acts.length === 0 ? {} : { acts }) };
}

/** What Fleet is serving for each Job, and who is told when it moves. `slices/lifecycles.ts` answers from here. */
const served = new Map<string, JobLifecycles>();
const hearing = new Set<(held: JobLifecycles) => void>();

export const servedLifecycles = {
  get: (sessionId: string): JobLifecycles | undefined => served.get(sessionId),
  listen(onChanged: (held: JobLifecycles) => void): () => void {
    hearing.add(onChanged);
    return () => void hearing.delete(onChanged);
  },
};

function serve(held: JobLifecycles): void {
  const was = served.get(held.session_id);
  served.set(held.session_id, held);
  if (was !== undefined && JSON.stringify(was) === JSON.stringify(held)) return;
  for (const one of hearing) one(held);
}

/**
 * A Job's lanes at a moment, drawn from what Fleet would serve of it. **A Job that holds nothing has no wire to speak of**, so it is drawn as the fixture has it: that is the Job
 * that changed its own copy before it held an instance, which no read serves yet.
 */
export function throughWire(sessionId: string, fixture: SessionLanes): SessionLanes {
  const { job } = fixture;
  if (job.instances.length === 0) return fixture;
  const held = wireOf(sessionId, job);
  serve(held);
  const steps = job.copy.steps.map(({ id, label }) => ({ id, label }));
  const stands = fixture.at >= 0 && fixture.at < steps.length;
  const visits = steps.slice(0, Math.max(0, fixture.at + 1)).map((_, step) => ({ step, since: stepStart(step) }));
  const drawn = lanesOfJob({
    held,
    working: fixture.working === true,
    ...(job.waiting === undefined ? {} : { waiting: job.waiting }),
    rules: job.copy.rules.filter(carried).map(ruleWire),
    workflow: { name: fixture.workflow, steps, began: fixture.at >= 0, ...(stands ? { at: fixture.at } : {}), ...(fixture.skipped === undefined ? {} : { skipped: fixture.skipped }), visits },
  })!;
  const mine = new Map(job.instances.map((one) => [instanceId(one.lifecycle, one.id), one] as const));
  const idOf = (fixtureId: string): string => {
    const one = job.instances.find((each) => each.id === fixtureId);
    return one === undefined ? fixtureId : instanceId(one.lifecycle, one.id);
  };
  const extras = (one: Instance): Instance => {
    const own = mine.get(one.id);
    return { ...one, ...(own?.title === undefined ? {} : { title: own.title }), ...(own?.assignee === undefined ? {} : { assignee: own.assignee }), ...(own?.says === undefined ? {} : { says: own.says }) };
  };
  return {
    ...drawn,
    job: {
      ...drawn.job,
      workspace: job.workspace,
      copy: {
        ...drawn.job.copy,
        steps: drawn.job.copy.steps.map((step) => {
          const waits = job.copy.steps.find((each) => each.id === step.id)?.endsOn;
          return waits === undefined ? step : { ...step, endsOn: waits };
        }),
        // Kept in the fixture's order: the rules the wire carries as it read them, and the others as the fixture has them.
        rules: job.copy.rules.flatMap((rule) => (carried(rule) ? (drawn.job.copy.rules.find((one) => one.id === rule.id) ?? []) : [rule])),
      },
      instances: drawn.job.instances.map(extras),
      // What the wire carries is what it read, with the card a rule asked laid over; what it does not stays the fixture's.
      fired: [
        ...drawn.job.fired.map((one) => {
          const needs = job.fired.find((each) => each.rule === one.rule && idOf(each.instance) === one.instance)?.needs;
          return needs === undefined ? one : { ...one, needs };
        }),
        ...job.fired.filter((one) => drawn.job.fired.every((each) => each.rule !== one.rule)).map((one) => ({ ...one, instance: idOf(one.instance) })),
      ],
      changes: job.changes.map((one) => ({ ...one, lane: idOf(one.lane) })),
      ...(job.triage === undefined ? {} : { triage: { ...job.triage, instance: idOf(job.triage.instance) } }),
    },
  };
}
