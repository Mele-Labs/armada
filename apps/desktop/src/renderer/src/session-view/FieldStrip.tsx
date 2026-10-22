// The drones of the Job over its Field, as one menu (the owner, 22 Oct 2026: a strip of buttons cut off in the side panel). The trigger
// names the drone the card follows and what it is doing (`field-now.ts`): the pinned one, else the newest writer, else the Job's own.
// The menu lists Newest writer first, then the Job's drone and its subagents as they started, a finished one greyed. Picking a drone
// pins it and frames the camera on it; Newest writer lets go. Drawn again only when what it shows changes.

import { memo } from "react";
import { DropdownMenu, type DropdownMenuEntry } from "@armada/components";

import type { StripEntry } from "./field-now";

/** The item that lets go of the pin. A drone's key starts `session:`, so none is this. */
const NEWEST = "newest";

export const FieldStrip = memo(function FieldStrip({ entries, pinned, following, onPick }: { entries: readonly StripEntry[]; pinned: string | undefined; following: string | undefined; onPick: (key: string | undefined) => void }) {
  const shown = entries.find((one) => one.key === pinned) ?? entries.find((one) => one.key === following) ?? entries[0];
  const items: DropdownMenuEntry[] = [
    { kind: "item", id: NEWEST, label: "Newest writer", selected: pinned === undefined },
    { kind: "separator", id: "rule" },
    ...entries.map((one): DropdownMenuEntry => ({ kind: "item", id: one.key, label: one.name, detail: one.word, muted: one.done, selected: pinned === one.key })),
  ];
  return (
    <div className="sv-fieldstrip" role="group" aria-label="Drones">
      {shown === undefined ? null : (
        <DropdownMenu
          align="start"
          portal
          triggerLabel={shown.name}
          triggerDetail={shown.word}
          triggerAriaLabel={`${shown.name}: ${shown.word}`}
          triggerTooltip="Drones"
          entries={items}
          onSelect={(id) => onPick(id === NEWEST ? undefined : id)}
        />
      )}
    </div>
  );
});
