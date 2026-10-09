// Steps a walk over the Dashboard shares. Not a walk: it lives beside `walk.ts`, since `walks/`
// is read as a list of walks.

import { region, role, tab } from "./walk";
import type { Step, Target } from "./walk";

/** The Dashboard's one panel. */
export const dashboard = region("Dashboard");

/**
 * Put off `calls` calls standing in front of the panel. A call comes forward over the panel whatever
 * the filter, and what is behind it can still be pressed, but the pane beside the grid is under the
 * card, so a walk that works in the pane puts the cards off first.
 */
export function putOff(calls: number): Step[] {
  return Array.from({ length: calls }, (): Step => ({ key: "l", on: dashboard, say: "Put it off for later" }));
}

/** A tile picked on Active, its detail and acts in the pane beside the grid. */
export function pick(title: RegExp | string, say: string): Step[] {
  const tile: Target = role("option", title);
  return [{ press: tab("Active"), say: "Active" }, { press: tile, say }];
}
