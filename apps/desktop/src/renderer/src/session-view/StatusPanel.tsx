// The Status area: where the Session's progress is read, in the place the status ring had. The workflow as a picker, changeable at any time,
// over its steps down a column so a workflow of twelve reads as well as one of four, with the step the
// Session is at lit and kept in view. Soft: the steps show where the Session is and block nothing. The plan is a panel of its own.

import { useEffect, useRef } from "react";
import { CircleCheck, CircleDashed, CircleDot, Lightbulb } from "lucide-react";
import { Select, Tooltip } from "@armada/components";
import type { SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { SvPanel } from "./SvPanel";

export const MARK = {
  done: { Glyph: CircleCheck, said: "Done" },
  now: { Glyph: CircleDot, said: "Now" },
  next: { Glyph: CircleDashed, said: "Next" },
} as const;

// With no step named every step is a step to come: none is lit.
export const stateOf = (i: number, at: number | undefined) => (at === undefined || i > at ? "next" : i < at ? "done" : "now");

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

/** Keeps the step the Session is at in view: a long workflow scrolls to it and the steps far from it stay out of sight. */
function useKeepNow(at: number | undefined) {
  const list = useRef<HTMLOListElement>(null);
  useEffect(() => {
    const now = list.current?.querySelector<HTMLElement>('[data-state="now"]');
    const scroller = now?.closest<HTMLElement>(".sv-steps, .sv-panel__body");
    if (now === null || now === undefined || scroller === null || scroller === undefined) return;
    // Two rows above the lit one are kept, from a whole row: the list never rests with its first row cut off under the band.
    const lead = now.previousElementSibling?.previousElementSibling ?? now.previousElementSibling ?? now;
    scroller.scrollTop += lead.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
  }, [at]);
  return list;
}

/** The steps as a list, scrolling to the lit one. */
function Steps({ steps, at }: { steps: readonly string[]; at: number | undefined }) {
  const list = useKeepNow(at);
  return (
    <div className="sv-steps">
      <ol ref={list} className="sv-outline" aria-label="Workflow">
        {steps.map((step, i) => {
          const state = stateOf(i, at);
          return (
            <li key={step} className="sv-step" data-state={state} aria-current={state === "now" ? "step" : undefined}>
              <div className="sv-step__row">
                <StepMark state={state} />
                <span className="sv-step__name">{step}</span>
              </div>
            </li>
          );
        })}
      </ol>
    </div>
  );
}

/** The workflow's steps as one line: a glyph each, joined by a hairline, with only the step the Session is at named. The rest are named by their tooltips. */
export function StepRail({ steps, at }: { steps: readonly string[]; at: number | undefined }) {
  return (
    <ol className="sv-rail" aria-label="Workflow">
      {steps.map((step, i) => {
        const state = stateOf(i, at);
        const { Glyph, said } = MARK[state];
        return (
          <li key={step} className="sv-rail__step" data-state={state} aria-current={state === "now" ? "step" : undefined}>
            <Tooltip label={`${step}, ${said.toLowerCase()}`}>
              <span className="sv-rail__mark" role="img" aria-label={`${step}, ${said.toLowerCase()}`}>
                <Glyph size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
            {state === "now" ? <span className="sv-rail__name">{step}</span> : null}
          </li>
        );
      })}
    </ol>
  );
}

const NONE = "none";

/** The workflow as a picker, and None is a custom Session with no steps. A change applies from the next turn. */
export function Flow({ work, acts, id, manifestId }: { work: SessionWork; acts: SessionWorkActs | undefined; id: string; manifestId: string | undefined }) {
  const proposal = work.proposal;
  if (proposal === undefined || acts === undefined) {
    return work.workflow === undefined ? null : <h4 className="sv-flow__name">{work.workflow.name}</h4>;
  }
  return (
    <div className="sv-flow">
      <Select aria-label="Workflow" value={proposal.workflow ?? NONE} onChange={(event) => acts.choose(id, event.target.value === NONE ? null : event.target.value)}>
        <option value={NONE}>None</option>
        {acts.workflows
          .filter((one) => one.manifestId === undefined || manifestId === undefined || one.manifestId === manifestId)
          .map((one) => (
          <option key={one.id} value={one.id}>
            {one.name}
          </option>
        ))}
      </Select>
      {proposal.picked?.includes("workflow") === true ? (
        <Tooltip label="Picked by the proposer">
          <span className="sv-flow__proposed" role="img" aria-label="Picked by the proposer">
            <Lightbulb size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      ) : null}
    </div>
  );
}

export function StatusPanel({ work, place, id, manifestId, acts }: { work: SessionWork; place: "side" | "left"; id: string; manifestId?: string | undefined; acts: SessionWorkActs | undefined }) {
  return (
    <SvPanel label="Status" area="status" place={place}>
      <Flow work={work} acts={acts} id={id} manifestId={manifestId} />
      {work.workflow === undefined ? null : <Steps steps={work.workflow.steps} at={work.workflow.at} />}
    </SvPanel>
  );
}
