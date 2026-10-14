// The Cockpit on Active with two Jobs at work and nothing asking, then two calls that arrive one at a
// time: a Plan question, and after it a failed Check. Each Job is on the Board only from the moment it
// asks, as in `dashboard-cockpit`. The walk `cockpitCallsArrive` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import { featureRunning } from "@armada/jobs/fake";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";

const OWNER = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** The picked repository's, started `minutes` ago, on the step `id` of its workflow, asking where it does. */
function live(fixture: JobFixture, minutes: number, id: string, asking = false): JobFixture {
  const patch = { owner_manifest_id: OWNER, started_at: ago(minutes), current_step_id: id, ...(asking ? { asking: true } : {}) };
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, ...patch };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = { ...fixture.watched.detail.job, ...patch };
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const cache = live(asRow(featureRunning(), 101, "cache", "Cache the manifest read between dispatches"), 17, "tests");
const debounce = live(asRow(featureRunning(), 102, "debounce", "Debounce the Job Board's resize handler"), 41, "implement");

/** The Jobs that ask, on the Board only from the moment they do. */
const plan = live(asRow(featureRunning(), 103, "calls-plan", "Split the writer from the clock"), 26, "scope", true);
const check = live(asRow(featureRunning(), 104, "calls-check", "Shorten the reconnect wait"), 33, "implement", true);

const calls: Record<string, CallView> = {
  [plan.job.id]: {
    request: "Pull the clock out of Fleet's event log writer so the heartbeat never waits on the writer. Keep the log format as it is.",
    running: [{ key: "r", of: "drone", name: "Drone on Plan", line: "Weighing the split against the wrap", state: "running" }],
    asks: [
      {
        key: "p",
        kind: "plan",
        context: ["Writer::tick() reads Clock::now() under the same lock that guards the buffer.", "Splitting touches 6 call sites and the Writer constructor."],
        decisions: [
          {
            id: "shape",
            question: "Split the clock out of the writer, or wrap it in place?",
            options: [
              { id: "split", label: "Split it out" },
              { id: "wrap", label: "Wrap it in place" },
            ],
          },
        ],
      },
    ],
  },
  [check.job.id]: {
    request: "Cut the first reconnect wait to 5s and back off from there, keeping the cap at 60s.",
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
  [cache.job.id]: { running: [{ key: "r", of: "drone", name: "Drone on Write tests", line: "cargo test -p manifest read_again", state: "running" }] },
  [debounce.job.id]: { running: [{ key: "r", of: "drone", name: "Drone on Implement", line: "Editing resize-handler.ts", state: "running" }] },
};

const nows: Record<string, NowView> = {
  [cache.job.id]: { running: [{ key: "d", of: "drone", name: "Drone on Write tests", line: "cargo test -p manifest read_again", state: "running" }] },
  [debounce.job.id]: { running: [{ key: "d", of: "drone", name: "Drone on Implement", line: "Editing resize-handler.ts", state: "running" }] },
  [plan.job.id]: {
    running: [{ key: "d", of: "drone", name: "Drone on Plan", line: "Weighing the split against the wrap", state: "running" }],
    waiting: [{ key: "w", kind: "transition", text: "Plan to Implement, on your answer" }],
  },
  [check.job.id]: {
    running: [
      { key: "d", of: "drone", name: "Drone on Implement", line: "Waiting on the store test", state: "running" },
      { key: "c", of: "check", name: "store", state: "failed" },
    ],
  },
};

function build(): Scenario {
  const board = [cache, debounce];
  const base = holding("cockpit-calls-arrive", "The Cockpit on Active with two Jobs at work, then a Plan question and a failed Check that arrive in its tray one after the other", board);
  const jobs = board.map((one) => one.job);
  return {
    ...base,
    state: { ...base.state, repository: null, jobs },
    reads: { ...base.reads, [plan.job.id]: plan, [check.job.id]: check },
    // The first moment is a beat with nothing in it: a call is only news once Bridge has settled on what it holds.
    later: [{}, { jobs: [...jobs, plan.job] }, { jobs: [...jobs, check.job] }],
    draft: { calls, now: nows },
  };
}

export const cockpitCallsArrive: Scenario = build();
