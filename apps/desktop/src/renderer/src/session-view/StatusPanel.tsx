// The Status area: where the Session's plan and progress are read, in the place the status ring had. The workflow's steps with the
// one the Session is at lit, the scope as paths, and the task groups with the type of subagent that takes each. The agent records
// the plan after planning with the person and it arrives with Approve; he can answer in the thread to change it.
// Edits Fleet saw are marked on the group. Soft: the steps show where the Session is and block nothing.
// Take A draws the steps as a strip over the plan. Take B draws one outline, the plan hung under the step that builds it.

import { BadgeCheck, Bot, CircleCheck, CircleDashed, CircleDot, PencilLine } from "lucide-react";
import { Button, TaskMark, Tooltip } from "@armada/components";
import type { SessionWork, WorkPlan } from "@armada/screens/src/draft/sessions";

import { SvPanel, type PanelState } from "./SvPanel";
import type { Take } from "./take";

const MARK = {
  done: { Glyph: CircleCheck, said: "Done" },
  now: { Glyph: CircleDot, said: "Now" },
  next: { Glyph: CircleDashed, said: "Next" },
} as const;

const stateOf = (i: number, at: number) => (i < at ? "done" : i === at ? "now" : "next");

function StepMark({ state }: { state: keyof typeof MARK }) {
  const { Glyph, said } = MARK[state];
  return (
    <Tooltip label={said}>
      <span className="sv-step__mark" role="img" aria-label={said}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** Take A: the steps as a strip, each a mark over its name, joined by a line. */
function Strip({ steps, at }: { steps: readonly string[]; at: number }) {
  return (
    <ol className="sv-strip" aria-label="Workflow">
      {steps.map((step, i) => {
        const state = stateOf(i, at);
        return (
          <li key={step} className="sv-step" data-state={state} aria-current={state === "now" ? "step" : undefined}>
            <StepMark state={state} />
            <span className="sv-step__name">{step}</span>
          </li>
        );
      })}
    </ol>
  );
}

function Scope({ plan }: { plan: WorkPlan }) {
  return (
    <ul className="sv-scope" aria-label="Scope">
      {plan.scope.map((path) => (
        <li key={path}>
          <code>{path}</code>
        </li>
      ))}
    </ul>
  );
}

function Groups({ plan }: { plan: WorkPlan }) {
  return (
    <>
      {plan.groups.map((group) => {
        const live = group.tasks.some((task) => task.state === "working");
        const complete = group.tasks.every((task) => task.state === "done");
        return (
          <section key={group.id} className="sv-group" data-state={complete ? "done" : live ? "live" : undefined} aria-label={group.title}>
            <header className="sv-group__head">
              <Tooltip label="Subagent type">
                <span className="sv-group__agent" role="img" aria-label={`Subagent type ${group.agent}`}>
                  <Bot size={12} strokeWidth={2} aria-hidden />
                  <code>{group.agent}</code>
                </span>
              </Tooltip>
              {group.tasks.some((task) => task.seen === true) ? (
                <Tooltip label="Edits seen">
                  <span className="sv-group__seen" role="img" aria-label="Edits seen">
                    <PencilLine size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              ) : null}
            </header>
            <h4 className="sv-group__title">{group.title}</h4>
            <ul className="sv-tasks">
              {group.tasks.map((task) => (
                <li key={task.id} className="sv-task" data-state={task.state}>
                  <TaskMark state={task.state} />
                  <span>{task.text}</span>
                </li>
              ))}
            </ul>
          </section>
        );
      })}
    </>
  );
}

/** Take B: one outline. Steps in a column; the plan hangs under the step that builds, which is the lit one while it runs. */
function Outline({ work, plan }: { work: SessionWork; plan: WorkPlan | undefined }) {
  const flow = work.workflow!;
  const builds = Math.max(flow.steps.indexOf("Build"), 0);
  return (
    <ol className="sv-outline" aria-label="Workflow">
      {flow.steps.map((step, i) => {
        const state = stateOf(i, flow.at);
        return (
          <li key={step} className="sv-step" data-state={state} data-open={(i === builds && plan !== undefined) || undefined} aria-current={state === "now" ? "step" : undefined}>
            <div className="sv-step__row">
              <StepMark state={state} />
              <span className="sv-step__name">{step}</span>
            </div>
            {i === builds && plan !== undefined ? (
              <div className="sv-step__plan">
                <Scope plan={plan} />
                <Groups plan={plan} />
              </div>
            ) : null}
          </li>
        );
      })}
    </ol>
  );
}

export function StatusPanel({ work, take, place, onApprove }: { work: SessionWork; take: Take; place: "side" | "left"; onApprove: () => void }) {
  const plan = work.plan;
  const finished = plan !== undefined && plan.approved && plan.groups.every((group) => group.tasks.every((task) => task.state === "done"));
  const state: PanelState | undefined = plan === undefined ? undefined : !plan.approved ? "asks" : finished ? "done" : "live";
  const acts =
    plan === undefined ? undefined : plan.approved ? (
      <Tooltip label="Plan approved">
        <span className="sv-panel__mark" role="img" aria-label="Plan approved">
          <BadgeCheck size={16} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    ) : (
      <Button variant="primary" size="sm" onClick={onApprove}>
        Approve
      </Button>
    );
  return (
    <SvPanel label="Status" area="status" place={place} state={state} acts={acts}>
      {work.workflow !== undefined && take === "b" ? (
        <Outline work={work} plan={plan} />
      ) : (
        <>
          {work.workflow === undefined ? null : <Strip steps={work.workflow.steps} at={work.workflow.at} />}
          {plan === undefined ? null : (
            <>
              <Scope plan={plan} />
              <Groups plan={plan} />
            </>
          )}
        </>
      )}
    </SvPanel>
  );
}
