// The workflow at the head of the Job's story (`StoryColumn.tsx`), a band of its own: the name at reading size with its picker, then the steps under it. Two takes of the
// steps, chosen by `?take=` in the mock: `labelled` writes each step's name along the band, `glyphs` draws larger glyphs with the current step's name beside them. Where the
// Job's workflow can be changed, pressing the name opens the picker as a menu under it, and ⌥W puts the focus on it. A change applies from the next turn.

import { useRef, useState } from "react";
import { ChevronDown, Lightbulb } from "lucide-react";
import { DropdownMenu, Tooltip, type DropdownMenuEntry } from "@armada/components";
import type { SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { MARK, StepRail, stateOf } from "./StatusPanel";
import { BAND_KEYS } from "./story";

const NONE = "none";
export type StepTake = "labelled" | "glyphs";

/** The take of the stepper being looked at: labelled unless the page asks for glyphs. */
export const stepTakeOf = (search: string = typeof window === "undefined" ? "" : window.location.search): StepTake => (new URLSearchParams(search).get("take") === "glyphs" ? "glyphs" : "labelled");

/** Each step named along the band: done ones checked, the current one lit, the ones to come dim. */
function LabelledSteps({ steps, at }: { steps: readonly string[]; at: number | undefined }) {
  return (
    <ol className="sv-steps" aria-label="Workflow">
      {steps.map((step, i) => {
        const state = stateOf(i, at);
        const { Glyph, said } = MARK[state];
        return (
          <li key={step} className="sv-steps__step" data-state={state} aria-current={state === "now" ? "step" : undefined}>
            <Tooltip asChild label={said}>
              <span className="sv-steps__mark" role="img" aria-label={said}>
                <Glyph size={14} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
            <span className="sv-steps__name">{step}</span>
          </li>
        );
      })}
    </ol>
  );
}

/** Larger glyphs, each naming its step on hover, and the current step written beside them. */
function GlyphSteps({ steps, at }: { steps: readonly string[]; at: number | undefined }) {
  const current = at === undefined ? undefined : steps[at];
  return (
    <div className="sv-steps-glyphs">
      <StepRail steps={steps} at={at} size={16} />
      {current === undefined ? null : <span className="sv-steps-glyphs__now">{current}</span>}
    </div>
  );
}

const KEY = BAND_KEYS.find((one) => one.area === "workflow")?.cap;

export function WorkflowHead({ work, acts, id, manifestId }: { work: SessionWork; acts: SessionWorkActs | undefined; id: string; manifestId: string | undefined; }) {
  const proposal = work.proposal;
  const flow = work.workflow;
  const button = useRef<HTMLButtonElement | null>(null);
  const [at, setAt] = useState<{ x: number; y: number } | undefined>(undefined);
  const choices = acts === undefined ? [] : acts.workflows.filter((one) => one.manifestId === undefined || manifestId === undefined || one.manifestId === manifestId);
  const chosen = proposal?.workflow === undefined ? undefined : choices.find((one) => one.id === proposal.workflow);
  const name = chosen?.name ?? flow?.name ?? "None";
  const entries: DropdownMenuEntry[] = [
    { kind: "item", id: NONE, label: "None", selected: proposal?.workflow === undefined },
    ...choices.map((one): DropdownMenuEntry => ({ kind: "item", id: one.id, label: one.name, selected: proposal?.workflow === one.id })),
  ];
  // The tooltip wraps the button and takes its ref, so the button is kept from the press that opened the menu.
  const openMenu = (from: HTMLButtonElement) => {
    button.current = from;
    const box = from.getBoundingClientRect();
    setAt({ x: box.left, y: box.bottom + 4 });
  };
  const picking = proposal !== undefined && acts !== undefined;
  const take = stepTakeOf();
  return (
    <div className="sv-workflow" data-take={take}>
      <div className="sv-workflow__top">
      {picking ? (
        <>
          <Tooltip asChild label="Workflow" shortcut={KEY}>
            <button
              type="button"
              className="sv-workflow__pick"
              data-workflow-pick=""
              aria-haspopup="menu"
              aria-expanded={at !== undefined}
              aria-label={`Workflow ${name}`}
              onMouseDown={(event) => at !== undefined && event.stopPropagation()}
              onClick={(event) => (at === undefined ? openMenu(event.currentTarget) : setAt(undefined))}
              onKeyDown={(event) => {
                if (event.key !== "ArrowDown" || at !== undefined) return;
                event.preventDefault();
                openMenu(event.currentTarget);
              }}
            >
              <span className="sv-workflow__title">{name}</span>
              <ChevronDown size={14} strokeWidth={2} aria-hidden />
            </button>
          </Tooltip>
          <DropdownMenu
            triggerLabel="Workflow"
            entries={entries}
            at={at ?? { x: 0, y: 0 }}
            open={at !== undefined}
            onOpenChange={(next) => {
              if (next) return;
              setAt(undefined);
              button.current?.focus();
            }}
            onSelect={(pick) => acts.choose(id, pick === NONE ? null : pick)}
          />
        </>
      ) : (
        <Tooltip asChild label="Workflow">
          <span className="sv-workflow__name">
            <span className="sv-workflow__title">{name}</span>
          </span>
        </Tooltip>
      )}
      {proposal?.picked?.includes("workflow") !== true ? null : (
        <Tooltip label="Picked by the proposer">
          <span className="sv-flow__proposed" role="img" aria-label="Picked by the proposer">
            <Lightbulb size={14} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      )}
      </div>
      {flow === undefined ? null : take === "glyphs" ? <GlyphSteps steps={flow.steps} at={flow.at} /> : <LabelledSteps steps={flow.steps} at={flow.at} />}
    </div>
  );
}
