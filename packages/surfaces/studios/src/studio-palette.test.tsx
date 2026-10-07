// A Studio's acts in the command palette, through `App` — the owner, 2 Oct 2026: the palette's Add a
// note, link and sketch draw the rail's icons.
//
// **The glyph is read as lucide's class on the drawn `svg`**, `land-real-job.test.tsx`'s precedent:
// an icon is `aria-hidden`, so no role or name carries which one it is.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountApp, type Mounted, studying } from "@armada/desktop/mock";

const SLICES = { slices: ["core", "studios"] } as const;

const windows: { app: Mounted; host: HTMLElement }[] = [];

afterEach(() => {
  for (const one of windows.splice(0)) {
    one.app.unmount();
    one.host.remove();
  }
});

function open(scenario: Parameters<typeof mountApp>[0]): Mounted {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const app = mountApp(scenario, host, undefined, SLICES);
  windows.push({ app, host });
  return app;
}

const rail = () => page.getByRole("group", { name: "What you can put on this Studio" });

async function openTheStudio(): Promise<void> {
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("cell", { name: "The Board's legend", exact: true }).click();
  await expect.element(rail()).toBeVisible();
}

/** ⌘K, and the palette's list once it is up. */
async function palette() {
  await userEvent.keyboard("{Meta>}k{/Meta}");
  const list = page.getByRole("dialog", { name: "Command palette" });
  await expect.element(list).toBeVisible();
  return list;
}

/** Which lucide glyph an element draws, by the class lucide gives its `svg`. */
const glyphOf = (element: Element) =>
  [...(element.querySelector("svg")?.classList ?? [])].find((one) => one.startsWith("lucide-") && one !== "lucide-icon");

test("the palette's Add a note, link and sketch draw the rail's icons", async () => {
  open(studying().scenario);
  await openTheStudio();
  const drawn = {
    note: glyphOf(rail().getByRole("button", { name: "Add a Note", exact: true }).element()),
    link: glyphOf(rail().getByRole("button", { name: "Add a Link", exact: true }).element()),
    sketch: glyphOf(rail().getByRole("button", { name: "Add a Sketch", exact: true }).element()),
  };
  expect(drawn).toEqual({ note: "lucide-sticky-note", link: "lucide-link", sketch: "lucide-shapes" });

  const list = await palette();
  for (const [kind, glyph] of Object.entries(drawn)) {
    const option = list.getByRole("option", { name: new RegExp(`^Add a ${kind}`) });
    await expect.element(option).toBeInTheDocument();
    expect(glyphOf(option.element()), kind).toBe(glyph);
  }
});

const runMenu = () => page.getByRole("menuitem", { name: "typecheck", exact: true });

test("R opens the rail's Run menu, and so does the palette's Run, and a pick starts it", async () => {
  const app = open(studying().scenario);
  await openTheStudio();
  await page.getByRole("button", { name: "Continue" }).click();
  await expect.element(rail().getByRole("button", { name: "Run", exact: true })).toBeEnabled();

  await userEvent.keyboard("R");
  await expect.element(runMenu()).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => runMenu().query()).toBeNull();

  const list = await palette();
  const row = list.getByRole("option", { name: /^Run\b/ });
  expect(glyphOf(row.element())).toBe("lucide-zap");
  await row.click();
  await expect.element(runMenu()).toBeVisible();

  const startStudioRun = vi.spyOn(app.api, "startStudioRun");
  await runMenu().click();
  await expect.poll(() => startStudioRun.mock.calls.length).toBe(1);
  expect(startStudioRun.mock.calls[0]![1]).toBe("typecheck");
});

test("while read-only, R opens nothing and the palette's Run is off with the rail's reason", async () => {
  open(studying().scenario);
  await openTheStudio();

  await userEvent.keyboard("R");
  await new Promise((resolve) => setTimeout(resolve, 200));
  expect(runMenu().query()).toBeNull();

  const list = await palette();
  const row = list.getByRole("option", { name: /^Run\b/ });
  await expect.element(row).toHaveAttribute("aria-disabled", "true");
  await expect.element(row).toHaveTextContent("Continue this Studio to run something.");
  // Pressed anyway: a dimmed row is drawn to say why, and choosing it opens nothing.
  await row.click({ force: true });
  await new Promise((resolve) => setTimeout(resolve, 200));
  expect(runMenu().query()).toBeNull();
});

// The palette's Add rows arm their kind on the rail, as `N`, `V`, `S` and `Z` do — the owner, 2 Oct
// 2026. They said "the Studio's own + Node control", a control the rail replaced, and Add a zone
// was drawn on and did nothing.

const KINDS = [
  ["note", "Note"],
  ["link", "Link"],
  ["sketch", "Sketch"],
  ["zone", "Zone"],
] as const;

test("the palette's Add rows arm their kind on the rail, Zone among them", async () => {
  open(studying().scenario);
  await openTheStudio();
  await page.getByRole("button", { name: "Continue" }).click();

  for (const [kind, noun] of KINDS) {
    const list = await palette();
    const option = list.getByRole("option", { name: new RegExp(`^Add a ${kind}`) });
    await expect.element(option).not.toHaveAttribute("aria-disabled", "true");
    await option.click();
    await expect
      .element(rail().getByRole("button", { name: `Add a ${noun}`, exact: true }))
      .toHaveAttribute("aria-pressed", "true");
  }
});

test("while read-only, the palette's Add rows are off with the rail's reason", async () => {
  open(studying().scenario);
  await openTheStudio();

  const list = await palette();
  for (const [kind, noun] of KINDS) {
    const option = list.getByRole("option", { name: new RegExp(`^Add a ${kind}`) });
    await expect.element(option).toHaveAttribute("aria-disabled", "true");
    await expect.element(option).toHaveTextContent("Continue this Studio to add to it.");
    // Pressed anyway: a dimmed row is drawn to say why, and choosing it arms nothing.
    await option.click({ force: true });
    await expect
      .element(rail().getByRole("button", { name: `Add a ${noun}`, exact: true }))
      .toHaveAttribute("aria-pressed", "false");
  }
});

/** The words of the menu item focus is on, or `null` where focus is not on one. */
const focusedItem = () =>
  document.activeElement?.getAttribute("role") === "menuitem" ? document.activeElement.textContent : null;

test("R and the palette's Run put focus in the menu, so ↓ and Enter start a run", async () => {
  const app = open(studying().scenario);
  await openTheStudio();
  await page.getByRole("button", { name: "Continue" }).click();
  const startStudioRun = vi.spyOn(app.api, "startStudioRun");

  await userEvent.keyboard("R");
  await expect.element(runMenu()).toBeVisible();
  await expect.poll(focusedItem).not.toBeNull();
  const first = focusedItem();
  await userEvent.keyboard("{ArrowDown}");
  await expect.poll(focusedItem).not.toBe(first);
  const second = focusedItem();
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => startStudioRun.mock.calls.length).toBe(1);
  expect(startStudioRun.mock.calls[0]![1]).toBe(second);

  const list = await palette();
  await list.getByRole("option", { name: /^Run\b/ }).click();
  await expect.element(runMenu()).toBeVisible();
  await expect.poll(focusedItem).toBe(first);
});
