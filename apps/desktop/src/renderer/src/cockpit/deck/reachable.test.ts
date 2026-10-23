// No page is reachable only by the rail. The rail is gone, so every page in the roster must be reached from a tab, the System strip,
// a link on the deck or the palette by name, and this walks the roster to prove it. #2301.
//
// The palette's rows are the roster itself (`App` hands `SURFACES` to it), so what is asserted for `⌘K` is that the page is in the
// roster; the walks `checks-with-their-logs`, `the-merge-line` and `a-pool-of-worktree-slots` press through it.

import { describe, expect, it } from "vitest";
import { DECK_TABS, SURFACE, SURFACES, SYSTEM_PAGES } from "@armada/shell";

import { DECK_LINK } from "./links";

type Way = "tab" | "system" | "link" | "palette";

function ways(surface: string): Way[] {
  return [
    ...(DECK_TABS.some((tab) => tab.id !== "system" && tab.surface === surface) ? (["tab"] as const) : []),
    ...(SYSTEM_PAGES.some((page) => page.surface === surface) ? (["system"] as const) : []),
    ...(Object.values(DECK_LINK).includes(surface as never) ? (["link"] as const) : []),
    ...(SURFACES.some((one) => one.id === surface) ? (["palette"] as const) : []),
  ];
}

describe("every page in the roster", () => {
  it("is reached from a tab, a System entry, a deck link or the palette, and from more than the palette where it has a place", () => {
    for (const surface of SURFACES) {
      const found = ways(surface.id);
      expect(found, `${surface.label} has no way in`).not.toEqual([]);
      expect(found, `${surface.label} is reached only by the palette`).toContain("palette");
    }
  });

  it("names the pages that only the palette reaches, so a new one is a decision", () => {
    const paletteOnly = SURFACES.filter((one) => ways(one.id).length === 1).map((one) => one.id).sort();
    // Account and Doctor also open from the title menu's rows, and Account from Settings' Account section, which this roster does not count.
    // Requests is a panel and not a page: it also opens from a tile on a Job's Flight control.
    // Armada fixes is the Logbook's lower part: the palette row and a retro card's link bring it into view.
    expect(paletteOnly).toEqual([SURFACE.account, SURFACE.checks, SURFACE.doctor, SURFACE.usage, SURFACE.requests, SURFACE.lessons].sort());
  });

  it("is reached by a tab or a System entry for every page the title bar draws, and System opens one of them", () => {
    for (const tab of DECK_TABS) expect(SURFACES.some((one) => one.id === tab.surface)).toBe(true);
    expect(SYSTEM_PAGES.map((page) => page.surface)).toContain(DECK_TABS.find((tab) => tab.id === "system")!.surface);
  });
});
