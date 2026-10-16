import { expect, test } from "vitest";

import { LEAVE_MS, SWITCH_MS, steadyTips, type Shown } from "./tip-hover";

function clocked() {
  let now = 0;
  let next = 1;
  const waiting = new Map<number, { at: number; run: () => void }>();
  const tips = steadyTips({
    set: (run, ms) => (waiting.set(next, { at: now + ms, run }), next++),
    clear: (id) => void waiting.delete(id),
  });
  const settled: (Shown | undefined)[] = [];
  tips.onSettle((one) => settled.push(one));
  const advance = (ms: number) => {
    now += ms;
    for (const [id, one] of [...waiting]) if (one.at <= now) (waiting.delete(id), one.run());
  };
  return { tips, settled, advance };
}

const thing = (name: string): Shown => ({ tip: { label: name, title: name, hue: "calm" }, anchor: { target: { name } as unknown as Element }, hot: name });
const A = thing("a");
const B = thing("b");

test("crossing off a thing and on to it again changes nothing", () => {
  const { tips, settled, advance } = clocked();
  tips.show(A);
  tips.leave();
  advance(LEAVE_MS - 10);
  tips.show(A);
  advance(LEAVE_MS * 3);
  expect(settled).toHaveLength(1);
  expect(tips.get()?.hot).toBe("a");
});

test("it goes only after the pointer has been off for a moment", () => {
  const { tips, settled, advance } = clocked();
  tips.show(A);
  tips.leave();
  advance(LEAVE_MS - 1);
  expect(tips.get()).toBeDefined();
  advance(1);
  expect(tips.get()).toBeUndefined();
  expect(settled).toHaveLength(2);
});

test("another thing is shown only once it has held the pointer, and a pass across it shows nothing", () => {
  const { tips, advance } = clocked();
  tips.show(A);
  tips.leave();
  tips.show(B);
  advance(SWITCH_MS - 1);
  expect(tips.get()?.hot).toBe("a");
  tips.leave();
  advance(SWITCH_MS);
  expect(tips.get()?.hot).toBe("a");
  advance(LEAVE_MS);
  expect(tips.get()).toBeUndefined();

  tips.show(A);
  tips.leave();
  tips.show(B);
  advance(SWITCH_MS);
  expect(tips.get()?.hot).toBe("b");
});

test("clear is at once", () => {
  const { tips } = clocked();
  tips.show(A);
  tips.clear();
  expect(tips.get()).toBeUndefined();
});
