// The Cockpit after a night of Sleep mode: one Job blocked on a failed Check, which needs the owner
// whatever Sleep did, and the decisions Sleep made arrive as calls behind it once the moon is turned
// off (`mock/sleep.ts`). The walk `sleepCockpit` plays it.

import { featureRunning } from "@armada/jobs/fake";
import type { CallView } from "@armada/jobs/draft/calls";
import { repository } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";

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

function build(): Scenario {
  const base = holding("sleep-cockpit", "The Cockpit with a Job blocked on a Check, and a night of Sleep decisions to review", [blocked]);
  return { ...base, state: { ...base.state, repository: repository().root, jobs: [blocked.job] }, draft: { calls: now } };
}

export const s209SleepCockpit: Scenario = build();
