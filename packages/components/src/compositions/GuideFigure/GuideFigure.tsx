import { useState, type CSSProperties, type ReactNode } from "react";

import type { GuideFigureId } from "../../guides/guide";
import { ADVANCE_GATE, JOB_STATUS, STEP_STATE } from "../../generated/vocabulary";
import { JobMembers, type JobMemberJoin, type JobMemberRow } from "../JobMembers/JobMembers";
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
  "members-landing":
    "Three pull requests landing in order, numbered one to three, each saying which link it " +
    "carries to the one before it.",
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
      <div className="armada-guide-figure__stage" aria-hidden="true">
        <Drawing figure={figure} />
      </div>
    </div>
  );
}

function Drawing({ figure }: { figure: GuideFigureId }) {
  if (figure === "members-landing") return <MembersLanding />;
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
 * The three steps of the `feature` workflow, as `WorkflowCanvas` draws them —
 * the same `WorkflowStepCard`, the same facts, the same gate sentence.
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

/** One member's badge, off the registry the Board's own rows read. */
function badge(status: string): JobMemberRow["state"] {
  const rendering = JOB_STATUS[status];
  if (rendering?.verb == null || rendering.icon == null || rendering.badgeStatus == null) {
    return { as: "text", wire: status, missing: `No row in the registry for ${status}` };
  }
  return { as: "badge", status: rendering.badgeStatus, icon: rendering.icon, label: rendering.verb };
}

/**
 * Three members, as `JobMembers` draws them on a job: the number carrying the
 * order, and the line between two rows saying how they are related.
 *
 * Nothing here is a control. No member takes `onOpen`, `onDrop` or a branch,
 * so the real list renders no button and the figure holds no focus stop.
 *
 * **None of the three is running.** A running badge breathes on a loop, and a
 * loop says *still working* — `docs/contracts/design-system.md`.
 */
const MEMBERS: readonly JobMemberRow[] = [
  {
    id: "m1",
    ordinal: 1,
    title: "Add the settings reader",
    state: badge("completed_success"),
    landed: true,
  },
  {
    id: "m2",
    ordinal: 2,
    title: "Read the settings out of one place",
    state: badge("awaiting_review"),
    link: "stacked",
  },
  {
    id: "m3",
    ordinal: 3,
    title: "Publish the settings package",
    state: badge("queued"),
    link: "published",
  },
];

/**
 * The three joins, in the words the real screen uses. They are the figure's
 * subject: what a link *is*, rather than which one a member has.
 */
const JOINS: readonly JobMemberJoin[] = [
  { link: "stacked", said: "Member 2 branches off member 1, so it keeps working." },
  { link: "published", said: "Member 3 waits on what member 2's merge publishes." },
];

/**
 * Several pull requests landing in order, which is guide 2's whole subject.
 *
 * **One list, not three.** The line joining one row to the next is drawn
 * between siblings, so three lists would be three runs of one. The stylesheet
 * stages each member by its place in that one list.
 */
function MembersLanding() {
  return (
    <div className="armada-guide-figure__members">
      <JobMembers bare members={MEMBERS} joins={JOINS} completeWhen="Every member has landed." />
    </div>
  );
}
