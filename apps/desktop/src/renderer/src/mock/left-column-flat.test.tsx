// The left column is flat while the canvas's cards keep the card treatment,
// through `App`: a computed style is only true of what the real cascade draws.
// The owner, 7 Oct 2026: "Remove the bevels and make this a bit more flat."

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const panels = () => [...document.querySelectorAll(".armada-shell__left > :is(.armada-sidebar, .armada-panel)")] as HTMLElement[];
const surface = (element: Element) => getComputedStyle(element);
const layer = (element: Element) => getComputedStyle(element, "::before");

test("Navigation (Work and Machine) and Fleet are flat, and the Dashboard's call is still a raised card", async () => {
  await page.viewport(1440, 900);
  mount("every-state");
  await expect.element(page.getByText("pid")).toBeVisible();

  const flat = panels();
  expect(flat).toHaveLength(3);
  for (const panel of flat) {
    expect(surface(panel).boxShadow).toBe("none");
    expect(surface(panel).backgroundImage).toBe("none");
    expect(layer(panel).content).toBe("none");
    expect(layer(panel).backdropFilter).toBe("none");
  }

  await expect.poll(() => document.querySelector(".armada-callcard")).not.toBeNull();
  expect(surface(document.querySelector(".armada-callcard")!).boxShadow).not.toBe("none");
});
