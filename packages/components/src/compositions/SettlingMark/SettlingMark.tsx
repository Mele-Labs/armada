import { TextCursor } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A field on a proposing Job the proposer has not written yet: a blinking
 * caret, the owner's pick of 3 Oct 2026 (`text-cursor`, group `Proposing` in
 * `packages/icons/icons/`).
 *
 * **It stands in the cell, and the cell's own heading names it.** The tooltip
 * says the field is still being settled, which is the state the caret is.
 * Under reduced motion the caret holds still and the cell still reads as
 * unsettled, which is why nothing else is lost.
 */
export function SettlingMark({ field }: { field: string }) {
  const said = `${field}, still being settled`;
  return (
    <Tooltip asChild label={said}>
      <span className="armada-settling-mark" role="img" aria-label={said}>
        <TextCursor size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}
