// The plan along the Field's foot: one bar of a segment for each task, drawn as the map card's plan bar draws it (`BuildCard.tsx`), the
// groups set apart by a wider gap. A segment's tooltip is its task and its state. Large, each group is named under its stretch.
// No count is drawn beside the items it counts.

import type { CSSProperties } from "react";
import { Tooltip } from "@armada/components";

import "../cockpit/deck/build.css";
import "./field-plan.css";
import { PIECE_SAID, type FieldGroup } from "./field-plan";

export function PlanBar({ groups, titled }: { groups: readonly FieldGroup[]; titled: boolean }) {
  return (
    <div className="sv-fieldplan" role="group" aria-label="Plan" data-titled={titled || undefined}>
      {groups.map((group) => (
        <div key={group.id} className="sv-fieldplan__group" role="group" aria-label={group.title} style={{ "--tasks": group.tasks.length } as CSSProperties}>
          <ul className="bf-build__plan">
            {group.tasks.map((task) => (
              <Tooltip key={task.id} label={`${task.text} · ${PIECE_SAID[task.piece]}`} asChild>
                <li className="bf-build__seg" data-piece={task.piece} tabIndex={titled ? 0 : undefined} aria-label={`${task.text}, ${PIECE_SAID[task.piece]}`} />
              </Tooltip>
            ))}
          </ul>
          {titled ? <span className="sv-fieldplan__title">{group.title}</span> : null}
        </div>
      ))}
    </div>
  );
}
