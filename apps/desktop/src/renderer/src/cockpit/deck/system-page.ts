// The System page last used, which the System tab opens on. Kit where none has been. #2301.

import { DECK_TABS, SURFACE, SYSTEM_PAGES } from "@armada/shell";

import { DECK_SYSTEM_KEY, readRemembered, writeRemembered } from "../../remembered-views";

export const systemPageOf = (pages: readonly string[], first: string): string => {
  const kept = readRemembered(DECK_SYSTEM_KEY, (value) => value ?? "");
  return pages.includes(kept) ? kept : first;
};
export const rememberSystemPage = (surface: string): void => writeRemembered(DECK_SYSTEM_KEY, surface);

/** The pages the System strip draws: every one the window has, less what a layout or a missing source leaves off. */
export const systemPagesOf = (hidden: readonly string[]): readonly { surface: string; label: string }[] => SYSTEM_PAGES.filter((page) => !hidden.includes(page.surface));

/**
 * Where a tab's press, or its digit, goes: the surface itself, and for System the page last used there, or the first the strip
 * still draws. Kit stands in for the tab in `DECK_TABS`, so it is the one surface that is not what it names.
 */
export function opensFor(surface: string, hidden: readonly string[]): string {
  const system = DECK_TABS.find((tab) => tab.id === "system")!.surface;
  if (surface !== system) return surface;
  const pages = systemPagesOf(hidden).map((page) => page.surface);
  return systemPageOf(pages, pages[0] ?? SURFACE.kit);
}

/** What the window leaves off its places: what nothing serves yet, and what a layout hides. The palette still reaches it by name. */
export const placesOff = (hidden: readonly string[], layout: readonly { id: string; visible: boolean }[]): readonly string[] => [
  ...hidden,
  ...layout.filter((one) => !one.visible).map((one) => one.id),
];
