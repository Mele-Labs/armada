// The Cockpit with Autopilot running and the person back at the window: ten minutes passed with nobody
// there, so the first press asks whether to keep it on. What it answered meanwhile (`slices/autopilot-decisions.ts`)
// waits in the HUD the switch's lamp opens. Three were answered in Jobs, which are here to be opened. The walk
// `autopilot-back` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";

import type { Session } from "@armada/screens/src/draft/sessions";

import { connected } from "../moment";
import type { Scenario } from "../moment";
import { BRANCH_SESSION, LUNCH_SESSION, NOTES_SESSION } from "../slices/autopilot-decisions";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** A Session that stopped on one question, the one Autopilot answered. */
const asked = (id: string, title: string, request: string, question: string, at: number): Session => ({
  id,
  title,
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(at),
  rows: [
    { id: `${id}-1`, at: "02:04:10", kind: "message", from: { kind: "you" }, text: request },
    { id: `${id}-2`, at: "02:10:31", kind: "message", from: { kind: "agent" }, text: question },
  ],
  attachments: [],
});

const lunch = asked(LUNCH_SESSION, "Friday lunch", "Order the lunch for Friday.", "Which size should the order be?", 250);
const reconnect = asked(BRANCH_SESSION, "Reconnect fix", "Name the branch for the reconnect change.", "Branch prefix: fix or feat?", 190);
const notes = asked(NOTES_SESSION, "Release notes", "Draft the release notes for this build.", "Which version heading?", 140);

function build(): Scenario {
  const sessions = s200Sessions.draft!.sessions!;
  return {
    name: "autopilot-back",
    says: "The Cockpit with Autopilot running and its answers to review",
    state: { ...connected([], [repository()]), repository: repository().root },
    draft: { sessions: (board) => sessions(board, [lunch, reconnect, notes]) },
  };
}

export const s209AutopilotBack: Scenario = build();
