// The canvas beside the Now panel: every step a panel row belongs to stays lit and every other node
// stands back under a scrim, and a hidden panel or a panel with no step named dims nothing.

import { describe, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** Which canvas nodes stand back, by node id. */
function dimmed(): Record<string, boolean> {
  return Object.fromEntries(
    [...document.querySelectorAll(".react-flow__node")].map((node) => [node.getAttribute("data-id") ?? "", node.querySelector(".armada-now-dimmed") !== null]),
  );
}

async function settled(): Promise<void> {
  await expect.element(page.getByRole("region", { name: "Now" })).toBeVisible();
  await expect.poll(() => document.querySelectorAll(".react-flow__node").length).toBeGreaterThan(5);
}

// One name, so the whole file runs as one Check: `armada check app_smoke "job now canvas focus"`.
describe("job now canvas focus", () => {
  test("the step a row belongs to stays lit, with what hangs on it, and everything else stands back", async () => {
    mount("job-now-drone");
    await onScreen();
    await settled();

    const nodes = dimmed();
    for (const lit of ["implement", "implement:checks", "implement:judge", "group:G1", "task:T1"]) expect(nodes[lit], lit).toBe(false);
    for (const back of ["plan", "plan:checks", "tests", "tests:checks", "handoff", "handoff:judge", "brief", "base", "done", "pr", "land"]) {
      expect(nodes[back], back).toBe(true);
    }

    // A scrim that paints something, or a dim nobody can see is a dim that is not there.
    const scrim = getComputedStyle(document.querySelector(".armada-now-dimmed")!, "::after").backgroundColor;
    expect(scrim).not.toBe("rgba(0, 0, 0, 0)");
    expect(scrim).not.toBe("transparent");
  });

  test("a panel with no step named dims nothing", async () => {
    mount("job-now-idle");
    await onScreen();
    await settled();
    expect(Object.values(dimmed()).some(Boolean)).toBe(false);
  });

  test("a hidden panel lets the canvas go, and showing it focuses again", async () => {
    mount("job-now-drone");
    await onScreen();
    await settled();
    expect(Object.values(dimmed()).some(Boolean)).toBe(true);
    await page.getByRole("button", { name: "Hide now" }).click();
    await expect.poll(() => Object.values(dimmed()).some(Boolean)).toBe(false);
    await page.getByRole("button", { name: "Show now" }).click();
    await expect.poll(() => Object.values(dimmed()).some(Boolean)).toBe(true);
  });
});
