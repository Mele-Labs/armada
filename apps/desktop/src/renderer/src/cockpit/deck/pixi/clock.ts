// What the effects read to know where they are in their motion. A play is one of the activity log's events turned into something that
// moves; it starts the first time the canvas sees it and runs once. The loops of a working drone have no start: they are a function of
// the time, so two frames of one loop agree and nothing is kept between them.

import type { PlayKind } from "../plays";

export type Clock = {
  now: number;
  /** How long a loop that says still working takes, in milliseconds; 0 where he has asked for no motion. */
  pulse: number;
  /** Milliseconds since the play began and its own delay passed: negative while it waits. */
  since: (id: string, delay: number) => number;
};

/** The position of `now` in a loop of `period` milliseconds, 0 to 1. */
export const turn = (now: number, period: number): number => (((now % period) + period) % period) / period;

/** A triangle wave of that position: 0 to 1 and back, as an `alternate` animation does. */
export const there = (t: number): number => (t < 0.5 ? t * 2 : 2 - t * 2);

/** `ease-in-out`, for what an alternating animation eases along. */
export const soft = (t: number): number => t * t * (3 - 2 * t);

/** How long after its delay each kind of play still moves, in pulses of the loop and then plain milliseconds. */
const LIFE: Record<PlayKind, [pulses: number, ms: number]> = {
  launch: [2.6, 1000],
  sweep: [2.1, 300],
  payload: [1.5, 0],
  capture: [1.7, 1350],
  tracer: [2.6, 300],
  blip: [1.8, 0],
  flare: [0, 0],
  dock: [1.2, 0],
};

/** Whether the play has finished by `ms` after it began. */
export const over = (kind: PlayKind, delay: number, pulse: number, ms: number): boolean => {
  const [pulses, plain] = LIFE[kind];
  return ms > delay + plain + pulses * pulse + 400;
};
