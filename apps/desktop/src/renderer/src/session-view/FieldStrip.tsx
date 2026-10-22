// One button for each drone of the Job, over its Field: the Job's own first, then its subagents as they started, each named and saying
// what it is doing now (`field-now.ts`). A finished subagent stays, greyed. A press frames the camera on that drone and pins it, the
// hook a card that follows a drone will use. One row that scrolls sideways and never wraps. Drawn again only when an entry changes.

import { memo } from "react";
import { Button } from "@armada/components";

import type { StripEntry } from "./field-now";

export const FieldStrip = memo(function FieldStrip({ entries, pinned, onPick }: { entries: readonly StripEntry[]; pinned: string | undefined; onPick: (key: string) => void }) {
  return (
    <div className="sv-fieldstrip" role="group" aria-label="Drones">
      {entries.map((one) => (
        <Button key={one.key} variant="ghost" size="sm" className="sv-fieldstrip__drone" aria-label={`${one.name}: ${one.word}`} aria-pressed={pinned === one.key} data-done={one.done || undefined} onClick={() => onPick(one.key)}>
          <span className="sv-fieldstrip__name">{one.name}</span>
          <span className="sv-fieldstrip__now">{one.word}</span>
        </Button>
      ))}
    </div>
  );
});
