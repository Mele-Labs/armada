// Mock-only: sleep mode with no Fleet behind it. Turning it on starts a night — the grace here is a
// second or so, where the proposal is two minutes — and each decision lands in turn as a call in the
// Cockpit. Turning it off lets the rest of the night pass at once, so a walk reads the same deck every
// time. Keeping or changing a decision takes its row off, which takes its call off the Cockpit.

import type { SleepSource, SleepState } from "../sleep";

const GRACE_MS = 700;

type Night = (state: SleepState) => SleepState;

const decide = (id: string, who: string, asked: string, chose: string): Night => (state) => ({
  ...state,
  decided: [...state.decided, { id, who, asked, chose }],
});

const NIGHT: readonly Night[] = [
  decide("d1", "Order the lunch", "Which size?", "Large"),
  decide("d2", "Name the branch", "fix or feat?", "fix"),
  decide("d3", "Job 44", "The check timed out twice", "Retried with the larger timeout"),
];

const EMPTY: SleepState = { on: false, decided: [], blocked: [], landed: [], walks: [] };

export function createMockSleep(): SleepSource & { reset(): void } {
  let state = EMPTY;
  let next = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const listeners = new Set<() => void>();
  const set = (to: SleepState) => {
    state = to;
    listeners.forEach((on) => on());
  };
  const pass = () => {
    const event = NIGHT[next++];
    if (event === undefined) return;
    set(event(state));
    timer = setTimeout(pass, GRACE_MS);
  };
  const stop = () => clearTimeout(timer);
  return {
    get: () => state,
    subscribe: (on) => (listeners.add(on), () => void listeners.delete(on)),
    reset: () => {
      stop();
      next = 0;
      set(EMPTY);
    },
    toggle: () => {
      if (!state.on) {
        set({ ...state, on: true });
        timer = setTimeout(pass, GRACE_MS);
        return;
      }
      stop();
      let to = { ...state, on: false };
      for (let at = next; at < NIGHT.length; at++) to = NIGHT[at]!(to);
      next = NIGHT.length;
      set(to);
    },
    // A decision answered either way is reviewed: its call leaves the Cockpit.
    override: (id, _text) => set({ ...state, decided: state.decided.filter((one) => one.id !== id) }),
    review: (id) => set({ ...state, decided: state.decided.filter((one) => one.id !== id) }),
  };
}

export const mockSleep = createMockSleep();
