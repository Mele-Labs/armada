// How often Pulse's reading is taken again: main's `resources-poll.ts` runs the
// timer on it. The board no longer says the interval — its head reads only
// `Updated 4s ago` — so nothing on screen can promise a poll nothing runs,
// which is how `#1571` began.

/** The design's *every 10s while open*. */
export const PULSE_INTERVAL_MS = 10_000;
