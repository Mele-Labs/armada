// Overview: one lead, a strip of figures, and a card per destination.
//
// **A quick view of everything, and a door to each of them.** The owner's
// note of 29 Sep 2026, left on the arrangement this replaced: *"Basically
// what you would expect on an overview. A quick view of everything that lets
// you quickly get a picture of what's going on, but you can jump to the tabs
// to get way more details. We don't need a lot of text or data, just enough
// to show what's happening."*
//
// What it replaced was `InsideAJob` — the run tree, the plan well, the
// pointers and the step inspector, which is the Job's detail rather than its
// state. The decision is *Overview leads with what releases the most*, 29 Sep
// 2026, in the decisions register.

import {
  Badge,
  DestinationCard,
  FigureList,
  GroupShape,
  GUIDE_PLAN,
  GUIDE_PULSE,
  GUIDE_WORKFLOW,
  WorkflowCanvas,
} from "@armada/components";
import type { Figure, WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";
import type { LucideIcon } from "lucide-react";

import type { DetailTab } from "./detail-tabs";
import { JobLead, type JobLeadProps } from "./JobLead";

/** The run, as the Workflow destination's own canvas draws it. */
export type OverviewWorkflow = {
  nodes: readonly WorkflowCanvasNode[];
  edges: readonly WorkflowCanvasEdge[];
  label: string;
  opensOn?: readonly (readonly string[])[];
};

export type OverviewPlan = {
  /** The tasks a Drone is on this second, in plan order. Empty between groups. */
  working: readonly { id: string; title: string }[];
  /** One line per group: how it runs, and where it stands. */
  /**
   * One line per group: how it runs, where it stands, and the hue that state
   * carries. **`token` is the state machine's own** — `GROUP_STATE_WORDS` —
   * so `retrying` reads amber and `failed` red without this file knowing which
   * states are bad.
   */
  groups: readonly {
    ordinal: number;
    /** `2 tasks` — the count alone. How they run is drawn, not named. */
    tasks: string;
    /** How many, and whether together: what `GroupShape` draws. */
    count: number;
    concurrent: boolean;
    /** What the drawing says to somebody who cannot see it. */
    shapeLabel: string;
    /** The state's word, its status stem and its glyph, all `GROUP_STATE`'s. */
    said: string;
    status: string;
    icon: LucideIcon;
  }[];
};

export type OverviewBoardProps = {
  lead: JobLeadProps;
  workflow?: OverviewWorkflow;
  /** Why there is no run to draw, where there is none. */
  workflowAbsent?: string;
  plan?: OverviewPlan;
  planAbsent?: string;
  pulse: Figure[];
  pulseAbsent?: string;
  brief?: string;
  briefAbsent?: string;
  /** What froze and what somebody has changed since, in one line. */
  settings: string;
  onOpenTab: (tab: DetailTab) => void;
};

export function OverviewBoard({
  lead,
  workflow,
  workflowAbsent,
  plan,
  planAbsent,
  pulse,
  pulseAbsent,
  brief,
  briefAbsent,
  settings,
  onOpenTab,
}: OverviewBoardProps) {
  return (
    <div className="armada-detail-tab armada-overview-board" role="tabpanel" aria-label="Overview">
      <JobLead {...lead} />

      <div className="armada-overview-board__cards">
        {/* **What the Job is for, before what it is doing.** It took the
            figures strip's place at the owner's word, 29 Sep 2026: *"maybe
            the brief should replace where the figures list is right now."*
            The one card with no destination behind it. */}
        <DestinationCard label="Brief">
          <p className="armada-overview-board__brief">{brief ?? briefAbsent ?? "No brief was written."}</p>
        </DestinationCard>

        <DestinationCard label="Workflow" guide={GUIDE_WORKFLOW} onOpen={() => onOpenTab("workflow")}>
          {/* **The Workflow destination's own canvas, opened small** — the
              owner's call of 29 Sep: *"some workflows are not linear so the
              canvas is the best way to show it without managing two different
              components to represent workflow."* Nothing is pressable inside
              it; the card around it is the press. */}
          {workflow === undefined ? (
            <p className="armada-inside__absent" role="note">
              {workflowAbsent ?? "This Job's frozen workflow has no steps."}
            </p>
          ) : (
            <div className="armada-overview-board__canvas">
              <WorkflowCanvas
                nodes={workflow.nodes}
                edges={workflow.edges}
                label={workflow.label}
                {...(workflow.opensOn === undefined ? {} : { opensOn: workflow.opensOn })}
              />
            </div>
          )}
        </DestinationCard>

        {/* **No count in the head.** The bar draws every task and the list
            draws every group, so `5 of 8 done · 4 groups` was the number
            beside the things it counts — hard rule 7, `design-system.md`.
            `said` stays as the bar's own label, where the items are not. */}
        <DestinationCard label="Plan" guide={GUIDE_PLAN} onOpen={() => onOpenTab("plan")}>
          {plan === undefined ? (
            <p className="armada-inside__absent" role="note">
              {planAbsent ?? "No plan has been recorded."}
            </p>
          ) : (
            <>
              {/* **What is being worked, then the plan's shape** — the owner
                  asked for both on 29 Sep 2026, against a card that drew a
                  bar and nothing else.

                  **No task bar over them.** Eight grey marks said nothing the
                  group rows do not say better, and unlabelled they read as
                  decoration — he asked what they were. */}
              {plan.working.length === 0 ? null : (
                <dl className="armada-overview-board__working">
                  <dt>Working now</dt>
                  {plan.working.map((task) => (
                    <dd key={task.id}>
                      <span className="armada-overview-board__task-id">{task.id}</span>
                      {task.title}
                    </dd>
                  ))}
                </dl>
              )}
              <ul className="armada-overview-board__groups">
                {plan.groups.map((group) => (
                  // **A row per group, the owner's sketch of 29 Sep 2026.**
                  // Its name, how much work it holds, a drawing of how that
                  // work runs, and where it stands — with a rule between, so
                  // four groups read as four things rather than a list.
                  //
                  // The shape is drawn rather than named. Three passes tried
                  // words for it and each was a label for something a reader
                  // pictures anyway.
                  <li key={group.ordinal}>
                    <p className="armada-overview-board__group-name">Group {group.ordinal}</p>
                    <p className="armada-overview-board__tasks">{group.tasks}</p>
                    <GroupShape
                      tasks={group.count}
                      concurrent={group.concurrent}
                      label={group.shapeLabel}
                    />
                    <Badge status={group.status} icon={group.icon}>
                      {group.said}
                    </Badge>
                  </li>
                ))}
              </ul>
            </>
          )}
        </DestinationCard>

        <DestinationCard label="Pulse" guide={GUIDE_PULSE} onOpen={() => onOpenTab("pulse")}>
          {pulse.length === 0 ? (
            <p className="armada-inside__absent" role="note">
              {pulseAbsent ?? "Nothing has been read from this machine yet."}
            </p>
          ) : (
            <FigureList figures={pulse} column="fit" />
          )}
        </DestinationCard>

        <DestinationCard label="Settings" onOpen={() => onOpenTab("settings")}>
          <p className="armada-overview-board__brief">{settings}</p>
        </DestinationCard>
      </div>
    </div>
  );
}
