// Mock-only: the agent signed out for the whole Fleet, with no Fleet behind it. It starts signed out; the press
// signs in for about two seconds and then settles.

import type { AgentSignIn, AgentSignInSource } from "../agent-sign-in";

const SIGNING_IN_MS = 2_000;

export function createMockAgentSignIn(): AgentSignInSource {
  let state: AgentSignIn = { signed_out_since: new Date(Date.now() - 5 * 60_000).toISOString(), signing_in: false };
  const listeners = new Set<() => void>();
  const set = (to: AgentSignIn): void => {
    state = to;
    listeners.forEach((on) => on());
  };
  return {
    get: () => state,
    subscribe: (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    signIn: () => {
      if (state.signing_in || state.signed_out_since === undefined) return;
      set({ ...state, signing_in: true });
      globalThis.setTimeout(() => set({ signing_in: false }), SIGNING_IN_MS);
    },
  };
}
