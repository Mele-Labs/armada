// The Delivery panel's route: the steps this Job's delivery will pass through as a vertical stepper, every one pending until its data arrives. A
// step's mark is the task mark of its state, with its tooltip; what the Job has made for that step (the pull request rows, the document, the mods)
// stands under it. The steps are `routeOf` (`route.ts`).

import type { ReactNode } from "react";
import { TASK_GLYPH, TaskMark, Tooltip, taskMarkSaid } from "@armada/components";

import type { RouteStep } from "./route";

export function Mark({ state }: { state: RouteStep["state"] }) {
  if (state !== "working") return <TaskMark state={state} />;
  const said = taskMarkSaid(state);
  return (
    <Tooltip asChild label={said}>
      <span className="armada-session-mark armada-task-mark" data-state={state} data-pulsing role="img" aria-label={said}>
        <TASK_GLYPH.working size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

export function DeliveryRoute({ steps, bodies }: { steps: readonly RouteStep[]; /** What stands under a step, by its key. */ bodies: Readonly<Record<string, ReactNode>> }) {
  return (
    <ol className="sv-route" aria-label="Route">
      {steps.map((step) => (
        <li key={step.key} className="sv-route__step" data-state={step.state} aria-current={step.state === "working" ? "step" : undefined}>
          <Mark state={step.state} />
          <span className="sv-route__label">{step.label}</span>
          {bodies[step.key] === undefined || bodies[step.key] === null ? null : <div className="sv-route__body">{bodies[step.key]}</div>}
        </li>
      ))}
    </ol>
  );
}
