// The Cockpit window's four tabs and the pages under System: which surface each reaches, which tab a surface
// is under, and what ⌘1 to ⌘5 reach. In node: nothing here needs a document.

import { describe, expect, it } from "vitest";
import { DECK_TABS, SURFACE, SURFACES, SYSTEM_PAGES, deckTabOf, surfaceOfDigit } from "@armada/shell";

describe("the tabs", () => {
  it("maps each tab to its surface", () => {
    expect(DECK_TABS.map((tab) => [tab.id, tab.surface])).toEqual([
      ["cockpit", SURFACE.overview],
      ["jobs", SURFACE.jobs],
      ["sessions", SURFACE.sessions],
      ["system", SURFACE.kit],
    ]);
  });

  it("puts every surface under one tab, and System holds the strip's eight pages", () => {
    expect(deckTabOf(SURFACE.overview)).toBe("cockpit");
    expect(deckTabOf(SURFACE.jobs)).toBe("jobs");
    expect(deckTabOf(SURFACE.sessions)).toBe("sessions");
    for (const page of SYSTEM_PAGES) expect(deckTabOf(page.surface), page.surface).toBe("system");
    expect(SYSTEM_PAGES.map((page) => page.label)).toEqual(["Kit", "Mods", "Settings", "Guides", "Workflows", "Doctor", "Studios", "Retros"]);
  });

  it("reads each page's name from the roster the strip and the palette draw", () => {
    for (const page of SYSTEM_PAGES) expect(SURFACES.find((one) => one.id === page.surface)?.label).toBe(page.label);
  });

  it("puts no tab under a surface the deck does not place", () => {
    expect(deckTabOf(SURFACE.manifest)).toBeUndefined();
    expect(deckTabOf(SURFACE.mergeLine)).toBeUndefined();
    expect(deckTabOf("nowhere")).toBeUndefined();
  });
});

describe("⌘1 to ⌘5", () => {
  it("switch tabs, and ⌘5 is Settings", () => {
    expect([1, 2, 3, 4, 5].map((digit) => surfaceOfDigit(digit))).toEqual([SURFACE.overview, SURFACE.jobs, SURFACE.sessions, SURFACE.kit, SURFACE.settings]);
  });

  it("reach nothing past ⌘5", () => {
    for (const digit of [6, 7, 8, 9]) expect(surfaceOfDigit(digit), String(digit)).toBeUndefined();
  });

  it("are the keys the palette shows beside a place, and no other place shows one", () => {
    const keyed = SURFACES.filter((one) => one.shortcut !== undefined).map((one) => [one.id, one.shortcut]);
    expect(keyed).toEqual([
      [SURFACE.overview, "⌘1"],
      [SURFACE.jobs, "⌘2"],
      [SURFACE.sessions, "⌘3"],
      [SURFACE.settings, "⌘5"],
    ].sort((a, b) => SURFACES.findIndex((one) => one.id === a[0]) - SURFACES.findIndex((one) => one.id === b[0])));
  });
});
