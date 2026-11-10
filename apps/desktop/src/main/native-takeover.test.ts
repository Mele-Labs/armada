// Classic's banner, sound and badge stand down while the native app runs, and come back when it quits.

import { afterEach, describe, expect, it, vi } from "vitest";

import { NATIVE_BUNDLE_ID, NativeTakeover, foundRunning, watchNative } from "./native-takeover";

function watching(): {
  takeover: NativeTakeover;
  badges: string[];
} {
  const badges: string[] = [];
  return {
    takeover: new NativeTakeover({
      badge: (label) => badges.push(label),
    }),
    badges,
  };
}

describe("looking for the native app", () => {
  it("names the native bundle, apart from Classic's own", () => {
    expect(NATIVE_BUNDLE_ID).toBe("dev.armada.bridge.native");
    expect(NATIVE_BUNDLE_ID).not.toBe("dev.armada.bridge");
  });

  it("reads an ASN line as running and an empty answer as not", () => {
    expect(foundRunning('ASN:0x0-0x3ed2ecf-"Armada":\n')).toBe(true);
    expect(foundRunning("\n")).toBe(false);
    expect(foundRunning("")).toBe(false);
  });
});

describe("Classic while the native app is not running", () => {
  it("is active for nothing and badges the count as before", () => {
    const { takeover, badges } = watching();
    takeover.read(false);
    takeover.dockCount(3);
    takeover.dockCount(0);
    expect(takeover.active).toBe(false);
    expect(badges).toEqual([
      "3",
      "",
    ]);
  });
});

describe("Classic while the native app runs", () => {
  it("stands down: no badge is written for a count, and the one showing is cleared", () => {
    const { takeover, badges } = watching();
    takeover.dockCount(2);
    takeover.read(true);
    takeover.dockCount(5);
    expect(takeover.active).toBe(true);
    expect(badges).toEqual([
      "2",
      "",
    ]);
  });

  it("takes the badge back, with the count that stood when the native app quit", () => {
    const { takeover, badges } = watching();
    takeover.read(true);
    takeover.dockCount(4);
    takeover.read(false);
    expect(takeover.active).toBe(false);
    expect(badges).toEqual([
      "",
      "4",
    ]);
  });

  it("writes nothing for a reading that has not changed", () => {
    const { takeover, badges } = watching();
    takeover.read(true);
    takeover.read(true);
    takeover.read(false);
    takeover.read(false);
    expect(badges).toEqual([
      "",
      "",
    ]);
  });
});

describe("watching", () => {
  afterEach(() => vi.useRealTimers());

  it("looks at once and again each interval, and a failed look changes nothing", async () => {
    vi.useFakeTimers();
    const { takeover } = watching();
    const answers: Array<boolean | null> = [
      true,
      null,
      false,
    ];
    const look = vi.fn(() => Promise.resolve(answers.shift() ?? null));
    const stop = watchNative(takeover, look, 1000);
    await vi.advanceTimersByTimeAsync(0);
    expect(takeover.active).toBe(true);
    await vi.advanceTimersByTimeAsync(1000);
    expect(takeover.active).toBe(true);
    await vi.advanceTimersByTimeAsync(1000);
    expect(takeover.active).toBe(false);
    stop();
    expect(look).toHaveBeenCalledTimes(3);
  });
});
