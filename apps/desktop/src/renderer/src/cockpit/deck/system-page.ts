// Where the System tab opens: the entry last picked on the System page (`system-entry.ts`). The strip helpers below are the old five-page strip's. #2301.

import { DECK_TABS, SYSTEM_PAGES } from "@armada/shell";

import { DECK_SYSTEM_KEY, readRemembered, writeRemembered } from "../../remembered-views";
import { surfaceOfEntry } from "../../system-entries";
import { systemEntryNow } from "../../system-entry";

export const systemPageOf = (pages: readonly string[], first: string): string => {
  const kept = readRemembered(DECK_SYSTEM_KEY, (value) => value ?? "");
  return pages.includes(kept) ? kept : first;
};
export const rememberSystemPage = (surface: string): void => writeRemembered(DECK_SYSTEM_KEY, surface);

/** The pages the System strip draws: every one the window has, less what a layout or a missing source leaves off. */
export const systemPagesOf = (hidden: readonly string[]): readonly { surface: string; label: string }[] => SYSTEM_PAGES.filter((page) => !hidden.includes(page.surface));

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
