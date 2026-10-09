// Command Central as a cockpit: five Jobs and four Sessions under way and nothing asking, then three
// calls that arrive one at a time — a Plan question, a failed Check, a Session waiting on a command —
// each as a walk's `later` step lets time pass. A call's Job is on the Board only from the moment it
// asks, since a call is read off a Job's own view and the mock holds those from the start. The walk
// `dashboard-cockpit` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";
import { mergeLines } from "@armada/screens/src/fixtures/build/merge-line";
import type { Session } from "@armada/screens/src/draft/sessions";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import { featureRunning } from "@armada/jobs/fake";
import { queued, running } from "@armada/jobs/fixtures/build/index";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore, type SessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** The picked repository's, started `minutes` ago, on step `step` of its workflow where one is named. */
function live(fixture: JobFixture, minutes: number, step?: number, asking = false): JobFixture {
  const id = step === undefined ? undefined : fixture.workflows[0]?.steps[step]?.step_id;
  const patch = {
    owner_manifest_id: OWNER,
    started_at: ago(minutes),
    ...(asking ? { asking: true } : {}),
    ...(id === undefined ? {} : { current_step_id: id }),
  };
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, ...patch };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = { ...fixture.watched.detail.job, ...patch };
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const debounce = live(asRow(featureRunning(), 80, "debounce", "Debounce the Job Board's resize handler"), 41, 1);
const cache = live(asRow(featureRunning(), 81, "cache", "Cache the manifest read between dispatches"), 17, 2);
const mainChecks = live(asRow(running(), 82, "fix-main", "Fix components_test on main"), 9);
const migrate = live(asRow(featureRunning(), 83, "migrate", "Order the store migrations"), 3, 0);
const pause = live(asRow(queued(), 84, "pause", "Store a pause marker on the Job"), 1);

/** The Jobs that ask, on the Board only from the moment they do. */
const plan = live(asRow(featureRunning(), 85, "dash-plan", "Split the writer from the clock"), 26, 0, true);
const check = live(asRow(featureRunning(), 86, "dash-issue", "Shorten the reconnect wait"), 33, 1, true);

const now: Record<string, CallView> = {
  [plan.job.id]: {
    running: [{ key: "r", of: "drone", name: "Drone on Plan", line: "Weighing the split against the wrap", state: "running" }],
    asks: [
      {
        key: "p",
        kind: "plan",
        context: [
          "Writer::tick() reads Clock::now() under the same lock that guards the buffer.",
          "Splitting touches 6 call sites and the Writer constructor.",
          "Wrapping keeps the lock and adds a second timer; the stall stays.",
        ],
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
    running: [{ key: "r", of: "drone", name: "Drone on Implement", line: "Waiting on the store test", state: "running" }],
    issues: [
      {
        key: "i1",
        of: "check",
        name: "store",
        text: "store: 2 failed",
        said: "Check failed",
        context: [
          "FAIL store::reconnect::waits_five_seconds",
          "  expected: 5s",
          "  actual:   30s",
          "FAIL store::reconnect::caps_at_sixty",
          "  assertion failed: backoff(9) <= 60s",
        ],
      },
    ],
  },
  [debounce.job.id]: { running: [{ key: "r", of: "drone", name: "Drone on Implement", line: "Editing resize-handler.ts", state: "running" }] },
  [cache.job.id]: { running: [{ key: "r", of: "drone", name: "Drone on Write tests", line: "cargo test -p manifest read_again", state: "running" }] },
  [mainChecks.job.id]: { running: [{ key: "r", of: "drone", name: "Drone on Fix", line: "Reading the failing snapshot", state: "running" }] },
  [migrate.job.id]: { running: [{ key: "r", of: "drone", name: "Drone on Scope", line: "Listing the migrations since 0042", state: "running" }] },
};

/** What each Job runs now, and what waits on what: the Now panel's own data, by Job id. */
const nows: Record<string, NowView> = {
  [debounce.job.id]: {
    running: [
      { key: "d", of: "drone", name: "Drone on Implement", line: "Editing resize-handler.ts", step: { id: "implement", name: "Implement" }, state: "running", tail: ["Read resize-handler.ts", "Edit resize-handler.ts: debounce the observer", "Run pnpm test -p desktop resize"] },
      { key: "c", of: "check", name: "lint", step: { id: "implement", name: "Implement" }, state: "running" },
    ],
  },
  [cache.job.id]: {
    running: [
      { key: "d", of: "drone", name: "Drone on Write tests", line: "cargo test -p manifest read_again", state: "running" },
      { key: "c1", of: "check", name: "typecheck", state: "passed" },
      { key: "c2", of: "check", name: "manifest_test", state: "running" },
    ],
  },
  [mainChecks.job.id]: {
    running: [
      { key: "d", of: "drone", name: "Drone on Fix", line: "Reading the failing snapshot", state: "running" },
      { key: "j", of: "judge", name: "Judge on the plan", line: "Reading the plan against the diff", state: "running" },
    ],
  },
  [migrate.job.id]: { running: [{ key: "d", of: "drone", name: "Drone on Scope", line: "Listing the migrations since 0042", state: "running" }] },
  [pause.job.id]: { waiting: [{ key: "w", kind: "resource", text: "A free worktree slot" }] },
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

const quiet = { state: "idle" } as const;
const said = (id: string, text: string) => [{ id, at: "14:00:00", kind: "message", from: { kind: "agent" }, text } as const];

/** Two at work, one that will ask for a command, two idle beside them. */
const SESSIONS: Session[] = [
  { id: "s9", title: "Flaky store test", turn: { state: "working" }, lastTurn: "14:02", lastTurnAt: ago(2), rows: said("s9-0", "Replacing the 200ms sleep in store_test.rs with a wait on the channel."), attachments: [] },
  { id: "s10", title: "Migration notes", turn: { state: "working" }, lastTurn: "14:05", lastTurnAt: ago(1), rows: said("s10-0", "Reading the migrations since 0042."), attachments: [] },
  { id: "s12", title: "Theme token audit", turn: quiet, lastTurn: "13:51", lastTurnAt: ago(14), rows: said("s12-0", "Eleven tokens have no caller."), attachments: [] },
];

/** Sessions as the walk's other scenarios hold them, with `s9` turning to wait on a command at the third moment. */
function asking(store: SessionsStore): SessionsStore {
  const listeners = new Set<() => void>();
  let moment = 0;
  let version = 0;
  let cached: { base: readonly Session[]; version: number; out: readonly Session[] } | undefined;
  const get = () => {
    const base = store.get();
    if (cached !== undefined && cached.base === base && cached.version === version) return cached.out;
    const out = version === 0 ? base : base.map((one) => (one.id === "s9" ? { ...one, turn: quiet, lastTurnAt: ago(0), asked: { command: "git push --force-with-lease origin fix/flaky-store" } } : one));
    cached = { base, version, out };
    return out;
  };
  return {
    ...store,
    get,
    subscribe: (onChange) => {
      listeners.add(onChange);
      const off = store.subscribe(onChange);
      return () => (listeners.delete(onChange), off());
    },
    later() {
      moment += 1;
      if (moment !== 3) return;
      version += 1;
      listeners.forEach((one) => one());
    },
  };
}

function build(): Scenario {
  const board = [debounce, cache, mainChecks, migrate, pause];
  const base = holding("dashboard-cockpit", "Command Central with Jobs and Sessions at work, and three calls that arrive one at a time", board);
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  const jobs = board.map((one) => one.job);
  const state = { ...base.state, repository: repository().root, mergeLines: mergeLines(), jobs };
  return {
    ...base,
    state,
    reads: { ...base.reads, [plan.job.id]: plan, [check.job.id]: check },
    later: [{ jobs: [...jobs, plan.job] }, { jobs: [...jobs, plan.job, check.job] }, {}],
    draft: { calls: now, now: nows, sessions: (control) => asking(sessionsStore([none, none], none, [], control, [], SESSIONS)) },
  };
}

export const s206DashboardCockpit: Scenario = build();
