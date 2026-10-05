import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { ConceptLabel } from "../../concepts";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_ALWAYS_LOOKS } from "../../guides";

/**
 * One step's gate: **three independent boxes, and a fourth state the boxes
 * cannot express** (#1548).
 *
 * Checks, Judge and You are three facts about one step and not three points on
 * a scale, so they are boxes rather than a menu — "a Judge reads this, and no
 * Check runs on it" is a sentence the menu could not say. The fourth state is
 * the repository deciding, which belongs to no box because it is not this
 * Job's answer at all; overriding it hands the step back to the three.
 */
export type ProposalGateRow = {
  /** The step's own id, as the workflow declares it. */
  id: string;
  /** What the step is called. */
  label: string;
  /**
   * Whether that name is the `step_id`, so it renders in mono.
   *
   * **Mono is for a value a person copies, and a step's name is not one**
   * (`0bgt`, 28 Sep) — every step here was mono, including `Plan the change`.
   * A workflow that declares no label gets its id substituted by Fleet
   * (`WorkflowSummary.steps`), and *that* is an identifier; `bug` is the
   * reference sample and every one of its seven steps reads that way. The
   * spelling is `Inspector.tsx`'s, which is where this rule already lives.
   */
  labelIsAnIdentifier?: boolean;
  checks: boolean;
  judge: boolean;
  you: boolean;
  /**
   * The repository policy that decides this step, where one does and this Job
   * has not taken it over. The policy's own name — `auto_merge`, `review_gate`.
   */
  repositoryDecides?: string;
  /** Whether this Job took the decision off the repository for itself. */
  overridden?: boolean;
  /**
   * The `advance_gate` this combination is on the wire today.
   *
   * **Held, and not drawn.** The wire stays an enum until the schema lock, so
   * the field stays; a raw `auto_if_judge_passes` beside a step name is
   * Fleet's spelling in a place a person is deciding, and the sentence under
   * the boxes already says what it does.
   */
  advanceGate: string;
  /** What Fleet does with it, in one sentence. */
  does: string;
  /**
   * What this combination asks for that Fleet has nothing to do, where it asks
   * for anything. **The sentence, not a flag** — what is missing differs per
   * step, and "not supported" would say nothing about which part.
   */
  unmeant?: string;
};

/** Which of the three boxes moved. */
export type GateBox = "checks" | "judge" | "you";

/** One workflow this Job could run, as the picker offers it. */
export type WorkflowChoice = { id: string; name: string; steps: number };

export type ProposalGatesProps = {
  /** The workflow this Job runs, by the id its own file declares. */
  workflow: string;
  /** Every workflow the picker offers. Empty draws the name and no control. */
  workflowChoices: readonly WorkflowChoice[];
  /** Another workflow picked. Absent draws the name frozen. */
  onWorkflow?: (workflowId: string) => void;
  /**
   * What kind of request the workflow is for, in its own definition's words —
   * `WorkflowSummary.for_requests`. Absent draws nothing under the name.
   */
  forRequests?: string;
  steps: readonly ProposalGateRow[];
  /** Approved, so the heading says the gate cannot move any more. */
  frozen?: boolean;
  /** One box moved on one step. Absent draws every gate as a frozen reading. */
  onGate?: (stepId: string, box: GateBox, ticked: boolean) => void;
  /** Take the decision off the repository for this Job, or hand it back. */
  onOverride?: (stepId: string, overridden: boolean) => void;
  /**
   * The step rows alone, with no heading and no picker — one step's gate on
   * the approval canvas, where the node it opens from already names the step
   * and the workflow is picked on a node of its own.
   */
  stepsOnly?: boolean;
};

/**
 * The workflow, and what gates each of its steps.
 *
 * **One section since 28 Sep 2026** (`2b4j`). The picker was a field in the
 * run of settings at the top and the gates were a region at the bottom, with
 * the tier map between them — so the thing being chosen and the steps it
 * brings with it were two blocks a screen apart. They are one region now: pick
 * the workflow, then customise each step it declares.
 */
export function ProposalGates({
  workflow,
  workflowChoices,
  onWorkflow,
  forRequests,
  steps,
  frozen,
  onGate,
  onOverride,
  stepsOnly = false,
}: ProposalGatesProps) {
  const named = workflowChoices.find((one) => one.id === workflow);
  return (
    <section className="armada-proposal__region" aria-label={stepsOnly ? "Gate" : "Workflow"}>
      {stepsOnly ? null : (
      <>
      {/* The `?` on the heading, not on a row: what the ticks cannot turn off
          is the same about every step, and a mark per row would read as a
          property of that row. The sentence that used to stand here is guide
          9 — it was true of a Job nobody had approved (#1602). */}
      <div className="armada-proposal__heading-row">
        <h3 className="armada-proposal__heading">{frozen ? "Workflow, frozen" : "Workflow"}</h3>
        <GuideMark guide={GUIDE_ALWAYS_LOOKS} />
      </div>
      {/* The picker carries no label of its own: the heading above it is the
          label, and `Workflow — Workflow` is what a field under it would
          read. The steps beneath are what it brings with it. */}
      {/* **The name alone, read.** Every step is listed under it, so a count
          beside the name is the number beside the things it counts — hard
          rule 7, `design-system.md`. The picker's options keep theirs: the
          steps of a workflow not picked are drawn nowhere. */}
      {onWorkflow === undefined || workflowChoices.length === 0 ? (
        <p className="armada-proposal__workflow-said">{named?.name ?? workflow}</p>
      ) : (
        <Select
          aria-label="Workflow"
          value={workflow}
          onChange={(event) => onWorkflow(event.target.value)}
        >
          {/* A Job naming a workflow this Fleet has no record of still reads
              as itself: dropping it would make the picker answer a different
              question from the one the Job is on. */}
          {named === undefined ? <option value={workflow}>{workflow}</option> : null}
          {workflowChoices.map((choice) => (
            <option key={choice.id} value={choice.id}>
              {stepsSaid(choice.name, choice.steps)}
            </option>
          ))}
        </Select>
      )}
      {/* **What the workflow promises, under its name**, so a workflow picked
          for the wrong kind of request reads as wrong before it runs — the
          owner's Job 1 of 1 Oct 2026 ran `refactor`, which promises nothing
          anyone sees changes, for a visible change. */}
      {forRequests === undefined ? null : (
        <p className="armada-proposal__workflow-promise">{forRequests}</p>
      )}
      </>
      )}
      <ul className="armada-proposal__gates">
        {steps.map((step) => (
          <li className="armada-proposal__gate" key={step.id} aria-label={step.label}>
            <span
              className="armada-proposal__gate-step"
              data-identifier={step.labelIsAnIdentifier === true ? "true" : undefined}
            >
              {step.label}
            </span>
            {step.repositoryDecides !== undefined && step.overridden !== true ? (
              <Deferred
                step={step}
                {...(onOverride === undefined ? {} : { onOverride })}
              />
            ) : (
              <Boxes step={step} {...(onGate === undefined ? {} : { onGate })} />
            )}
            <p className="armada-proposal__gate-does">{step.does}</p>
            {step.unmeant === undefined ? null : (
              <p className="armada-proposal__unmeant" role="note">
                {step.unmeant}
              </p>
            )}
            {/* What the button would change, read before it is pressed. The
                only explanation used to be the overrode sentence below, which
                a person reads after overriding — so the choice was made
                blind (`rhxt`, 29 Sep). Nothing to press, nothing to say. */}
            {step.repositoryDecides === undefined ||
            step.overridden === true ||
            onOverride === undefined ? null : (
              <p className="armada-proposal__gate-choice">
                Deciding it for this Job hands this step to the three boxes instead.
              </p>
            )}
            {step.repositoryDecides === undefined || step.overridden !== true ? null : (
              <p className="armada-proposal__overrode">
                {`This Job decides this step for itself, in place of the repository's ${step.repositoryDecides}.`}
                {onOverride === undefined ? null : (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => onOverride(step.id, false)}
                  >
                    Give it back to the repository
                  </Button>
                )}
              </p>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

/**
 * A workflow with how many steps it runs, as the picker offers it. A workflow
 * of one step says `step`, because `1 steps` is how a screen reads as
 * generated.
 */
function stepsSaid(name: string, steps: number): string {
  return `${name} — ${steps} ${steps === 1 ? "step" : "steps"}`;
}

/** The three boxes, ticked or read. */
function Boxes({
  step,
  onGate,
}: {
  step: ProposalGateRow;
  onGate?: (stepId: string, box: GateBox, ticked: boolean) => void;
}) {
  const boxes: [GateBox, string, boolean][] = [
    ["checks", "Checks", step.checks],
    ["judge", "Judge", step.judge],
    ["you", "You", step.you],
  ];
  // Frozen, the ticks are a reading rather than a control: a disabled checkbox
  // is a control somebody will press, and what it says about a Job that has
  // already started is a fact.
  if (onGate === undefined) {
    const on = boxes.filter(([, , ticked]) => ticked).map(([, label]) => label);
    return (
      <p className="armada-proposal__gate-frozen">
        {on.length === 0 ? "Nobody looks" : on.join(" · ")}
      </p>
    );
  }
  return (
    <div className="armada-proposal__boxes">
      {boxes.map(([box, label, ticked]) => (
        <Checkbox
          key={box}
          checked={ticked}
          aria-label={`${label} on ${step.label}`}
          onChange={(event) => onGate(step.id, box, event.target.checked)}
        >
          {label}
        </Checkbox>
      ))}
    </div>
  );
}

/** The fourth state: the repository decides, and this Job may take it over. */
function Deferred({
  step,
  onOverride,
}: {
  step: ProposalGateRow;
  onOverride?: (stepId: string, overridden: boolean) => void;
}) {
  return (
    <div className="armada-proposal__deferred">
      <span className="armada-proposal__deferred-said">
        {"The repository decides — "}
        {/* The key as `armada.yml` writes it, and `concepts.tsx` says what it
            is: a bare identifier gets a tooltip naming it, which is the rule
            `ProposalFields` labels already keep. */}
        <ConceptLabel className="armada-proposal__deferred-policy">
          {step.repositoryDecides}
        </ConceptLabel>
      </span>
      {onOverride === undefined ? null : (
        <Button variant="secondary" size="sm" onClick={() => onOverride(step.id, true)}>
          Decide it for this Job
        </Button>
      )}
    </div>
  );
}
