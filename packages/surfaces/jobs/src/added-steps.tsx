// Steps added to one Job from its workflow, a mock: a `+` after the step the Job
// is on and after each step still to come, offering a Script, a Skill or a Drone
// step. The canvas and the stacked run draw the same steps and the same `+`.
// Nothing reaches Fleet; the steps live in `WorkflowHooks/hooks.ts`.

import {
  AddStep,
  Button,
  changeInserted,
  HookFields,
  hookFromInserted,
  insertedCard,
  InsertedFields,
  insertStep,
  keepHook,
  removeInserted,
  Sheet,
  useInserted,
} from "@armada/components";
import type { Hook, Inserted, WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";
import { useState } from "react";
import type { ReactNode } from "react";

import type { JobDetail as JobWhole } from "@armada/protocol";
import { ordered } from "@armada/screens/src/facts";

import { stepNodeId, type WorkflowRun } from "./workflow-canvas";

/** What one `+` takes on the canvas, and what an added step does: the same spacing the spine uses. */
const PLUS_HEIGHT = 44;
const ADDED_HEIGHT = 132;
const LAST_APART = 132;

export type AddedSteps = {
  jobId: string;
  workflow: string;
  inserted: readonly Inserted[];
  open: string | null;
  onOpen: (id: string | null) => void;
  keeping: Hook | null;
  onKeeping: (hook: Hook | null) => void;
};

export function useAddedSteps(jobId: string, workflow: string): AddedSteps {
  const inserted = useInserted(jobId);
  const [open, onOpen] = useState<string | null>(null);
  const [keeping, onKeeping] = useState<Hook | null>(null);
  return { jobId, workflow, inserted, open, onOpen, keeping, onKeeping };
}

/** The step an added one hangs from, through any added steps between. */
function rootOf(added: AddedSteps, one: Inserted): string {
  const before = added.inserted.find((other) => other.id === one.after);
  return before === undefined ? one.after : rootOf(added, before);
}

const chainAfter = (added: AddedSteps, anchor: string): Inserted[] => {
  const next = added.inserted.find((one) => one.after === anchor);
  return next === undefined ? [] : [next, ...chainAfter(added, next.id)];
};

/** Which steps may take one: the step the Job is on and every step after it. A step already done cannot. */
function takers(whole: JobWhole): Set<string> {
  const steps = ordered(whole);
  const at = steps.findIndex((step) => step.step_id === whole.job.current_step_id);
  return new Set(at === -1 ? [] : steps.slice(at).map((step) => step.step_id));
}

const dummy = { kind: "step", name: "", activity: "not_started", said: "" } as const;

/** The run with its added steps and the `+` between steps, on the canvas and stacked. */
export function withAddedSteps(added: AddedSteps, whole: JobWhole, run: WorkflowRun): WorkflowRun {
  const may = takers(whole);
  const addButton = (anchor: string, name: string) => (
    <AddStep label={`Add a step after ${name}`} onPick={(kind) => added.onOpen(insertStep(added.jobId, anchor, kind))} />
  );
  const cardOf = (one: Inserted) => insertedCard(one, added.open === one.id, () => added.onOpen(one.id));

  const nodes: WorkflowCanvasNode[] = [];
  const edges: WorkflowCanvasEdge[] = run.edges.map((edge) => edge);
  let shift = 0;
  run.nodes.forEach((node, at) => {
    const stepId = ordered(whole).find((step) => stepNodeId(step.step_id) === node.id)?.step_id;
    nodes.push({ ...node, position: { x: node.position.x, y: node.position.y + shift } });
    if (stepId === undefined || !may.has(stepId)) return;
    const nextNode = run.nodes[at + 1];
    const apart = nextNode === undefined ? LAST_APART : nextNode.position.y - node.position.y;
    let y = node.position.y + shift + apart;
    let previous = node.id;
    const lead = (to: string) => edges.push({ id: `${previous}>${to}`, source: previous, target: to, kind: "leads" });
    const place = (id: string, anchor: string, name: string) => {
      nodes.push({ id: `add:${id}`, position: { x: node.position.x, y }, card: dummy, drawn: <div className="armada-hooks__plus">{addButton(anchor, name)}</div> });
      lead(`add:${id}`);
      previous = `add:${id}`;
      y += PLUS_HEIGHT;
      shift += PLUS_HEIGHT;
    };
    // Whatever led from this step now leads through what was added after it.
    const direct = edges.findIndex((edge) => edge.kind === "leads" && edge.source === node.id);
    const target = direct === -1 ? undefined : edges[direct]!.target;
    if (direct !== -1) edges.splice(direct, 1);
    place(node.id, stepId, node.card.name);
    for (const one of chainAfter(added, stepId)) {
      nodes.push({ id: one.id, position: { x: node.position.x, y }, card: cardOf(one) });
      lead(one.id);
      previous = one.id;
      y += ADDED_HEIGHT;
      shift += ADDED_HEIGHT;
      place(one.id, one.id, one.name === "" ? "Drone step" : one.name);
    }
    if (target !== undefined) lead(target);
  });

  const rows = run.rows.flatMap((row) => {
    const stepId = ordered(whole).find((step) => stepNodeId(step.step_id) === row.id)?.step_id;
    if (stepId === undefined || row.under !== undefined || !may.has(stepId)) return [row];
    return [
      { ...row, trailing: addButton(stepId, row.card.name) },
      ...chainAfter(added, stepId).map((one) => ({
        id: one.id,
        card: cardOf(one),
        trailing: addButton(one.id, one.name === "" ? "Drone step" : one.name),
      })),
    ];
  });
  return { ...run, nodes, edges, rows };
}

/** The panels for an added step, and for keeping it for every Job. */
export function AddedSheets({ added, whole }: { added: AddedSteps; whole: JobWhole }): ReactNode {
  const one = added.inserted.find((other) => other.id === added.open);
  const steps = ordered(whole).map((step) => step.step_id);
  if (added.keeping !== null) {
    const hook = added.keeping;
    return (
      <Sheet
        kind="workflow-edit"
        open
        floating
        title={hook.name}
        closeLabel="Close"
        closeBinding="Esc"
        onClose={() => added.onKeeping(null)}
        footer={
          <Button
            variant="primary"
            onClick={() => {
              keepHook(hook);
              if (one !== undefined) changeInserted(added.jobId, one.id, { kept: hook.place });
              added.onKeeping(null);
            }}
          >
            Keep
          </Button>
        }
      >
        <div className="armada-wf-panel">
          <HookFields
            hook={hook}
            workflow={added.workflow}
            steps={steps}
            onChange={(next) => added.onKeeping({ ...hook, ...next })}
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
          onKeep={() => added.onKeeping(hookFromInserted(one, added.workflow, rootOf(added, one)))}
          onRemove={() => {
            removeInserted(added.jobId, one.id);
            added.onOpen(null);
          }}
        />
      </div>
    </Sheet>
  );
}
