// Skipping a workflow step: the icon in a current or coming step's band, the Undo in a skipped one's, and the band's right-press menu with the same act.
// Fleet answers the record, which moves the current step past a skipped one; the last, delivering step offers neither.

import { SkipForward, Undo2 } from "lucide-react";
import { Button, Tooltip, type DropdownMenuEntry } from "@armada/components";

import type { BandMenu } from "./SvPanel";

export const SKIP_SAID = "Skip step";
export const UNDO_SAID = "Undo skip";

/** A bare glyph to skip, as Ask for an update is; on a skipped step a worded Undo, so the way back reads without a hover. */
export function SkipStep({ skipped, onPress }: { skipped: boolean; onPress: () => void }) {
  if (skipped) {
    return (
      <Button variant="ghost" size="sm" aria-label={UNDO_SAID} onClick={onPress}>
        <Undo2 size={16} aria-hidden="true" />
        Undo
      </Button>
    );
  }
  return (
    <Tooltip label={SKIP_SAID} asChild>
      <Button variant="ghost" size="sm" iconOnly aria-label={SKIP_SAID} onClick={onPress}>
        <SkipForward size={16} aria-hidden="true" />
      </Button>
    </Tooltip>
  );
}

/** The band's menu for a step that can be skipped, or whose skip can be undone. */
export function skipMenu(skipped: boolean, onPress: () => void): BandMenu {
  const entry: DropdownMenuEntry = skipped ? { kind: "item", id: "skip", label: UNDO_SAID, icon: Undo2 } : { kind: "item", id: "skip", label: SKIP_SAID, icon: SkipForward };
  return { entries: [entry], onSelect: (chosen) => chosen === "skip" && onPress() };
}
