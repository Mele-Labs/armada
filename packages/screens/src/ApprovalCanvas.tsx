// What a Job at its dispatch gate will do, drawn as the run it will be rather
// than as a form: brief, base branch, the workflow starting, each step and its
// gate, the pull request, and where it lands — top to bottom. A node pressed
// opens a card beside it holding what that node tunes for this Job.
//
// **Prototype** (the owner, 3 Oct 2026: *What you are approving* read as a
// boring form). It takes `Approving`'s place and its props, and moves the same
// `ProposalEdits`, so *Approve dispatch* still sends what moved — `approvalOf`.
// What the wire has no field for is `edits.tuning`, `draft/tuning.ts`, and is
// never sent.
//
// **The cards are the proposal's own regions**, each drawing the fields its
// node owns: `ProposalGates` one step's row, `ProposalLanding` the landing
// fields, `ProposalTiers` the tiers and cap, `ProposalDoneWhen` the criteria.

import { useState } from "react";
import { X } from "lucide-react";

import {
  AUTO,
  BranchPicker,
  Button,
  Checkbox,
  Input,
  ProposalDoneWhen,
  ProposalField,
  ProposalFields,
  ProposalGates,
  ProposalLanding,
  ProposalTiers,
  Select,
  Switch,
  Textarea,
  Tooltip,
  WorkflowCanvas,
  WorkflowStepCard,
} from "@armada/components";
import type { GateBox, ProposalLandingValue, WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";

import { NOT_STARTED, approvalNodesOf, checksOf, dispatchesFromOf, perTask, stepsReadOf } from "./approval-canvas";
import type { ApprovalNode, LifeRead, StepRead } from "./approval-canvas";
import type { ApprovingProps } from "./approving";
import type { TaskView } from "./draft/task";
import { taskCard } from "./plan-canvas";
import { baseBranch } from "./draft/branches";
import type { ProposalView } from "./draft/proposal";
import { EFFORTS, HARNESSES, checksOffWith, deliveryOf, tunedStep, tuningOf } from "./draft/tuning";
import type { ApprovalTuning, Delivery, Effort, StepTuning } from "./draft/tuning";
import {
  completeChoices,
  criteriaAdded,
  criteriaRowsOf,
  criteriaWith,
  criteriaWithout,
  gateRowsOf,
  gatesWith,
  landingValueOf,
  landingWith,
  proposalOnWorkflow,
  repositorySaysOf,
  stepsDeclaredOf,
  withoutCap,
  workflowChoicesOf,
} from "./tab-proposal-read";
import type { ProposalEdits } from "./tab-proposal-read";

/**
 * How tall a card is with nothing under its name, and the gap under it for the
 * arrow: `--space-12`. Numbers because React Flow places by number; the run's
 * own canvas counts its spine the same way (`workflow-canvas.ts`).
 */
const CARD = 36;
const GAP = 48;

/** One row under a card's name — its line, its facts, its gate: `--leading-xs` and `--space-2`. */
const ROW = 28;

/** The room an edge's own words take in the gap: `merges on its own` at --text-2xs. */
const LABELLED = 24;

const rowsOf = (node: ApprovalNode): number =>
  Number(node.line !== undefined) + Number(node.facts.length > 0) + Number(node.gate !== undefined);

/** What a node's card is called: its own name, and a gate's step beside it. */
function dialogNameOf(node: ApprovalNode, steps: readonly StepRead[]): string {
  const step = steps.find((one) => one.id === node.stepId);
  return node.kind === "checks" && step !== undefined ? `Checks on ${step.label}` : node.name;
}

/**
 * What a run too long to read whole opens on: the node the Job is at with two
 * before it, or the top of the run at the gate.
 */
function opensOnOf(nodes: readonly ApprovalNode[]): string[] {
  const at = Math.max(0, nodes.findIndex((node) => node.life?.current === true) - 2);
  return nodes.slice(at, at + 6).map((node) => node.id);
}

/** How far apart two Jobs of one band of the wave sit: a card's width and `--space-8`. */
const ACROSS = 196 + 32;

/**
 * Where each node sits, top to bottom (the owner, 4 Oct 2026): one under the
 * last, by what each draws. **A wave's Jobs of one depth share a row**, side by
 * side about the run's own line, so what may run at once reads as at once.
 */
function placesOf(
  nodes: readonly ApprovalNode[],
  edges: readonly WorkflowCanvasEdge[],
): { x: number; y: number }[] {
  const places: { x: number; y: number }[] = [];
  let y = 0;
  for (const [at, node] of nodes.entries()) {
    const band = node.band;
    if (band !== undefined && band.index > 0) {
      // Beside the first of its band, on that one's row.
      const first = places[at - band.index]!;
      places.push({ x: (band.index - (band.of - 1) / 2) * ACROSS, y: first.y });
      continue;
    }
    if (edges.some((edge) => edge.target === node.id && edge.label !== undefined)) y += LABELLED;
    places.push({ x: band === undefined ? 0 : (-(band.of - 1) / 2) * ACROSS, y });
    const tallest =
      band === undefined ? rowsOf(node) : Math.max(...nodes.slice(at, at + band.of).map(rowsOf));
    y += CARD + tallest * ROW + GAP;
  }
  return places;
}

/** What each answer to the delivery control is called. */
const DELIVERY: Record<Delivery, string> = {
  local: "Local only",
  draft: "Draft pull request",
  ready: "Pull request",
};

export type ApprovalCanvasProps = ApprovingProps & {
  /**
   * Where a running Job is on each node, which is the canvas drawn after the
   * gate: the Overview for the Job's whole life (the owner, 4 Oct 2026).
   * Absent is the gate itself. Drawn with no `onEdits`, so every card reads.
   */
  life?: LifeRead;
  /** Open a Job the wave dispatched, as the wave graph did. */
  onOpenJob?: (jobId: string) => void;
};

export function ApprovalCanvas({
  life,
  onOpenJob,
  whole,
  edits,
  onEdits,
  workflows,
  manifest,
  branches,
  models,
  machineCap,
}: ApprovalCanvasProps) {
  const [open, setOpen] = useState<string | null>(null);
  const { proposal, landing } = edits;
  const declared = stepsDeclaredOf(workflows, proposal.workflow_id);
  const steps = stepsReadOf(proposal.gates, whole, declared);
  const tuning = edits.tuning ?? tuningOf(steps.map((step) => ({ step_id: step.id, judge_checks: step.judges })));
  const base = baseBranch(branches);
  // **Where it lands, picked late.** A Job frozen with no branch to land in
  // has to be given one before it can land (the owner, 4 Oct 2026), so past
  // the gate that one field stays a picker. Held here: nothing on the wire
  // takes it yet — `draft/tuning.ts` says what is owed.
  const [late, setLate] = useState("");
  const frozenValue = landingValueOf(landing, base);
  const picksLate = life !== undefined && frozenValue.target === "";
  const value = picksLate ? { ...frozenValue, target: late } : frozenValue;
  const workflow = workflows.find(
    (one) => one.id === proposal.workflow_id && one.manifest_id === whole.job.owner_manifest_id,
  );

  const moved =
    onEdits === undefined
      ? undefined
      : (change: Partial<ProposalEdits>) => onEdits({ ...edits, tuning, ...change });
  const tuned = (next: ApprovalTuning) => moved?.({ tuning: next });
  const landed = (next: ProposalLandingValue) => moved?.({ landing: landingWith(landing, next, base) });

  const dispatchesFrom = dispatchesFromOf(proposal.workflow_id, steps);
  const { nodes, edges } = approvalNodesOf({
    title: proposal.title,
    from: value.from,
    workflowName: workflow?.name ?? proposal.workflow_id,
    steps,
    gates: proposal.gates,
    tuning,
    prMode: landing.pr_mode,
    target: value.target,
    ...(life === undefined ? {} : { life }),
    ...(dispatchesFrom === undefined ? {} : { dispatchesFrom }),
  });
  // A node a workflow change took away closes its card with it.
  const opened = nodes.find((node) => node.id === open);

  const places = placesOf(nodes, edges);
  const placed: WorkflowCanvasNode[] = nodes.map((node, at) => ({
    id: node.id,
    position: places[at]!,
    card: {
      kind: "task",
      name: node.name,
      activity: node.life?.activity ?? "not_started",
      said: node.life?.said ?? NOT_STARTED,
      ...(node.life?.mark === undefined ? {} : { mark: node.life.mark }),
      ...(node.life?.current === true ? { current: true } : {}),
      ordinal: node.ordinal,
      facts: node.facts,
      ...(node.line === undefined ? {} : { line: node.line }),
      ...(node.gate === undefined ? {} : { gate: node.gate }),
      selected: node.id === open,
      ...(node.opensJob !== undefined
        ? onOpenJob === undefined
          ? {}
          : { onOpen: () => onOpenJob(node.opensJob!) }
        : node.inert === true
          ? {}
          : { onOpen: () => setOpen(node.id === open ? null : node.id) }),
    },
  }));

  const card =
    opened === undefined
      ? undefined
      : {
          nodeId: opened.id,
          // A gate node is `Checks` on the canvas, where its place says whose;
          // the card it opens says so in words.
          label: dialogNameOf(opened, steps),
          children: (
            <>
              <div className="armada-approval-canvas__card-head">
                <h3 className="armada-proposal__heading">{dialogNameOf(opened, steps)}</h3>
                <Tooltip label="Close">
                  <Button variant="ghost" size="sm" iconOnly aria-label="Close" onClick={() => setOpen(null)}>
                    <X size={16} aria-hidden />
                  </Button>
                </Tooltip>
              </div>
              <NodeCard
                node={opened}
                step={steps.find((one) => one.id === opened.stepId)}
                edits={edits}
                tuning={tuning}
                moved={moved}
                tuned={onEdits === undefined ? undefined : tuned}
                landed={onEdits === undefined ? undefined : landed}
                {...(picksLate ? { landsLate: (target: string) => setLate(target) } : {})}
                value={value}
                branches={branches}
                models={models}
                machineCap={machineCap}
                workflows={workflows}
                whole={whole}
                manifest={manifest}
                forRequests={workflow?.for_requests}
              />
            </>
          ),
        };

  return (
    <section
      className="armada-approval-canvas armada-glass"
      aria-label={life === undefined ? "What you are approving" : "This Job's run"}
    >
      <WorkflowCanvas
        nodes={placed}
        edges={edges}
        label="Run"
        runsDown
        centred
        hangsFromTop
        opensOn={[opensOnOf(nodes)]}
        {...(card === undefined ? {} : { opened: card })}
      />
    </section>
  );
}

type NodeCardProps = {
  node: ApprovalNode;
  step: StepRead | undefined;
  edits: ProposalEdits;
  tuning: ApprovalTuning;
  moved: ((change: Partial<ProposalEdits>) => void) | undefined;
  tuned: ((next: ApprovalTuning) => void) | undefined;
  landed: ((next: ProposalLandingValue) => void) | undefined;
  value: ProposalLandingValue;
  branches: ApprovingProps["branches"];
  models: readonly string[];
  machineCap: number | null;
  workflows: ApprovingProps["workflows"];
  whole: ApprovingProps["whole"];
  manifest: ApprovingProps["manifest"];
  forRequests: string | undefined;
  /** Past the gate, where it lands is still to pick: that one field's mover. */
  landsLate?: (target: string) => void;
};

/** What one node tunes. */
function NodeCard(props: NodeCardProps) {
  const { node } = props;
  switch (node.kind) {
    case "brief":
      return <BriefCard {...props} />;
    case "base":
      return <BaseCard {...props} />;
    case "start":
      return <StartCard {...props} />;
    case "step":
      return props.step === undefined ? null : <StepCard {...props} step={props.step} />;
    case "checks":
      return props.step === undefined ? null : <GateCard {...props} step={props.step} />;
    case "done":
      return <DoneWhen {...props} />;
    case "group":
      return <GroupCard tasks={node.tasks ?? []} />;
    case "groups":
    case "jobs":
    case "job":
      return null;
    case "pr":
      return <PullRequestCard {...props} />;
    case "land":
      return <LandCard {...props} />;
  }
}

function BriefCard({ edits, moved, tuning, tuned }: NodeCardProps) {
  const { proposal } = edits;
  const [note, setNote] = useState("");
  return (
    <>
      <ProposalFields>
        <ProposalField label="Title" bare={moved !== undefined}>
          {moved === undefined ? (
            proposal.title
          ) : (
            <Input
              aria-label="Title"
              value={proposal.title}
              onChange={(event) => moved({ proposal: { ...proposal, title: event.target.value } })}
            />
          )}
        </ProposalField>
        <ProposalField label="Request" bare={moved !== undefined}>
          {moved === undefined ? (
            (proposal.asked ?? "")
          ) : (
            <Textarea
              aria-label="What was asked"
              value={proposal.asked ?? ""}
              onChange={(event) => moved({ proposal: { ...proposal, asked: event.target.value } })}
            />
          )}
        </ProposalField>
      </ProposalFields>
      <DoneWhen edits={edits} moved={moved} />
      {tuned === undefined ? null : (
        <section className="armada-proposal__region" aria-label="To the proposer">
          <h3 className="armada-proposal__heading">To the proposer</h3>
          {tuning.to_proposer.length === 0 ? null : (
            <ul className="armada-approval-canvas__sent">
              {tuning.to_proposer.map((sent, at) => (
                <li key={at}>{sent}</li>
              ))}
            </ul>
          )}
          <Textarea aria-label="Note to the proposer" value={note} onChange={(event) => setNote(event.target.value)} />
          <div className="armada-approval-canvas__acts">
            <Button
              variant="secondary"
              size="sm"
              disabled={note.trim() === ""}
              onClick={() => {
                tuned({ ...tuning, to_proposer: [...tuning.to_proposer, note.trim()] });
                setNote("");
              }}
            >
              Send to proposer
            </Button>
          </div>
        </section>
      )}
    </>
  );
}

/** A plan group's tasks, each the Plan graph's own task card. */
function GroupCard({ tasks }: { tasks: readonly TaskView[] }) {
  return (
    <ul className="armada-approval-canvas__tasks" aria-label="Tasks">
      {tasks.map((task) => (
        <li key={task.id}>
          <WorkflowStepCard {...taskCard(task, undefined)} />
        </li>
      ))}
    </ul>
  );
}

/**
 * What counts as the work being done. **One list, drawn on two nodes** — Brief
 * and Done when, before it leaves — and both edit `edits.criteria`.
 */
function DoneWhen({ edits, moved }: Pick<NodeCardProps, "edits" | "moved">) {
  const { criteria } = edits;
  return (
    <ProposalDoneWhen
      criteria={criteriaRowsOf(criteria)}
      {...(moved === undefined
        ? {}
        : {
            onCriterion: (at: number, text: string) => moved({ criteria: criteriaWith(criteria, at, text) }),
            onAdd: () => moved({ criteria: criteriaAdded(criteria) }),
            onRemove: (at: number) => moved({ criteria: criteriaWithout(criteria, at) }),
          })}
    />
  );
}

function BaseCard({ value, landed, branches }: NodeCardProps) {
  return (
    <ProposalFields>
      <ProposalField label="Base branch" bare={landed !== undefined || value.from === ""}>
        {landed === undefined && value.from !== "" ? (
          value.from
        ) : (
          // `offerNew`: a name the repository does not hold is a branch to cut.
          // Empty, a branch to pick, and nothing said about why.
          <BranchPicker
            label="Base branch"
            labelledByRow
            value={value.from}
            onValue={(from) => landed?.({ ...value, from })}
            branches={branches}
            offerNew
            required
            disabled={landed === undefined}
          />
        )}
      </ProposalField>
    </ProposalFields>
  );
}

function StartCard({ edits, moved, tuning, workflows, whole, forRequests }: NodeCardProps) {
  const { proposal } = edits;
  return (
    <ProposalGates
      workflow={proposal.workflow_id}
      workflowChoices={workflowChoicesOf(workflows, whole.job.owner_manifest_id)}
      {...(forRequests === undefined ? {} : { forRequests })}
      steps={[]}
      {...(moved === undefined
        ? {}
        : {
            // Another workflow rebuilds every gate, and every step node with it.
            // A step's tuning belongs to that step, so it goes with it.
            onWorkflow: (workflowId: string) => {
              const picked = workflows.find((one) => one.id === workflowId)?.steps ?? [];
              moved({
                proposal: proposalOnWorkflow(proposal, workflows, workflowId),
                tuning: { ...tuning, steps: tuningOf(picked).steps },
              });
            },
          })}
    />
  );
}

/** A Drone's settings for one step: model, effort, harness, context — and on the step worked per task, the tiers and the cap. */
function StepCard({ step, edits, tuning, tuned, moved, models, machineCap, ...rest }: NodeCardProps & { step: StepRead }) {
  const mine = tuning.steps[step.id];
  const set = (change: Partial<StepTuning>) => tuned?.(tunedStep(tuning, step.id, change));
  const { proposal } = edits;
  const open = tuned !== undefined;
  return (
    <>
      <ProposalFields>
        <ProposalField label="Model" bare={open}>
          {!open ? (
            (mine?.model ?? AUTO)
          ) : (
            <Select
              aria-label={`Model on ${step.label}`}
              value={mine?.model ?? ""}
              onChange={(event) => set({ model: event.target.value === "" ? null : event.target.value })}
            >
              <option value="">{AUTO}</option>
              {models.map((model) => (
                <option key={model} value={model}>
                  {model}
                </option>
              ))}
            </Select>
          )}
        </ProposalField>
        <ProposalField label="Effort" bare={open}>
          {!open ? (
            (mine?.effort ?? AUTO)
          ) : (
            <Select
              aria-label={`Effort on ${step.label}`}
              value={mine?.effort ?? ""}
              onChange={(event) => set({ effort: event.target.value === "" ? null : (event.target.value as Effort) })}
            >
              <option value="">{AUTO}</option>
              {EFFORTS.map((effort) => (
                <option key={effort} value={effort}>
                  {effort}
                </option>
              ))}
            </Select>
          )}
        </ProposalField>
        <ProposalField label="Harness" bare={open}>
          {!open ? (
            (mine?.harness ?? AUTO)
          ) : (
            <Select
              aria-label={`Harness on ${step.label}`}
              value={mine?.harness ?? ""}
              onChange={(event) => set({ harness: event.target.value === "" ? null : event.target.value })}
            >
              <option value="">{AUTO}</option>
              {HARNESSES.map((harness) => (
                <option key={harness} value={harness}>
                  {harness}
                </option>
              ))}
            </Select>
          )}
        </ProposalField>
        <ProposalField label="Context" bare={open}>
          {!open ? (
            (mine?.context ?? "")
          ) : (
            <Textarea
              aria-label={`Context for ${step.label}`}
              value={mine?.context ?? ""}
              onChange={(event) => set({ context: event.target.value })}
            />
          )}
        </ProposalField>
      </ProposalFields>
      {perTask(step) ? (
        <ProposalTiers
          tiers={proposal.tiers}
          models={models}
          {...(proposal.drone_cap === undefined ? {} : { droneCap: proposal.drone_cap })}
          machineCap={machineCap}
          {...(moved === undefined
            ? {}
            : {
                onTiers: (tiers: ProposalView["tiers"]) => moved({ proposal: { ...proposal, tiers } }),
                onDroneCap: (cap: number | undefined) =>
                  moved({
                    proposal: cap === undefined ? withoutCap(proposal) : { ...proposal, drone_cap: cap },
                  }),
              })}
        />
      ) : null}
      {/* A step whose gate is no node of its own carries its row here. */}
      {step.checks.length === 0 && step.judges.length === 0 ? (
        <GateRow step={step} edits={edits} moved={moved} {...rest} />
      ) : null}
    </>
  );
}

/** Which Checks run, how many Judges, and whether it stops for you. */
function GateCard({ step, edits, tuning, tuned, moved, ...rest }: NodeCardProps & { step: StepRead }) {
  const mine = tuning.steps[step.id];
  const set = (change: Partial<StepTuning>) => tuned?.(tunedStep(tuning, step.id, change));
  const gate = edits.proposal.gates.find((one) => one.step_id === step.id);
  const open = tuned !== undefined;
  const checks = checksOf(step, mine?.checks_off ?? []);
  return (
    <>
      <GateRow step={step} edits={edits} moved={moved} {...rest} />
      {checks.length === 0 ? null : (
        <section className="armada-proposal__region" aria-label={`Checks on ${step.label}`}>
          <h3 className="armada-proposal__heading">Checks</h3>
          <div className="armada-approval-canvas__checks">
            {checks.map((check) => (
              <Checkbox
                key={check.name}
                checked={check.runs}
                disabled={!open || gate?.checks !== true}
                aria-label={`${check.name} on ${step.label}`}
                onChange={(event) =>
                  set({ checks_off: checksOffWith(mine?.checks_off ?? [], check.name, event.target.checked) })
                }
              >
                {check.name}
              </Checkbox>
            ))}
          </div>
        </section>
      )}
      {step.judges.length === 0 || gate?.judge !== true ? null : (
        <ProposalFields>
          <ProposalField label="Judges" bare={open}>
            {!open ? (
              String(mine?.judges ?? 1)
            ) : (
              <Input
                aria-label={`Judges on ${step.label}`}
                type="number"
                min={1}
                value={String(mine?.judges ?? 1)}
                onChange={(event) => set({ judges: Math.max(1, Number(event.target.value) || 1) })}
              />
            )}
          </ProposalField>
        </ProposalFields>
      )}
    </>
  );
}

/** One step's gate boxes, `ProposalGates`' own row. */
function GateRow({
  step,
  edits,
  moved,
  whole,
  workflows,
  manifest,
}: Pick<NodeCardProps, "edits" | "moved" | "whole" | "workflows" | "manifest"> & { step: StepRead }) {
  const { proposal } = edits;
  const rows = gateRowsOf(
    proposal.gates.filter((gate) => gate.step_id === step.id),
    whole,
    stepsDeclaredOf(workflows, proposal.workflow_id),
    repositorySaysOf(manifest),
  );
  return (
    <ProposalGates
      stepsOnly
      workflow={proposal.workflow_id}
      workflowChoices={[]}
      steps={rows}
      {...(moved === undefined
        ? {}
        : {
            onGate: (stepId: string, box: GateBox, ticked: boolean) =>
              moved({ proposal: { ...proposal, gates: gatesWith(proposal.gates, stepId, { [box]: ticked }) } }),
            onOverride: (stepId: string, overridden: boolean) =>
              moved({ proposal: { ...proposal, gates: gatesWith(proposal.gates, stepId, { overridden }) } }),
          })}
    />
  );
}

/**
 * How the work leaves the worktree. **One control for pr_mode and local**, so
 * the end of the run reshapes off one answer — `deliveryOf`.
 */
function PullRequestCard({ edits, tuning, tuned, moved }: NodeCardProps) {
  const delivery = deliveryOf(tuning.local, edits.landing.pr_mode);
  return <DeliveryFields delivery={delivery} edits={edits} tuning={tuning} tuned={tuned} moved={moved} />;
}

function DeliveryFields({
  delivery,
  edits,
  tuning,
  tuned,
  moved,
}: {
  delivery: Delivery;
  edits: ProposalEdits;
  tuning: ApprovalTuning;
  tuned: NodeCardProps["tuned"];
  moved: NodeCardProps["moved"];
}) {
  const open = moved !== undefined;
  return (
    <ProposalFields>
      <ProposalField label="Delivery" bare={open}>
        {!open ? (
          DELIVERY[delivery]
        ) : (
          <Select
            aria-label="Delivery"
            value={delivery}
            onChange={(event) => {
              const picked = event.target.value as Delivery;
              moved({
                tuning: { ...tuning, local: picked === "local" },
                ...(picked === "local" ? {} : { landing: { ...edits.landing, pr_mode: picked } }),
              });
            }}
          >
            {(Object.keys(DELIVERY) as Delivery[]).map((one) => (
              <option key={one} value={one}>
                {DELIVERY[one]}
              </option>
            ))}
          </Select>
        )}
      </ProposalField>
      {/* The switch names itself, so it takes no row label beside it. */}
      {delivery === "local" ? null : !open ? (
        <ProposalField label="Auto-merge">{tuning.auto_merge ? "On" : "Off"}</ProposalField>
      ) : (
        <Switch checked={tuning.auto_merge} onChange={(event) => tuned?.({ ...tuning, auto_merge: event.target.checked })}>
          Auto-merge
        </Switch>
      )}
    </ProposalFields>
  );
}

/** Where it lands, and the delivery again: local only is answered here once the pull request node is gone. */
function LandCard({ value, landed, landsLate, branches, edits, tuning, tuned, moved }: NodeCardProps) {
  const delivery = deliveryOf(tuning.local, edits.landing.pr_mode);
  return (
    <>
      <ProposalLanding
        landing={value}
        {...(landsLate !== undefined
          ? { onLanding: (next: ProposalLandingValue) => landsLate(next.target), editable: ["target"] as const }
          : landed === undefined
            ? {}
            : { onLanding: landed })}
        completeChoices={completeChoices()}
        branches={branches}
        fields={["target", "branching", "completeWhen"]}
      />
      <DeliveryFields delivery={delivery} edits={edits} tuning={tuning} tuned={tuned} moved={moved} />
    </>
  );
}
