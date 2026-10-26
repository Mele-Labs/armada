// The Cockpit window's eight tabs and the pages under System: which surface each reaches, which tab a surface
// is under, and what ⌘1 to ⌘6 and ⌘, reach. In node: nothing here needs a document.

import { describe, expect, it } from "vitest";
import { DECK_TABS, SURFACE, SURFACES, SYSTEM_PAGES, deckTabOf, surfaceOfDigit } from "@armada/shell";

describe("the tabs", () => {
  it("maps each tab to its surface", () => {
    expect(DECK_TABS.map((tab) => [tab.id, tab.surface])).toEqual([
      ["cockpit", SURFACE.overview],
      ["jobs", SURFACE.jobs],
      ["sessions", SURFACE.sessions],
      ["studios", SURFACE.studios],
      ["logbook", SURFACE.logbook],
      ["reviews", SURFACE.reviews],
      ["issues", SURFACE.issues],
      ["projects", SURFACE.projects],
      ["system", SURFACE.kit],
    ]);
  });

  it("puts every surface under one tab, and System holds the five pages its sidebar replaced", () => {
    expect(deckTabOf(SURFACE.overview)).toBe("cockpit");
    expect(deckTabOf(SURFACE.jobs)).toBe("jobs");
    expect(deckTabOf(SURFACE.sessions)).toBe("sessions");
    expect(deckTabOf(SURFACE.studios)).toBe("studios");
    expect(deckTabOf(SURFACE.logbook)).toBe("logbook");
    // Armada fixes is the Logbook's lower part, not a tab: the roster row brings it into view.
    expect(deckTabOf(SURFACE.lessons)).toBeUndefined();
    expect(deckTabOf(SURFACE.reviews)).toBe("reviews");
    expect(deckTabOf(SURFACE.issues)).toBe("issues");
    for (const page of SYSTEM_PAGES) expect(deckTabOf(page.surface), page.surface).toBe("system");
    expect(SYSTEM_PAGES.map((page) => page.label)).toEqual(["Kit", "Mods", "Settings", "Guides", "Workflows"]);
  });

  it("reads each page's name from the roster the palette draws", () => {
    for (const page of SYSTEM_PAGES) expect(SURFACES.find((one) => one.id === page.surface)?.label).toBe(page.label);
  });

  it("puts no tab under a surface the deck does not place", () => {
    expect(deckTabOf(SURFACE.manifest)).toBeUndefined();
    expect(deckTabOf(SURFACE.mergeLine)).toBeUndefined();
    // Doctor is a row of the title bar's menu: not a tab and not a page under System.
    expect(deckTabOf(SURFACE.doctor)).toBeUndefined();
    expect(SYSTEM_PAGES.map((page) => page.surface)).not.toContain(SURFACE.doctor);
    expect(deckTabOf("nowhere")).toBeUndefined();
  });
});

describe("⌘1 to ⌘6", () => {
  it("open the places bar's tabs in the order it draws them", () => {
    expect([1, 2, 3, 4, 5, 6].map((digit) => surfaceOfDigit(digit))).toEqual([SURFACE.overview, SURFACE.jobs, SURFACE.sessions, SURFACE.studios, SURFACE.logbook, SURFACE.kit]);
  });

  it("reach nothing past ⌘6", () => {
    for (const digit of [7, 8, 9]) expect(surfaceOfDigit(digit), String(digit)).toBeUndefined();
  });

  it("are the keys the palette shows beside a place, with Settings on ⌘,", () => {
    const keyed = SURFACES.filter((one) => one.shortcut !== undefined).map((one) => [one.id, one.shortcut]);
    expect(keyed).toEqual([
      [SURFACE.overview, "⌘1"],
      [SURFACE.jobs, "⌘2"],
      [SURFACE.studios, "⌘4"],
      [SURFACE.settings, "⌘,"],
      [SURFACE.logbook, "⌘5"],
      [SURFACE.sessions, "⌘3"],
    ].sort((a, b) => SURFACES.findIndex((one) => one.id === a[0]) - SURFACES.findIndex((one) => one.id === b[0])));
  });
});
