// Mock-only: Settings → Phone with no Gateway behind it. One source per window, reset to the
// moment a scenario names. A press does what the Gateway would: the code lasts five minutes, and
// a phone posts it a few seconds after it is drawn.

import type { PhoneSource, PhoneState } from "@armada/settings";

const MINUTE = 60_000;
export const PAIR_URL = "https://studio.tail1a2b3c.ts.net/pair?code=9f2c41d07be3a56c18d4e0f7a3b25c69";

export type PhoneMoment = "paired" | "not_running" | "tailscale";

const gateway = {
  paired: { state: "running" },
  not_running: { state: "not_running" },
  tailscale: { state: "tailscale", said: "Tailscale is not signed in on this Mac. Sign in to it, then start pairing again." },
} as const;

function start(moment: PhoneMoment): PhoneState {
  const now = Date.now();
  return {
    gateway: gateway[moment],
    phones: [
      { id: "p1", name: "iPhone", seenAt: now - 2 * MINUTE },
      { id: "p2", name: "iPad", seenAt: now - 3 * 24 * 60 * MINUTE },
    ],
    pairing: null,
    claim: null,
  };
}

export function createMockPhone(): PhoneSource & { reset(moment: PhoneMoment): void } {
  let state = start("paired");
  let claiming: ReturnType<typeof setTimeout> | undefined;
  let expiring: ReturnType<typeof setTimeout> | undefined;
  const listeners = new Set<() => void>();
  const set = (next: PhoneState) => {
    state = next;
    listeners.forEach((on) => on());
  };
  const stop = () => {
    clearTimeout(claiming);
    clearTimeout(expiring);
  };
  return {
    get: () => state,
    subscribe: (on) => (listeners.add(on), () => void listeners.delete(on)),
    reset: (moment) => {
      stop();
      set(start(moment));
    },
    pair: () => {
      stop();
      set({ ...state, pairing: { url: PAIR_URL, expiresAt: Date.now() + 5 * MINUTE }, claim: null });
      claiming = setTimeout(() => set({ ...state, claim: { device: "Pixel 9" } }), 4000);
      expiring = setTimeout(() => set({ ...state, pairing: null, claim: null }), 5 * MINUTE);
    },
    confirm: () => {
      stop();
      set({
        ...state,
        pairing: null,
        claim: null,
        phones: [...state.phones, { id: `p${state.phones.length + 1}`, name: state.claim?.device ?? "Phone", seenAt: Date.now() }],
      });
    },
    unpair: (id) => set({ ...state, phones: state.phones.filter((one) => one.id !== id) }),
  };
}

export const mockPhone = createMockPhone();
