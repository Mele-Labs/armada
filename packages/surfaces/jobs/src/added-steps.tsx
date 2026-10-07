// Steps added to one Job from its workflow, a mock: a `+` on the connector after
// the step the Job is on and after each step still to come, offering a Script, a
// Skill or a Drone step. The canvas and the stacked run draw the same steps and
// the same `+`. Nothing reaches Fleet; the steps live in `WorkflowTriggers/triggers.ts`.
//
// **Where a step hangs from is a key.** A workflow step's own id, `before:` and
// a step's id for the gap ahead of the first, or `at:` and a node's id for a
// place in the delivery lane, which has no step of its own. An added step hangs
// from the step or the added step before it.

import {
  AddStep,
  Button,
  changeInserted,
  insertedCard,
  InsertedFields,
  insertStep,
  keepTrigger,
  removeInserted,
  Sheet,
  TriggerFields,
  triggerFromInserted,
  useInserted,
} from "@armada/components";
import type { Inserted, Trigger, WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";
import { useState } from "react";
import type { ReactNode } from "react";

import type { JobDetail as JobWhole } from "@armada/protocol";
import { ordered } from "@armada/screens/src/facts";

import { stepNodeId, type WorkflowRun } from "./workflow-canvas";

/** What one `+` takes on the canvas, and what an added step does: the same spacing the spine uses. */
export const PLUS_HEIGHT = 44;
export const ADDED_HEIGHT = 132;
const LAST_APART = 132;

/** A place in the gap ahead of a step, and a place in the delivery lane. */
export const BEFORE = "before:";
export const AT = "at:";

export type AddedSteps = {
  jobId: string;
  workflow: string;
  inserted: readonly Inserted[];
  open: string | null;
  onOpen: (id: string | null) => void;
  keeping: Trigger | null;
  onKeeping: (trigger: Trigger | null) => void;
};

export function useAddedSteps(jobId: string, workflow: string): AddedSteps {
  const inserted = useInserted(jobId);
  const [open, onOpen] = useState<string | null>(null);
  const [keeping, onKeeping] = useState<Trigger | null>(null);
  return { jobId, workflow, inserted, open, onOpen, keeping, onKeeping };
}

/** The step an added one hangs from, through any added steps between. */
function rootOf(added: AddedSteps, one: Inserted): string {
  const before = added.inserted.find((other) => other.id === one.after);
  return before === undefined ? one.after : rootOf(added, before);
}

export const chainAfter = (added: AddedSteps, anchor: string): Inserted[] => {
  const next = added.inserted.find((one) => one.after === anchor);
  return next === undefined ? [] : [next, ...chainAfter(added, next.id)];
};

/** The added steps in the delivery lane, in the order their places were first used. */
export const deliveryChains = (added: AddedSteps): Inserted[] =>
  [...new Set(added.inserted.filter((one) => one.after.startsWith(AT)).map((one) => one.after))].flatMap((key) =>
    chainAfter(added, key),
  );

/**
 * Where a step may be added. Before the Job starts nothing is done, so every
 * step takes one and so does the gap before the first. Once it runs, the step
 * it is on and every step after it do — the gap ahead of the first is already
 * past. The delivery lane is the step that delivers, so it follows that step.
 */
export function reach(whole: JobWhole): { may: Set<string>; unstarted: boolean; deliverer: string | undefined } {
  const steps = ordered(whole);
  const unstarted = steps.every((step) => step.state === "not_started");
  const at = steps.findIndex((step) => step.step_id === whole.job.current_step_id);
  const may = new Set(unstarted ? steps.map((step) => step.step_id) : at === -1 ? [] : steps.slice(at).map((step) => step.step_id));
  const deliverer = (steps.find((step) => step.delivers === true) ?? steps[steps.length - 1])?.step_id;
  return { may, unstarted, deliverer };
}

const dummy = { kind: "step", name: "", activity: "not_started", said: "" } as const;

/** A `+` that is a node: at the head of a line with nothing before it, or at the end of one. */
export function plusNode(id: string, x: number, y: number, button: ReactNode): WorkflowCanvasNode {
  return { id, position: { x, y }, card: dummy, drawn: <div className="armada-triggers__plus">{button}</div> };
}

/** The run with its added steps and the `+` on each connector, on the canvas and stacked. */
export function withAddedSteps(added: AddedSteps, whole: JobWhole, run: WorkflowRun): WorkflowRun {
  const { may, unstarted, deliverer } = reach(whole);
  const addButton = (anchor: string, name: string, where: "after" | "before" = "after") => (
    <AddStep label={`Add a step ${where} ${name}`} onPick={(kind) => added.onOpen(insertStep(added.jobId, anchor, kind))} />
  );
  const cardOf = (one: Inserted) => insertedCard(one, added.open === one.id, () => added.onOpen(one.id));
  const nameOf = (one: Inserted) => (one.name === "" ? "Drone step" : one.name);
  const steps = ordered(whole);

  const nodes: WorkflowCanvasNode[] = [];
  const edges: WorkflowCanvasEdge[] = [...run.edges];
  let shift = 0;
  run.nodes.forEach((node, at) => {
    const stepId = steps.find((step) => stepNodeId(step.step_id) === node.id)?.step_id;
    const x = node.position.x;

    // The gap ahead of the first: a `+` at the head of the line, and what was added there.
    if (at === 0 && unstarted && stepId !== undefined) {
      let y = node.position.y;
      let previous = "add:before";
      nodes.push(plusNode(previous, x, y, addButton(BEFORE + stepId, node.card.name, "before")));
      y += PLUS_HEIGHT;
      shift += PLUS_HEIGHT;
      let anchor: string | undefined;
      for (const one of chainAfter(added, BEFORE + stepId)) {
        nodes.push({ id: one.id, position: { x, y }, card: cardOf(one) });
        edges.push({ id: `${previous}>${one.id}`, source: previous, target: one.id, kind: "leads", ...(anchor === undefined ? {} : { add: addButton(anchor, nameOf(added.inserted.find((o) => o.id === anchor)!)) }) });
        previous = one.id;
        anchor = one.id;
        y += ADDED_HEIGHT;
        shift += ADDED_HEIGHT + PLUS_HEIGHT;
      }
      edges.push({
        id: `${previous}>${node.id}`,
        source: previous,
        target: node.id,
        kind: "leads",
        ...(anchor === undefined ? {} : { add: addButton(anchor, nameOf(added.inserted.find((o) => o.id === anchor)!)) }),
      });
    }

    nodes.push({ ...node, position: { x, y: node.position.y + shift } });
    if (stepId === undefined || !may.has(stepId)) return;

    const nextNode = run.nodes[at + 1];
    const apart = nextNode === undefined ? LAST_APART : nextNode.position.y - node.position.y;
    const items = [...chainAfter(added, stepId), ...(stepId === deliverer ? deliveryChains(added) : [])];
    let y = node.position.y + shift + apart;
    // Whatever led from this step now leads through what was added after it.
    const direct = edges.findIndex((edge) => edge.kind === "leads" && edge.source === node.id);
    const target = direct === -1 ? undefined : edges[direct]!.target;
    if (direct !== -1) edges.splice(direct, 1);
    let previous = node.id;
    let from: { key: string; name: string } = { key: stepId, name: node.card.name };
    shift += PLUS_HEIGHT;
    y += PLUS_HEIGHT;
    for (const one of items) {
      nodes.push({ id: one.id, position: { x, y }, card: cardOf(one) });
      edges.push({ id: `${previous}>${one.id}`, source: previous, target: one.id, kind: "leads", add: addButton(from.key, from.name) });
      previous = one.id;
      from = { key: one.id, name: nameOf(one) };
      y += ADDED_HEIGHT + PLUS_HEIGHT;
      shift += ADDED_HEIGHT + PLUS_HEIGHT;
    }
    if (target !== undefined) {
      edges.push({ id: `${previous}>${target}`, source: previous, target, kind: "leads", add: addButton(from.key, from.name) });
    } else {
      // The end of the run: a stub that ends at a `+`, with no arrowhead going into it.
      const stub = `add:${previous}`;
      nodes.push(plusNode(stub, x, y - PLUS_HEIGHT / 2, addButton(from.key, from.name)));
      edges.push({ id: `${previous}>${stub}`, source: previous, target: stub, kind: "leads", plain: true });
    }
  });

  const rows = run.rows.flatMap((row, index) => {
    const stepId = steps.find((step) => stepNodeId(step.step_id) === row.id)?.step_id;
    if (stepId === undefined || row.under !== undefined) return [row];
    const first = index === 0 && unstarted;
    const ahead = first ? addButton(BEFORE + stepId, row.card.name, "before") : undefined;
    const before = first ? chainAfter(added, BEFORE + stepId) : [];
    const items = may.has(stepId) ? [...chainAfter(added, stepId), ...(stepId === deliverer ? deliveryChains(added) : [])] : [];
    return [
      ...before.map((one, i) => ({ id: one.id, card: cardOf(one), trailing: addButton(one.id, nameOf(one)), ...(i === 0 && ahead !== undefined ? { leading: ahead } : {}) })),
      {
        ...row,
        ...(before.length === 0 && ahead !== undefined ? { leading: ahead } : {}),
        ...(may.has(stepId) ? { trailing: addButton(stepId, row.card.name) } : {}),
      },
      ...items.map((one) => ({ id: one.id, card: cardOf(one), trailing: addButton(one.id, nameOf(one)) })),
    ];
  });
  return { ...run, nodes, edges, rows };
}

/** The panels for an added step, and for keeping it for every Job. */
export function AddedSheets({ added, whole }: { added: AddedSteps; whole: JobWhole }): ReactNode {
  const one = added.inserted.find((other) => other.id === added.open);
  const steps = ordered(whole).map((step) => step.step_id);
  if (added.keeping !== null) {
    const trigger = added.keeping;
    return (
      <Sheet
        kind="workflow-edit"
        open
        floating
        title={trigger.name}
        closeLabel="Close"
        closeBinding="Esc"
        onClose={() => added.onKeeping(null)}
        footer={
          <Button
            variant="primary"
            onClick={() => {
              keepTrigger(trigger);
              if (one !== undefined) changeInserted(added.jobId, one.id, { kept: trigger.place });
              added.onKeeping(null);
            }}
          >
            Keep
          </Button>
        }
      >
        <div className="armada-wf-panel">
          <TriggerFields
            trigger={trigger}
            workflow={added.workflow}
            steps={steps}
            onChange={(next) => added.onKeeping({ ...trigger, ...next })}
          />
        </div>
      </Sheet>
    );
  }
  if (one === undefined) return null;
  return (
    <Sheet
      kind="workflow-edit"
      open
      floating
      title={one.kind === "drone" ? "Drone step" : one.name}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={() => added.onOpen(null)}
    >
      <div className="armada-wf-panel">
        <InsertedFields
          one={one}
          onChange={(next) => changeInserted(added.jobId, one.id, next)}
          onKeep={() => added.onKeeping(triggerFromInserted(one, added.workflow, rootOf(added, one)))}
          onRemove={() => {
            removeInserted(added.jobId, one.id);
            added.onOpen(null);
          }}
        />
      </div>
    </Sheet>
  );
}
