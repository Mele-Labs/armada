// The agents' sign-in: the wire is answered by Fleet alone, so the mock has none for it. The alert the
// `agent-signed-out` walk plays is `mock/agent-sign-in.ts`, a source of its own.

import type { AgentSignInApi } from "../../../../shared/api/agent-sign-in";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const agentSignIn: Slice<AgentSignInApi, Record<string, never>> = {
  name: "agentSignIn",
  state: {},
  api: () => ({
    getAgentSignIn: async () => ({ ok: false, outcome: unanswered("/agent/sign_in") }),
    startAgentSignIn: async () => ({ ok: false, outcome: unanswered("/agent/sign_in") }),
    onAgentSignInChanged: () => () => {},
  }),
};
