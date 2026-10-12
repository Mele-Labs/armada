// A long thread is drawn from its newest rows up, and is whole a few frames later.

import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";

import { FIRST_ROWS, useBackfilled } from "./backfilled";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const rows = (count: number): number[] => Array.from({ length: count }, (_, at) => at);

function Probe({ of, k }: { of: number[]; k: string }) {
  const drawn = useBackfilled(of, k);
  return <output data-first={drawn[0]} data-count={drawn.length} data-last={drawn[drawn.length - 1]} />;
}

function mount() {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  const show = (of: number[], k = "a") => flushSync(() => root.render(<Probe of={of} k={k} />));
  const out = () => host.querySelector("output")!.dataset;
  return { show, out };
}

test("draws the newest rows on the first frame and every row a few frames later", async () => {
  const { show, out } = mount();
  show(rows(401));
  expect(out().count).toBe(String(FIRST_ROWS));
  expect(out().last).toBe("400");
  await expect.poll(() => out().count).toBe("401");
  expect(out().first).toBe("0");
});

test("keeps a row that arrives while the rest is still being drawn, and drops none the person saw", async () => {
  const { show, out } = mount();
  show(rows(401));
  show(rows(402));
  expect(out().count).toBe(String(FIRST_ROWS + 1));
  expect(out().last).toBe("401");
  await expect.poll(() => out().count).toBe("402");
});

test("draws a short thread whole, and keeps it whole as it grows past the first frame's rows", () => {
  const { show, out } = mount();
  show(rows(FIRST_ROWS));
  expect(out().count).toBe(String(FIRST_ROWS));
  show(rows(FIRST_ROWS + 5));
  expect(out().count).toBe(String(FIRST_ROWS + 5));
  expect(out().first).toBe("0");
});

test("starts again for another thread", async () => {
  const { show, out } = mount();
  show(rows(401), "a");
  await expect.poll(() => out().count).toBe("401");
  show(rows(300), "b");
  expect(out().count).toBe(String(FIRST_ROWS));
  await expect.poll(() => out().count).toBe("300");
});

test("draws nothing for a thread with no rows yet, and holds back from the first rows that arrive", async () => {
  const { show, out } = mount();
  show([]);
  expect(out().count).toBe("0");
  show(rows(200));
  expect(out().count).toBe(String(FIRST_ROWS));
  await expect.poll(() => out().count).toBe("200");
});
