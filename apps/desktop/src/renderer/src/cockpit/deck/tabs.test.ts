// The Cockpit window's four tabs and the pages under System: which surface each reaches, which tab a surface
// is under, and what ⌘1 to ⌘4 reach with the rail off. In node: nothing here needs a document.

import { describe, expect, it } from "vitest";
import { DECK_TABS, SURFACE, SURFACES, SYSTEM_PAGES, deckSurfaceOfDigit, deckTabOf } from "@armada/shell";

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

  it("reads each page's name from the roster the rail and the palette draw", () => {
    for (const page of SYSTEM_PAGES) expect(SURFACES.find((one) => one.id === page.surface)?.label).toBe(page.label);
  });

  it("puts no tab under a surface the deck does not place", () => {
    expect(deckTabOf(SURFACE.manifest)).toBeUndefined();
    expect(deckTabOf(SURFACE.mergeLine)).toBeUndefined();
    expect(deckTabOf("nowhere")).toBeUndefined();
  });
});

describe("⌘1 to ⌘4", () => {
  it("switch tabs when the rail is off", () => {
    expect([1, 2, 3, 4].map((digit) => deckSurfaceOfDigit(digit, true))).toEqual([SURFACE.overview, SURFACE.jobs, SURFACE.sessions, SURFACE.kit]);
  });

  it("leave ⌘5 to ⌘9 to the rail either way", () => {
    for (const digit of [5, 6, 7, 8, 9]) expect(deckSurfaceOfDigit(digit, true), String(digit)).toBeUndefined();
  });

  it("reach nothing here when the rail is on", () => {
    for (const digit of [1, 2, 3, 4, 5, 9]) expect(deckSurfaceOfDigit(digit, false), String(digit)).toBeUndefined();
  });
});
