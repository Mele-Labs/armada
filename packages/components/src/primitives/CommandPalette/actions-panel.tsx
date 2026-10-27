// The palette's second panel: ⌘K on a row opens what can be done to that row, in the palette, laid out as Raycast lays
// its own (the owner, 27 Oct 2026): the row's title over the acts, each act's key at its right, a rule between groups,
// and a field at the foot that narrows them. ↑ ↓ move, ↵ runs one, an act's own key runs it from the results or from
// here, and Esc goes back to the results with the query kept.

import { useEffect, useRef } from "react";
import type { LucideIcon } from "lucide-react";

import { KbdChord } from "../Kbd/Kbd";
import type { PaletteEntry } from "./CommandPalette";

export type PaletteAction = {
  id: string;
  label: string;
  icon?: LucideIcon;
  /**
   * The chord that runs it while the palette is up, in the registry's spelling: `⇧⌘F`, `⌘⌫`. `↵` on the first act
   * only draws, since ↵ already runs the row. Absent draws no key.
   */
  shortcut?: string;
  /** Starts a group: a rule is drawn above it. */
  apart?: true;
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

/** The acts whose label holds what was typed in the panel's field. */
export function narrowed(actions: readonly PaletteAction[], query: string): readonly PaletteAction[] {
  const wanted = query.trim().toLowerCase();
  return wanted === "" ? actions : actions.filter((action) => action.label.toLowerCase().includes(wanted));
}

/** Whether a key press is an act's chord. Letters are read off `code`, since ⌥ turns `key` into another character. */
export function pressedChord(shortcut: string, event: KeyboardEvent): boolean {
  const keys = Array.from(shortcut);
  const key = keys[keys.length - 1];
  if (key === undefined || key === "↵") return false;
  const held = new Set(keys.slice(0, -1));
  if (held.has("⌘") !== event.metaKey || held.has("⇧") !== event.shiftKey || held.has("⌥") !== event.altKey || held.has("⌃") !== event.ctrlKey) return false;
  if (key === "⌫") return event.key === "Backspace";
  return /^[A-Z]$/.test(key) ? event.code === `Key${key}` : event.key === key;
}

export function ActionsPanel({
  entry,
  actions,
  at,
  query,
  active,
  onQuery,
  onKey,
  onEnter,
  onChoose,
}: {
  entry: PaletteEntry;
  /** Already narrowed by `query`. */
  actions: readonly PaletteAction[];
  at: number;
  query: string;
  active: React.Ref<HTMLDivElement>;
  onQuery: (query: string) => void;
  onKey: (event: React.KeyboardEvent<HTMLInputElement>) => void;
  onEnter: (at: number) => void;
  onChoose: (action: PaletteAction) => void;
}) {
  const field = useRef<HTMLInputElement>(null);
  // Focus moves here while the panel is up, as Raycast's does, and back to the palette's field when it goes.
  useEffect(() => field.current?.focus(), []);
  return (
    <div className="armada-palette__actions">
      <div className="armada-palette__actions-title">{entry.label}</div>
      <div className="armada-palette__actions-list" role="listbox" aria-label={`Actions on ${entry.label}`}>
        {actions.map((action, i) => {
          const Glyph = action.icon;
          const dormant = action.dormant !== undefined;
          const classes = ["armada-palette__row", i === at ? "armada-palette__row--active" : "", action.destructive === true ? "armada-palette__row--danger" : "", dormant ? "armada-palette__row--dormant" : ""].filter((one) => one !== "").join(" ");
          return (
            <div key={action.id} className="armada-palette__actions-item">
              {action.apart === true && query === "" && i > 0 ? <div className="armada-palette__rule" role="separator" /> : null}
              <div
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
                <span className="armada-palette__label">
                  <span className="armada-palette__title">{action.label}</span>
                  {dormant ? <span className="armada-palette__subtitle">{action.dormant}</span> : null}
                </span>
                {action.shortcut === undefined ? null : <KbdChord keys={Array.from(action.shortcut)} aria-hidden />}
              </div>
            </div>
          );
        })}
        {actions.length === 0 ? <p className="armada-palette__empty">{`Nothing matches “${query}”.`}</p> : null}
      </div>
      <input
        ref={field}
        className="armada-palette__actions-field"
        type="text"
        role="combobox"
        aria-expanded="true"
        aria-label="Search for actions"
        aria-activedescendant={actions[at] === undefined ? undefined : actionRowId(actions[at])}
        placeholder="Search for actions"
        value={query}
        onChange={(event) => onQuery(event.target.value)}
        onKeyDown={onKey}
      />
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
