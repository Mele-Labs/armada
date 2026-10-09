// A Drone's sketch beside the Now panel stands in for the canvas while its ask is open. Hovering or
// focusing a Plan option draws its sketch against the current one, a pick keeps it, a press on a part
// asks about it, the view pans and zooms, the arrows flow and the owner draws on top.

import { describe, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const sketch = () => document.querySelector<HTMLElement>('figure[aria-label="Sketch"]');
const titles = () => [...(sketch()?.querySelectorAll(".armada-scene-node__title") ?? [])].map((one) => one.textContent ?? "").join("|");
const inSketch = (selector: string) => sketch()?.querySelectorAll(selector).length ?? 0;
const canvasNodes = () => [...document.querySelectorAll(".react-flow__node")].filter((one) => one.closest(".armada-sketch") === null).length;
const viewport = () => sketch()?.querySelector<HTMLElement>(".react-flow__viewport")?.style.transform ?? "";
const scale = () => Number(/scale\(([\d.]+)\)/.exec(viewport())?.[1] ?? "0");

async function settled(): Promise<void> {
  await expect.element(page.getByRole("region", { name: "Now" })).toBeVisible();
}

async function drawn(): Promise<void> {
  await expect.poll(() => inSketch(".react-flow__node")).toBeGreaterThan(2);
}

const leave = () => page.getByRole("heading", { name: "Now" }).hover();

// One name, so the whole file runs as one Check: `armada check app_smoke "job sketch"`.
describe("job sketch", () => {
  test("a Plan decision draws its sketch, Next changes it, and a decision with none gives the canvas back", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await drawn();
    await expect.poll(titles).toContain("Writer");
    expect(canvasNodes()).toBe(0);
    expect(inSketch("[data-change]")).toBe(0);

    await page.getByText("Wrap it in place", { exact: true }).click();
    await page.getByRole("button", { name: "Next", exact: true }).click();
    await expect.poll(() => inSketch(".armada-scene-node[data-kind='lane']")).toBe(2);
    await expect.poll(titles).toContain("Wall clock");

    await page.getByText("Add a fake clock", { exact: true }).click();
    await page.getByRole("button", { name: "Next", exact: true }).click();
    await expect.poll(sketch).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
  });

  test("the switch puts the canvas back and the sketch again", async () => {
    mount("job-sketch-judge");
    await onScreen();
    await settled();
    await drawn();
    await expect.poll(titles).toContain("Retrying");
    await expect.element(page.getByRole("button", { name: "Sketch", exact: true })).toHaveAttribute("aria-pressed", "true");

    await page.getByRole("button", { name: "Canvas", exact: true }).click();
    await expect.poll(sketch).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);

    await page.getByRole("button", { name: "Sketch", exact: true }).click();
    await drawn();
    expect(canvasNodes()).toBe(0);
  });

  test("an ask with no sketch leaves the canvas, with no switch", async () => {
    mount("job-sketch-none");
    await onScreen();
    await settled();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
    expect(sketch()).toBeNull();
    await expect.element(page.getByRole("button", { name: "Sketch", exact: true })).not.toBeInTheDocument();
  });

  test("hiding the panel gives the canvas back, and showing it brings the sketch", async () => {
    mount("job-sketch-judge");
    await onScreen();
    await settled();
    await drawn();
    await page.getByRole("button", { name: "Hide now" }).click();
    await expect.poll(sketch).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
    await page.getByRole("button", { name: "Show now" }).click();
    await drawn();
  });

  test("answering the last decision gives the canvas back", { timeout: 45_000 }, async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await page.getByText("Wrap it in place", { exact: true }).click();
    await page.getByRole("button", { name: "Next", exact: true }).click();
    await page.getByText("Add a fake clock", { exact: true }).click();
    await page.getByRole("button", { name: "Next", exact: true }).click();
    await page.getByText("Take it here", { exact: true }).click();
    await page.getByRole("button", { name: "Answer", exact: true }).click();
    await expect.poll(sketch).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
  });

  test("hovering an option draws its sketch against the current one, and leaving returns to it", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await drawn();

    await page.getByText("Split it out", { exact: true }).hover();
    await expect.poll(() => inSketch('[data-change="added"]')).toBeGreaterThan(0);
    await expect.poll(() => inSketch('[data-change="changed"]')).toBeGreaterThan(0);
    await expect.poll(titles).toContain("Clock");
    await page.getByText("Wrap it in place", { exact: true }).hover();
    await expect.poll(titles).toContain("Wrapper");
    await expect.poll(() => inSketch('[data-change="removed"]')).toBeGreaterThan(0);
    await leave();
    await expect.poll(() => inSketch("[data-change]")).toBe(0);
    expect(titles()).not.toContain("Wrapper");
  });

  test("added, removed and changed parts are painted apart, and what stays stands back", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await drawn();
    await page.getByText("Wrap it in place", { exact: true }).hover();
    await expect.poll(() => inSketch('[data-change="added"]')).toBeGreaterThan(0);
    await expect.poll(() => inSketch('.armada-scene-edge[data-change="removed"]')).toBeGreaterThan(0);
    const edge = (change: string) => getComputedStyle(sketch()!.querySelector(`.armada-scene-edge[data-change="${change}"] .armada-scene-edge__line`)!).stroke;
    const added = edge("added");
    const removed = edge("removed");
    expect(added).not.toBe(removed);
    await page.getByText("Split it out", { exact: true }).hover();
    await expect.poll(() => inSketch('.armada-scene-node[data-change="changed"]')).toBeGreaterThan(0);
    const frame = (change: string) => getComputedStyle(sketch()!.querySelector(`.armada-scene-node[data-change="${change}"]`)!).borderColor;
    const colours = new Set([frame("added"), frame("changed"), frame("same")]);
    expect(colours.size).toBe(3);
    expect(Number(getComputedStyle(sketch()!.querySelector('.armada-scene-node[data-change="same"]')!).opacity)).toBeLessThan(1);
    expect(Number(getComputedStyle(sketch()!.querySelector('.armada-scene-node[data-change="added"]')!).opacity)).toBe(1);
  });

  test("keyboard focus draws an option's sketch as hover does", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await drawn();
    await leave();
    const wrap = page.getByRole("radio", { name: "Wrap it in place" }).element() as HTMLInputElement;
    wrap.focus();
    await expect.poll(titles).toContain("Wrapper");
    wrap.blur();
    await expect.poll(() => inSketch("[data-change]")).toBe(0);
  });

  test("a picked option keeps its sketch after the pointer leaves, and another pick replaces it", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await drawn();
    await page.getByText("Wrap it in place", { exact: true }).click();
    (document.activeElement as HTMLElement | null)?.blur();
    await leave();
    await expect.poll(titles).toContain("Wrapper");
    await page.getByText("Split it out", { exact: true }).click();
    (document.activeElement as HTMLElement | null)?.blur();
    await leave();
    await expect.poll(titles).not.toContain("Wrapper");
    await expect.poll(titles).toContain("Clock");
  });

  test("a press on a part asks about it, and the question lands under the ask", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await drawn();
    await page.elementLocator(sketch()!.querySelector<HTMLElement>('.react-flow__node[data-id="writer"]')!).click();
    const ask = page.getByRole("textbox", { name: "Ask about Writer" });
    await expect.element(ask).toBeVisible();
    await ask.fill("Does it still flush on close?");
    await page.getByRole("button", { name: "Ask", exact: true }).click();
    await expect.element(page.getByRole("region", { name: "Now" }).getByText("Does it still flush on close?")).toBeVisible();
    await expect.element(page.getByRole("region", { name: "Now" }).getByText("Writer", { exact: true })).toBeVisible();
    await expect.element(page.getByRole("textbox", { name: "Ask about Writer" })).not.toBeInTheDocument();
  });

  test("the sketch zooms and pans, and fit brings it back", async () => {
    mount("job-sketch-judge");
    await onScreen();
    await settled();
    await drawn();
    await expect.poll(scale).toBeGreaterThan(0);
    const start = scale();
    const startAt = viewport();
    await page.getByRole("button", { name: "Zoom in" }).click();
    await expect.poll(scale).toBeGreaterThan(start);
    await page.getByRole("button", { name: "Fit" }).click();
    await expect.poll(scale).toBeCloseTo(start, 1);

    sketch()!.focus();
    await page.getByRole("button", { name: "Fit" }).click();
    sketch()!.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true }));
    await expect.poll(viewport).not.toBe(startAt);
    sketch()!.dispatchEvent(new KeyboardEvent("keydown", { key: "-", bubbles: true }));
    await expect.poll(scale).toBeLessThan(start);
  });

  test("arrows flow and Play the steps lights the nodes in order", async () => {
    mount("job-sketch-judge");
    await onScreen();
    await settled();
    await drawn();
    expect(inSketch(".armada-scene-edge[data-flow]")).toBeGreaterThan(0);
    await page.getByRole("button", { name: "Play the steps" }).click();
    await expect.poll(() => sketch()?.querySelector("[data-current] .armada-scene-node__title")?.textContent).toBe("Waiting");
    await expect.poll(() => inSketch(".armada-scene-node[data-dim]")).toBeGreaterThan(0);
  });

  test("the owner draws on top, and the line is there when the sketch comes back", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await drawn();
    await page.getByRole("button", { name: "Draw" }).click();
    const pen = sketch()!.querySelector(".armada-sketch-pad__pen")!;
    const at = pen.getBoundingClientRect();
    pen.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, clientX: at.left + 60, clientY: at.top + 60 }));
    window.dispatchEvent(new PointerEvent("pointermove", { clientX: at.left + 120, clientY: at.top + 90 }));
    window.dispatchEvent(new PointerEvent("pointermove", { clientX: at.left + 180, clientY: at.top + 70 }));
    window.dispatchEvent(new PointerEvent("pointerup", {}));
    await expect.poll(() => inSketch(".armada-sketch-pad__ink path[role='img']")).toBe(1);

    await page.getByText("Split it out", { exact: true }).hover();
    await expect.poll(titles).toContain("Clock");
    expect(inSketch(".armada-sketch-pad__ink path[role='img']")).toBe(0);
    await leave();
    await expect.poll(() => inSketch(".armada-sketch-pad__ink path[role='img']")).toBe(1);
    await page.getByRole("button", { name: "Undo" }).click();
    await expect.poll(() => inSketch(".armada-sketch-pad__ink path[role='img']")).toBe(0);
  });
});
