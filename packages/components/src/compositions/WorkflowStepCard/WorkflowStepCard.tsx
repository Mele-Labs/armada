import { FactChip, type FactChipNamed } from "../FactChip/FactChip";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";
import { StepBar, type TaskBarSegment } from "../StepBar/StepBar";

/**
 * One node of a Job's graphs — a step, a group inside a plan, or a task
 * inside a group. **The same card on the canvas and
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

/**
 * Which of the three step-card designs draws a step (owner, 30 Sep 2026: *show
 * me options*). **Exploration, not a setting**: he picks one by looking in the
 * mock, and the other two are removed before anything lands. Absent draws
 * today's card.
 *
 * - `progress` says how far the work has got.
 * - `needs` leads with what waits on a person or went wrong, and is otherwise
 *   the mark and the name alone.
 * - `compact` is the design board's: narrower, the name in mono, the state as
 *   a coloured second line.
 */
export type WorkflowStepCardDesign = "progress" | "needs" | "compact";

/** One thing waiting on a person, or gone wrong. A value, never a sentence. */
export type WorkflowStepNeed = {
  says: string;
  /** `waiting` is a person's turn; `failed` is something that broke. */
  tone: "waiting" | "failed";
};

export type WorkflowStepCardProps = {
  /**
   * A step of the workflow, a group of a plan, or a task inside a group. The
   * two narrower widths say *inside a plan*.
   */
  kind: "step" | "group" | "task";
  /** The step's label, the group's name, or the task's title. */
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
  /**
   * The board's second row: how long, and where it has got to — `9m 38s ·
   * advanced`. **A line rather than chips** on a step of the run, so the card
   * reads the way the Workflow board draws it; a node that passes none draws
   * its facts as chips instead.
   */
  line?: string;
  /** The step the Job is on. The card takes a stronger edge; what loops is `running`. */
  current?: boolean;
  /** Open in the inspector. */
  selected?: boolean;
  /** The gate's own word, where the step waits for a person. Absent on `auto`. */
  gate?: string;
  /** Opens it in the inspector. Absent draws a card that is not a control. */
  onOpen?: () => void;
  /** Which design under comparison draws it. Absent draws today's card. */
  design?: WorkflowStepCardDesign;
  /**
   * `progress`'s second line on the step at work: one segment per group of the
   * plan, coloured by the group's state, and the line beside it — how long,
   * and how many Drones are on it now. `label` is the bar's tooltip, which is
   * where the count lives: a count beside the segments it counts is the
   * aggregate the owner ruled out on 29 Sep 2026.
   */
  progress?: { groups: readonly TaskBarSegment[]; label: string; line?: string };
  /** `needs`'s lines: what waits on a person or went wrong, most pressing first. */
  needs?: readonly WorkflowStepNeed[];
};

export function WorkflowStepCard({
  kind,
  name,
  nameIsAnIdentifier,
  activity,
  said,
  ordinal,
  facts = [],
  line,
  current = false,
  selected = false,
  gate,
  onOpen,
  design,
  progress,
  needs = [],
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

  // The line under the name, as each design draws it. Today's and `compact`
  // draw the one line they are handed; `compact`'s stylesheet colours it by
  // the state.
  const plain = line === undefined ? null : <span className="armada-wf-card__line">{line}</span>;
  const second =
    design === "needs" ? (
      needs.length === 0 ? null : (
        <span className="armada-wf-card__needs">
          {needs.map((need) => (
            <span key={need.says} className="armada-wf-card__need" data-tone={need.tone}>
              {need.says}
            </span>
          ))}
        </span>
      )
    ) : design === "progress" ? (
      progress !== undefined ? (
        <span className="armada-wf-card__progress">
          {progress.groups.length === 0 ? null : <StepBar tasks={progress.groups} label={progress.label} />}
          {progress.line === undefined ? null : <span className="armada-wf-card__line">{progress.line}</span>}
        </span>
      ) : activity === "not_started" ? null : (
        plain
      )
    ) : (
      plain
    );

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
      {second}
      {facts.length === 0 ? null : (
        <span className="armada-wf-card__facts">
          {facts.map((fact) => (
            <FactChip key={fact.value} named={fact.named}>
              {fact.value}
            </FactChip>
          ))}
        </span>
      )}
      {gate === undefined || design === "needs" ? null : (
        <span className="armada-wf-card__gate" data-chip={line === undefined ? undefined : "true"}>
          {gate}
        </span>
      )}
    </>
  );

  const attributes = {
    className: "armada-wf-card",
    "data-kind": kind,
    "data-current": current || undefined,
    "data-working": working || undefined,
    "data-design": design,
    "data-activity": activity,
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
