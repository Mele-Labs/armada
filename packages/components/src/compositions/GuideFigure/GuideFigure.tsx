import { useState, type CSSProperties, type ReactNode } from "react";

import type { GuideFigureId } from "../../guides/guide";
import { ADVANCE_GATE, STEP_STATE } from "../../generated/vocabulary";
import { WorkflowStepCard, type WorkflowStepCardProps } from "../WorkflowStepCard/WorkflowStepCard";

/**
 * A guide's drawing: **the real app, small and moving.** The owner, 26
 * September 2026, of the diagram that came before it — *"I assumed the figure
 * was an animation of the real app."*
 *
 * So every figure here mounts the components the screen mounts. A screen that
 * changes breaks its own guide, which is the cost he took: a drawing made of
 * anonymous boxes would have drifted from the app in silence.
 */
export type GuideFigureProps = {
  figure: GuideFigureId;
  /** Where it is drawn. The card is a narrow layer and the panel is not. */
  scale: "panel" | "card";
};

/**
 * What the drawing says for somebody who cannot see it. A sentence, never
 * "diagram" — and the whole subtree is hidden behind it, so a real card's own
 * roles do not leak into the guide's step list.
 */
const READING: Record<GuideFigureId, string> = {
  "group-order":
    "Two groups of a plan, one after the other. The first group's tasks are done and the group " +
    "has passed its checks; the second group and its tasks have not started.",
  "workflow-steps":
    "The three steps of a workflow, in order: plan the change, implement, review the change. " +
    "The last one waits for a person.",
};

export function GuideFigure({ figure, scale }: GuideFigureProps) {
  // Read at first render rather than in an effect, so the first painted frame is
  // already still — `travel.ts` and `useHold.ts` take the preference the same
  // way. The tokens zero --duration-travel under the same query as well.
  const [still] = useState(
    () => typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );

  return (
    <div
      className="armada-guide-figure"
      data-figure={figure}
      data-scale={scale}
      data-still={still || undefined}
      role="img"
      aria-label={READING[figure]}
    >
      {/* Hidden rather than trusted to `role="img"`: a real component brings
          its own list, its own headings and its own labels, and a guide's step
          list is read by role. Nothing in here is focusable. */}
      <div className="armada-guide-figure__stage" aria-hidden="true" inert>
        <Drawing figure={figure} />
      </div>
    </div>
  );
}

function Drawing({ figure }: { figure: GuideFigureId }) {
  if (figure === "group-order") return <GroupOrder />;
  return <WorkflowSteps />;
}

/**
 * One element of a drawing, arriving in its turn.
 *
 * **One animation per element, and the wrapper is what carries it** — nothing
 * here reaches into a real component's own classes to move its parts.
 */
function Arriving({ at, children }: { at: number; children: ReactNode }) {
  return (
    // Its place in the queue, which is what staggers it. Not a duration:
    // --duration-travel is the duration and this multiplies it.
    <div className="armada-guide-figure__arriving" style={{ "--armada-figure-order": at } as CSSProperties}>
      {children}
    </div>
  );
}

/** A step state's own word, off the registry rather than typed here. */
const said = (state: string): string => STEP_STATE[state]?.verb ?? state;

/**
 * The three steps of the `feature` workflow, drawn with `WorkflowStepCard` —
 * the same facts, the same gate sentence.
 *
 * The labels and gates are `featureWorkflow()`'s, the fixture every arc
 * scenario runs on.
 */
const WORKFLOW_STEPS: readonly WorkflowStepCardProps[] = [
  { kind: "step", name: "Plan the change", activity: "advanced", said: said("advanced"), ordinal: 1 },
  // `current` is off, and that is the rule rather than the fixture: it pulses
  // the step's mark on a loop, and a loop says *still working*.
  {
    kind: "step",
    name: "Implement",
    activity: "running",
    said: said("running"),
    ordinal: 2,
    facts: [{ value: "2 groups" }],
  },
  {
    kind: "step",
    name: "Review the change",
    activity: "not_started",
    said: said("not_started"),
    ordinal: 3,
    facts: [{ value: "delivers" }],
    gate: ADVANCE_GATE["human_always"]?.verb ?? "a person answers",
  },
];

/** A job runs its workflow's steps, in order. Guides 15 and 19 both name it. */
function WorkflowSteps() {
  return (
    <div className="armada-guide-figure__run">
      {WORKFLOW_STEPS.map((card, at) => (
        <Arriving key={card.name} at={at}>
          <WorkflowStepCard {...card} />
        </Arriving>
      ))}
    </div>
  );
}

/** One group's card and its tasks, as the canvas draws a group and a task. */
const GROUPS: readonly { group: WorkflowStepCardProps; tasks: readonly WorkflowStepCardProps[] }[] = [
  {
    group: {
      kind: "group",
      name: "Group 1",
      activity: "advanced",
      said: "passed",
      facts: [{ value: "2 tasks" }, { value: "7 checks" }],
    },
    tasks: [
      { kind: "task", name: "Read the settings out of one place", activity: "advanced", said: "done", facts: [{ value: "T1" }] },
      { kind: "task", name: "Hand the store in", activity: "advanced", said: "done", facts: [{ value: "T2" }] },
    ],
  },
  {
    group: {
      kind: "group",
      name: "Group 2",
      activity: "not_started",
      said: "pending",
      facts: [{ value: "2 tasks" }, { value: "7 checks" }],
    },
    tasks: [
      { kind: "task", name: "Add a test that does not construct the store", activity: "not_started", said: "open", facts: [{ value: "T3" }] },
      { kind: "task", name: "Update the package's README", activity: "not_started", said: "open", facts: [{ value: "T4" }] },
    ],
  },
];

/**
 * The groups of a plan, one at a time. The first is through its checks before
 * the second starts, which is the relation guides 4 and 17 both name.
 */
function GroupOrder() {
  let at = 0;
  return (
    <div className="armada-guide-figure__groups">
      {GROUPS.map(({ group, tasks }) => (
        <div key={group.name} className="armada-guide-figure__group">
          <Arriving at={at++}>
            <WorkflowStepCard {...group} />
          </Arriving>
          <div className="armada-guide-figure__tasks">
            {tasks.map((task) => (
              <Arriving key={task.name} at={at++}>
                <WorkflowStepCard {...task} />
              </Arriving>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
