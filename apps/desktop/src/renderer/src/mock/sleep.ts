// Mock-only: sleep mode with no Fleet behind it. Turning it on starts a night — the grace here is a
// second or so, where the proposal is two minutes — and each decision lands in turn as a call in the
// Cockpit. Turning it off lets the rest of the night pass at once, so a walk reads the same deck every
// time. Marking a decision done or changing it takes its row off, which takes its call off the Cockpit.
// `overnight` is a night already over and Sleep still on, the state the owner comes back to.

import type { MorningDecided, SleepSource, SleepState } from "../sleep";

const GRACE_MS = 700;

type Night = (state: SleepState) => SleepState;

/** The Sessions the decisions were made in; the scenario `sleep-cockpit` opens them. */
export const LUNCH_SESSION = "s21";
export const BRANCH_SESSION = "s22";

const LUNCH: MorningDecided = { id: "d1", who: "Order the lunch", asked: "Which size?", chose: "Large", sessionId: LUNCH_SESSION };
const BRANCH: MorningDecided = { id: "d2", who: "Name the branch", asked: "fix or feat?", chose: "fix", sessionId: BRANCH_SESSION };

const decide = (one: MorningDecided): Night => (state) => ({ ...state, decided: [...state.decided, one] });

const NIGHT: readonly Night[] = [decide(LUNCH), decide(BRANCH)];

const EMPTY: SleepState = { on: false, decided: [], blocked: [], landed: [], walks: [] };

/** Sleep still on after the night: every decision made, none looked at. */
export const overnight = (): SleepState => ({ ...EMPTY, on: true, decided: [LUNCH, BRANCH] });

export function createMockSleep(): SleepSource & { reset(to?: SleepState): void } {
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
    // A night handed in is already over: the rest of it has nothing left to pass.
    reset: (to) => {
      stop();
      next = to === undefined ? 0 : NIGHT.length;
      set(to ?? EMPTY);
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
