// Steps added to one Job, drawn: a `+` on the connector after the step the Job is on and after
// each step still to come, offering a Script, a Skill or a Drone step. The canvas and the stacked
// run draw the same steps and the same `+`, and so does the approval canvas (`approval-added.tsx`).
//
// **The row is Fleet's.** A running Job's steps are `JobDetail.additions` and each add and removal
// is a call that answers with the row. At the gate they are held in the approval's edits until the
// press, which sends them as `additions`. `added-reach.ts` says where a place is and whether it is
// still ahead.
//
// **A place holds the steps added there, in the order they were added, and has one `+`**, after
// the last of them: Fleet fires a place's steps in that order, and a `+` between two of them
// would say it could put one there.

import { AddedFields, addedCard, AddStep, addedName, Button, Sheet, TriggerSheet } from "@armada/components";
import type { AddedKind, WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";
import type { WorkflowStackedRow } from "@armada/components";
import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

import type { AddedStep, AddStep as AddStepBody, JobDetail as JobWhole, JobSummary, KeptFrom, TriggerSaved, TriggerScope } from "@armada/protocol";
import { ordered } from "@armada/screens/src/facts";

import { addStepOf, ahead, blankAddition, chainAt, delivererOf, heldId, NAMES_NOTHING, sameGap } from "./added-reach";
import type { Gap } from "./added-reach";
import { stepNodeId, type WorkflowRun } from "./workflow-canvas";

/** What one `+` takes on the canvas, and what an added step does: the same spacing the spine uses. */
export const PLUS_HEIGHT = 44;
export const ADDED_HEIGHT = 132;
const LAST_APART = 132;

/** What the window hands a Job's surface for its added steps: the two acts, and the save that keeps one for every Job. */
export type AddedBinding = {
  /** The Commands the repository's `armada.yml` declares, which a Script may name. */
  commands: readonly string[];
  /** Fleet's answer, whole; a refusal has been said where the window says what a command answered. */
  onAdd: (jobId: string, body: AddStepBody) => Promise<{ ok: true; added: AddedStep } | { ok: false }>;
  onRemove: (jobId: string, id: string) => Promise<boolean>;
  onKeep: (
    scope: TriggerScope,
    definition: string,
    overwrite: boolean,
    keptFrom?: KeptFrom,
  ) => Promise<{ ok: true; saved: TriggerSaved } | { ok: false; said: string; exists?: true }>;
};

/** The approval's additions, held until the press. */
export type AddedGate = { additions: readonly AddedStep[]; onAdditions: (next: readonly AddedStep[]) => void };

export type AddedSteps = {
  jobId: string;
  workflow: string;
  steps: readonly string[];
  /** Every step the Job holds: Fleet's rows, or the approval's, at the gate. */
  rows: readonly AddedStep[];
  /** Whether the Job is still at its gate, where a step is held and not yet Fleet's. */
  gated: boolean;
  /** The step open in its panel. */
  open: string | null;
  onOpen: (id: string | null) => void;
  /** A step being filled in before it is added. */
  composing: AddedStep | null;
  onComposing: (one: AddedStep | null) => void;
  keeping: AddedStep | null;
  onKeeping: (one: AddedStep | null) => void;
  /** The step just added, which the canvas pans to. */
  reveal: string | null;
  binding: AddedBinding | undefined;
  gate: AddedGate | undefined;
  /** Whether a `+` may be drawn at all: nothing is offered where nothing can take the answer. */
  offers: boolean;
  /** Press on a `+`: a step to fill in, at that place. */
  onPick: (gap: Gap, kind: AddedKind) => void;
  /** The panel's Add: held at the gate, a call to Fleet otherwise. */
  onAdd: () => Promise<void>;
  /** Whether Fleet's call is out. */
  adding: boolean;
  onRemove: (one: AddedStep) => Promise<void>;
  onChange: (one: AddedStep, next: Partial<Pick<AddedStep, "runs" | "block" | "repair">>) => void;
};

export function useAddedSteps(
  job: JobSummary,
  whole: JobWhole | null,
  binding: AddedBinding | undefined,
  gate: AddedGate | undefined,
): AddedSteps {
  const { id: jobId, workflow_id: workflow } = job;
  const gated = job.status === "proposing" || job.status === "awaiting_approval";
  const [open, onOpen] = useState<string | null>(null);
  const [composing, onComposing] = useState<AddedStep | null>(null);
  const [keeping, onKeeping] = useState<AddedStep | null>(null);
  const [reveal, setReveal] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  // What Fleet answered for an add, drawn until the Job's own re-read carries it.
  const [echo, setEcho] = useState<readonly AddedStep[]>([]);
  const held = whole?.additions ?? [];
  const rows = gated
    ? (gate?.additions ?? [])
    : [...held, ...echo.filter((one) => !held.some((other) => other.id === one.id))];
  const offers = gated ? gate !== undefined : binding !== undefined;

  const onPick = (gap: Gap, kind: AddedKind) => onComposing(blankAddition(gap, kind));

  async function onAdd() {
    if (composing === null || NAMES_NOTHING(composing.runs)) return;
    if (gated) {
      if (gate === undefined) return;
      const id = heldId(gate.additions);
      gate.onAdditions([...gate.additions, { ...composing, id }]);
      setReveal(id);
      onComposing(null);
      return;
    }
    if (binding === undefined || adding) return;
    setAdding(true);
    const answer = await binding.onAdd(jobId, addStepOf(composing));
    setAdding(false);
    if (!answer.ok) return;
    setEcho((was) => [...was, answer.added]);
    setReveal(answer.added.id);
    onComposing(null);
  }

  async function onRemove(one: AddedStep) {
    if (gated) {
      gate?.onAdditions(gate.additions.filter((other) => other.id !== one.id));
      onOpen(null);
      return;
    }
    if (binding === undefined) return;
    if (await binding.onRemove(jobId, one.id)) {
      setEcho((was) => was.filter((other) => other.id !== one.id));
      onOpen(null);
    }
  }

  const onChange: AddedSteps["onChange"] = (one, next) => {
    if (one.id === "") onComposing({ ...one, ...next });
    else if (gated) gate?.onAdditions(gate.additions.map((other) => (other.id === one.id ? { ...other, ...next } : other)));
  };

  return {
    jobId,
    workflow,
    steps: ordered(whole).map((step) => step.step_id),
    rows,
    gated,
    open,
    onOpen,
    composing,
    onComposing,
    keeping,
    onKeeping,
    reveal,
    binding,
    gate,
    offers,
    onPick,
    onAdd,
    adding,
    onRemove,
    onChange,
  };
}

/** The node id an added step is drawn under, so the canvas can pan to it. */
export const addedNodeId = (id: string): string => `added:${id}`;

const dummy = { kind: "step", name: "", activity: "not_started", said: "" } as const;

/** A `+` that is a node: at the head of a line with nothing before it, or at the end of one. */
export function plusNode(id: string, x: number, y: number, button: ReactNode): WorkflowCanvasNode {
  return { id, position: { x, y }, card: dummy, drawn: <div className="armada-triggers__plus">{button}</div> };
}

/** The `+` for a place, named for the step it is after or before. */
export function plusFor(added: AddedSteps, gap: Gap, name: string, where: "after" | "before" = "after"): ReactNode {
  return <AddStep label={`Add a step ${where} ${name}`} onPick={(kind) => added.onPick(gap, kind)} />;
}

const cardOf = (added: AddedSteps, one: AddedStep) => addedCard(one, added.open === one.id, () => added.onOpen(one.id));

/**
 * What runs between one step's card and the next, in the order the moments come: the step passing,
 * the next step starting, and the pull request opening on the step that delivers. The first
 * region is the gap ahead of the first step.
 */
export type Region = {
  /** The place the region's `+` adds to. Absent where none may be added. */
  gap?: Gap;
  /** What the `+` is named after. */
  name: string;
  where: "after" | "before";
  chain: AddedStep[];
};

function regionsOf(added: AddedSteps, whole: JobWhole, names: ReadonlyMap<string, string>): Region[] {
  const steps = ordered(whole);
  const deliverer = delivererOf(whole);
  const lead = (step: string): Gap[] => [
    { when: "step_starts", step },
    ...(step === deliverer ? [{ when: "pr_opened" as const, step }] : []),
  ];
  const chainOf = (gaps: Gap[]) => gaps.flatMap((gap) => chainAt(added.rows, gap));
  const offered = (gap: Gap): Gap | undefined => (added.offers && ahead(whole, gap) ? gap : undefined);
  const regions: Region[] = [];
  const first = steps[0];
  if (first !== undefined) {
    const gap = offered({ when: "step_starts", step: first.step_id });
    regions.push({ ...(gap === undefined ? {} : { gap }), name: names.get(first.step_id) ?? first.label, where: "before", chain: chainOf(lead(first.step_id)) });
  }
  steps.forEach((step, at) => {
    const passes: Gap = { when: "step_passes", step: step.step_id };
    const next = steps[at + 1];
    const gap = offered(passes);
    regions.push({
      ...(gap === undefined ? {} : { gap }),
      name: names.get(step.step_id) ?? step.label,
      where: "after",
      chain: chainOf([passes, ...(next === undefined ? [] : lead(next.step_id))]),
    });
  });
  return regions;
}

/** Where a region's `+` sits among its steps: after the last one added at its place, or on the connector leaving the card when none are. */
const plusAfter = (region: Region): number =>
  region.gap === undefined ? -1 : region.chain.filter((one) => sameGap(one, region.gap!)).length - 1;

/** One region drawn between `from` and `into`: its steps, and the `+` after the last added at its place. Returns the room it took. */
export function drawRegion(
  added: AddedSteps,
  region: Region,
  at: { x: number; y: number },
  from: string | undefined,
  into: string | undefined,
  out: { nodes: WorkflowCanvasNode[]; edges: WorkflowCanvasEdge[] },
): number {
  const stop = plusAfter(region);
  const plusOn = (item: number): ReactNode | undefined =>
    region.gap === undefined || item !== stop
      ? undefined
      : item === -1
        ? plusFor(added, region.gap, region.name, from === undefined ? "before" : "after")
        : plusFor(added, region.gap, addedName(region.chain[item]!.runs));
  let { y } = at;
  let used = 0;
  let previous = from;
  let item = -1;
  // Nothing leads into the first step, so a `+` ahead of it is a node of its own at the head of the line.
  if (from === undefined && region.gap !== undefined && stop === -1) {
    const head = "add:before";
    out.nodes.push(plusNode(head, at.x, y, plusOn(-1)));
    previous = head;
    item = -2;
    y += PLUS_HEIGHT;
    used += PLUS_HEIGHT;
  }
  const lead = (target: string) => {
    const add = item === -2 ? undefined : plusOn(item);
    out.edges.push({ id: `${previous}>${target}`, source: previous!, target, kind: "leads", ...(add === undefined ? {} : { add }) });
  };
  region.chain.forEach((one, index) => {
    const id = addedNodeId(one.id);
    out.nodes.push({ id, position: { x: at.x, y }, card: cardOf(added, one) });
    if (previous !== undefined) lead(id);
    previous = id;
    item = index;
    y += ADDED_HEIGHT + PLUS_HEIGHT;
    used += ADDED_HEIGHT + PLUS_HEIGHT;
  });
  if (into !== undefined) {
    if (previous !== undefined) lead(into);
  } else if (region.gap !== undefined && previous !== undefined && item === stop) {
    // The end of the run: a stub that ends at a `+`, with no arrowhead going into it.
    const stub = `add:${previous}`;
    out.nodes.push(plusNode(stub, at.x, y - PLUS_HEIGHT / 2, plusOn(item)));
    out.edges.push({ id: `${previous}>${stub}`, source: previous, target: stub, kind: "leads", plain: true });
  }
  return used;
}

/** The run with its added steps and the `+` on each connector, on the canvas and stacked. */
export function withAddedSteps(added: AddedSteps, whole: JobWhole, run: WorkflowRun): WorkflowRun {
  const steps = ordered(whole);
  const names = new Map(steps.map((step) => [step.step_id, step.label]));
  const regions = regionsOf(added, whole, names);
  const out = { nodes: [] as WorkflowCanvasNode[], edges: [...run.edges] };
  let shift = 0;

  run.nodes.forEach((node, index) => {
    const at = steps.findIndex((step) => stepNodeId(step.step_id) === node.id);
    const x = node.position.x;
    // The gap ahead of the first step.
    if (at === 0) {
      const head = regions[0]!;
      if (head.gap !== undefined || head.chain.length > 0) shift += drawRegion(added, head, { x, y: node.position.y }, undefined, node.id, out);
    }
    out.nodes.push({ ...node, position: { x, y: node.position.y + shift } });
    if (at === -1) return;

    const region = regions[at + 1]!;
    if (region.gap === undefined && region.chain.length === 0) return;
    const nextStep = steps[at + 1];
    const nextNode = nextStep === undefined ? undefined : run.nodes.slice(index + 1).find((one) => one.id === stepNodeId(nextStep.step_id));
    const apart = nextNode === undefined ? LAST_APART : nextNode.position.y - node.position.y;
    // Whatever led from this step now leads through what was added after it.
    const direct = out.edges.findIndex((edge) => edge.kind === "leads" && edge.source === node.id && edge.target === nextNode?.id);
    const target = direct === -1 ? undefined : out.edges[direct]!.target;
    if (direct !== -1) out.edges.splice(direct, 1);
    const y = node.position.y + shift + apart + PLUS_HEIGHT;
    shift += PLUS_HEIGHT;
    shift += drawRegion(added, region, { x, y }, node.id, target, out);
  });

  const rows: WorkflowStackedRow[] = [];
  const itemRows = (region: Region) => {
    const stop = plusAfter(region);
    return region.chain.map(
      (one, index): WorkflowStackedRow => ({
        id: addedNodeId(one.id),
        card: cardOf(added, one),
        ...(region.gap !== undefined && index === stop ? { trailing: plusFor(added, region.gap, addedName(one.runs)) } : {}),
      }),
    );
  };
  run.rows.forEach((row) => {
    const at = steps.findIndex((step) => stepNodeId(step.step_id) === row.id);
    if (at === -1 || row.under !== undefined) {
      rows.push(row);
      return;
    }
    const head = at === 0 ? regions[0]! : undefined;
    const ahead = head?.gap !== undefined && plusAfter(head) === -1 ? plusFor(added, head.gap, head.name, "before") : undefined;
    const before = head === undefined ? [] : itemRows(head);
    if (ahead !== undefined) {
      if (before[0] !== undefined) before[0] = { ...before[0], leading: ahead };
    }
    rows.push(...before);
    const region = regions[at + 1]!;
    const bare = region.gap !== undefined && plusAfter(region) === -1 ? plusFor(added, region.gap, region.name) : undefined;
    rows.push({
      ...row,
      ...(ahead !== undefined && before.length === 0 ? { leading: ahead } : {}),
      ...(bare === undefined ? {} : { trailing: bare }),
    });
    rows.push(...itemRows(region));
  });
  return { ...run, nodes: out.nodes, edges: out.edges, rows };
}

/** The panels for an added step: one being filled in, one open, and keeping one for every Job. */
export function AddedSheets({ added }: { added: AddedSteps }): ReactNode {
  const open = added.rows.find((other) => other.id === added.open);
  const binding = added.binding;
  const commands = binding?.commands ?? [];
  const close = useRef<() => void>(() => undefined);
  close.current = () => added.onComposing(null);
  // A panel for a step the Job no longer holds closes.
  useEffect(() => {
    if (added.open !== null && open === undefined) added.onOpen(null);
  }, [added, open]);

  if (added.keeping !== null && binding !== undefined) {
    const one = added.keeping;
    const runs = one.runs;
    return (
      <TriggerSheet
        binding={{
          triggers: [],
          commands,
          onOpen: async () => ({ ok: false, said: "" }),
          onSave: binding.onKeep,
          onRemove: async () => ({ ok: false, said: "" }),
        }}
        target={{
          kind: "new",
          init: {
            name: runs.kind === "script" ? runs.command : runs.kind === "skill" ? runs.skill : "",
            when: one.when,
            step: one.when === "pr_opened" ? "" : one.step,
            workflow: added.workflow,
            runs: runs.kind === "script" ? "command" : "skill",
            with: runs.kind === "script" ? runs.command : runs.kind === "skill" ? runs.skill : "",
            block: one.block,
            repair: one.repair,
            scope: "machine",
          },
        }}
        workflow={added.workflow}
        steps={added.steps}
        keeping={{ job_id: added.jobId, addition_id: one.id }}
        onSaved={() => undefined}
        onRemoved={() => undefined}
        onClose={() => added.onKeeping(null)}
      />
    );
  }
  const composing = added.composing;
  if (composing !== null) {
    return (
      <Sheet
        kind="workflow-edit"
        open
        floating
        title={composing.runs.kind === "drone" ? "Drone step" : composing.runs.kind === "script" ? "Script" : "Skill"}
        closeLabel="Close"
        closeBinding="Esc"
        onClose={() => added.onComposing(null)}
        footer={
          <Button variant="primary" disabled={NAMES_NOTHING(composing.runs) || added.adding} onClick={() => void added.onAdd()}>
            Add
          </Button>
        }
      >
        <div className="armada-wf-panel">
          <AddedFields one={composing} commands={commands} editable onChange={(next) => added.onChange(composing, next)} />
        </div>
      </Sheet>
    );
  }
  if (open === undefined) return null;
  const pending = open.state === "pending";
  return (
    <Sheet
      kind="workflow-edit"
      open
      floating
      title={addedName(open.runs)}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={() => added.onOpen(null)}
    >
      <div className="armada-wf-panel">
        <AddedFields
          one={open}
          commands={commands}
          editable={added.gated}
          onChange={(next) => added.onChange(open, next)}
          {...(added.gated || binding === undefined ? {} : { onKeep: () => added.onKeeping(open) })}
          {...(pending ? { onRemove: () => void added.onRemove(open) } : {})}
        />
      </div>
    </Sheet>
  );
}
