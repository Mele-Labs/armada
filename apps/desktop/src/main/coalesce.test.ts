// A burst of publishes reaches the windows as one send — a Job's backfill
// replays up to 2048 rows, and each used to cost a structured clone of the
// whole state on both sides of the IPC.

import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { coalesce, FRAME_MS } from "./coalesce";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

it("sends a burst once, carrying the last value", async () => {
  const send = vi.fn();
  const publish = coalesce<number>(send);

  for (let row = 1; row <= 2048; row += 1) publish(row);
  expect(send).not.toHaveBeenCalled();

  await vi.advanceTimersByTimeAsync(FRAME_MS);
  expect(send).toHaveBeenCalledExactlyOnceWith(2048);
});

it("still sends what arrives after a flush", async () => {
  const send = vi.fn();
  const publish = coalesce<string>(send);

  publish("first");
  await vi.advanceTimersByTimeAsync(FRAME_MS);
  publish("later");
  await vi.advanceTimersByTimeAsync(FRAME_MS);

  expect(send.mock.calls).toEqual([["first"], ["later"]]);
});
