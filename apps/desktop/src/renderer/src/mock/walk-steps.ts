// Steps a walk over the Cockpit shares. Not a walk: it lives beside `walk.ts`, since `walks/`
// is read as a list of walks.

import { deckRow, region } from "./walk";
import type { Step } from "./walk";

/** The Cockpit's list of Sessions: a keystroke has somewhere on the deck to land. */
export const dashboard = region("Jobs", { exact: true });

/**
 * Calls standing in front of a pane, put off. A call on the deck is a pane of its own, chosen from Incoming, and is never in front of
 * what a walk works in, so there is nothing to put off and no step is drawn. The count is kept so a walk reads as it was told.
 */
export function putOff(_calls: number): Step[] {
  return [];
}

/** A row of the list picked, its page on the stage with its acts. */
export function pick(title: RegExp | string, say: string): Step[] {
  return [{ press: deckRow(title), say }];
}
