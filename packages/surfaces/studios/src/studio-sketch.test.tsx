// A Studio's Sketch is the pad, through `App` — the owner's call of 1 Oct 2026. Placed from the rail
// it opens the pad; drawn on and closed it lands on the Studio; opened again it is the drawing that
// was left; and Dispatch puts that drawing on the composer's pad, made from that node.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountApp, type Mounted, studying } from "@armada/desktop/mock";

const SLICES = { slices: ["core", "studios"] } as const;

let mounted: { app: Mounted; host: HTMLElement } | null = null;

afterEach(() => {
  mounted?.app.unmount();
  mounted?.host.remove();
  mounted = null;
});

const fleet = studying();
const sketches = () =>
  fleet
    .studios()
    .find((one) => one.name === "The Board's legend")!
    .nodes.filter((node) => node.kind === "sketch");
const pad = () => page.getByRole("dialog", { name: "Sketch" });
/** One act on what is picked, in the bar over it — the top bar has a Dispatch of its own. */
const act = (name: string) =>
  page.getByRole("group", { name: "What is picked" }).getByRole("button", { name, exact: true });

/** The Studio open and Continued, so it is the person's to add to. */
async function openEditable(): Promise<void> {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  mounted = { app: mountApp(fleet.scenario, host, undefined, SLICES), host };
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("cell", { name: "The Board's legend", exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  await expect.element(page.getByRole("group", { name: /^Note: The legend under the step bar/ })).toBeVisible();
}

/** A real press on a free point of empty board. */
async function pressBoard(): Promise<void> {
  const pane = document.querySelector<HTMLElement>(".armada-studio-whiteboard .react-flow__pane")!;
  const box = pane.getBoundingClientRect();
  for (let y = box.top + 40; y < box.bottom - 240; y += 60) {
    for (let x = box.left + 40; x < box.right - 280; x += 60) {
      if (document.elementFromPoint(x, y) === pane) {
        await userEvent.click(page.elementLocator(pane), { position: { x: x - box.left, y: y - box.top } });
        return;
      }
    }
  }
  throw new Error("no free point on the board");
}

/** Two boxes written on the open pad, and a line from the first to the second. */
async function drawTwoJoined(): Promise<void> {
  const inPad = pad();
  await inPad.getByRole("button", { name: "Add a box" }).click();
  await inPad.getByRole("textbox", { name: "The words in this box" }).fill("The rail");
  await inPad.getByRole("button", { name: "Add a box" }).click();
  await inPad.getByRole("group", { name: "An empty box" }).getByRole("textbox").fill("The board");
  await inPad.getByRole("group", { name: "Box: The rail" }).click();
  await userEvent.keyboard("{Meta>}");
  await inPad.getByRole("group", { name: "Box: The board" }).click();
  await userEvent.keyboard("{/Meta}");
  await inPad.getByRole("button", { name: "Join" }).click();
  await expect.element(inPad.getByRole("group", { name: /^A line from / })).toBeInTheDocument();
}

/** The Sketch placed last, picked on the board. */
async function pickPlaced(): Promise<string> {
  const placed = sketches().at(-1)!;
  const card = document.querySelector<HTMLElement>(`.react-flow__node[data-id="${placed.id}"]`)!;
  await userEvent.click(page.elementLocator(card));
  return placed.id;
}

test("a Sketch placed from the rail is drawn on the pad, kept when it closes, and reopens as it was left", async () => {
  await openEditable();
  const before = sketches().length;
  await page.getByRole("button", { name: "Add a Sketch" }).click();
  await pressBoard();
  await expect.element(pad()).toBeVisible();
  await drawTwoJoined();

  await pad().getByRole("button", { name: "Close" }).click();
  await expect.element(pad()).not.toBeInTheDocument();
  await expect.poll(() => sketches().length).toBe(before + 1);
  const kept = sketches().at(-1);
  expect(kept?.kind === "sketch" && kept.drawing.boxes.map((one) => one.body)).toEqual(["The rail", "The board"]);
  expect(kept?.kind === "sketch" && kept.drawing.joins).toHaveLength(1);
  // On the board, the card draws the drawing.
  await expect.element(page.getByRole("img", { name: "The sketch drawn on this Studio" }).first()).toBeVisible();

  await pickPlaced();
  await act("Open").click();
  await expect.element(pad().getByRole("group", { name: "Box: The rail" })).toBeVisible();
  await expect.element(pad().getByRole("group", { name: "Box: The board" })).toBeVisible();
  await expect.element(pad().getByRole("group", { name: /^A line from / })).toBeInTheDocument();
});

test("dispatching from a Sketch puts its drawing on the composer's pad, made from that node", async () => {
  await openEditable();
  const before = sketches().length;
  await page.getByRole("button", { name: "Add a Sketch" }).click();
  await pressBoard();
  await drawTwoJoined();
  await pad().getByRole("button", { name: "Close" }).click();
  await expect.element(pad()).not.toBeInTheDocument();
  await expect.poll(() => sketches().length).toBe(before + 1);

  const id = await pickPlaced();
  await act("Dispatch").click();
  // The drawing is attached at once — the chip — and drawn under Sketch.
  await expect.element(page.getByText("sketch 1", { exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "Sketch" }).click();
  const composer = page.getByLabelText("The picture attached to this request");
  await expect.element(composer).toBeVisible();
  await expect.element(composer.getByRole("group", { name: "Box: The rail" })).toBeVisible();
  await expect.element(composer.getByRole("group", { name: "Box: The board" })).toBeVisible();
  await expect.element(composer.getByRole("group", { name: /^A line from / })).toBeInTheDocument();
  await expect.element(page.getByText(id, { exact: true })).toBeVisible();
});
