// The workflow in Progress: a picker, changeable at any time, and its steps as one line of glyphs, the one the Session is at lit. Both stand in the
// panel's band, quiet, and not in its body. Soft: the steps show where the Session is and block nothing.

import { CircleCheck, CircleDashed, CircleDot, Lightbulb } from "lucide-react";
import { Inspect, InspectCard, Select, Tooltip } from "@armada/components";
import type { SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

export const MARK = {
  done: { Glyph: CircleCheck, said: "Done" },
  now: { Glyph: CircleDot, said: "Now" },
  next: { Glyph: CircleDashed, said: "Next" },
} as const;

// With no step named every step is a step to come: none is lit.
export const stateOf = (i: number, at: number | undefined) => (at === undefined || i > at ? "next" : i < at ? "done" : "now");

/** What a step reads as in its hover card: "Build · current". */
const WORD = { done: "done", now: "current", next: "next" } as const;

/** One step as its state glyph, which names itself and its state on hover or focus: "Draft · done". The rail and the spine's step nodes (`StoryParts.tsx`) draw the same. */
export function StepGlyph({ step, state }: { step: string; state: "done" | "now" | "next" }) {
  const { Glyph } = MARK[state];
  const said = `${step} · ${WORD[state]}`;
  return (
    <Inspect card={<InspectCard kind={{ Glyph, label: "Step" }} title={step} state={{ said: WORD[state].charAt(0).toUpperCase() + WORD[state].slice(1) }} />} name={said} side="below" pin={false}>
      <span className="sv-rail__mark" role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Inspect>
  );
}

/** The workflow's steps as one line of glyphs joined by a hairline; the current one is the dotted circle. Hovering or focusing a step names it and its state. `named` writes each step's name beside its glyph. */
export function StepRail({ steps, at, named = false }: { steps: readonly string[]; at: number | undefined; named?: boolean }) {
  return (
    <ol className="sv-rail" aria-label="Workflow" data-named={named || undefined}>
      {steps.map((step, i) => {
        const state = stateOf(i, at);
        return (
          <li key={step} className="sv-rail__step" data-state={state} aria-current={state === "now" ? "step" : undefined}>
            <StepGlyph step={step} state={state} />
            {named ? <span className="sv-rail__name">{step}</span> : null}
          </li>
        );
      })}
    </ol>
  );
}

const NONE = "none";

/** The workflow as a small quiet picker for the band, and None is a custom Session with no steps. A change applies from the next turn. With no picker it is the workflow's name. */
export function Flow({ work, acts, id, manifestId }: { work: SessionWork; acts: SessionWorkActs | undefined; id: string; manifestId: string | undefined }) {
  const proposal = work.proposal;
  if (proposal === undefined || acts === undefined) {
    return work.workflow === undefined ? null : (
      <Tooltip label="Workflow">
        <span className="sv-flow__name">{work.workflow.name}</span>
      </Tooltip>
    );
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
