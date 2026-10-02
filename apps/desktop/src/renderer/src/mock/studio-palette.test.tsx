// A Studio's acts in the command palette, through `App` — the owner, 2 Oct 2026: the palette's Add a
// note, link and sketch draw the rail's icons.
//
// **The glyph is read as lucide's class on the drawn `svg`**, `land-real-job.test.tsx`'s precedent:
// an icon is `aria-hidden`, so no role or name carries which one it is.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountApp, type Mounted } from "./mount";
import { studying } from "./studio-fleet";

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
  const app = mountApp(scenario, host);
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
