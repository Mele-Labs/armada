// A Fleet that is down when Bridge opens is the Cockpit powering on, not a fault: the deck is drawn and the Fleet banner held,
// until the grace is out (`cockpit/boot.ts`). Past it the empty board and the banner say Fleet is not running. The walk
// `cockpit-power-on` shows how it reads.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { POWER_GRACE_MS } from "../cockpit/boot";
import { scenarioNamed, type Scenario } from "./scenario";
import { timePasses } from "./time-passes";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();
afterEach(() => vi.useRealTimers());

/** No runtime file, and one more reading of the same, which a `later` step publishes so the window reads the clock again. */
function notRunning(): Scenario {
  const down = scenarioNamed("fleet-not-running")!;
  return { ...down, later: [{ connection: { ...down.state.connection } }] };
}

const board = () => page.getByText("Fleet is not connected, so there is nothing to show.");
const banner = () => page.getByText("Bridge rereads the file every 2 seconds and connects when Fleet writes one.");

test("a Fleet that is down when Bridge opens draws the deck powering on, with no empty board and no banner", async () => {
  mount(notRunning());
  await onScreen();
  await expect.element(page.getByRole("region", { name: "Jobs", exact: true })).toBeVisible();
  expect(board().query()).toBeNull();
  expect(banner().query()).toBeNull();
});

test("past the grace a Fleet still down draws the empty board and the banner", async () => {
  vi.useFakeTimers({ toFake: ["Date"] });
  mount(notRunning());
  await onScreen();
  await expect.element(page.getByRole("region", { name: "Jobs", exact: true })).toBeVisible();
  vi.setSystemTime(Date.now() + POWER_GRACE_MS + 1);
  timePasses();
  await expect.element(board()).toBeVisible();
  await expect.element(banner()).toBeVisible();
  expect(page.getByRole("region", { name: "Jobs", exact: true }).query()).toBeNull();
});
