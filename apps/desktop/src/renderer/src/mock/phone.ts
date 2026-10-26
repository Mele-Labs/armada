// Mock-only: Settings → Phone with no Fleet or Gateway behind it. One source per window, reset to the
// moment a scenario names. A press does what they would: the switch answers after a moment, the code
// lasts five minutes, and a phone posts it a few seconds after it is drawn. In the way in, Get Tailscale
// installs it and signs in a moment later, Turn on Funnel names the tailnet's page, and opening that page
// allows Funnel.

import type { PhoneSetup, PhoneSource, PhoneState } from "@armada/settings";

const MINUTE = 60_000;
export const PAIR_URL = "https://studio.tail1a2b3c.ts.net/pair?code=9f2c41d07be3a56c18d4e0f7a3b25c69";

export type PhoneMoment = "paired" | "off" | "port_taken" | "tailscale" | "set_up" | "named";

const ADDRESS = "https://studio.tail1a2b3c.ts.net";
const ALLOW_PAGE = "https://login.tailscale.com/f/funnel?node=nStudio1CNTRL";

const setups: Record<PhoneMoment, PhoneSetup> = {
  paired: { tailscale: "running", served: "funnel", named: false, address: ADDRESS },
  off: { tailscale: "running", served: "funnel", named: false, address: ADDRESS },
  port_taken: { tailscale: "running", served: "funnel", named: false, address: ADDRESS },
  tailscale: { tailscale: "signed_out", served: "none", named: false },
  set_up: { tailscale: "missing", served: "none", named: false },
  named: { tailscale: "missing", served: "none", named: true, address: "https://armada.example.com" },
};

const gateway = {
  paired: { state: "running" },
  off: { state: "running" },
  port_taken: { state: "said", said: "Port 8443 is in use by another program." },
  tailscale: { state: "said", said: "Tailscale is not signed in on this Mac. Sign in to it, then start pairing again." },
  set_up: { state: "running" },
  named: { state: "running" },
} as const;

/** How long Fleet takes to answer the switch, so the press shows it waiting. */
const TURNING_MS = 400;

function start(moment: PhoneMoment): PhoneState {
  const now = Date.now();
  return {
    access: { on: moment !== "off", turning: false },
    gateway: gateway[moment],
    phones:
      moment === "set_up"
        ? []
        : [
            { id: "p1", name: "iPhone", seenAt: now - 2 * MINUTE },
            { id: "p2", name: "iPad", seenAt: now - 3 * 24 * 60 * MINUTE },
          ],
    pairing: null,
    claim: null,
    setup: moment === "off" ? null : setups[moment],
  };
}

export function createMockPhone(): PhoneSource & { reset(moment: PhoneMoment): void } {
  let state = start("paired");
  let claiming: ReturnType<typeof setTimeout> | undefined;
  let expiring: ReturnType<typeof setTimeout> | undefined;
  let turning: ReturnType<typeof setTimeout> | undefined;
  const listeners = new Set<() => void>();
  const set = (next: PhoneState) => {
    state = next;
    listeners.forEach((on) => on());
  };
  const stop = () => {
    clearTimeout(claiming);
    clearTimeout(expiring);
    clearTimeout(turning);
  };
  return {
    get: () => state,
    subscribe: (on) => (listeners.add(on), () => void listeners.delete(on)),
    reset: (moment) => {
      stop();
      set(start(moment));
    },
    // Turning on again finds the port free; the phones paired stay paired while it is off.
    turn: (on) => {
      stop();
      set({ ...state, access: { on: state.access?.on ?? false, turning: true } });
      turning = setTimeout(
        () => set({ ...state, access: { on, turning: false }, gateway: { state: "running" }, pairing: null, claim: null, setup: on ? (state.setup ?? setups.paired) : null }),
        TURNING_MS,
      );
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
    funnel: () => {
      if (state.setup === null) return;
      set({ ...state, setup: { ...state.setup, funnel: { state: "turning" } } });
      turning = setTimeout(() => state.setup !== null && set({ ...state, setup: { ...state.setup, funnel: { state: "approve", page: ALLOW_PAGE } } }), TURNING_MS);
    },
    // Tailscale's download installs it and the owner signs in; the tailnet's page allows Funnel, and the command it held finishes.
    open: (page) => {
      if (state.setup === null) return;
      const setup = state.setup;
      turning = setTimeout(() => {
        if (page === ALLOW_PAGE) return set({ ...state, setup: { tailscale: "running", served: "funnel", named: false, address: ADDRESS } });
        set({ ...state, setup: { ...setup, tailscale: "running", address: ADDRESS } });
      }, TURNING_MS);
    },
  };
}

export const mockPhone = createMockPhone();
