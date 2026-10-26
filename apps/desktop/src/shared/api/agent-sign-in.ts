// The agents' account signed out for the whole Fleet: the state Fleet holds, the one press that signs it in
// again, and the event that carries it whole. A slice imports protocol, never another slice; `../api.ts` and
// `../bridge.ts` compose them.

import type { AgentSignIn, Outcome } from "@armada/protocol";

/** What reading or pressing came to: the state as Fleet holds it now, or the refusal to word. */
export type AgentSignInRead = { ok: true; value: AgentSignIn } | { ok: false; outcome: Outcome };

export type AgentSignInApi = {
  /** Whether the agents' account is signed out, and whether Fleet is signing it in. */
  getAgentSignIn: () => Promise<AgentSignInRead>;
  /** Fleet runs the sign-in in the person's browser. A press while one runs starts nothing. */
  startAgentSignIn: () => Promise<AgentSignInRead>;
  /** `agent_sign_in.changed`: the state, whole, whenever Fleet moves it. */
  onAgentSignInChanged: (onChanged: (state: AgentSignIn) => void) => () => void;
};

export const AGENT_SIGN_IN_CHANNELS = {
  getAgentSignIn: "bridge:get-agent-sign-in",
  startAgentSignIn: "bridge:start-agent-sign-in",
  /** Main to every window, not a request. */
  agentSignInChanged: "bridge:agent-sign-in-changed",
} as const;
