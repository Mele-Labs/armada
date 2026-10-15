// A Session's head is one row, and the context is a tank. The terminal glyph holds the address in its tooltip, the title
// carries the whole of itself in its tooltip (that it keeps to one line is the `HeadOneRow` story's, in a narrow frame), the acts are icons, and the model is not in the head: the
// composer's Model select reads it. The tank fills to the percent and turns to the warning tone at 80. Through `App`
// over the `session-head` scenario. The walk `session-head` shows how it reads.

import { expect, onTestFinished, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const TITLE = "How does armada interface with GitHub (PRs, Issues, Projects)";

const head = () => page.getByRole("region", { name: "Session s-e07cfe09" });
const row = () => page.elementLocator(head().element().querySelector("header")!);
const model = () => page.getByRole("combobox", { name: "Model", exact: true });

async function open(title: RegExp) {
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: title }));
}

/** How much of the tank is filled, read off its fill as drawn, and the tone it is drawn in. */
function tank() {
  const one = head().getByRole("button", { name: /^Context/ }).element() as HTMLElement;
  return {
    share: parseFloat(getComputedStyle(one, "::before").width) / one.clientWidth,
    tone: getComputedStyle(one).color,
  };
}

test("The head is one row: the address in the glyph's tooltip, a one-line title, icon acts, and no model", async () => {
  mount("session-head");
  await onScreen();
  await open(/How does armada interface/);

  const glyph = head().getByRole("img", { name: "s-e07cfe09", exact: true });
  await expect.element(glyph).toBeVisible();
  await expect.element(page.elementLocator(glyph.element().parentElement!)).toHaveAccessibleDescription("s-e07cfe09");
  await expect.element(row().getByText("s-e07cfe09")).not.toBeVisible();

  const title = head().getByRole("button", { name: `${TITLE}, rename` });
  await expect.element(title).toHaveAccessibleDescription(TITLE);
  const box = title.element() as HTMLElement;

  const end = head().getByRole("button", { name: "End session" });
  await expect.element(end).toBeVisible();
  expect(end.element().textContent).toBe("");
  await expect.element(head().getByRole("button", { name: "Retro" })).toBeVisible();

  // The glyph, the title, the act, the tank and the state mark stand on one line.
  const centre = (one: Element) => one.getBoundingClientRect().top + one.getBoundingClientRect().height / 2;
  for (const one of [box, end.element(), head().getByRole("button", { name: /^Context/ }).element(), row().getByRole("img", { name: "Working" }).element()]) {
    expect(Math.abs(centre(one) - centre(glyph.element()))).toBeLessThan(8);
  }

  await expect.element(row().getByRole("img", { name: /^Model/ })).not.toBeInTheDocument();
  await expect.element(row().getByText(/Opus/)).not.toBeInTheDocument();
  await expect.element(model()).toHaveDisplayValue("Opus 5.5");
});

test("The tank fills to the percent and turns to the warning tone at 80, not before", async () => {
  mount("session-head");
  await onScreen();
  await open(/How does armada interface/);

  const low = tank();
  expect(low.share).toBeCloseTo(0.05, 1);

  timePasses();
  await expect.element(head().getByRole("button", { name: "Context 84%" })).toHaveTextContent("840k · 84%");
  const high = tank();
  expect(high.share).toBeCloseTo(0.84, 1);
  expect(high.tone).not.toBe(low.tone);

  // 72% is still the normal tone.
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: /Cart totals rounding/ }));
  const resting = page.getByRole("region", { name: "Session s-b4a1d2c7" }).getByRole("button", { name: /^Context/ }).element() as HTMLElement;
  expect(parseFloat(getComputedStyle(resting, "::before").width) / resting.clientWidth).toBeCloseTo(0.72, 1);
  expect(getComputedStyle(resting).color).toBe(low.tone);
});

test("The composer keeps the alias a person picks, whatever the last turn resolved", async () => {
  mount("session-head");
  await onScreen();
  await open(/How does armada interface/);
  await expect.element(model()).toHaveDisplayValue("Opus 5.5");

  await userEvent.selectOptions(model(), "sonnet");
  await expect.element(model()).toHaveDisplayValue("sonnet");
});

/** Both cards hang from the trailing edge of the head's marks, so they open leftward inside the Session's panel; the frame clips what runs past it. */
test("The context card and the fresh card open inside the Session's panel, with the ledger open", async () => {
  await page.viewport(1512, 949);
  onTestFinished(() => page.viewport(1440, 900));
  mount("session-head");
  await onScreen();
  await open(/Cart totals rounding/);

  const frame = page.getByRole("region", { name: "Session s-b4a1d2c7" });
  await expect.element(page.getByRole("region", { name: "Attachments" })).toBeVisible();
  const inside = (card: Element) => {
    const edge = frame.element().getBoundingClientRect();
    const box = card.getBoundingClientRect();
    expect(box.width).toBeGreaterThan(0);
    expect(box.right).toBeLessThanOrEqual(edge.right);
    expect(box.left).toBeGreaterThanOrEqual(edge.left);
  };

  await userEvent.click(frame.getByRole("button", { name: /^Context/ }));
  await expect.element(frame.getByRole("dialog", { name: "Context" })).toBeVisible();
  inside(frame.getByRole("dialog", { name: "Context" }).element());

  await userEvent.click(frame.getByRole("button", { name: "Fresh start" }));
  await expect.element(frame.getByRole("dialog", { name: "Fresh start" })).toBeVisible();
  inside(frame.getByRole("dialog", { name: "Fresh start" }).element());
});

test("A done Session's head draws End session in a glyph other than the Done mark's", async () => {
  mount("session-head");
  await onScreen();
  await open(/Release notes draft/);

  const done = page.getByRole("region", { name: "Session s-c9e3a5f1" });
  const header = done.element().querySelector("header")!;
  // The callout beneath the thread has an End session of its own, in words; the head's is the icon.
  const act = header.querySelector('button[aria-label="End session"]')!;
  const mark = header.querySelector('[role="img"][aria-label="Done"]')!;
  const glyph = (one: Element) => one.querySelector("svg")!.getAttribute("class")!.split(" ").find((name) => name.startsWith("lucide-"));
  expect(glyph(mark)).toBe("lucide-power");
  expect(glyph(act)).toBeDefined();
  expect(glyph(act)).not.toBe(glyph(mark));
});
