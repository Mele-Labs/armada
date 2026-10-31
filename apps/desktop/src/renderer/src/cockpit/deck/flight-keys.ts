// Which flight control a press reaches: the tile whose key it is, wherever on the Cockpit the person is, found among the tiles drawn (`SessionTiles`,
// the deck's own). The keys themselves are read in `flight-controls.ts`; the deck's handler (`DeckHost.tsx`) decides when to ask.

import { holdsText } from "@armada/screens/src/keys";

import { keyPressed } from "./flight-controls";
import { PAUSE_KEY } from "./session-tiles";

// The tiles, the acts a Job in panes draws in its Thread head, and the buttons of the views in the Job's head (`JobViews.tsx`).
const DRAWN = ".deck-controls .deck-tile[data-key], .job-acts [data-key], button[data-view][data-key]";

/** The drawn tile whose key, a letter or a chord, this press is. */
export const flightTileFor = (event: KeyboardEvent): HTMLElement | null => [...document.querySelectorAll<HTMLElement>(DRAWN)].find((one) => keyPressed(one.dataset.key ?? "", event)) ?? null;

/**
 * The tile a press with ⌘, ⌃ or ⌥ held reaches. Not while the tiles are edited or a menu is open, and in a text box only with ⌘ or ⌃, where
 * ⌥ and ⇧ type a character.
 */
export function chordTileFor(event: KeyboardEvent): HTMLElement | null {
  const target = event.target as HTMLElement;
  if (target.closest?.("[data-flight-editing]") != null || target.closest?.('[role="menu"]') != null) return null;
  if (holdsText(event.target) && !event.metaKey && !event.ctrlKey) return null;
  return flightTileFor(event);
}

/**
 * Scrolls a tile its key reached into its row's view, so the press is seen. Only the row scrolls (`container`, which the DOM types do not
 * name yet): a folded panel hides its tiles by clipping, and would otherwise be scrolled open.
 */
export const showTile = (tile: HTMLElement) => tile.scrollIntoView({ block: "nearest", inline: "nearest", container: "nearest" } as ScrollIntoViewOptions);

/** What P presses: the list row's Pause, else the open Job's Pause tile while it wears P. With another key on it, P is left to whichever tile was given P. */
export function pauseForP(row: HTMLElement | null): HTMLElement | null {
  if (row !== null) return row;
  const tile = document.querySelector<HTMLElement>('.deck-controls__block[data-subject="session"] [data-act="pause"], .job-acts [data-act="pause"]');
  return tile?.dataset.key === PAUSE_KEY ? tile : null;
}
