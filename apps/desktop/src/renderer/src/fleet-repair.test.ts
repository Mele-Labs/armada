// Which act the notice over Fleet offers: down starts it, and a mismatch mends the stale side.

import { expect, test } from "vitest";
import { repairOf, staleSide } from "@armada/shell";

const OLD = "2026-11-12T211515Z-2f88990";
const NEW = "2026-11-13T004852Z-126b4bb";

test("a stale Fleet is restarted", () => {
  expect(
    staleSide(OLD, {
      bridge: NEW,
      current: NEW,
    }),
  ).toBe("restart");
});

test("a stale Bridge is reopened", () => {
  expect(
    staleSide(NEW, {
      bridge: OLD,
      current: NEW,
    }),
  ).toBe("reopen");
});

test("both stale restarts Fleet, which is the side that comes back first", () => {
  expect(
    staleSide(OLD, {
      bridge: OLD,
      current: NEW,
    }),
  ).toBe("restart");
});

test("neither from a kept build restarts Fleet, as a mismatch always has", () => {
  expect(
    staleSide(null, {
      bridge: null,
      current: null,
    }),
  ).toBe("restart");
  expect(
    staleSide(null, {
      bridge: null,
      current: NEW,
    }),
  ).toBe("restart");
});

test("a Fleet that names no build leaves a stale Bridge to be reopened", () => {
  expect(
    staleSide(null, {
      bridge: OLD,
      current: NEW,
    }),
  ).toBe("reopen");
});

test("down starts Fleet, a mismatch waits for the builds, and anything else offers nothing", () => {
  const read = {
    bridge: NEW,
    current: NEW,
  };
  expect(repairOf("bridge.fleet.not_running", null, null)).toBe("start");
  expect(repairOf("bridge.fleet.protocol_mismatch", OLD, null)).toBeNull();
  expect(repairOf("bridge.fleet.protocol_mismatch", OLD, read)).toBe("restart");
  expect(repairOf("bridge.fleet.unreachable", null, read)).toBeNull();
  expect(repairOf("bridge.fleet.runtime_file_refused", null, read)).toBeNull();
});
