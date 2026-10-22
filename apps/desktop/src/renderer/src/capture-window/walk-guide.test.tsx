// A press step shows a cursor going to the target and rings growing from it, then both are gone.

import { createRoot } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { commands } from "vitest/browser";

import { WalkGuide } from "@armada/components";
import "../styles/index.css";

const box = { x: 120, y: 200, width: 220, height: 32 };
const press = { x: 230, y: 216, n: 1 };

const draw = (withPress: boolean) => {
  const host = document.body.appendChild(document.createElement("div"));
  createRoot(host).render(<WalkGuide name="Findings" at={2} of={5} said="Open the menu" box={box} top={72} onFold={() => {}} {...(withPress ? { press } : {})} />);
  return host;
};

const settled = async () => {
  await vi.waitFor(() => expect(document.getAnimations().length).toBeGreaterThan(0));
  await Promise.all(document.getAnimations().map((one) => one.finished));
};

afterEach(async () => {
  document.body.replaceChildren();
  await commands.motion(false);
});

test("a press shows a cursor at the target's middle and rings that are gone once they have grown", async () => {
  // This project runs with reduced motion, which draws none of it.
  await commands.motion(true);
  const host = draw(true);
  const cursor = await vi.waitFor(() => {
    const found = host.querySelector<HTMLElement>(".armada-walk-guide__cursor");
    expect(found).not.toBeNull();
    return found!;
  });
  const rings = [...host.querySelectorAll<HTMLElement>(".armada-walk-guide__pulse")];
  expect(rings.length).toBeGreaterThan(0);

  await settled();
  const tip = cursor.getBoundingClientRect();
  expect([Math.round(tip.left), Math.round(tip.top)]).toEqual([press.x, press.y]);
  for (const ring of rings) expect(getComputedStyle(ring).opacity).toBe("0");
});

test("a step with no press draws no cursor and no rings", () => {
  const host = draw(false);
  expect(host.querySelector(".armada-walk-guide__cursor")).toBeNull();
  expect(host.querySelector(".armada-walk-guide__pulse")).toBeNull();
});

test("reduced motion draws no cursor and no rings", () => {
  const host = draw(true);
  expect(host.querySelector(".armada-walk-guide__cursor")).toBeNull();
  expect(host.querySelector(".armada-walk-guide__pulse")).toBeNull();
});
