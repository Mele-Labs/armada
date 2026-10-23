// The live view's picture of a page: taken when the page loads or moves within itself, and every few seconds, and sent only when it differs.

import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { keepSnapshots, type SnapshotSource } from "./session-snapshot";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

/** A page whose picture is whatever `shows` holds, which can be told it changed, and whose capture the test can hold open. */
function page(first: number[] | null = [1]) {
  let shows: number[] | undefined = first ?? undefined;
  let changed = (): void => undefined;
  let release: (() => void) | undefined;
  let holding = false;
  const taken = vi.fn(async () => {
    // The page as it reads when the picture is taken, not when it reaches the caller.
    const read = shows;
    if (holding) await new Promise<void>((done) => (release = done));
    return read === undefined ? undefined : Uint8Array.from(read);
  });
  const source: SnapshotSource = { take: taken, onChange: (on) => void (changed = on) };
  return {
    source,
    taken,
    shows: (next: number[] | undefined) => void (shows = next),
    changed: () => changed(),
    hold: () => void (holding = true),
    letGo: () => {
      holding = false;
      release?.();
    },
  };
}

it("takes a picture when the page loads or moves, and sends it", async () => {
  const one = page([1, 2]);
  const sent: number[][] = [];
  keepSnapshots(one.source, (png) => sent.push([...png]), 3000);
  one.changed();
  await vi.advanceTimersByTimeAsync(0);
  expect(sent).toEqual([[1, 2]]);
});

it("takes another every few seconds while it is kept, and sends it only when it differs from the last", async () => {
  const one = page([1]);
  const sent: number[][] = [];
  keepSnapshots(one.source, (png) => sent.push([...png]), 3000);
  await vi.advanceTimersByTimeAsync(3000);
  await vi.advanceTimersByTimeAsync(3000);
  expect(one.taken).toHaveBeenCalledTimes(2);
  expect(sent).toEqual([[1]]);
  one.shows([9]);
  await vi.advanceTimersByTimeAsync(3000);
  expect(sent).toEqual([[1], [9]]);
});

it("sends nothing for a page that could not be taken, and stops taking once stopped", async () => {
  const one = page(null);
  const sent: number[][] = [];
  const stop = keepSnapshots(one.source, (png) => sent.push([...png]), 3000);
  await vi.advanceTimersByTimeAsync(3000);
  expect(sent).toEqual([]);
  stop();
  one.shows([5]);
  await vi.advanceTimersByTimeAsync(9000);
  expect(one.taken).toHaveBeenCalledTimes(1);
  expect(sent).toEqual([]);
});

it("does not take a second picture while one is being taken, and takes one more after it for what changed meanwhile", async () => {
  const one = page([1]);
  const sent: number[][] = [];
  keepSnapshots(one.source, (png) => sent.push([...png]), 3000);
  one.hold();
  one.changed();
  one.changed();
  await vi.advanceTimersByTimeAsync(0);
  expect(one.taken).toHaveBeenCalledTimes(1);
  one.shows([2]);
  one.letGo();
  await vi.advanceTimersByTimeAsync(0);
  expect(one.taken).toHaveBeenCalledTimes(2);
  expect(sent).toEqual([[1], [2]]);
});
