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
  Button,
  DestinationCard,
  FigureList,
  GroupShape,
  GUIDE_PLAN,
  GUIDE_PULSE,
  GUIDE_WORKFLOW,
  JobBriefSkeleton,
  NowPanel,
  PlanGroupStateMark,
  Prose,
  SkeletonText,
  Tooltip,
  WorkflowCanvas,
} from "@armada/components";
import type { Figure, NowPanelProps, PlanGroupState, WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";
import type { FromStudio } from "@armada/protocol";
import { PanelRightOpen } from "lucide-react";
import type { ReactNode } from "react";

import type { DetailTab } from "./detail-tabs";
import { litSteps } from "./draft/now";
import { useNowHidden } from "./now-hidden";
import { JobLead, type JobLeadProps } from "./JobLead";
import { studioName } from "@armada/screens/src/studio";
import type { OpenStudioFrom } from "@armada/screens/src/open-studio";

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
    /**
     * Where it stands, drawn as Plan's views draw it: `GROUP_STATE`'s glyph
     * with a tooltip naming `said`, never a word (owner, 2 Oct 2026).
     */
    state: PlanGroupState;
    said: string;
  }[];
};

export type OverviewBoardProps = {
  lead: JobLeadProps;
  /**
   * The thing the lead is about, where a person answers it: a Drone's question
   * and its answers, or the Allow / Always allow / Reject choice for a command
   * it was not given.
   *
   * **Inside the lead, because the lead is its sentence.** Under it until
   * 30 Sep 2026, when the owner read the two and saw one thing said twice:
   * *"This panel duplicates what is shown in the panel below. Why can't we
   * just have one panel?"*
   */
  waiting?: ReactNode;
  /**
   * What the lead's Approve dispatch approves, under it and over the Brief —
   * `approving.tsx`. Absent on every Job not waiting to be dispatched.
   */
  approving?: ReactNode;
  /**
   * The Job's run as its canvas, past the gate, under the lead where the
   * approval stood. **The Brief stays beside it**: it reads the approved words
   * and names the Studio the work came from, which the canvas does not.
   */
  run?: ReactNode;
  workflow?: OverviewWorkflow;
  /** Why there is no run to draw, where there is none. */
  workflowAbsent?: string;
  plan?: OverviewPlan;
  /** The `*Absent` sentences: a fact or a failure. Absent draws nothing. */
  planAbsent?: string;
  pulse: Figure[];
  pulseAbsent?: string;
  brief?: string;
  briefAbsent?: string;
  /**
   * The Studio this Job was dispatched from, and the press back to it — #1674.
   * **Both or neither**: a Job off no Studio, or off a deleted one, draws
   * nothing in the Brief's head.
   */
  fromStudio?: FromStudio;
  onOpenStudio?: OpenStudioFrom;
  /** What froze and what somebody has changed since, in one line. */
  settings: string;
  /**
   * Whether this Job's own read has yet to answer. **A card with nothing to
   * draw then stands in with its shape**, never its empty answer: "No plan has
   * been recorded" before the plan was read is a wrong answer, not a wait.
   */
  reading?: boolean;
  onOpenTab: (tab: DetailTab) => void;
  /**
   * What the Now panel beside the canvas draws: what the Job asks, what is
   * wrong, and what is running. **Absent draws no panel.** The panel hides from
   * its own head and shows again from a button beside the cards.
   */
  now?: Omit<NowPanelProps, "onHide">;
};

export function OverviewBoard({
  lead,
  waiting,
  approving,
  run,
  workflow,
  workflowAbsent,
  plan,
  planAbsent,
  pulse,
  pulseAbsent,
  brief,
  briefAbsent,
  fromStudio,
  onOpenStudio,
  settings,
  reading = false,
  onOpenTab,
  now,
}: OverviewBoardProps) {
  const [hidden, hide] = useNowHidden();
  // **Every step a Now row belongs to stays lit and the rest stand back**, for as long as the
  // panel has rows (owner, 8 Oct 2026). No step named, no dimming.
  const lit = hidden ? new Set<string>() : litSteps(now);
  const main = (
    <>
      {approving}
      {approving === undefined ? run : null}

      {/* **No Brief while the panel holds the request** (the owner, 3 Oct
          2026). The panel's field is the request, editable; a second copy
          beside it read the words as they arrived while the field moved. After
          approval the panel is gone and the Brief reads the approved words. */}
      {/* **The canvas is the whole Overview** (the owner, 4 Oct 2026): where it
          draws, every card folds into it — the Brief and its Studio onto their
          nodes, the run onto the lanes, the plan onto the groups, the live
          step's line onto its node, and a setting moved since the approval
          onto the nodes it governs, in accent. */}
      <div className="armada-overview-board__cards" data-brief={approving === undefined && run === undefined ? undefined : "in-panel"}>
        {approving !== undefined || run !== undefined ? null : (
        <>
        {/* **What the Job is for, before what it is doing.** It took the
            figures strip's place at the owner's word, 29 Sep 2026: *"maybe
            the brief should replace where the figures list is right now."*
            The one card with no destination behind it.

            **Where the work came from, named, at the head's trailing edge** —
            the owner's call of 2 Oct 2026 on #1674. One press, the header
            sentence's own: the Studio's canvas with this Job's node picked. */}
        <DestinationCard
          label="Brief"
          trailing={
            fromStudio === undefined || onOpenStudio === undefined ? undefined : (
              <Tooltip label={`Open ${studioName(fromStudio)}`}>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onOpenStudio(fromStudio.studio_id, fromStudio.node_id)}
                >
                  {studioName(fromStudio)}
                </Button>
              </Tooltip>
            )
          }
        >
          {/* The Proposer's words through `Prose`, so a brief written in
              markdown reads as its structure — the owner's ask of 1 Oct 2026.
              The sentences for an absent brief are Bridge's own and stay text. */}
          {brief === undefined && reading ? (
            <JobBriefSkeleton />
          ) : brief === undefined ? (
            briefAbsent === undefined ? null : <p className="armada-overview-board__brief">{briefAbsent}</p>
          ) : (
            // **The requester's words as the structure they carry.** An issue
            // body Fleet pastes in has headings, code and paragraphs, and drawn
            // as one `<p>` it ran together — the owner's Job 1, 1 Oct 2026.
            <div className="armada-overview-board__brief">
              <Prose text={brief} />
            </div>
          )}
        </DestinationCard>

        <DestinationCard label="Workflow" guide={GUIDE_WORKFLOW} onOpen={() => onOpenTab("workflow")}>
          {/* **The Workflow destination's own canvas, opened small** — the
              owner's call of 29 Sep: *"some workflows are not linear so the
              canvas is the best way to show it without managing two different
              components to represent workflow."* Nothing is pressable inside
              it; the card around it is the press. */}
          {workflow === undefined && reading ? (
            // The canvas's own frame, empty, at the height the run lands at.
            <div className="armada-overview-board__canvas" role="status" aria-label="Reading the run" aria-busy />
          ) : workflow === undefined ? (
            workflowAbsent === undefined ? null : (
              <p className="armada-inside__absent" role="note">
                {workflowAbsent}
              </p>
            )
          ) : (
            <div className="armada-overview-board__canvas">
              <WorkflowCanvas
                nodes={
                  lit.size === 0
                    ? workflow.nodes
                    : workflow.nodes.map((node) =>
                        node.backdrop === true || node.card.kind !== "step" || lit.has(node.id.split(":")[0] ?? node.id) ? node : { ...node, card: { ...node.card, dimmed: true } },
                      )
                }
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
          {plan === undefined && reading ? (
            <SkeletonText />
          ) : plan === undefined ? (
            planAbsent === undefined ? null : (
              <p className="armada-inside__absent" role="note">
                {planAbsent}
              </p>
            )
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
                    <PlanGroupStateMark state={group.state} says={group.said} />
                  </li>
                ))}
              </ul>
            </>
          )}
        </DestinationCard>

        <DestinationCard label="Pulse" guide={GUIDE_PULSE} onOpen={() => onOpenTab("pulse")}>
          {/* Every figure here but the machine's is the Job's own — `Checks
              running 0` before the read is a count nobody took. */}
          {reading ? (
            <SkeletonText />
          ) : pulse.length === 0 ? (
            pulseAbsent === undefined ? null : (
              <p className="armada-inside__absent" role="note">
                {pulseAbsent}
              </p>
            )
          ) : (
            <FigureList figures={pulse} column="fit" />
          )}
        </DestinationCard>

        <DestinationCard label="Settings" onOpen={() => onOpenTab("settings")}>
          <p className="armada-overview-board__brief">{settings}</p>
        </DestinationCard>
        </>
        )}
      </div>
    </>
  );
  return (
    <div className="armada-detail-tab armada-overview-board" role="tabpanel" aria-label="Overview">
      <JobLead {...lead} waiting={waiting} />
      {now === undefined ? (
        main
      ) : (
        <div className="armada-overview-board__with-now">
          <div className="armada-overview-board__main">{main}</div>
          {hidden ? (
            <Tooltip label="Show now">
              <Button variant="ghost" size="sm" aria-label="Show now" onClick={() => hide(false)}>
                <PanelRightOpen size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          ) : (
            <NowPanel {...now} onHide={() => hide(true)} />
          )}
        </div>
      )}
    </div>
  );
}
