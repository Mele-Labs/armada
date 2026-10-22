import { expect, test, vi } from "vitest";
import type { FileChangeRead } from "@armada/screens/src/draft/sessions";

import { ASK_MS, LensAsks } from "./lens-asks";

/** A clock the test moves, and the timers it runs as it moves. */
function clock() {
  let now = 0;
  let id = 0;
  const due = new Map<number, { at: number; run: () => void }>();
  return {
    timers: { set: (run: () => void, ms: number) => (due.set(++id, { at: now + ms, run }), id), clear: (one: number) => void due.delete(one), now: () => now },
    async pass(ms: number) {
      const end = now + ms;
      for (;;) {
        const next = [...due].filter(([, one]) => one.at <= end).sort((a, b) => a[1].at - b[1].at)[0];
        if (next === undefined) break;
        due.delete(next[0]);
        now = next[1].at;
        next[1].run();
        await Promise.resolve();
        await Promise.resolve();
      }
      now = end;
    },
    pending: () => due.size,
  };
}

const settle = async () => {
  for (let k = 0; k < 4; k += 1) await Promise.resolve();
};

function asker(answer: (seen: string | undefined) => FileChangeRead) {
  const time = clock();
  const ask = vi.fn(async (_path: string, seen: string | undefined) => answer(seen));
  const answered = vi.fn();
  let held: string | undefined;
  const asks = new LensAsks(ask, () => held, (path, read) => {
    if (read !== undefined) held = read.stamp;
    answered(path, read);
  }, time.timers);
  return { time, ask, answered, asks };
}

test("a file being written is asked at once and then no more than once in two seconds, with the stamp held", async () => {
  const { time, ask, answered, asks } = asker((seen) => (seen === "s1" ? { ok: true, stamp: "s1", diff: "", unchanged: true } : { ok: true, stamp: "s1", diff: "d" }));
  asks.want(["a.ts"]);
  await settle();
  expect(ask).toHaveBeenCalledTimes(1);
  expect(ask).toHaveBeenLastCalledWith("a.ts", undefined);
  // Edits inside the two seconds ask nothing more until they have passed.
  asks.edited("a.ts");
  asks.edited("a.ts");
  await time.pass(ASK_MS - 1);
  expect(ask).toHaveBeenCalledTimes(1);
  await time.pass(1);
  expect(ask).toHaveBeenCalledTimes(2);
  expect(ask).toHaveBeenLastCalledWith("a.ts", "s1");
  // An unchanged file changes nothing.
  expect(answered).toHaveBeenCalledTimes(1);
  await time.pass(60_000);
  expect(ask.mock.calls.length).toBeLessThanOrEqual(32);
});

test("nothing is asked while nothing is wanted, and a file no longer written is asked no more", async () => {
  const { time, ask, asks } = asker(() => ({ ok: true, stamp: "s", diff: "d" }));
  asks.edited("a.ts");
  await time.pass(10_000);
  expect(ask).not.toHaveBeenCalled();
  asks.want(["a.ts"]);
  await settle();
  asks.stop();
  await time.pass(10_000);
  expect(ask).toHaveBeenCalledTimes(1);
  expect(time.pending()).toBe(0);
});

test("a refused file reports a refusal once and is asked again only after an edit names it", async () => {
  const { time, ask, answered, asks } = asker(() => ({ ok: false }));
  asks.want(["a.ts"]);
  await settle();
  expect(answered).toHaveBeenCalledWith("a.ts", undefined);
  await time.pass(10_000);
  expect(ask).toHaveBeenCalledTimes(1);
  asks.edited("a.ts");
  await settle();
  expect(ask).toHaveBeenCalledTimes(2);
});

test("a Fleet without the route is never asked again", async () => {
  const { time, ask, asks } = asker(() => ({ ok: false, noRoute: true }));
  asks.want(["a.ts"]);
  await settle();
  asks.want(["a.ts", "b.ts"]);
  asks.edited("a.ts");
  await time.pass(10_000);
  expect(ask).toHaveBeenCalledTimes(1);
});
