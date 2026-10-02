// Run on a Studio's rail — the owner, 2 Oct 2026: beside Note, Link and Sketch,
// a press opens the checkout's commands, and while the Studio is read-only it is
// drawn off with its reason rather than hidden.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { sheet } from "./manifest-fleet";
import { mountApp, type Mounted } from "./mount";
import { studying } from "./studio-fleet";

const windows: { app: Mounted; host: HTMLElement }[] = [];

function open(scenario: Parameters<typeof mountApp>[0]): Mounted {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const app = mountApp(scenario, host);
  windows.push({ app, host });
  return app;
}

afterEach(() => {
  for (const one of windows.splice(0)) {
    one.app.unmount();
    one.host.remove();
  }
});

const rail = () => page.getByRole("group", { name: "What you can put on this Studio" });
const run = () => rail().getByRole("button", { name: "Run", exact: true });

async function openTheStudio(): Promise<void> {
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("cell", { name: "The Board's legend", exact: true }).click();
}

test("the rail offers Run beside the kinds, and its press opens the commands", async () => {
  const app = open(studying().scenario);
  await openTheStudio();
  await page.getByRole("button", { name: "Continue" }).click();

  await expect.element(rail().getByRole("button", { name: "Add a Note" })).toBeVisible();
  await expect.element(run()).toBeEnabled();

  const startStudioRun = vi.spyOn(app.api, "startStudioRun");
  await run().click();
  await expect.element(page.getByRole("menuitem", { name: "storybook_dev", exact: true })).toBeVisible();
  await page.getByRole("menuitem", { name: "typecheck", exact: true }).click();
  await expect.poll(() => startStudioRun.mock.calls.length).toBe(1);
  expect(startStudioRun.mock.calls[0]![1]).toBe("typecheck");
  expect(startStudioRun.mock.calls[0]![2]).toEqual({ x: expect.any(Number), y: expect.any(Number) });
});

test("while the Studio is read-only, all four are drawn off and say why", async () => {
  open(studying().scenario);
  await openTheStudio();

  await expect.element(run()).toBeDisabled();
  await expect.element(run()).toHaveAccessibleDescription("Continue this Studio to run something.");
  // The owner, 2 Oct 2026: greyed and never hidden, so the rail is the same in both modes.
  for (const name of ["Add a Note", "Add a Link", "Add a Sketch"]) {
    const kind = rail().getByRole("button", { name, exact: true });
    await expect.element(kind).toBeDisabled();
    await expect.element(kind).toHaveAccessibleDescription("Continue this Studio to add to it.");
  }
});

test("where the checkout declares nothing to run, Run is off and says so", async () => {
  const fleet = studying();
  open({
    ...fleet.scenario,
    behaves: (handle) => ({
      ...fleet.scenario.behaves?.(handle),
      watchCheckoutRunSheet: async (want: boolean) =>
        handle.publish({
          checkoutRunSheet: want
            ? { state: "read", sheet: sheet({ checks: [], commands: [], servers: [] }) }
            : { state: "none" },
        }),
    }),
  });
  await openTheStudio();
  await page.getByRole("button", { name: "Continue" }).click();

  await expect.element(run()).toBeDisabled();
  await expect.element(run()).toHaveAccessibleDescription("This checkout declares nothing a Studio can run.");
});
