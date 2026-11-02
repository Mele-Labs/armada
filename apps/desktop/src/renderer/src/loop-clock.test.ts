import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { startLoopClock } from "./loop-clock";

/** The parts of a CSS animation the clock touches. */
function loop(currentTime: number, iterations = Infinity, target = "running"): Animation {
  return {
    currentTime,
    effect: {
      getComputedTiming: () => ({
        iterations,
      }),
      target,
    },
    pause: vi.fn(),
  } as unknown as Animation;
}

let now = 1000;
let animations: Animation[] = [];
let marks = new Set<string>();
let stop: () => void = () => undefined;

beforeEach(() => {
  vi.useFakeTimers();
  now = 1000;
  marks = new Set();
  vi.stubGlobal("document", {
    documentElement: {
      hasAttribute: (name: string) => marks.has(name),
    },
    timeline: {
      get currentTime() {
        return now;
      },
    },
    getAnimations: () => animations,
  });
  vi.stubGlobal("getComputedStyle", (target: string) => ({
    animationPlayState: target,
  }));
});

afterEach(() => {
  stop();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

function advance(ms: number): void {
  now += ms;
  vi.advanceTimersByTime(ms);
}

test("moves every loop on the same tick and keeps each one's phase", () => {
  const first = loop(900);
  const second = loop(300);
  animations = [
    first,
    second,
  ];
  stop = startLoopClock();

  advance(200);
  expect(first.pause).toHaveBeenCalled();
  expect(second.pause).toHaveBeenCalled();
  expect(first.currentTime).toBe(900);
  expect(second.currentTime).toBe(300);

  advance(200);
  expect(first.currentTime).toBe(1100);
  expect(second.currentTime).toBe(500);
});

test("leaves an animation that ends alone", () => {
  const entrance = loop(50, 1);
  animations = [
    entrance,
  ];
  stop = startLoopClock();

  advance(600);
  expect(entrance.pause).not.toHaveBeenCalled();
  expect(entrance.currentTime).toBe(50);
});

test("moves nothing while the window is still", () => {
  const one = loop(0);
  animations = [
    one,
  ];
  stop = startLoopClock();
  advance(200);

  marks.add("data-still");
  advance(600);
  expect(one.currentTime).toBe(0);
});

test("holds a loop the idle rules pause, and moves the others", () => {
  const running = loop(0);
  const held = loop(0, Infinity, "paused");
  animations = [
    running,
    held,
  ];
  stop = startLoopClock();
  advance(200);

  marks.add("data-idle");
  advance(200);
  expect(running.currentTime).toBe(200);
  expect(held.currentTime).toBe(0);
});
