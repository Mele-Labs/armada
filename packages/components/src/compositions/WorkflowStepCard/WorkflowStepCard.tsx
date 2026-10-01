import type { CSSProperties } from "react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
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
 * the caller names one. A node whose state is not a step's — a plan group's
 * `joining`, `retrying` — hands its registry row's glyph and token through
 * `mark` and its verb through `said`, and the activity it passes only decides
 * the sweep.
 */

/** One short fact under the name. A value, never a sentence. */
export type WorkflowStepFact = {
  value: string;
  named?: FactChipNamed;
};

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
   * A mark of the caller's own, read from a registry row — its `icon` and its
   * `statusToken` (`--status-not-started`). **Present, it replaces the
   * activity mark's glyph and hue**; `activity` still decides whether the card
   * sweeps. A plan group passes `GROUP_STATE`'s row, so the graph draws what
   * the list draws. Absent changes nothing.
   */
  mark?: { icon: LucideIcon; token: string };
  /**
   * The state in words — the registry's verb for a step, the group machine's
   * own value for a group. Read to somebody who cannot see the mark.
   */
  said: string;
  /** Its position in the run, counted from one. Stands in for a glyph on a step nothing entered. */
  ordinal?: number;
  facts?: readonly WorkflowStepFact[];
  /**
   * How long, then where it has got to — `9m 38s · advanced` on a finished
   * step, `55m · 2 Drones` on the one at work. **Absent on a step nothing
   * entered**, which is the mark and the name alone. A node that passes none
   * draws its facts as chips instead.
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
  /**
   * What waits on a person or went wrong, most pressing first (owner, 30 Sep
   * 2026), each in its hue and **above everything else under the name**.
   */
  needs?: readonly WorkflowStepNeed[];
  /**
   * The plan's groups on the step at work, one segment each, coloured by the
   * group's state, with `line` beside it. `label` is the bar's tooltip, which
   * is where the count lives: a count beside the segments it counts is the
   * aggregate the owner ruled out on 29 Sep 2026.
   */
  bar?: { groups: readonly TaskBarSegment[]; label: string };
};

export function WorkflowStepCard({
  kind,
  name,
  nameIsAnIdentifier,
  activity,
  mark,
  said,
  ordinal,
  facts = [],
  line,
  current = false,
  selected = false,
  gate,
  onOpen,
  needs = [],
  bar,
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

  // Under the name: what needs a person first, then where the work has got
  // to — the group bar beside the line on the step at work.
  const timed = line === undefined ? null : <span className="armada-wf-card__line">{line}</span>;
  const below = (
    <>
      {needs.length === 0 ? null : (
        <span className="armada-wf-card__needs">
          {needs.map((need) => (
            <span key={need.says} className="armada-wf-card__need" data-tone={need.tone}>
              {need.says}
            </span>
          ))}
        </span>
      )}
      {bar === undefined || bar.groups.length === 0 ? (
        timed
      ) : (
        <span className="armada-wf-card__progress">
          <StepBar tasks={bar.groups} label={bar.label} />
          {timed}
        </span>
      )}
    </>
  );

  const body = (
    <>
      {/* The bar, drawn before the head so it lands on the card's own top edge
          rather than inside its content box. Nothing to read: the mark and the
          word beside it already say running. */}
      {working ? <span className="armada-wf-card__sweep" aria-hidden="true" /> : null}
      <span className="armada-wf-card__head">
        {mark === undefined ? (
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
        ) : (
          <OwnMark mark={mark} label={said} says={`${name}, ${said}`} />
        )}
        <span className="armada-wf-card__name" data-identifier={nameIsAnIdentifier || undefined}>
          {name}
        </span>
      </span>
      {below}
      {facts.length === 0 ? null : (
        <span className="armada-wf-card__facts">
          {facts.map((fact) => (
            <FactChip key={fact.value} named={fact.named}>
              {fact.value}
            </FactChip>
          ))}
        </span>
      )}
      {/* The board's outlined chip on a step of the run, which draws no facts
          and may draw no line; a fragment under the facts everywhere else. */}
      {gate === undefined ? null : (
        <span className="armada-wf-card__gate" data-chip={kind === "step" && facts.length === 0 ? "true" : undefined}>
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

/**
 * The caller's mark, in `StepActivityMark`'s slot and geometry — 16px, a 12px
 * glyph at stroke 2, the same hidden name and the same tooltip — with the hue
 * its registry row names rather than one keyed off `data-activity`.
 */
function OwnMark({
  mark,
  label,
  says,
}: {
  mark: NonNullable<WorkflowStepCardProps["mark"]>;
  label: string;
  says: string;
}) {
  const Icon = mark.icon;
  return (
    <Tooltip asChild label={says}>
      <span
        className="armada-step-mark armada-wf-card__mark"
        style={{ "--armada-wf-card-mark": `var(${mark.token})` } as CSSProperties}
      >
        <Icon size={12} strokeWidth={2} aria-hidden />
        <span className="armada-step-mark__name">{label}</span>
      </span>
    </Tooltip>
  );
}
