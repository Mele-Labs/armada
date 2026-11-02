// Autopilot's answers, as Fleet serves them: the ones not yet reviewed, in the order they were given. A
// scenario that has some says so by name; every other window starts with none. An answer reviewed or
// corrected leaves the list, as Fleet's does, and the list is published whole.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { AutopilotDecided, AutopilotDecisions, Outcome } from "@armada/protocol";

import type { AutopilotDecisionsActed, AutopilotDecisionsApi } from "../../../../shared/api/autopilot-decisions";
import type { Slice } from "../fake-context";

/** The Sessions the answers were given in; the scenario `autopilot-back` opens them. */
export const LUNCH_SESSION = "s21";
export const BRANCH_SESSION = "s22";
export const NOTES_SESSION = "s23";
export const BACK_MANIFEST = repository().manifest!.id;

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const LUNCH: AutopilotDecided = {
  id: `${LUNCH_SESSION}:q1`,
  manifest_id: BACK_MANIFEST,
  session_id: LUNCH_SESSION,
  who: "Friday lunch",
  kind: "recommended",
  asked: "Which size should the order be?",
  chose: "Large",
  because: "The Job recommended Large, and the request names no size.",
  at: minutesAgo(250),
};
const BRANCH: AutopilotDecided = {
  id: `${BRANCH_SESSION}:q1`,
  manifest_id: BACK_MANIFEST,
  session_id: BRANCH_SESSION,
  who: "Reconnect fix",
  kind: "open",
  asked: "Branch prefix: fix or feat?",
  chose: "fix",
  because: "The change repairs a reconnect that dropped, so it fixes and adds nothing.",
  at: minutesAgo(190),
};
const NOTES: AutopilotDecided = {
  id: `${NOTES_SESSION}:q1`,
  manifest_id: BACK_MANIFEST,
  session_id: NOTES_SESSION,
  who: "Release notes",
  kind: "open",
  asked: "Which version heading?",
  chose: "0.14",
  because: "The last heading in the changelog is 0.13, and this build is the next minor.",
  at: minutesAgo(140),
};
/** A pull request landed: nobody to tell, so Fleet refuses a correction. */
const LANDED: AutopilotDecided = {
  id: "armada:land:1894:7f3c2d1",
  manifest_id: BACK_MANIFEST,
  who: "Pull request 1894",
  kind: "land",
  asked: "Land pull request 1894?",
  chose: "Landed",
  because: "Every check was green.",
  at: minutesAgo(95),
};

const noDecision = (): { ok: false; outcome: Outcome } => ({
  ok: false,
  outcome: { ok: false, why: "refused", error: { code: "fleet.autopilot_no_decision", message: "No such answer", run_id: "", fields: {}, chain: [] } },
});

const BACK: readonly AutopilotDecided[] = [LUNCH, BRANCH, NOTES, LANDED];

export const autopilotDecisions: Slice<AutopilotDecisionsApi, Record<string, never>> = {
  name: "autopilotDecisions",
  state: {},
  api: (scenario) => {
    let kept: readonly AutopilotDecided[] = scenario.name === "autopilot-back" ? BACK : [];
    const told = new Set<(decisions: AutopilotDecisions) => void>();
    const whole = (): AutopilotDecisions => ({ to_review: [...kept] });
    const taken = (id: string): AutopilotDecisionsActed => {
      if (!kept.some((one) => one.id === id)) return noDecision();
      kept = kept.filter((one) => one.id !== id);
      told.forEach((one) => one(whole()));
      return { ok: true, value: whole() };
    };
    return {
      getAutopilotDecisions: async () => ({ ok: true, value: whole() }),
      reviewAutopilotDecision: async (id) => taken(id),
      overrideAutopilotDecision: async (id, _text) => taken(id),
      onAutopilotDecisionsChanged: (onChanged) => {
        told.add(onChanged);
        return () => void told.delete(onChanged);
      },
    };
  },
};
