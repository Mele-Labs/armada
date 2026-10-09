// What stands in for the canvas on the left while something asks: the sketch where the ask drew one,
// else the asker's own view, else the canvas. The owner's switch offers what applies, and answering or
// hiding the panel gives the canvas back.

import { describe, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const asker = () => document.querySelector<HTMLElement>('figure[aria-label="Asker"]');
const sketch = () => document.querySelector<HTMLElement>('figure[aria-label="Sketch"]');
const canvasNodes = () => [...document.querySelectorAll(".react-flow__node")].filter((one) => one.closest(".armada-sketch") === null).length;

async function settled(): Promise<void> {
  await expect.element(page.getByRole("region", { name: "Now" })).toBeVisible();
}

const leave = () => page.getByRole("heading", { name: "Now" }).hover();

// One name, so the whole file runs as one Check: `armada check app_smoke "job asker"`.
describe("job asker", () => {
  test("a Drone question with no sketch shows the Drone: its output, what it changed and the file it asks about", async () => {
    mount("job-asker-drone");
    await onScreen();
    await settled();
    await expect.poll(() => asker()?.textContent).toContain("Implement Drone");
    expect(sketch()).toBeNull();
    expect(canvasNodes()).toBe(0);
    expect(asker()!.querySelector("[aria-label='Output']")?.textContent).toContain("two clocks reach the fixture");
    expect(asker()!.querySelector("[data-asking]")?.textContent).toContain("crates/store/tests/fixtures.rs");
    expect(asker()!.querySelector("[data-asking] [aria-label='Asking about this']")).not.toBeNull();
    expect(asker()!.querySelector("[data-sign='+']")).not.toBeNull();
    expect(asker()!.querySelector("[data-sign='-']")).not.toBeNull();
    await expect.element(page.getByRole("button", { name: "Asker", exact: true })).toHaveAttribute("aria-pressed", "true");
    await expect.element(page.getByRole("button", { name: "Sketch", exact: true })).not.toBeInTheDocument();
  });

  test("the switch offers the canvas and the asker back, and a press on a file opens it", async () => {
    mount("job-asker-drone");
    await onScreen();
    await settled();
    await expect.poll(asker).not.toBeNull();
    await page.getByRole("button", { name: "Canvas", exact: true }).click();
    await expect.poll(asker).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
    await page.getByRole("button", { name: "Asker", exact: true }).click();
    await expect.poll(asker).not.toBeNull();
    expect(canvasNodes()).toBe(0);
    await page.getByRole("button", { name: /^Open Changed, crates\/store\/tests\/fixtures\.rs/ }).click();
    await expect.element(page.getByText("Open crates/store/tests/fixtures.rs")).toBeVisible();
  });

  test("a Judge question shows its work product beside the checks it is reading", async () => {
    mount("job-asker-judge");
    await onScreen();
    await settled();
    await expect.poll(asker).not.toBeNull();
    const product = asker()!.querySelector("[aria-label='Work product']")!.getBoundingClientRect();
    const checks = asker()!.querySelector("[aria-label='Checks it is reading']")!.getBoundingClientRect();
    expect(Math.abs(product.top - checks.top)).toBeLessThan(2);
    expect(checks.left).toBeGreaterThan(product.left);
    expect(asker()!.textContent).toContain("clippy::needless_return");
  });

  test("a Plan question shows the Plan Drone, an option's sketch while it is hovered or picked, and the canvas once answered", async () => {
    mount("job-asker-plan");
    await onScreen();
    await settled();
    await expect.poll(() => asker()?.textContent).toContain("Plan Drone");
    expect(sketch()).toBeNull();

    await page.getByText("Split it out", { exact: true }).hover();
    await expect.poll(sketch).not.toBeNull();
    expect(asker()).toBeNull();
    await leave();
    await expect.poll(asker).not.toBeNull();
    expect(sketch()).toBeNull();

    await page.getByText("Split it out", { exact: true }).click();
    (document.activeElement as HTMLElement | null)?.blur();
    await leave();
    await expect.poll(sketch).not.toBeNull();

    await page.getByRole("button", { name: "Next", exact: true }).click();
    await expect.poll(() => asker()?.textContent).toContain("Plan Drone");
    expect(sketch()).toBeNull();
    await page.getByText("Take it here", { exact: true }).click();
    await page.getByRole("button", { name: "Answer", exact: true }).click();
    await expect.poll(asker).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
  });

  test("hiding the panel gives the canvas back", async () => {
    mount("job-asker-drone");
    await onScreen();
    await settled();
    await expect.poll(asker).not.toBeNull();
    await page.getByRole("button", { name: "Hide now" }).click();
    await expect.poll(asker).toBeNull();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
    // Hidden is the window's to remember, so leave it as found.
    await page.getByRole("button", { name: "Show now" }).click();
  });

  test("an ask with a sketch shows the sketch, and no ask leaves the canvas", async () => {
    mount("job-sketch-judge");
    await onScreen();
    await settled();
    await expect.poll(sketch).not.toBeNull();
    expect(asker()).toBeNull();
  });

  test("with nothing asking the canvas stays, and an ask with neither sketch nor asker leaves it too", async () => {
    mount("job-now-drone");
    await onScreen();
    await settled();
    await expect.poll(canvasNodes).toBeGreaterThan(5);
    expect(asker()).toBeNull();
    expect(sketch()).toBeNull();
  });
});
