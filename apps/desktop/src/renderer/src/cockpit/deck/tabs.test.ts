// The Cockpit window's tabs, the Operator's first, and the pages under System: which surface each reaches, which tab a surface
// is under, and what ⌘1 to ⌘8 and ⌘, reach. In node: nothing here needs a document.

import { describe, expect, it } from "vitest";
import {
  DECK_TABS,
  SURFACE,
  SURFACES,
  SYSTEM_PAGES,
  deckTabOf,
  drawnTabs,
  surfaceOfDigit,
} from "@armada/shell";

describe("the tabs", () => {
  it("maps each tab to its surface", () => {
    expect(
      DECK_TABS.map((tab) => [
        tab.id,
        tab.surface,
      ]),
    ).toEqual([
      [
        "operator",
        SURFACE.operator,
      ],
      [
        "cockpit",
        SURFACE.overview,
      ],
      [
        "jobs",
        SURFACE.jobs,
      ],
      [
        "sessions",
        SURFACE.sessions,
      ],
      [
        "studios",
        SURFACE.studios,
      ],
      [
        "logbook",
        SURFACE.logbook,
      ],
      [
        "reviews",
        SURFACE.reviews,
      ],
      [
        "issues",
        SURFACE.issues,
      ],
      [
        "projects",
        SURFACE.projects,
      ],
      [
        "system",
        SURFACE.kit,
      ],
    ]);
  });

  it("puts every surface under one tab, and System holds the five pages its sidebar replaced", () => {
    expect(deckTabOf(SURFACE.operator)).toBe("operator");
    expect(deckTabOf(SURFACE.overview)).toBe("cockpit");
    expect(deckTabOf(SURFACE.jobs)).toBe("jobs");
    expect(deckTabOf(SURFACE.sessions)).toBe("sessions");
    expect(deckTabOf(SURFACE.studios)).toBe("studios");
    expect(deckTabOf(SURFACE.logbook)).toBe("logbook");
    // Retros is a tab inside the Logbook, not a deck tab: the roster row opens the Logbook on it.
    expect(deckTabOf(SURFACE.lessons)).toBeUndefined();
    expect(deckTabOf(SURFACE.reviews)).toBe("reviews");
    expect(deckTabOf(SURFACE.issues)).toBe("issues");
    for (const page of SYSTEM_PAGES) expect(deckTabOf(page.surface), page.surface).toBe("system");
    expect(SYSTEM_PAGES.map((page) => page.label)).toEqual([
      "Kit",
      "Mods",
      "Settings",
      "Guides",
      "Workflows",
    ]);
  });

  it("reads each page's name from the roster the palette draws", () => {
    for (const page of SYSTEM_PAGES)
      expect(SURFACES.find((one) => one.id === page.surface)?.label).toBe(page.label);
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

describe("⌘1 to ⌘9", () => {
  const OLD_JOBS = [
    SURFACE.jobs,
  ];

  it("open the drawn tabs in the order the bar draws them, System last", () => {
    expect(drawnTabs(OLD_JOBS).map((tab) => tab.id)).toEqual([
      "operator",
      "cockpit",
      "sessions",
      "studios",
      "logbook",
      "reviews",
      "issues",
      "projects",
      "system",
    ]);
    expect(
      [
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
      ].map((digit) => surfaceOfDigit(digit, OLD_JOBS)),
    ).toEqual([
      SURFACE.operator,
      SURFACE.overview,
      SURFACE.sessions,
      SURFACE.studios,
      SURFACE.logbook,
      SURFACE.reviews,
      SURFACE.issues,
      SURFACE.projects,
      SURFACE.kit,
    ]);
  });

  it("give a tab that is not drawn no digit, and close the digits up behind it", () => {
    const flight = [
      SURFACE.jobs,
      SURFACE.sessions,
      SURFACE.reviews,
      SURFACE.issues,
      SURFACE.projects,
    ];
    expect(
      [
        1,
        2,
        3,
        4,
        5,
      ].map((digit) => surfaceOfDigit(digit, flight)),
    ).toEqual([
      SURFACE.operator,
      SURFACE.overview,
      SURFACE.studios,
      SURFACE.logbook,
      SURFACE.kit,
    ]);
    const noStudios = [
      SURFACE.jobs,
      SURFACE.studios,
    ];
    expect(
      [
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
      ].map((digit) => surfaceOfDigit(digit, noStudios)),
    ).toEqual([
      SURFACE.operator,
      SURFACE.overview,
      SURFACE.sessions,
      SURFACE.logbook,
      SURFACE.reviews,
      SURFACE.issues,
      SURFACE.projects,
      SURFACE.kit,
    ]);
  });

  it("reach nothing past the last tab drawn", () => {
    for (const digit of [
      10,
    ])
      expect(surfaceOfDigit(digit, OLD_JOBS), String(digit)).toBeUndefined();
    expect(
      surfaceOfDigit(6, [
        SURFACE.jobs,
        SURFACE.sessions,
        SURFACE.reviews,
        SURFACE.issues,
        SURFACE.projects,
      ]),
    ).toBeUndefined();
  });

  it("are the keys the palette shows beside a place, as the app last said what is left off, with Settings on ⌘,", () => {
    const keyed = () =>
      SURFACES.filter((one) => one.shortcut !== undefined).map((one) => [
        one.id,
        one.shortcut,
      ]);
    const placed = (...ids: string[]) =>
      ids.map((id) => [
        id,
        SURFACES.find((one) => one.id === id)!.shortcut,
      ]);
    // Before the app says, only the old Jobs tab is off.
    expect(
      placed(
        SURFACE.operator,
        SURFACE.overview,
        SURFACE.sessions,
        SURFACE.studios,
        SURFACE.logbook,
        SURFACE.reviews,
        SURFACE.issues,
        SURFACE.projects,
      ),
    ).toEqual([
      [
        SURFACE.operator,
        "⌘1",
      ],
      [
        SURFACE.overview,
        "⌘2",
      ],
      [
        SURFACE.sessions,
        "⌘3",
      ],
      [
        SURFACE.studios,
        "⌘4",
      ],
      [
        SURFACE.logbook,
        "⌘5",
      ],
      [
        SURFACE.reviews,
        "⌘6",
      ],
      [
        SURFACE.issues,
        "⌘7",
      ],
      [
        SURFACE.projects,
        "⌘8",
      ],
    ]);
    expect(keyed().map(([id]) => id)).not.toContain(SURFACE.jobs);
    expect(keyed().map(([id]) => id)).not.toContain(SURFACE.kit);
    expect(SURFACES.find((one) => one.id === SURFACE.settings)!.shortcut).toBe("⌘,");
  });
});
