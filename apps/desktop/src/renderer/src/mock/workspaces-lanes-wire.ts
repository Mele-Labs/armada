// The mock Jobs' lanes as Fleet would serve them. `workspaces-lanes-data.ts` models what the Jobs hold; this turns it into `get_job_lifecycles` and the sentences
// `workflow_waiting_on` words, and draws the lanes from those by the same function a real Job's are (`lanesOfJob`), so a change to how they are read shows on the walks.
// **What the wire does not carry yet stays the fixtures'**: a ticket's title and assignee, who is on a pull request, the changes a Job made to its own copy, the comments
// under a pull request, the steps' own waits, and the rules and what they did. Each is laid over what the wire gave, and goes when a read carries it.

import type { HeldInstance, InstanceMove, JobLifecycles, LifecycleDefinition } from "@armada/protocol";
import type { Instance, JobLanes, Lifecycle, SessionLanes } from "@armada/components";
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
  return { session_id: sessionId, lifecycles: used.map(definitionOf), instances, moves };
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
  const drawn = lanesOfJob({
    held,
    working: fixture.working === true,
    ...(job.waiting === undefined ? {} : { waiting: job.waiting }),
    workflow: { name: fixture.workflow, steps, began: fixture.at >= 0, ...(stands ? { at: fixture.at } : {}), ...(fixture.skipped === undefined ? {} : { skipped: fixture.skipped }) },
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
        rules: job.copy.rules,
      },
      instances: drawn.job.instances.map(extras),
      fired: job.fired.map((one) => ({ ...one, instance: idOf(one.instance) })),
      changes: job.changes.map((one) => ({ ...one, lane: idOf(one.lane) })),
      ...(job.triage === undefined ? {} : { triage: { ...job.triage, instance: idOf(job.triage.instance) } }),
    },
  };
}
