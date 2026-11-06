import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { onLoopTick, startLoopClock } from "./loop-clock";

/** An element a loop runs on: inside a region marked for the clock, or not, with a box of this many pixels a side. */
function place(inRegion: boolean, side = 16): Element {
  return {
    closest: (selector: string) => (inRegion && selector === "[data-loop-clock]" ? {} : null),
    getBoundingClientRect: () => ({
      width: side,
      height: side,
    }),
  } as unknown as Element;
}

/** The parts of a CSS animation the clock touches. A loop sits on the map unless given a place. */
function loop(
  currentTime: number,
  iterations = Infinity,
  target: unknown = place(true),
  duration = 1600,
): Animation {
  const one = {
    currentTime,
    playState: "running",
    effect: {
      getComputedTiming: () => ({
        iterations,
        duration,
        endTime: iterations * duration,
      }),
      target,
    },
    pause: vi.fn(() => {
      one.playState = "paused";
    }),
    play: vi.fn(() => {
      one.playState = "running";
    }),
    finish: vi.fn(),
  };
  return one as unknown as Animation;
}

let now = 1000;
let animations: Animation[] = [];
let marks = new Set<string>();
let stop: () => void = () => undefined;
let sight: (
  changes: {
    target: unknown;
    isIntersecting: boolean;
  }[],
) => void = () => undefined;

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
  vi.stubGlobal("window", {
    innerWidth: 1000,
    innerHeight: 1000,
  });
  vi.stubGlobal(
    "IntersectionObserver",
    class {
      constructor(
        hear: (
          changes: {
            target: unknown;
            isIntersecting: boolean;
          }[],
        ) => void,
      ) {
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

test("steps a slow fade on the same tick, holds it while idle, and finishes it at its end", () => {
  const fade = loop(0, 1, {}, 45_000);
  animations = [
    fade,
  ];
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
  animations = [
    one,
  ];
  stop = startLoopClock();
  advance(200);

  marks.add("data-still");
  advance(600);
  expect(one.currentTime).toBe(0);
});

test("moves nothing in a window with no input for a minute, and picks each loop up again in phase", () => {
  const one = loop(0);
  animations = [
    one,
  ];
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
  animations = [
    late,
  ];
  advance(200);
  expect(late.pause).toHaveBeenCalled();
  expect(late.currentTime).toBe(0);
});

test("a canvas's loop follows the same tick, holds while the window is still or idle, and stops when told", () => {
  const follow = vi.fn();
  const off = onLoopTick(follow);
  stop = startLoopClock();

  advance(600);
  expect(follow).toHaveBeenCalledTimes(3);

  marks.add("data-still");
  advance(400);
  marks.delete("data-still");
  marks.add("data-idle");
  advance(400);
  expect(follow).toHaveBeenCalledTimes(3);

  marks.delete("data-idle");
  advance(200);
  expect(follow).toHaveBeenCalledTimes(4);

  off();
  advance(400);
  expect(follow).toHaveBeenCalledTimes(4);
});

test("a loop out of sight does not move, and the others do", () => {
  const away = place(true);
  const seen = loop(0);
  const scrolled = loop(0, Infinity, away);
  animations = [
    seen,
    scrolled,
  ];
  stop = startLoopClock();
  advance(200);
  sight([
    {
      target: away,
      isIntersecting: false,
    },
  ]);

  advance(200);
  expect(seen.currentTime).toBe(200);
  expect(scrolled.currentTime).toBe(0);

  sight([
    {
      target: away,
      isIntersecting: true,
    },
  ]);
  advance(200);
  expect(scrolled.currentTime).toBe(400);
});

test("a small loop outside the map runs at the display's rate, untouched by the clock", () => {
  const spinner = loop(300, Infinity, place(false));
  animations = [
    spinner,
  ];
  stop = startLoopClock();

  advance(600);
  expect(spinner.pause).not.toHaveBeenCalled();
  expect(spinner.play).not.toHaveBeenCalled();
  expect(spinner.currentTime).toBe(300);
});

test("a loop larger than a tenth of the window takes the clock wherever it is", () => {
  const wide = loop(0, Infinity, place(false, 400));
  animations = [
    wide,
  ];
  stop = startLoopClock();

  advance(200);
  expect(wide.pause).toHaveBeenCalled();
  advance(200);
  expect(wide.currentTime).toBe(200);
});

test("a free loop holds while the window is idle, and runs again on the next input", () => {
  const spinner = loop(0, Infinity, place(false));
  animations = [
    spinner,
  ];
  stop = startLoopClock();
  advance(200);

  marks.add("data-idle");
  advance(200);
  expect(spinner.playState).toBe("paused");

  marks.delete("data-idle");
  advance(200);
  expect(spinner.play).toHaveBeenCalledTimes(1);
  expect(spinner.playState).toBe("running");
});

test("a free loop out of sight holds until it is back", () => {
  const away = place(false);
  const spinner = loop(0, Infinity, away);
  animations = [
    spinner,
  ];
  stop = startLoopClock();
  advance(200);
  sight([
    {
      target: away,
      isIntersecting: false,
    },
  ]);

  advance(200);
  expect(spinner.playState).toBe("paused");

  sight([
    {
      target: away,
      isIntersecting: true,
    },
  ]);
  advance(200);
  expect(spinner.playState).toBe("running");
});

test("a free loop the clock has played holds in a still window too, since the still mark's CSS no longer reaches it", () => {
  const spinner = loop(0, Infinity, place(false));
  animations = [
    spinner,
  ];
  stop = startLoopClock();
  marks.add("data-idle");
  advance(200);
  marks.delete("data-idle");
  advance(200);
  expect(spinner.playState).toBe("running");

  marks.add("data-still");
  advance(200);
  expect(spinner.playState).toBe("paused");

  marks.delete("data-still");
  advance(200);
  expect(spinner.playState).toBe("running");
});

test("a free loop the still mark's CSS paused is left to it", () => {
  const spinner = loop(0, Infinity, place(false));
  animations = [
    spinner,
  ];
  stop = startLoopClock();
  advance(200);

  marks.add("data-still");
  (spinner as unknown as { playState: string }).playState = "paused";
  advance(400);
  marks.delete("data-still");
  advance(200);
  expect(spinner.pause).not.toHaveBeenCalled();
  expect(spinner.play).not.toHaveBeenCalled();
});
