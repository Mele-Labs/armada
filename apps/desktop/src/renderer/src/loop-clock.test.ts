import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { startLoopClock } from "./loop-clock";

/** The parts of a CSS animation the clock touches. */
function loop(currentTime: number, iterations = Infinity, target: unknown = {}, duration = 1600): Animation {
  return { currentTime, effect: { getComputedTiming: () => ({ iterations, duration, endTime: iterations * duration }), target }, pause: vi.fn(), finish: vi.fn() } as unknown as Animation;
}

let now = 1000;
let animations: Animation[] = [];
let marks = new Set<string>();
let stop: () => void = () => undefined;
let sight: (changes: { target: unknown; isIntersecting: boolean }[]) => void = () => undefined;

beforeEach(() => {
  vi.useFakeTimers();
  now = 1000;
  marks = new Set();
  vi.stubGlobal("document", {
    documentElement: { hasAttribute: (name: string) => marks.has(name) },
    timeline: { get currentTime() { return now; } },
    getAnimations: () => animations,
  });
  vi.stubGlobal(
    "IntersectionObserver",
    class {
      constructor(hear: (changes: { target: unknown; isIntersecting: boolean }[]) => void) {
        sight = hear;
      }
      observe(): void {}
      disconnect(): void {}
    },
  );
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
  animations = [first, second];
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
  animations = [entrance];
  stop = startLoopClock();

  advance(600);
  expect(entrance.pause).not.toHaveBeenCalled();
  expect(entrance.currentTime).toBe(50);
});

test("steps a slow fade on the same tick, holds it while idle, and finishes it at its end", () => {
  const fade = loop(0, 1, {}, 45_000);
  animations = [fade];
  stop = startLoopClock();

  advance(200);
  expect(fade.pause).toHaveBeenCalled();
  advance(200);
  expect(fade.currentTime).toBe(200);

  marks.add("data-idle");
  advance(600);
  expect(fade.currentTime).toBe(200);
  marks.delete("data-idle");

  advance(45_000);
  expect(fade.finish).toHaveBeenCalled();
});

test("moves nothing while the window is still", () => {
  const one = loop(0);
  animations = [one];
  stop = startLoopClock();
  advance(200);

  marks.add("data-still");
  advance(600);
  expect(one.currentTime).toBe(0);
});

test("moves nothing in a window with no input for a minute, and picks each loop up again in phase", () => {
  const one = loop(0);
  animations = [one];
  stop = startLoopClock();
  advance(200);

  marks.add("data-idle");
  advance(600);
  expect(one.currentTime).toBe(0);

  marks.delete("data-idle");
  advance(200);
  expect(one.currentTime).toBe(800);
});

test("a loop that appears in an idle window is paused, so it does not run at the display's rate", () => {
  stop = startLoopClock();
  marks.add("data-idle");
  const late = loop(0);
  animations = [late];
  advance(200);
  expect(late.pause).toHaveBeenCalled();
  expect(late.currentTime).toBe(0);
});

test("a loop out of sight does not move, and the others do", () => {
  const away = {};
  const seen = loop(0);
  const scrolled = loop(0, Infinity, away);
  animations = [seen, scrolled];
  stop = startLoopClock();
  advance(200);
  sight([{ target: away, isIntersecting: false }]);

  advance(200);
  expect(seen.currentTime).toBe(200);
  expect(scrolled.currentTime).toBe(0);

  sight([{ target: away, isIntersecting: true }]);
  advance(200);
  expect(scrolled.currentTime).toBe(400);
});
