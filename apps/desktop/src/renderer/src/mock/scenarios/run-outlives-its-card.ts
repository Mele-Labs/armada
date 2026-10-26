// A Job waiting on four commands. One keeps running, a login waiting on the browser, so leaving the Job and coming
// back finds it still going. The other three end three ways: an exit code, Stop, and a signal. Over the `sessions`
// scenario. The walk `run-outlives-its-card` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const LOGIN = "gh auth login --web";
const TESTS = "pnpm vitest run";
const LOG = "tail -f build.log";
const DOCS = "./scripts/render-docs";

const waiting: Session = {
  id: "s18",
  title: "Sign in to GitHub",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(2),
  rows: [
    { id: "o1", at: "10:02:10", kind: "message", from: { kind: "you" }, text: "Get the docs build going on the new host." },
    { id: "o2", at: "10:06:41", kind: "message", from: { kind: "agent" }, text: "I need you to sign in, and to run the other three so I can read how each ends." },
  ],
  attachments: [{ kind: "slot", slot: 4 }, { kind: "branch", name: "docs/new-host", slot: 4 }],
  waitingFor: [
    { id: "o-login", text: "Sign in to GitHub", since: minutesAgo(3), source: "agent", act: { kind: "run", target: LOGIN } },
    { id: "o-tests", text: "Run the tests", since: minutesAgo(3), source: "agent", act: { kind: "run", target: TESTS } },
    { id: "o-log", text: "Watch the build log", since: minutesAgo(3), source: "agent", act: { kind: "run", target: LOG } },
    { id: "o-docs", text: "Render the docs", since: minutesAgo(3), source: "agent", act: { kind: "run", target: DOCS } },
  ],
};

export const s265RunOutlivesItsCard: Scenario = {
  ...s200Sessions,
  name: "run-outlives-its-card",
  says: "A Job waiting on a login that keeps running while the Job is left, and on three commands that end three ways",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [waiting]) },
};
