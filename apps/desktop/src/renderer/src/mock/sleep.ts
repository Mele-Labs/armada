// Mock-only: sleep mode with no Fleet behind it. Turning it on starts a night — the grace here is a
// second or so, where the proposal is two minutes — and each event lands in turn: a question answered
// for the owner, a pull request merged, a walk held. Turning it off lets the rest of the night pass at
// once, so a walk reads the same review every time. An Override marks the row corrected.

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
  (state) => ({ ...state, landed: [...state.landed, { id: "l1", who: "Job 41", title: "Retire the sleep calls", pr: "#2031" }] }),
  decide("d3", "Job 44", "The check timed out twice", "Retried with the larger timeout"),
  (state) => ({ ...state, walks: [...state.walks, { id: "w1", who: "Job 47", title: "Dispatch panel spacing" }] }),
  (state) => ({
    ...state,
    blocked: [
      ...state.blocked,
      { id: "b1", who: "Job 52", text: "Pull request #2044 waits on your approval" },
      { id: "b2", who: "Order the lunch", text: "Refused: a write outside its slot" },
    ],
  }),
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
    override: (id, text) =>
      set({ ...state, decided: state.decided.map((one) => (one.id === id ? { ...one, corrected: text } : one)) }),
  };
}

export const mockSleep = createMockSleep();
