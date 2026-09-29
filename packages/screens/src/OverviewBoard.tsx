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
// state. `.claude/decisions/2026-09-29-overview-leads-with-what-releases-the-most.md`.

import { ChevronRight } from "lucide-react";
import type { ReactNode } from "react";

import { Card, CardContent, FigureList, StepBar, WorkflowCanvas } from "@armada/components";
import type { Figure, TaskBarSegment, WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";

import type { DetailTab } from "./detail-tabs";
import { Eyebrow } from "./InsideAJob";
import { JobLead, type JobLeadProps } from "./JobLead";

/** The run, as the Workflow destination's own canvas draws it. */
export type OverviewWorkflow = {
  nodes: readonly WorkflowCanvasNode[];
  edges: readonly WorkflowCanvasEdge[];
  label: string;
  opensOn?: readonly (readonly string[])[];
};

export type OverviewPlan = {
  /** One segment per task not dropped, in plan order. */
  segments: readonly TaskBarSegment[];
  /** `6 of 8 tasks done · 4 groups` — the caller's sentence, never composed here. */
  said: string;
};

export type OverviewBoardProps = {
  lead: JobLeadProps;
  /** Turns, spend and files — the Pulse band's own strip, not a second one. */
  figures: Figure[];
  workflow?: OverviewWorkflow;
  /** Why there is no run to draw, where there is none. */
  workflowAbsent?: string;
  plan?: OverviewPlan;
  planAbsent?: string;
  pulse: Figure[];
  pulseAbsent?: string;
  brief?: string;
  briefAbsent?: string;
  onOpenTab: (tab: DetailTab) => void;
};

/**
 * One card, and the destination it opens.
 *
 * **The whole card is the control**, because the card is a summary of one
 * destination and there is nothing else on it to press. A chevron says so
 * without a second word.
 */
function DestinationCard({
  tab,
  label,
  onOpenTab,
  children,
}: {
  tab: DetailTab;
  label: string;
  onOpenTab: (tab: DetailTab) => void;
  children: ReactNode;
}) {
  return (
    <Card className="armada-overview-board__card">
      <button
        type="button"
        className="armada-overview-board__door"
        onClick={() => onOpenTab(tab)}
        aria-label={`Open ${label}`}
      >
        <Eyebrow>{label}</Eyebrow>
        <ChevronRight size={14} strokeWidth={2} aria-hidden />
      </button>
      <CardContent>{children}</CardContent>
    </Card>
  );
}

export function OverviewBoard({
  lead,
  figures,
  workflow,
  workflowAbsent,
  plan,
  planAbsent,
  pulse,
  pulseAbsent,
  brief,
  briefAbsent,
  onOpenTab,
}: OverviewBoardProps) {
  return (
    <div className="armada-detail-tab armada-overview-board" role="tabpanel" aria-label="Overview">
      <JobLead {...lead} />
      {figures.length === 0 ? null : <FigureList figures={figures} column="strip" />}

      <div className="armada-overview-board__cards">
        <DestinationCard tab="workflow" label="Workflow" onOpenTab={onOpenTab}>
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

        <DestinationCard tab="plan" label="Plan" onOpenTab={onOpenTab}>
          {plan === undefined ? (
            <p className="armada-inside__absent" role="note">
              {planAbsent ?? "No plan has been recorded."}
            </p>
          ) : (
            <>
              <StepBar tasks={plan.segments} label={plan.said} />
              <p className="armada-overview-board__said">{plan.said}</p>
            </>
          )}
        </DestinationCard>

        <DestinationCard tab="pulse" label="Pulse" onOpenTab={onOpenTab}>
          {pulse.length === 0 ? (
            <p className="armada-inside__absent" role="note">
              {pulseAbsent ?? "Nothing has been read from this machine yet."}
            </p>
          ) : (
            <FigureList figures={pulse} column="fit" />
          )}
        </DestinationCard>

        {/* **The one card that is not a door.** Nothing else says what the Job
            is for, and there is no destination to send a reader to. */}
        <Card className="armada-overview-board__card">
          <CardContent>
            <Eyebrow>Brief</Eyebrow>
            <p className="armada-overview-board__brief">{brief ?? briefAbsent ?? "No brief was written."}</p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
