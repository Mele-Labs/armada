// The deck's clocks. The window ticks once a second for figures that read seconds; the deck's own are all in minutes, so it is handed
// the minute (`AGE_MS`) and a part that must move faster keeps a clock of its own, drawn by that part alone, rather than the whole deck
// drawing again for it.

import { useEffect, useState } from "react";

import { quantized } from "./field";

/** The step the deck reads the time in: an age, a held-for, a `since` are whole minutes. */
export const AGE_MS = 60_000;

/** The time in steps of `step`, redrawn at each step and not between. */
export function useClock(step: number): number {
  const [now, setNow] = useState(() => quantized(Date.now(), step));
  useEffect(() => {
    let timer: number;
    const arm = () => {
      timer = window.setTimeout(() => {
        setNow(quantized(Date.now(), step));
        arm();
      }, step - (Date.now() % step) + 5);
    };
    arm();
    return () => window.clearTimeout(timer);
  }, [step]);
  return now;
}
