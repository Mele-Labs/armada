import { FactChip, type FactChipNamed } from "../FactChip/FactChip";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

/**
 * One node of a Job's graphs — a step, the plan one step recorded, a group
 * inside a plan, or a task inside a group. **The same card on the canvas and
 * in the stacked run**, so a toggle between the two changes the arrangement
 * and never what a node says about itself.
 *
 * **Status colour is not chosen here.** The mark is `StepActivityMark`, whose
 * hue maps onto the step machine, and a chip takes a verdict's hue only where
 * the caller names one. A group's own state (`joining`, `checking`) has no
 * mark of its own, so the caller maps it to the nearest activity and hands the
 * group's word through `said` — nothing is lost and nothing is invented.
 */

/** One short fact under the name. A value, never a sentence. */
export type WorkflowStepFact = {
  value: string;
  named?: FactChipNamed;
};

export type WorkflowStepCardProps = {
  /**
   * A step of the workflow, the plan a step recorded, a group of that plan, or
   * a task inside a group. `plan` takes a step's own width: it is drawn beside
   * steps, and the two narrower widths say *inside a plan*.
   */
  kind: "step" | "plan" | "group" | "task";
  /** The step's label, `Plan`, the group's name, or the task's title. */
  name: string;
  /** Whether `name` is a `step_id` rather than a label, so it renders in mono. */
  nameIsAnIdentifier?: boolean;
  activity: StepActivity;
  /**
   * The state in words — the registry's verb for a step, the group machine's
   * own value for a group. Read to somebody who cannot see the mark.
   */
  said: string;
  /** Its position in the run, counted from one. Stands in for a glyph on a step nothing entered. */
  ordinal?: number;
  facts?: readonly WorkflowStepFact[];
  /** The step the Job is on. The card takes a stronger edge; what loops is `running`. */
  current?: boolean;
  /** Open in the inspector. */
  selected?: boolean;
  /** The gate's own word, where the step waits for a person. Absent on `auto`. */
  gate?: string;
  /** Opens it in the inspector. Absent draws a card that is not a control. */
  onOpen?: () => void;
};

export function WorkflowStepCard({
  kind,
  name,
  nameIsAnIdentifier,
  activity,
  said,
  ordinal,
  facts = [],
  current = false,
  selected = false,
  gate,
  onOpen,
}: WorkflowStepCardProps) {
  // **What is still working sweeps** — `design-system.md`, Motion: *what
  // animates on a loop is what is still working*, and the running node was the
  // one thing on these graphs a person could not tell was moving (owner, 28 Sep
  // 2026: *the step that is running should have more animation pop to it. On
  // the existing page the panel that is running had some nice animations*).
  // The panel he means is `Chapter` at `data-tone="running"`, and this is the
  // same three things, not a fourth invention: the running edge, the running
  // wash, and one segment travelling the top edge at `--duration-pulse`.
  const working = activity === "running";

  const body = (
    <>
      {/* The bar, drawn before the head so it lands on the card's own top edge
          rather than inside its content box. Nothing to read: the mark and the
          word beside it already say running. */}
      {working ? <span className="armada-wf-card__sweep" aria-hidden="true" /> : null}
      <span className="armada-wf-card__head">
        <StepActivityMark
          activity={activity}
          label={said}
          ordinal={ordinal}
          // **No `pulsing`, and that loses nothing.** The mark only ever
          // pulses on `running`, which is the one state that now sweeps — and
          // one loop per card means the sweep is the loop. A card that is not
          // working does not move at all.
          says={`${name}, ${said}`}
        />
        <span className="armada-wf-card__name" data-identifier={nameIsAnIdentifier || undefined}>
          {name}
        </span>
      </span>
      {facts.length === 0 ? null : (
        <span className="armada-wf-card__facts">
          {facts.map((fact) => (
            <FactChip key={fact.value} named={fact.named}>
              {fact.value}
            </FactChip>
          ))}
        </span>
      )}
      {gate === undefined ? null : <span className="armada-wf-card__gate">{gate}</span>}
    </>
  );

  const attributes = {
    className: "armada-wf-card",
    "data-kind": kind,
    "data-current": current || undefined,
    "data-working": working || undefined,
  };

  return onOpen === undefined ? (
    <span {...attributes} role="group" aria-label={`${name}, ${said}`}>
      {body}
    </span>
  ) : (
    <button
      {...attributes}
      type="button"
      aria-current={selected ? "true" : undefined}
      aria-label={`${name}, ${said}`}
      onClick={onOpen}
    >
      {body}
    </button>
  );
}
