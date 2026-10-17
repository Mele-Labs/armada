// The Cockpit after a night of Sleep mode: one Job blocked on a failed Check, which needs the owner
// whatever Sleep did, and Sleep still on with the night's decisions made (`mock/sleep.ts`). They arrive
// as calls behind it once the first press turns the moon off. Two were made in Sessions, which are
// here to be opened. The walk `sleepCockpit` plays it.

import { featureRunning } from "@armada/screens/src/fixtures/jobs/feature-running";
import type { CallView } from "@armada/screens/src/draft/calls";
import { repository } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { Session } from "@armada/screens/src/draft/sessions";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { BRANCH_SESSION, LUNCH_SESSION } from "../sleep";
import { s200Sessions } from "./sessions";

const OWNER = repository().manifest!.id;

/** A Job the Drone is asking about: running, flagged as asking, no longer cleared, and the picked repository's. */
function asking(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, asking: true, owner_manifest_id: OWNER };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = { ...fixture.watched.detail.job, asking: true, owner_manifest_id: OWNER };
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const blocked = asking(asRow(featureRunning(), 52, "sleep-blocked", "Shorten the reconnect wait"));

const now: Record<string, CallView> = {
  [blocked.job.id]: {
    request: "Cut the first reconnect wait from 30s to 5s.",
    running: [{ key: "r", of: "drone", name: "Drone on Implement", line: "Waiting on the store test", state: "running" }],
    issues: [
      {
        key: "i1",
        of: "check",
        name: "store",
        text: "store: 2 failed",
        said: "Check failed",
        context: ["FAIL store::reconnect::waits_five_seconds", "  expected: 5s", "  actual:   30s"],
      },
    ],
  },
};

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** A Session that stopped on one question, the one Sleep answered. */
const asked = (id: string, title: string, request: string, question: string, at: number): Session => ({
  id,
  title,
  turn: { state: "idle" },
  lastTurn: "02:10",
  lastTurnAt: minutesAgo(at),
  rows: [
    { id: `${id}-1`, at: "02:04:10", kind: "message", from: { kind: "you" }, text: request },
    { id: `${id}-2`, at: "02:10:31", kind: "message", from: { kind: "agent" }, text: question },
  ],
  attachments: [],
});

const lunch = asked(LUNCH_SESSION, "Order the lunch", "Order the lunch for Friday.", "Which size?", 420);
const naming = asked(BRANCH_SESSION, "Name the branch", "Name the branch for the reconnect change.", "fix or feat?", 380);

function build(): Scenario {
  const base = holding("sleep-cockpit", "The Cockpit with a Job blocked on a Check, and a night of Sleep decisions to review", [blocked]);
  const sessions = s200Sessions.draft!.sessions!;
  return {
    ...base,
    state: { ...base.state, repository: repository().root, jobs: [blocked.job] },
    draft: { calls: now, sessions: (board) => sessions(board, [lunch, naming]) },
  };
}

export const s209SleepCockpit: Scenario = build();
