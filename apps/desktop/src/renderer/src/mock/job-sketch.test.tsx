// A Drone's sketch beside the Now panel stands in for the canvas while its ask is open. The owner can
// switch to the canvas and back, Next changes the sketch, an ask with none leaves the canvas, and
// hiding the panel or answering gives the canvas back.

import { describe, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const canvasNodes = () => document.querySelectorAll(".react-flow__node").length;
const sketch = () => document.querySelector('[role="img"][aria-label="Sketch"]');
/** The words in the drawing, which mermaid draws into the pane's shadow root. */
const drawn = () => sketch()?.querySelector(".armada-sketch__drawing")?.shadowRoot?.textContent ?? "";

async function settled(): Promise<void> {
  await expect.element(page.getByRole("region", { name: "Now" })).toBeVisible();
}

// One name, so the whole file runs as one Check: `armada check app_smoke "job sketch"`.
describe("job sketch", () => {
  test("a Plan decision with a sketch shows it, Next changes it, and a decision with none gives the canvas back", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await expect.poll(drawn).toContain("one file");
    expect(canvasNodes()).toBe(0);

    await page.getByText("Wrap it in place", { exact: true }).click();
    await page.getByRole("button", { name: "Next", exact: true }).click();
    await expect.poll(drawn).toContain("Wall clock");

    await page.getByText("Add a fake clock", { exact: true }).click();
    await page.getByRole("button", { name: "Next", exact: true }).click();
    await expect.poll(sketch).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
    await expect.element(page.getByRole("button", { name: "Canvas", exact: true })).not.toBeInTheDocument();
  });

  test("the switch puts the canvas back and the sketch again", async () => {
    mount("job-sketch-judge");
    await onScreen();
    await settled();
    await expect.poll(drawn).toContain("Retrying");
    await expect.element(page.getByRole("button", { name: "Sketch", exact: true })).toHaveAttribute("aria-pressed", "true");

    await page.getByRole("button", { name: "Canvas", exact: true }).click();
    await expect.poll(sketch).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
    await expect.element(page.getByRole("button", { name: "Canvas", exact: true })).toHaveAttribute("aria-pressed", "true");

    await page.getByRole("button", { name: "Sketch", exact: true }).click();
    await expect.poll(drawn).toContain("Retrying");
    expect(canvasNodes()).toBe(0);
  });

  test("hovering an option previews its sketch, and leaving returns to the decision's own", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await expect.poll(drawn).toContain("one file");

    await page.getByText("Split it out", { exact: true }).hover();
    await expect.poll(drawn).toContain("Clock");
    await expect.poll(drawn).not.toContain("one file");
    await page.getByText("Wrap it in place", { exact: true }).hover();
    await expect.poll(drawn).toContain("Wrapper");
    await page.getByRole("heading", { name: "Now" }).hover();
    await expect.poll(drawn).toContain("one file");
  });

  test("keyboard focus previews an option as hover does", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    const wrap = page.getByRole("radio", { name: "Wrap it in place" }).element() as HTMLInputElement;
    wrap.focus();
    await expect.poll(drawn).toContain("Wrapper");
    wrap.blur();
    await expect.poll(drawn).toContain("one file");
  });

  test("a picked option keeps its sketch after the pointer leaves, and another pick replaces it", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await page.getByText("Wrap it in place", { exact: true }).click();
    (document.activeElement as HTMLElement | null)?.blur();
    await page.getByRole("heading", { name: "Now" }).hover();
    await expect.poll(drawn).toContain("Wrapper");
    await page.getByText("Split it out", { exact: true }).click();
    (document.activeElement as HTMLElement | null)?.blur();
    await page.getByRole("heading", { name: "Now" }).hover();
    await expect.poll(drawn).not.toContain("Wrapper");
    await expect.poll(drawn).toContain("Clock");
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
    await expect.poll(drawn).toContain("Retrying");
    await page.getByRole("button", { name: "Hide now" }).click();
    await expect.poll(sketch).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
    await page.getByRole("button", { name: "Show now" }).click();
    await expect.poll(drawn).toContain("Retrying");
  });

  test("answering the last decision gives the canvas back", async () => {
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
});
