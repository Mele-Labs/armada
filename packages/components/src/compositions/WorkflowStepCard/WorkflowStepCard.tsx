import type { CSSProperties } from "react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { FactChip, type FactChipNamed } from "../FactChip/FactChip";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";
import { StepBar, type TaskBarSegment } from "../StepBar/StepBar";
import { StepPhaseTrack, type StepPhasePart } from "../StepPhaseTrack/StepPhaseTrack";

/**
 * One node of a Job's graphs — a step, a group inside a plan, or a task
 * inside a group. **The same card on every canvas that
 * draws a step**, so what a node says about itself is one thing.
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
  /** What the fact leaves out, named on hover. Absent draws a bare chip. */
  hint?: string;
};

/** One thing waiting on a person, or gone wrong. A value, never a sentence. */
export type WorkflowStepNeed = {
  says: string;
  /** `waiting` is a person's turn; `failed` is something that broke. */
  tone: "waiting" | "failed";
};

/**
 * A header band over the card, in the bays' language (`PoolSlots`): a solid
 * band in a hue that carries **the step's name first**, its position before it
 * and a mono tag at the trailing edge. **The band says what the card is and
 * never how a run is going**, so the caller names the hue. What the hue stands
 * for is spelled out in the body's labelled `details`, never left to the colour.
 * Absent draws the card exactly as before.
 */
export type WorkflowStepBand = {
  /** A mono tag at the trailing edge: the step's id. */
  tag?: string;
  /** The band's hue, a custom property name — `--status-running`. */
  token: string;
  /** `dashed` is an outline with no fill, `hatched` the caution hatch a stranded bay wears. */
  look?: "solid" | "dashed" | "hatched";
};

/** One labelled line in a banded card's body: a mark, its word, and what it holds. */
export type WorkflowStepDetail = {
  icon: LucideIcon;
  /** The word beside the mark, always drawn. */
  label: string;
  /** Mono, clipped to one line and named in full on hover. Absent draws the label alone. */
  value?: string;
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
  /** Stands back while the canvas focuses on other steps: the scrim over it. */
  dimmed?: boolean;
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
  /**
   * The running step's parts in order, along the card's bottom edge — done,
   * now, next (owner, 3 Oct 2026, `ouqa`). **Drawn, it is the card's one loop**
   * and the sweep stands down. Absent on a step neither running nor waiting.
   */
  track?: readonly StepPhasePart[];
  band?: WorkflowStepBand;
  /** Labelled lines under the band, in order. Read only with `band`. */
  details?: readonly WorkflowStepDetail[];
  /** A mono line in the accent under the others, the card's own act — where it sends work back to. */
  action?: string;
  /** Takes the accent edge on its leading side: the card sends work back. */
  returns?: boolean;
  /** A step added to this Job alone: a dashed accent edge, so it reads apart from the workflow's own. */
  added?: boolean;
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
  dimmed = false,
  gate,
  onOpen,
  needs = [],
  bar,
  track,
  band,
  details = [],
  action,
  returns = false,
  added = false,
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
      {working && track === undefined ? <span className="armada-wf-card__sweep" aria-hidden="true" /> : null}
      {band === undefined ? null : (
        <span className="armada-wf-card__band">
          {ordinal === undefined ? null : <span className="armada-wf-card__band-order">{ordinal}</span>}
          <span className="armada-wf-card__band-name">{name}</span>
          {band.tag === undefined ? null : <span className="armada-wf-card__band-tag">{band.tag}</span>}
        </span>
      )}
      {band !== undefined ? null : <span className="armada-wf-card__head">
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
      </span>}
      {band === undefined || details.length === 0 ? null : (
        <span className="armada-wf-card__details">
          {details.map((one) => (
            <span key={one.label} className="armada-wf-card__detail">
              <one.icon size={12} strokeWidth={2} aria-hidden />
              <span className="armada-wf-card__detail-label">{one.label}</span>
              {one.value === undefined ? null : (
                <Tooltip asChild label={one.value}>
                  <span className="armada-wf-card__detail-value">{one.value}</span>
                </Tooltip>
              )}
            </span>
          ))}
        </span>
      )}
      {below}
      {facts.length === 0 ? null : (
        <span className="armada-wf-card__facts">
          {facts.map((fact) =>
            fact.hint === undefined ? (
              <FactChip key={fact.value} named={fact.named}>
                {fact.value}
              </FactChip>
            ) : (
              <Tooltip key={fact.value} asChild label={fact.hint}>
                <span>
                  <FactChip named={fact.named}>{fact.value}</FactChip>
                </span>
              </Tooltip>
            ),
          )}
        </span>
      )}
      {/* The board's outlined chip on a step of the run, which draws no facts
          and may draw no line; a fragment under the facts everywhere else. */}
      {gate === undefined ? null : (
        <span className="armada-wf-card__gate" data-chip={kind === "step" && facts.length === 0 ? "true" : undefined}>
          {gate}
        </span>
      )}
      {action === undefined ? null : <span className="armada-wf-card__action">{action}</span>}
      {track === undefined ? null : (
        <span className="armada-wf-card__track">
          <StepPhaseTrack parts={track} />
        </span>
      )}
    </>
  );

  // The part now is in the card's own name too: a button's contents are
  // presentational, so the track inside it reaches a screen reader no other way.
  const nowSaid = track?.find((part) => part.state === "now")?.label;
  const named = nowSaid === undefined ? `${name}, ${said}` : `${name}, ${said}, ${nowSaid}`;
  const attributes = {
    className: "armada-wf-card",
    "data-kind": kind,
    "data-current": current || undefined,
    "data-dimmed": dimmed || undefined,
    "data-working": working || undefined,
    "data-added": added || undefined,
    ...(band === undefined
      ? {}
      : {
          "data-banded": "",
          "data-look": band.look ?? "solid",
          "data-returns": returns || undefined,
          style: { "--armada-wf-band": `var(${band.token})` } as CSSProperties,
        }),
  };

  return onOpen === undefined ? (
    <span {...attributes} role="group" aria-label={named}>
      {body}
    </span>
  ) : (
    <button
      {...attributes}
      type="button"
      aria-current={selected ? "true" : undefined}
      aria-label={named}
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
