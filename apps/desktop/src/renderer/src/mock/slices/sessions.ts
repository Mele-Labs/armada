// Sessions' members: every act is a route this mock has none for, and no session is read, so a
// scenario without its own Fleet for them draws no Sessions. A test or a walk that wants the real
// path supplies its own members through `Scenario.behaves` (`sessions-fleet.ts`).

import type { SessionsApi, SessionsState } from "../../../../shared/api/sessions";
import { SESSIONS_NOTHING_YET } from "../../../../shared/api/sessions";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const sessions: Slice<SessionsApi, SessionsState> = {
  name: "sessions",
  state: SESSIONS_NOTHING_YET,
  api: () => ({
    startSession: async () => ({ ok: false, outcome: unanswered("/sessions/start") }),
    sendSessionMessage: async () => ({ ok: false, outcome: unanswered("/sessions/message") }),
    answerSessionAsk: async () => ({ ok: false, outcome: unanswered("/sessions/ask/answer") }),
    tuneSession: async () => ({ ok: false, outcome: unanswered("/sessions/tune") }),
    closeSession: async () => ({ ok: false, outcome: unanswered("/sessions/close") }),
    watchSession: async () => undefined,
    readSessionFile: async () => ({ ok: false, outcome: unanswered("/sessions/file") }),
    pressPullRequest: async () => ({ ok: false, outcome: unanswered("/pull_requests") }),
  }),
};
