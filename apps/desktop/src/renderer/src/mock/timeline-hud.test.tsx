// The Cockpit's timeline as a HUD over the map, through `App` on `timeline-hud`: the Jobs stuck now, then what each other came to since he
// last closed it, with a lamp on the key while there is something unread. The owner's pick of 2 Nov 2026.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { deckNow } from "../cockpit/deck/store";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const key = () => document.querySelector<HTMLElement>('.deck-tile[data-tile="timeline"]');
const lamp = () => key()?.hasAttribute("data-lamp") ?? false;
const hud = () =>
  page.getByRole("dialog", {
    name: "Timeline",
  });
const rowsIn = (name: string | RegExp) =>
  [
    ...(hud()
      .getByRole("list", {
        name,
      })
      .query()
      ?.querySelectorAll<HTMLElement>(".console-tl__row") ?? []),
  ].map((one) => one.querySelector(".console-tl__title")?.textContent);
const focused = () =>
  (document.activeElement as HTMLElement | null)?.querySelector(".console-tl__title")?.textContent;

async function opened() {
  mount("timeline-hud");
  await onScreen();
  await expect.poll(lamp).toBe(true);
  await page
    .getByRole("region", {
      name: "Flight controls",
    })
    .getByRole("button", {
      name: "Timeline",
    })
    .click();
  await expect.element(hud()).toBeVisible();
}

test("the stuck Jobs come first, then one row for each other that came to something, the newest first", async () => {
  await opened();
  await expect
    .poll(() => rowsIn("Stuck"))
    .toEqual([
      "Cache key",
      "Changelog script",
      "Service worker audit",
      "Fix the reconnect wait",
    ]);
  expect(rowsIn(/^Since /)).toEqual([
    "Migration notes",
    "armada main",
    "Release notes",
  ]);
  await expect.poll(focused).toBe("Cache key");
});

test("each row says what it came to, and a quiet Job says when it was last heard from", async () => {
  await opened();
  const row = (title: string) =>
    [
      ...document.querySelectorAll<HTMLElement>(".console-tl__row"),
    ].find((one) => one.querySelector(".console-tl__title")?.textContent === title);
  await expect
    .poll(() => row("Migration notes")?.querySelector(".console-tl__fact")?.textContent)
    .toBe("#1892 merged");
  expect(row("armada main")?.querySelector(".console-tl__fact")?.textContent).toMatch(
    /^red .*, green /,
  );
  expect(row("Service worker audit")?.querySelector(".console-tl__age")?.textContent).toContain(
    "ago",
  );
});

test("main and a Job already gone have no press, and the keys pass them by", async () => {
  await opened();
  await expect.poll(() => rowsIn(/^Since /).length).toBe(3);
  const presses = () =>
    [
      ...document.querySelectorAll<HTMLElement>("button.console-tl__row"),
    ].map((one) => one.querySelector(".console-tl__title")?.textContent);
  expect(presses()).not.toContain("armada main");
  expect(presses()).not.toContain("Release notes");
  await userEvent.keyboard("{End}");
  await expect.poll(focused).toBe("Migration notes");
  await userEvent.keyboard("{Home}");
  await expect.poll(focused).toBe("Cache key");
  await userEvent.keyboard("{ArrowDown}");
  await expect.poll(focused).toBe("Changelog script");
});

test("Enter opens the row's Job, with focus in it, and Esc comes back on that row with the list unread", async () => {
  await opened();
  await expect.poll(focused).toBe("Cache key");
  await userEvent.keyboard("{End}");
  await expect.poll(focused).toBe("Migration notes");
  await userEvent.keyboard("{Enter}");
  await expect
    .element(
      page.getByRole("region", {
        name: "Job: Migration notes",
      }),
    )
    .toBeVisible();
  expect(hud().query()).toBeNull();
  await expect.poll(() => document.activeElement?.closest(".deck-stage") != null).toBe(true);

  await userEvent.keyboard("{Escape}");
  await expect.element(hud()).toBeVisible();
  expect(deckNow().sel).toBeUndefined();
  await expect.poll(focused).toBe("Migration notes");
  expect(rowsIn(/^Since /)).toHaveLength(3);
  expect(lamp()).toBe(true);
});

test("closing it marks the list read: the lamp goes out, focus is back on the key, and only what is still stuck stays", async () => {
  await opened();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => hud().query()).toBeNull();
  await expect.poll(lamp).toBe(false);
  expect(document.activeElement).toBe(key());
  expect(Number(localStorage.getItem("armada.bridge.timeline-since"))).toBeGreaterThan(0);

  await userEvent.keyboard("t");
  await expect.element(hud()).toBeVisible();
  await expect.poll(() => rowsIn("Stuck").length).toBe(4);
  expect(
    hud()
      .getByRole("list", {
        name: /^Since /,
      })
      .query(),
  ).toBeNull();
});

test("its own key closes it, and t over a Job puts the Job away and opens it", async () => {
  await opened();
  await userEvent.keyboard("t");
  await expect.poll(() => hud().query()).toBeNull();

  await page
    .getByRole("region", {
      name: "Jobs",
      exact: true,
    })
    .getByRole("button", {
      name: "Migration notes",
    })
    .click();
  await expect
    .element(
      page.getByRole("region", {
        name: "Job: Migration notes",
      }),
    )
    .toBeVisible();
  (document.activeElement as HTMLElement | null)?.blur();
  await userEvent.keyboard("t");
  await expect.element(hud()).toBeVisible();
  expect(deckNow().sel).toBeUndefined();
});
