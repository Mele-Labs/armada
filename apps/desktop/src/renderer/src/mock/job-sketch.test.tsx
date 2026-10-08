// A Drone's sketch beside the Now panel stands in for the canvas while its ask is open. The owner can
// switch to the canvas and back, Next changes the sketch, an ask with none leaves the canvas, and
// hiding the panel or answering gives the canvas back.

import { describe, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const canvasNodes = () => document.querySelectorAll(".react-flow__node").length;
const sketch = () => document.querySelector('[role="img"][aria-label="Sketch"]');

async function settled(): Promise<void> {
  await expect.element(page.getByRole("region", { name: "Now" })).toBeVisible();
}

// One name, so the whole file runs as one Check: `armada check app_smoke "job sketch"`.
describe("job sketch", () => {
  test("a Plan decision with a sketch shows it, Next changes it, and a decision with none gives the canvas back", async () => {
    mount("job-sketch-plan");
    await onScreen();
    await settled();
    await expect.poll(() => sketch()?.textContent).toContain("Writer");
    expect(canvasNodes()).toBe(0);

    await page.getByText("Wrap it in place", { exact: true }).click();
    await page.getByRole("button", { name: "Next", exact: true }).click();
    await expect.poll(() => sketch()?.textContent).toContain("Pinned clock");

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
    await expect.poll(() => sketch()?.textContent).toContain("Retrying");
    await expect.element(page.getByRole("button", { name: "Sketch", exact: true })).toHaveAttribute("aria-pressed", "true");

    await page.getByRole("button", { name: "Canvas", exact: true }).click();
    await expect.poll(sketch).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
    await expect.element(page.getByRole("button", { name: "Canvas", exact: true })).toHaveAttribute("aria-pressed", "true");

    await page.getByRole("button", { name: "Sketch", exact: true }).click();
    await expect.poll(() => sketch()?.textContent).toContain("Retrying");
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
    await expect.poll(() => sketch()?.textContent).toContain("Retrying");
    await page.getByRole("button", { name: "Hide now" }).click();
    await expect.poll(sketch).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
    await page.getByRole("button", { name: "Show now" }).click();
    await expect.poll(() => sketch()?.textContent).toContain("Retrying");
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
