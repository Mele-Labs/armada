// The palette's second panel: ⌘K on a row opens what can be done to that row, in the palette. ↑ ↓ move, ↵ runs one,
// Esc goes back to the results with the query kept.

import type { LucideIcon } from "lucide-react";

import { KbdChord } from "../Kbd/Kbd";
import type { PaletteEntry } from "./CommandPalette";

export type PaletteAction = {
  id: string;
  label: string;
  icon?: LucideIcon;
  /** Confirms before it runs, by the palette's `onConfirm`. */
  destructive?: boolean;
  /** Why it cannot run here, in a few words. */
  dormant?: string;
  /** Back to the results afterwards, with the palette still up: Pin and Unpin. */
  stays?: true;
};

export const actionRowId = (action: PaletteAction): string => `armada-palette-action-${action.id}`;

/** The id an action on a row carries to `onConfirm`, which takes an entry: the row's, a bar, and the action's. */
export const actionEntryId = (entry: PaletteEntry, action: PaletteAction): string => `${entry.id}|${action.id}`;

export function ActionsPanel({
  entry,
  actions,
  at,
  active,
  onEnter,
  onChoose,
}: {
  entry: PaletteEntry;
  actions: readonly PaletteAction[];
  at: number;
  active: React.Ref<HTMLDivElement>;
  onEnter: (at: number) => void;
  onChoose: (action: PaletteAction) => void;
}) {
  return (
    <div className="armada-palette__actions" role="listbox" aria-label={`Actions on ${entry.label}`}>
      {actions.map((action, i) => {
        const Glyph = action.icon;
        const dormant = action.dormant !== undefined;
        const classes = ["armada-palette__row", i === at ? "armada-palette__row--active" : "", action.destructive === true ? "armada-palette__row--danger" : "", dormant ? "armada-palette__row--dormant" : ""].filter((one) => one !== "").join(" ");
        return (
          <div
            key={action.id}
            ref={i === at ? active : undefined}
            id={actionRowId(action)}
            role="option"
            aria-selected={i === at}
            aria-disabled={dormant ? true : undefined}
            className={classes}
            onMouseEnter={() => onEnter(i)}
            onClick={() => onChoose(action)}
          >
            <span className="armada-palette__glyph">{Glyph === undefined ? null : <Glyph size={16} strokeWidth={2} aria-hidden />}</span>
            <span className="armada-palette__label">{action.label}</span>
            {dormant ? <span className="armada-palette__dormant">{action.dormant}</span> : null}
          </div>
        );
      })}
    </div>
  );
}

/** The keys the panel answers, along the palette's foot while it is up. */
export function ActionKeys() {
  return (
    <div className="armada-palette__keys-foot" aria-hidden>
      <span>Run <KbdChord keys={["↵"]} /></span>
      <span>Back <KbdChord keys={["Esc"]} /></span>
    </div>
  );
}

/** `⌘K Actions`, which the foot adds while the row under the cursor has any. */
export function ActionsKey() {
  return <span>Actions <KbdChord keys={["⌘", "K"]} /></span>;
}

/** The foot of a plain result list whose row under the cursor has acts. */
export function ResultKeys() {
  return (
    <div className="armada-palette__keys-foot" aria-hidden>
      <span>Open <KbdChord keys={["↵"]} /></span>
      <ActionsKey />
    </div>
  );
}
