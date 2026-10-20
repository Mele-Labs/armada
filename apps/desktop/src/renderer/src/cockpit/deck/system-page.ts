// Where the System tab opens: the entry last picked on the System page (`system-entry.ts`). #2301.

import { DECK_TABS } from "@armada/shell";

import { surfaceOfEntry } from "../../system-entries";
import { systemEntryNow } from "../../system-entry";

/**
 * Where a tab's press, or its digit, goes: the surface itself, and for System the old surface that stands in for the entry last
 * picked there (`system-entries.ts`). Kit stands in for the tab in `DECK_TABS`, so it is the one surface that is not what it names.
 */
export function opensFor(surface: string, _hidden: readonly string[]): string {
  const system = DECK_TABS.find((tab) => tab.id === "system")!.surface;
  return surface !== system ? surface : surfaceOfEntry(systemEntryNow());
}

/** What the window leaves off its places: what nothing serves yet, and what a layout hides. The palette still reaches it by name. */
export const placesOff = (hidden: readonly string[], layout: readonly { id: string; visible: boolean }[]): readonly string[] => [
  ...hidden,
  ...layout.filter((one) => !one.visible).map((one) => one.id),
];
