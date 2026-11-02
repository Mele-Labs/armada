// The console through `App`: Calls and Activity open a HUD over the field and close back onto their facet, the rack seats an instrument by
// keyboard, and where the canvas does not start the buttons stand as plain keys. The look is the walk's (`walks/bays-look-like-bays.ts`).

import { beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { changeReplay, deckNow, startReplay } from "../cockpit/deck/store";
import { forgetBays } from "../cockpit/deck/ConsoleBays";
import { saveFlight } from "../cockpit/deck/flight-controls";
import { ConsoleStage } from "../cockpit/deck/pixi/console-stage";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();
// The seated bays are kept in the window's storage, so one test would hand them to the next.
beforeEach(() => {
  localStorage.clear();
  forgetBays();
});

const controls = () =>
  page.getByRole("region", {
    name: "Flight controls",
  });
const facet = (name: string) =>
  controls().getByRole("button", {
    name,
    exact: true,
  });
const hud = (kind: string) => document.querySelector(`[data-hud="${kind}"]`);
const calls = () =>
  page.getByRole("region", {
    name: "Calls",
    exact: true,
  });
const summary = () => document.querySelector(".console-calls__so-far")?.textContent ?? null;
const named = () =>
  document.querySelector('[data-hud="calls"] .console-hud__name')?.textContent ?? "";
const seated = () =>
  JSON.parse(localStorage.getItem("armada.console.bays") ?? "null") as string[] | null;

async function deck(scenario = "cockpit-ship-flightdeck") {
  const app = mount(scenario);
  await onScreen();
  await expect.element(controls()).toBeVisible();
  await expect.element(facet("Calls")).toBeInTheDocument();
  return app;
}

test("pressing Calls opens the HUD on the question, reads pressed, and a second press closes it", async () => {
  await deck();
  await expect.element(facet("Calls")).toHaveAttribute("aria-pressed", "false");
  await facet("Calls").click();
  await expect
    .element(
      calls().getByRole("radio", {
        name: /^Hash/,
      }),
    )
    .toBeVisible();
  await expect.element(facet("Calls")).toHaveAttribute("aria-pressed", "true");
  await facet("Calls").click();
  await expect.poll(() => hud("calls")).toBeNull();
});

test("answering a call moves the HUD to the next one", async () => {
  await deck();
  await facet("Calls").click();
  await expect
    .element(
      calls().getByRole("radio", {
        name: /^Hash/,
      }),
    )
    .toBeVisible();
  const first = named();
  await expect.poll(() => document.activeElement?.closest(".armada-needs-panel")).not.toBeNull();
  await userEvent.keyboard("1");
  await calls()
    .getByRole("button", {
      name: /^Answer/,
    })
    .click();
  await expect.poll(named).not.toBe(first);
  expect(hud("calls")).not.toBeNull();
});

test("Next call shows the next Job calling and comes round to the first", async () => {
  await deck();
  await facet("Calls").click();
  await expect
    .element(
      calls().getByRole("button", {
        name: /^Next call/,
      }),
    )
    .toBeVisible();
  const first = named();
  const seen = new Set([
    first,
  ]);
  for (let i = 0; i < 10; i += 1) {
    const was = named();
    await calls()
      .getByRole("button", {
        name: /^Next call/,
      })
      .click();
    await expect.poll(named).not.toBe(was);
    if (named() === first) break;
    seen.add(named());
  }
  expect(named()).toBe(first);
  expect(seen.size).toBeGreaterThan(1);
});

test("Esc closes the Calls HUD and gives the keys back to its facet", async () => {
  await deck();
  await facet("Calls").click();
  await expect
    .element(
      calls().getByRole("radio", {
        name: /^Hash/,
      }),
    )
    .toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => hud("calls")).toBeNull();
  expect(document.activeElement).toBe(facet("Calls").element());
});

test("⌥⇧2, the Calls key, opens the HUD and closes it, and does not jump to a row", async () => {
  await deck();
  const selected = () => document.querySelector(".deck-row[data-selected]");
  await userEvent.keyboard("{Alt>}{Shift>}2{/Shift}{/Alt}");
  await expect
    .element(
      calls().getByRole("radio", {
        name: /^Hash/,
      }),
    )
    .toBeVisible();
  await expect.element(facet("Calls")).toHaveAttribute("aria-pressed", "true");
  await userEvent.keyboard("{Alt>}{Shift>}2{/Shift}{/Alt}");
  await expect.poll(() => hud("calls")).toBeNull();
  expect(document.activeElement).toBe(facet("Calls").element());
  // The row was lit only for the HUD: nothing was opened.
  expect(selected()).toBeNull();
});

test("the Calls facet is named Calls", async () => {
  await deck();
  await expect.element(facet("Calls")).toBeInTheDocument();
  expect(facet("Needs me").query()).toBeNull();
});

test("pressing Activity opens its HUD, reads pressed, and Esc returns to the facet", async () => {
  await deck();
  await facet("Activity").click();
  await expect
    .element(
      page.getByRole("dialog", {
        name: "Activity",
      }),
    )
    .toBeVisible();
  await expect
    .poll(() => hud("activity")?.querySelectorAll(".armada-tl__dot").length ?? 0)
    .toBeGreaterThan(0);
  await expect.element(facet("Activity")).toHaveAttribute("aria-pressed", "true");
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => hud("activity")).toBeNull();
  expect(document.activeElement).toBe(facet("Activity").element());
  await expect.element(facet("Activity")).toHaveAttribute("aria-pressed", "false");
});

/** A row under the cursor with the map as the stage, so what Esc would do around the HUD is the cursor going. */
async function cursored() {
  await deck();
  await controls()
    .getByRole("button", {
      name: "Map",
    })
    .click();
  const row = document.querySelector<HTMLElement>(".deck-list .deck-row")!;
  (document.activeElement as HTMLElement | null)?.blur();
  row.focus();
  await expect.poll(() => deckNow().cursor).toBeDefined();
  return deckNow().cursor;
}
const around = () => {
  const now = deckNow();
  return {
    cursor: now.cursor,
    sel: now.sel,
    view: now.view,
    focus: now.focus,
    replay: now.replay,
    checks: now.checks,
  };
};

for (const [name, ready] of [
  [
    "Activity",
    async () =>
      expect
        .element(
          page.getByRole("dialog", {
            name: "Activity",
          }),
        )
        .toBeVisible(),
  ],
  [
    "Calls",
    async () =>
      expect
        .element(
          calls().getByRole("radio", {
            name: /^Hash/,
          }),
        )
        .toBeVisible(),
  ],
] as const) {
  test(`Esc in the ${name} HUD closes it and leaves the cursor, the selection and the view as they were`, async () => {
    const cursor = await cursored();
    await facet(name).click();
    await ready();
    const before = around();
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => hud(name.toLowerCase())).toBeNull();
    expect(around()).toEqual(before);
    expect(deckNow().cursor).toBe(cursor);
    expect(document.activeElement).toBe(facet(name).element());
  });

  test(`Esc in the ${name} HUD does not leave a replay`, async () => {
    await cursored();
    startReplay(Date.now() - 60_000);
    // Paused: a replay that plays on ends itself at Live, which is not what Esc did.
    changeReplay({
      playing: false,
    });
    await facet(name).click();
    await ready();
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => hud(name.toLowerCase())).toBeNull();
    expect(deckNow().replay).toBeDefined();
  });

  test(`Esc from the Jobs list with the ${name} HUD open closes only the HUD`, async () => {
    await cursored();
    await facet(name).click();
    await ready();
    const before = around();
    const row = document.querySelector<HTMLElement>(".deck-list .deck-row")!;
    row.focus();
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => hud(name.toLowerCase())).toBeNull();
    expect(around()).toEqual(before);
  });
}

test("Esc on the Timeline closes it back to the map, and a Job opened from it goes back to the Timeline first", async () => {
  await deck();
  await facet("Timeline").click();
  await expect.poll(() => deckNow().view).toBe("timeline");
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => deckNow().view).toBe("map");
  expect(deckNow().sel).toBeUndefined();

  await facet("Timeline").click();
  const row = document.querySelector<HTMLElement>(".deck-list .deck-row")!;
  row.click();
  await expect.poll(() => deckNow().sel).toBeDefined();
  // The Job's box may hold focus: its first Esc leaves the box, as it does today, and the next goes back.
  for (let i = 0; i < 3 && deckNow().sel !== undefined; i += 1) {
    await userEvent.keyboard("{Escape}");
    await new Promise((done) => window.setTimeout(done, 100));
  }
  expect(deckNow().sel).toBeUndefined();
  expect(deckNow().view).toBe("timeline");
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => deckNow().view).toBe("map");
});

test("Esc on the map at rest closes no view, and the Checks filter is not a view Esc leaves", async () => {
  await deck();
  await facet("Checks").click();
  await expect.element(facet("Checks")).toHaveAttribute("aria-pressed", "true");
  await userEvent.keyboard("{Escape}");
  expect(deckNow().view).toBe("map");
  expect(deckNow().checks).toBe(true);
});

test("an instrument is seated in a bay from the rack by keyboard, and Esc takes an edit back", async () => {
  await deck();
  await page
    .elementLocator(document.querySelector(".deck-controls .deck-band [aria-label='Edit']")!)
    .click();
  const socket = () =>
    page.getByRole("button", {
      name: /^Bay 1/,
    });
  await expect.element(socket()).toHaveAttribute("aria-label", "Bay 1, Checks");
  expect(document.activeElement).toBe(socket().element());
  // No sample instrument is on offer, and nothing makes one.
  expect(
    page
      .getByRole("option", {
        name: /Plan usage/,
      })
      .query(),
  ).toBeNull();
  expect(
    controls()
      .getByRole("button", {
        name: /Make one/,
      })
      .query(),
  ).toBeNull();

  await userEvent.keyboard("{Enter}");
  await expect.poll(() => document.activeElement?.closest(".console-tray")).not.toBeNull();
  const instrument = () => document.activeElement!.getAttribute("aria-label");
  while (instrument() !== null && !/Needles|Gauges/.test(instrument()!))
    await userEvent.keyboard("{ArrowRight}");
  const chosen = instrument()!;
  await userEvent.keyboard("{Enter}");
  await expect.element(socket()).toHaveAttribute("aria-label", `Bay 1, ${chosen}`);
  expect(seated()?.[0]).toBe("needles");

  await userEvent.keyboard("{Escape}");
  await expect.poll(() => seated()?.[0]).toBe("matrix");
});

const DOCS = {
  id: "my-docs",
  kind: "page",
  name: "Docs",
  text: "https://docs.example/guide",
} as const;
const edit = () =>
  page.elementLocator(document.querySelector(".deck-controls .deck-band [aria-label='Edit']")!);
const bay = (n: number) => document.querySelector(`.console-bay[data-n="${n}"]`);

/** Edit, then the keys: Enter on Bay 1 to reach the tray, the arrows to `name`, Enter to seat it, and Done. */
async function seatFirst(name: string) {
  await edit().click();
  await expect
    .element(
      page.getByRole("button", {
        name: /^Bay 1/,
      }),
    )
    .toHaveAttribute("aria-label", "Bay 1, Checks");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => document.activeElement?.closest(".console-tray")).not.toBeNull();
  for (let i = 0; i < 12 && document.activeElement?.getAttribute("aria-label") !== name; i += 1)
    await userEvent.keyboard("{ArrowRight}");
  expect(document.activeElement?.getAttribute("aria-label")).toBe(name);
  await userEvent.keyboard("{Enter}");
  await page
    .getByRole("button", {
      name: "Done",
      exact: true,
    })
    .click();
}

test("a Page tile on the deck is an instrument in the rack's tray, under its own label", async () => {
  await deck();
  await saveFlight("deck", [
    DOCS,
  ]);
  await edit().click();
  await expect
    .element(
      page.getByRole("option", {
        name: "Docs",
        exact: true,
      }),
    )
    .toBeInTheDocument();
});

test("a Page tile is seated in a bay by keyboard and its key stands in that bay", async () => {
  await deck();
  await saveFlight("deck", [
    DOCS,
  ]);
  await seatFirst("Docs");
  await expect.element(facet("Docs")).toBeInTheDocument();
  expect(bay(1)?.contains(facet("Docs").element())).toBe(true);
  expect(facet("Checks").query()).toBeNull();
  expect(seated()?.[0]).toBe("page:my-docs");
});

test("pressing a seated Page tile opens its address, and its tooltip says so", async () => {
  const app = await deck();
  const open = vi.spyOn(app.api, "openLink").mockResolvedValue({
    ok: true,
  });
  await saveFlight("deck", [
    DOCS,
  ]);
  await seatFirst("Docs");
  await facet("Docs").hover();
  await expect.element(page.getByText("Open https://docs.example/guide")).toBeInTheDocument();
  await facet("Docs").click();
  expect(open).toHaveBeenCalledWith("https://docs.example/guide");
});

test("a seated Page tile that is deleted leaves an empty socket", async () => {
  await deck();
  await saveFlight("deck", [
    DOCS,
  ]);
  await seatFirst("Docs");
  await expect.element(facet("Docs")).toBeInTheDocument();
  await saveFlight("deck", []);
  await expect.poll(() => facet("Docs").query()).toBeNull();
  expect(bay(1)?.hasAttribute("data-empty")).toBe(true);
  expect(bay(1)?.querySelector("button")).toBeNull();
});

test("the rack adds a Page tile with the form it always had, and the tile lands in the tray", async () => {
  await deck();
  await edit().click();
  await page
    .getByRole("option", {
      name: "Add action",
    })
    .click();
  await page
    .getByRole("textbox", {
      name: "Name",
    })
    .fill("Docs");
  await page
    .getByRole("textbox", {
      name: "Address",
    })
    .fill("https://docs.example/guide");
  await page
    .getByRole("button", {
      name: "Save",
    })
    .click();
  await expect
    .element(
      page.getByRole("option", {
        name: "Docs",
        exact: true,
      }),
    )
    .toBeInTheDocument();
});

test("a Job with a summary draws it", async () => {
  await deck();
  await facet("Calls").click();
  await expect.poll(summary).toContain("Read the manifest loader");
});

test("a Job with no summary draws no summary", async () => {
  await deck("session-waiting-on-you");
  await facet("Calls").click();
  await expect
    .element(
      calls().getByRole("button", {
        name: "Open Job",
      }),
    )
    .toBeVisible();
  expect(document.querySelector(".console-calls__side")).not.toBeNull();
  expect(summary()).toBeNull();
});

test("folded, the console's band names it and a press on it opens it again", async () => {
  localStorage.setItem("armada.panel-collapsed.controls", "1");
  mount("cockpit-ship-flightdeck");
  await onScreen();
  // The band keeps no name once the console is open, so it is found by its place.
  await expect.poll(() => document.querySelector(".deck-controls > .deck-band")).not.toBeNull();
  const band = document.querySelector<HTMLElement>(".deck-controls > .deck-band")!;
  expect(band.getAttribute("aria-expanded")).toBe("false");
  expect(band.querySelector(".deck-band__name")!.getBoundingClientRect().width).toBeGreaterThan(0);
  band.click();
  await expect.poll(() => band.getAttribute("aria-expanded")).toBe("true");
});

test("where the canvas does not start, the buttons stand as plain keys and still press", async () => {
  vi.spyOn(console, "error").mockImplementation(() => undefined);
  const start = vi
    .spyOn(
      ConsoleStage.prototype as unknown as {
        init(): Promise<void>;
      },
      "init",
    )
    .mockRejectedValue(new Error("no hardware WebGL"));
  try {
    await deck();
    await expect
      .poll(() => document.querySelector(".console-deck")?.hasAttribute("data-no-canvas"))
      .toBe(true);
    const box = facet("Calls").element().getBoundingClientRect();
    expect(box.width).toBeGreaterThan(40);
    expect(box.height).toBeGreaterThan(24);
    expect(getComputedStyle(facet("Calls").element()).opacity).not.toBe("0");
    await facet("Calls").click();
    await expect
      .element(
        calls().getByRole("radio", {
          name: /^Hash/,
        }),
      )
      .toBeVisible();
  } finally {
    start.mockRestore();
    vi.restoreAllMocks();
  }
});
