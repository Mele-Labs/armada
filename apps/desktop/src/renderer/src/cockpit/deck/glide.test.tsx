// A battlefield layout that changes glides there from where it was drawn, instead of being drawn at once in its new place, and snaps
// where nothing moved or motion is reduced. Browser: the glide runs on animation frames and reads the motion setting off the page.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";

import { layoutBetween, sameGeometry, useGlide } from "./glide";
import { bigOf, layoutOf, type Layout, type RepoIn } from "./layout";
import { motion } from "../../mock/testing";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const SIZE = { width: 1000, height: 700 };
const BIG = bigOf(SIZE);
const repo = (root: string): RepoIn => ({ root, name: root, pads: ["slot-1"], goal: true, areas: [{ dir: "crates", kids: [["fleet", 40], ["store", 20]] }] });
/** One repository across the whole field, then a second beside it, which moves the first. */
const ONE = layoutOf([repo("/a")], [], SIZE, BIG);
const TWO = layoutOf([repo("/a"), repo("/b")], [], SIZE, BIG);
const rectOf = (layout: Layout, root: string) => layout.territories.find((one) => one.root === root)!.rect;

const mounted: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of mounted.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

/** Draws `useGlide` over each target given, and hands back what it drew last. */
function glide(first: Layout) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted.push({ root, host });
  const drawn: { last: Layout } = { last: first };
  function Probe({ target }: { target: Layout }) {
    drawn.last = useGlide(target, "1000x700", true);
    return null;
  }
  const show = (target: Layout) => act(() => root.render(<Probe target={target} />));
  show(first);
  return { drawn, show };
}

test("a territory is drawn between its old and new rects on the way", () => {
  const from = rectOf(ONE, "/a");
  const to = rectOf(TWO, "/a");
  expect(to.w).toBeLessThan(from.w);
  const half = rectOf(layoutBetween(ONE, TWO, 0.5), "/a");
  expect(half.x).toBeCloseTo((from.x + to.x) / 2);
  expect(half.w).toBeCloseTo((from.w + to.w) / 2);
  expect(rectOf(layoutBetween(ONE, TWO, 1), "/a").w).toBeCloseTo(to.w);
});

test("a territory the old layout lacks grows from its own middle", () => {
  const to = rectOf(TWO, "/b");
  const seed = rectOf(layoutBetween(ONE, TWO, 0), "/b");
  expect(seed.w).toBeLessThan(to.w / 2);
  expect(seed.x + seed.w / 2).toBeCloseTo(to.x + to.w / 2);
  expect(seed.y + seed.h / 2).toBeCloseTo(to.y + to.h / 2);
});

test("a layout that moved is not drawn in its new place at once", async () => {
  await motion();
  const { drawn, show } = glide(ONE);
  show(TWO);
  expect(drawn.last).not.toBe(TWO);
  expect(rectOf(drawn.last, "/a").w).toBeCloseTo(rectOf(ONE, "/a").w);
});

test("a layout where nothing moved is drawn at once", async () => {
  await motion();
  const again = layoutOf([repo("/a")], [], SIZE, BIG);
  expect(again).not.toBe(ONE);
  expect(sameGeometry(ONE, again)).toBe(true);
  const { drawn, show } = glide(ONE);
  show(again);
  expect(drawn.last).toBe(again);
});

test("under reduced motion, which the browser tests run with, a layout that moved is drawn at once", () => {
  const { drawn, show } = glide(ONE);
  show(TWO);
  expect(drawn.last).toBe(TWO);
});
