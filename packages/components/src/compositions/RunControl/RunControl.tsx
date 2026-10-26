import { useRef, useState } from "react";
import { ChevronDown, Play, Plus } from "lucide-react";

import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import type { DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { SavedRun } from "./runs";

export * from "./runs";
export { RunPhaseButton, RunPhaseMark, RunWindowMark } from "./RunPhaseMark";

const ADD = "add-a-run";

/**
 * Run an app: the play glyph runs the last run used, and the caret beside it lists the repository's saved runs, ending on Add a way to run.
 * **The face and the caret are one bordered group of two segments on a hairline, drawn small for a list's heading and a Job's header.** `onRun` is called with the run to
 * start, and with none where the repository has none saved, which is the caller's to answer by opening the running panel on a draft.
 */
export function RunControl({
  subject,
  on,
  runs,
  last,
  onRun,
  onAdd,
}: {
  /** What the control is for, in its names: `armada`, `this Job`. */
  subject: string;
  /** Where it runs, in the face's tooltip: `main`, `this bay`. */
  on: string;
  runs: readonly SavedRun[];
  /** The run used last, by id. The face repeats it. */
  last?: string | undefined;
  onRun: (run: SavedRun | undefined) => void;
  onAdd: () => void;
}) {
  const [at, setAt] = useState<{ x: number; y: number } | undefined>();
  const caret = useRef<HTMLButtonElement>(null);
  const repeat = runs.find((one) => one.id === last) ?? runs[0];
  const entries: DropdownMenuEntry[] = [
    ...runs.map((one): DropdownMenuEntry => ({ kind: "item", id: one.id, label: one.name, detail: one.kind === "app" ? "app" : "browser", selected: one.id === repeat?.id })),
    ...(runs.length === 0 ? [] : [{ kind: "separator", id: "add-rule" } as const]),
    { kind: "item", id: ADD, label: "Add a way to run", icon: Plus },
  ];
  const open = () => {
    const box = caret.current?.getBoundingClientRect();
    if (box !== undefined) setAt({ x: box.left, y: box.bottom + 4 });
  };
  return (
    <span className="armada-run-control">
      <Tooltip label={repeat === undefined ? `Run on ${on}` : `${repeat.name} on ${on}`}>
        <button type="button" className="armada-run-control__segment" aria-label={`Run ${subject}`} onClick={() => onRun(repeat)}>
          <Play size={12} strokeWidth={2} aria-hidden="true" />
        </button>
      </Tooltip>
      <Tooltip label={`Runs for ${subject}`}>
        <button ref={caret} type="button" className="armada-run-control__segment" aria-label={`Runs for ${subject}`} aria-haspopup="menu" aria-expanded={at !== undefined} onClick={open} onKeyDown={(event) => event.key === "ArrowDown" && (event.preventDefault(), open())}>
          <ChevronDown size={12} strokeWidth={2} aria-hidden="true" />
        </button>
      </Tooltip>
      {at === undefined ? null : (
        <DropdownMenu
          triggerLabel={`Runs for ${subject}`}
          at={at}
          open
          lead
          entries={entries}
          onOpenChange={(next) => {
            if (!next) {
              setAt(undefined);
              caret.current?.focus();
            }
          }}
          onSelect={(id) => {
            if (id === ADD) onAdd();
            else onRun(runs.find((one) => one.id === id));
          }}
        />
      )}
    </span>
  );
}
