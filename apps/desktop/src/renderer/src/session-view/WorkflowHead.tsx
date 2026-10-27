// The workflow at the head of the Job's story (`StoryColumn.tsx`): its name as a reading, and its steps beside it as glyphs, the one the Job is at lit. Where the Job's
// workflow can be changed, pressing the name opens the picker as a menu under it, and ⌥W puts the focus on it. A change applies from the next turn.

import { useRef, useState } from "react";
import { ChevronDown, Lightbulb } from "lucide-react";
import { DropdownMenu, Readout, Tooltip, type DropdownMenuEntry } from "@armada/components";
import type { SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { StepRail } from "./StatusPanel";
import { BAND_KEYS } from "./story";

const NONE = "none";
const KEY = BAND_KEYS.find((one) => one.area === "workflow")?.cap;

export function WorkflowHead({ work, acts, id, manifestId }: { work: SessionWork; acts: SessionWorkActs | undefined; id: string; manifestId: string | undefined }) {
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
  return (
    <div className="sv-workflow">
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
              <Readout>{name}</Readout>
              <ChevronDown size={12} strokeWidth={2} aria-hidden />
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
            <Readout>{name}</Readout>
          </span>
        </Tooltip>
      )}
      {proposal?.picked?.includes("workflow") !== true ? null : (
        <Tooltip label="Picked by the proposer">
          <span className="sv-flow__proposed" role="img" aria-label="Picked by the proposer">
            <Lightbulb size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      )}
      {flow === undefined ? null : <StepRail steps={flow.steps} at={flow.at} />}
    </div>
  );
}
