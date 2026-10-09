// A `PhoneSource` over the Gateway's admin routes, asked through Bridge's main process. Reads when
// something is listening. A code that is shown and unexpired is polled for a claim every 2 s, and
// nothing else polls except a card showing a problem, which looks again every 5 s so that starting
// the Gateway or signing in to Tailscale clears it without a press.

import type { PhoneAnswer, PhoneRequest } from "./api";
import type { PhoneGateway, PhoneSource, PhoneState } from "./phone-source";

const CLAIM_EVERY = 2000;
const RETRY_EVERY = 5000;

type Device = { id: string; name: string; created_at: number; last_seen_at: number | null };
type Pending = { code: string; name: string; claimed_at: number };
type Started = { code: string; address: string; expires_at: number };

export type PhoneAsk = (request: PhoneRequest) => Promise<PhoneAnswer>;

export function createPhoneGatewaySource(ask: PhoneAsk, now: () => number = Date.now): PhoneSource {
  let state: PhoneState = { gateway: { state: "running" }, phones: [], pairing: null, claim: null };
  let code: string | null = null;
  let listening = 0;
  let claimTimer: ReturnType<typeof setInterval> | undefined;
  let expiry: ReturnType<typeof setTimeout> | undefined;
  let retry: ReturnType<typeof setInterval> | undefined;
  const listeners = new Set<() => void>();

  const set = (next: Partial<PhoneState>) => {
    state = { ...state, ...next };
    listeners.forEach((on) => on());
    sync();
  };

  /** What a failed call says about the Gateway; `null` where the answer was good. */
  const problem = (answer: PhoneAnswer): PhoneGateway | null =>
    answer.ok ? null : answer.why === "unreachable" ? { state: "not_running" } : { state: "said", said: answer.said };

  const devices = (list: Device[]) =>
    list.map((one) => ({ id: one.id, name: one.name, seenAt: (one.last_seen_at ?? one.created_at) * 1000 }));

  async function refresh(): Promise<void> {
    const status = await ask({ op: "status" });
    const bad = problem(status);
    if (bad !== null) return set({ gateway: bad, pairing: null, claim: null });
    const list = await ask({ op: "devices" });
    const failed = problem(list);
    if (failed !== null) return set({ gateway: failed });
    set({ gateway: { state: "running" }, phones: devices(list.ok && Array.isArray(list.body) ? (list.body as Device[]) : []) });
  }

  async function poll(): Promise<void> {
    const answer = await ask({ op: "pending" });
    const bad = problem(answer);
    if (bad !== null) return set({ gateway: bad, pairing: null, claim: null });
    const mine = (answer.ok && Array.isArray(answer.body) ? (answer.body as Pending[]) : []).find((one) => one.code === code);
    if (mine !== undefined && state.claim?.device !== mine.name) set({ claim: { device: mine.name } });
  }

  /** Timers follow the state: a claim poll for a live code, a retry for a problem. */
  function sync(): void {
    const wantsClaims = listening > 0 && state.pairing !== null && state.gateway.state === "running";
    if (wantsClaims && claimTimer === undefined) claimTimer = setInterval(() => void poll(), CLAIM_EVERY);
    if (!wantsClaims && claimTimer !== undefined) (clearInterval(claimTimer), (claimTimer = undefined));
    const wantsRetry = listening > 0 && state.gateway.state !== "running";
    if (wantsRetry && retry === undefined) retry = setInterval(() => void refresh(), RETRY_EVERY);
    if (!wantsRetry && retry !== undefined) (clearInterval(retry), (retry = undefined));
    if (state.pairing === null && expiry !== undefined) (clearTimeout(expiry), (expiry = undefined));
  }

  return {
    get: () => state,
    subscribe: (on) => {
      listeners.add(on);
      listening += 1;
      if (listening === 1) void refresh();
      sync();
      return () => {
        listeners.delete(on);
        listening -= 1;
        sync();
      };
    },
    pair: () => {
      void ask({ op: "start" }).then((answer) => {
        const bad = problem(answer);
        if (bad !== null) return set({ gateway: bad });
        const started = (answer as { ok: true; body: unknown }).body as Started;
        code = started.code;
        clearTimeout(expiry);
        expiry = setTimeout(() => set({ pairing: null, claim: null }), Math.max(0, started.expires_at * 1000 - now()));
        set({
          pairing: { url: `${started.address}/pair?code=${started.code}`, expiresAt: started.expires_at * 1000 },
          claim: null,
        });
      });
    },
    confirm: () => {
      if (code === null) return;
      void ask({ op: "confirm", code }).then(async (answer) => {
        const bad = problem(answer);
        if (bad !== null) return set({ gateway: bad, pairing: null, claim: null });
        code = null;
        set({ pairing: null, claim: null });
        await refresh();
      });
    },
    unpair: (id) => {
      void ask({ op: "unpair", id }).then(async (answer) => {
        const bad = problem(answer);
        if (bad !== null) return set({ gateway: bad });
        await refresh();
      });
    },
  };
}
