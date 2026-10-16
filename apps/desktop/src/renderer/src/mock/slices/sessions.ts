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
    pilotJob: async () => ({ ok: false, outcome: unanswered("/sessions/start") }),
    exitPilot: async () => unanswered("/jobs/:job_id/attest_complete"),
    sendSessionMessage: async () => ({ ok: false, outcome: unanswered("/sessions/message") }),
    answerSessionAsk: async () => ({ ok: false, outcome: unanswered("/sessions/ask/answer") }),
    answerWaiting: async () => ({ ok: false, outcome: unanswered("/sessions/waiting/answer") }),
    claimPullRequest: async () => ({ ok: false, outcome: unanswered("/sessions/claim_pull_request") }),
    dismissWaiting: async () => ({ ok: false, outcome: unanswered("/sessions/waiting/dismiss") }),
    approvePlan: async () => ({ ok: false, outcome: unanswered("/sessions/plan/approve") }),
    sendPlanChanges: async () => ({ ok: false, outcome: unanswered("/sessions/plan/changes") }),
    tuneSession: async () => ({ ok: false, outcome: unanswered("/sessions/tune") }),
    runSessionCommand: async () => ({ ok: false, outcome: unanswered("/sessions/command") }),
    renameSession: async () => ({ ok: false, outcome: unanswered("/sessions/rename") }),
    forkSession: async () => ({ ok: false, outcome: unanswered("/sessions/start") }),
    forkSessionAt: async () => ({ ok: false, outcome: unanswered("/sessions/start") }),
    retroSession: async () => unanswered("/sessions/:session_id/retro"),
    closeSession: async () => ({ ok: false, outcome: unanswered("/sessions/close") }),
    stopSession: async () => ({ ok: false, outcome: unanswered("/sessions/stop") }),
    setSessionWorkflow: async () => ({ ok: false, outcome: unanswered("/sessions/workflow") }),
    moveSession: async () => ({ ok: false, outcome: unanswered("/sessions/move") }),
    watchSession: async () => undefined,
    readSessionFile: async () => ({ ok: false, outcome: unanswered("/sessions/file") }),
    readSessionSubagent: async () => ({ ok: false, outcome: unanswered("/sessions/subagent") }),
    openSessionFile: async () => ({ ok: true }),
    openSessionWindow: async () => ({ ok: true }),
    getSessionWindows: async () => [],
    onSessionWindows: () => () => {},
    readSessionArtifact: async () => ({ ok: false, why: "unreadable" }),
    showSessionPage: async () => ({ ok: true }),
    moveSessionPage: async () => undefined,
    hideSessionPage: async () => undefined,
    onSessionPageEscape: () => () => undefined,
    pressPullRequest: async () => ({ ok: false, outcome: unanswered("/pull_requests") }),
    pressRepositoryPullRequest: async () => ({ ok: false, outcome: unanswered("/pull_requests") }),
    readPullRequestPage: async () => ({ ok: false, outcome: unanswered("/pull_request_details") }),
    readPullRequestFile: async () => ({ ok: false, outcome: unanswered("/pull_request_files") }),
    writePullRequest: async () => ({ ok: false, outcome: unanswered("/pull_request_writes") }),
    startSessionRun: async () => ({ ok: false, outcome: unanswered("/sessions/run/start") }),
    catchUpCheckout: async () => ({ ok: false, outcome: unanswered("/merge_lines/catch_up") }),
    sendCheckoutAsPullRequest: async () => ({ ok: false, outcome: unanswered("/merge_lines/checkout_to_pull_request") }),
    readCheckoutDiff: async () => ({ ok: false, outcome: unanswered("/merge_lines/checkout_diff") }),
    stopSessionRun: async () => ({ ok: false, outcome: unanswered("/sessions/run/stop") }),
    writeSessionRun: async () => ({ ok: false, outcome: unanswered("/sessions/run/write") }),
    resizeSessionRun: async () => ({ ok: false, outcome: unanswered("/sessions/run/resize") }),
    onSessionRunMessage: () => () => undefined,
  }),
};
