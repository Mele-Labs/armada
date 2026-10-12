// A long thread, drawn from its newest rows up: the last few at once, the rest on the frames after.
//
// A thread opens at its newest row, so the rows above it are off screen when it opens and a draw that
// waits for all of them is a blank page for nothing anyone can see. A 401-row conversation took
// 90 to 150 ms to commit whole in Chromium; its newest forty take a few. Everything is drawn within
// a few frames, so scrolling up, selecting and every press on a row work as they did.

import { useEffect, useState } from "react";

/** Rows drawn on the first frame, counted from the newest. */
export const FIRST_ROWS = 40;

/** Rows drawn once the first frame has painted; the rest follow a frame later. */
export const THEN_ROWS = 160;

/**
 * How many leading rows of `rows` are held back, fixed when `key`'s thread first has rows and
 * brought to nothing in two steps. **A row that arrives later is appended** and never held back,
 * so a live thread is never shorter than the person last saw it.
 */
export function useBackfilled<T>(rows: readonly T[], key: string): readonly T[] {
  const [held, setHeld] = useState<{ key: string; hidden: number | undefined }>({ key, hidden: undefined });
  let hidden = held.key === key ? held.hidden : undefined;
  if (hidden === undefined && rows.length > 0) {
    hidden = Math.max(0, rows.length - FIRST_ROWS);
    setHeld({ key, hidden });
  }
  useEffect(() => {
    if (hidden === undefined || hidden === 0) return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    // After the paint: a frame is asked for, and the step is taken once it has been drawn.
    const frame = requestAnimationFrame(() => {
      const target = Math.max(0, rows.length - THEN_ROWS);
      timer = setTimeout(() => setHeld({ key, hidden: hidden > target ? target : 0 }), 0);
    });
    return () => {
      cancelAnimationFrame(frame);
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [hidden, key, rows.length]);
  return hidden === undefined || hidden === 0 ? rows : rows.slice(hidden);
}
