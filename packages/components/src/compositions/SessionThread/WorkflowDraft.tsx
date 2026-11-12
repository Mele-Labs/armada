import { useState } from "react";
import { ChevronDown, ChevronRight, GitPullRequest, Signpost, Workflow } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Prose, ProseCommand, ProseRunner } from "../../primitives/Prose/Prose";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { GATE_GLYPH, GATE_NAMED, type GateKind } from "./gate-glyph";

/** One step of a proposed workflow, as its definition says it. */
export type ProposedWorkflowStep = {
  id: string;
  label: string;
  about?: string;
  /** What ends it, finishing "Ends when": `you approve the plan`. */
  endsWhen?: string;
  /** The gate that ends it, for its glyph and hue. */
  gate?: GateKind;
  asksDelivery?: boolean;
  delivers?: boolean;
  /** What the agent is told on the step. */
  instructions?: string;
};

/** A workflow a Job proposed: its id and where it would be kept, and once read, its name, its requests and its steps. */
export type ProposedWorkflow = {
  name: string;
  scope: "kit" | "repository";
  title?: string;
  forRequests?: string;
  steps?: readonly ProposedWorkflowStep[];
};

const SCOPE = {
  kit: "Every repository on this Mac",
  repository: "This repository",
} as const;

/** His own gates take his colour; the ones Fleet sees for itself take the kind they close. */
const GATE_HUE = (gate: GateKind | undefined) =>
  gate === undefined
    ? undefined
    : gate === "merged"
      ? "delivery"
      : gate === "checks_green" || gate === "jobs_ended"
        ? "gate"
        : "you";

/**
 * A proposed workflow drawn as itself: a header band with its name, id and scope, the requests it is for, then one lane
 * a step down a numbered rail, each with what ends it and what the agent is told folded under it. Only for a workflow
 * whose steps arrived; the draft card draws the definition's text otherwise.
 */
export function WorkflowDraft({ workflow }: { workflow: ProposedWorkflow }) {
  const steps = workflow.steps ?? [];
  return (
    <div className="armada-wf-draft">
      <header className="armada-wf-draft__band">
        <Workflow className="armada-wf-draft__kind" size={16} strokeWidth={2} aria-hidden />
        <p className="armada-wf-draft__name">{workflow.title ?? workflow.name}</p>
        <span className="armada-wf-draft__facts">
          <span className="armada-wf-draft__id">{workflow.name}</span>
          <span>{SCOPE[workflow.scope]}</span>
        </span>
      </header>
      {workflow.forRequests === undefined ? null : (
        <p className="armada-wf-draft__for">
          <span className="armada-wf-draft__label">For requests</span>
          <span>{workflow.forRequests}</span>
        </p>
      )}
      <ol className="armada-wf-draft__lanes" aria-label="Steps">
        {steps.map((step, at) => (
          <Lane key={step.id} step={step} n={at + 1} last={at === steps.length - 1} />
        ))}
      </ol>
    </div>
  );
}

function Lane({ step, n, last }: { step: ProposedWorkflowStep; n: number; last: boolean }) {
  const [told, setTold] = useState(false);
  const Gate = step.gate === undefined ? undefined : GATE_GLYPH[step.gate];
  const Caret = told ? ChevronDown : ChevronRight;
  return (
    <li
      className="armada-wf-lane"
      aria-label={step.label}
      data-delivers={step.delivers === true || undefined}
      data-last={last || undefined}
    >
      <span className="armada-wf-lane__rail" aria-hidden>
        <span className="armada-wf-lane__n">{n}</span>
      </span>
      <div className="armada-wf-lane__body">
        <p className="armada-wf-lane__label">{step.label}</p>
        {step.about === undefined ? null : <p className="armada-wf-lane__about">{step.about}</p>}
        {step.endsWhen === undefined ? null : (
          <p className="armada-wf-lane__ends" data-hue={GATE_HUE(step.gate)}>
            {Gate === undefined ? null : (
              <Tooltip label={GATE_NAMED[step.gate!]}>
                <span
                  className="armada-wf-lane__gate"
                  role="img"
                  aria-label={GATE_NAMED[step.gate!]}
                >
                  <Gate size={14} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            )}
            <span>Ends when {step.endsWhen}</span>
          </p>
        )}
        {step.instructions === undefined ? null : (
          <div className="armada-wf-lane__told">
            <Button size="sm" variant="ghost" aria-expanded={told} onClick={() => setTold(!told)}>
              <Caret size={14} strokeWidth={2} aria-hidden />
              What the agent is told
            </Button>
            {told ? (
              <div className="armada-wf-lane__instructions">
                <ProseRunner.Provider value={null}>
                  <ProseCommand.Provider value={null}>
                    <Prose text={step.instructions} />
                  </ProseCommand.Provider>
                </ProseRunner.Provider>
              </div>
            ) : null}
          </div>
        )}
      </div>
      <span className="armada-wf-lane__chips">
        {step.asksDelivery === true ? (
          <span className="armada-wf-lane__chip">
            <Signpost size={12} strokeWidth={2} aria-hidden />
            Asks how to deliver
          </span>
        ) : null}
        {step.delivers === true ? (
          <span className="armada-wf-lane__chip">
            <GitPullRequest size={12} strokeWidth={2} aria-hidden />
            Delivers
          </span>
        ) : null}
      </span>
    </li>
  );
}
