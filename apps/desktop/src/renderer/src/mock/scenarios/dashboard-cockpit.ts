// Command Central as a cockpit: five Jobs and four Sessions under way and nothing asking, then three
// calls that arrive one at a time — a Plan question, a failed Check, a Session waiting on a command —
// each as a walk's `later` step lets time pass. A call's Job is on the Board only from the moment it
// asks, since a call is read off a Job's own view and the mock holds those from the start. The walk
// `dashboard-cockpit` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";
import { mergeLines } from "@armada/screens/src/fixtures/build/merge-line";
import type { MergeLines, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import { featureRunning } from "@armada/jobs/fake";
import { completedFailed, completedSuccess, queued, running } from "@armada/jobs/fixtures/build/index";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore, type SessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** The picked repository's, started `minutes` ago, on the step `id` of its workflow where one is named. */
function live(fixture: JobFixture, minutes: number, id?: string, asking = false, extra: Record<string, unknown> = {}): JobFixture {
  const patch = {
    owner_manifest_id: OWNER,
    started_at: ago(minutes),
    ...(asking ? { asking: true } : {}),
    ...(id === undefined ? {} : { current_step_id: id }),
    ...extra,
  };
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, ...patch };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = { ...fixture.watched.detail.job, ...patch };
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const debounce = live(asRow(featureRunning(), 80, "debounce", "Debounce the Job Board's resize handler"), 41, "implement", false, { branch: "fleet/gate-policy-every-run" });
const cache = live(asRow(featureRunning(), 81, "cache", "Cache the manifest read between dispatches"), 17, "tests", false, { branch: "fleet/read-in-cluster-membership" });
const mainChecks = live(asRow(running(), 82, "fix-main", "Fix components_test on main"), 9);
const migrate = live(asRow(featureRunning(), 83, "migrate", "Order the store migrations"), 3, "scope", false, { branch: "armada/83-migrate" });
const pause = live(asRow(queued(), 84, "pause", "Store a pause marker on the Job"), 1, undefined, false, { waits_on: [debounce.job.id] });
/** Dispatched by the Migration notes Session's Job: a child on the map. */
const pin = live(asRow(featureRunning(), 87, "pin", "Pin the clock in the store tests"), 2, "scope", false, { dispatched_by: migrate.job.id, origin: "sub_dispatched" });

/** A second repository, so there are two clusters to read: a phone app that pairs with Fleet. */
const POCKET = "pocket";
const pwa = live(asRow(featureRunning(), 88, "pwa", "Pair a phone with Fleet"), 22, "implement", false, { owner_manifest_id: POCKET });
const code = live(asRow(queued(), 89, "code", "Read the pairing code from the runtime file"), 4, undefined, false, { owner_manifest_id: POCKET, waits_on: [pwa.job.id] });
const shell = live(asRow(featureRunning(), 90, "shell", "Install the PWA shell offline"), 11, "tests", false, { owner_manifest_id: POCKET });

/** Work that is over, for the Done filter: two Jobs and a Session. */
function over(fixture: JobFixture, ended: number): JobFixture {
  const patch = { owner_manifest_id: OWNER, ended_at: ago(ended) };
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, ...patch };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = { ...fixture.watched.detail.job, ...patch };
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}
const landed = over(asRow(completedSuccess(), 91, "landed", "Fold the two notification routes into one"), 55);
const broke = over(asRow(completedFailed(), 92, "broke", "Move the store reads behind one trait"), 130);

/** The Jobs that ask, on the Board only from the moment they do. */
const plan = live(asRow(featureRunning(), 85, "dash-plan", "Split the writer from the clock"), 26, "scope", true);
const check = live(asRow(featureRunning(), 86, "dash-issue", "Shorten the reconnect wait"), 33, "implement", true);

const now: Record<string, CallView> = {
  [plan.job.id]: {
    request: "Fleet's event log writer and its heartbeat clock share one struct, so a slow disk stalls the heartbeat and Bridge marks Fleet unreachable. Pull the clock out so the heartbeat never waits on the writer. Keep the log format as it is, and do not touch the retry cap while you are in there. The store tests should pass untouched.",
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
    request: "When Bridge loses its socket to Fleet, it waits a flat 30s before reconnecting. Cut the first wait to 5s and back off from there, keeping the cap at 60s.",
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
  [pause.job.id]: { waiting: [{ key: "w", kind: "job", text: "Debounce the Job Board's resize handler", target: debounce.job.id }] },
  [pin.job.id]: { running: [{ key: "d", of: "drone", name: "Drone on Scope", line: "Reading store_test.rs", state: "running" }] },
  [pwa.job.id]: { running: [{ key: "d", of: "drone", name: "Drone on Implement", line: "Writing the pairing screen", state: "running" }] },
  [code.job.id]: { waiting: [{ key: "w", kind: "job", text: "Pair a phone with Fleet", target: pwa.job.id }] },
  [shell.job.id]: { running: [{ key: "d", of: "drone", name: "Drone on Write tests", line: "Caching the shell in a service worker", state: "running" }] },
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
  { id: "s9", title: "Flaky store test", turn: { state: "working" }, lastTurn: "14:02", lastTurnAt: ago(2), rows: [
      { id: "s9-u", at: "13:58:00", kind: "message", from: { kind: "you" }, text: "The store test is flaky on CI. Find out why and fix it, then push the branch when it is green." },
      { id: "s9-t", at: "13:59:10", kind: "tool", text: "Read crates/store/tests/store_test.rs" },
      ...said("s9-0", "The flake is the 200ms sleep in store_test.rs. Replacing it with a wait on the channel."),
    ], attachments: [{ kind: "branch", name: "fix/flaky-store", slot: 3 }] },
  { id: "s10", title: "Migration notes", turn: { state: "working" }, lastTurn: "14:05", lastTurnAt: ago(1), rows: said("s10-0", "Reading the migrations since 0042."), attachments: [{ kind: "job", id: migrate.job.id, number: 83, title: "Order the store migrations", state: "running", branch: "armada/83-migrate" }] },
  { id: "s11", title: "Release script", dead: "ended", turn: quiet, lastTurn: "11:40", lastTurnAt: ago(150), rows: said("s11-1", "Done."), attachments: [] },
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

const FORGE = "https://git.example/armada/pull/";

/** The worktree pool of each repository: who holds each bay, on which branch, and since when. */
function pool(): WorktreesHeld {
  const bay = (manifest: string, n: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({
    manifest_id: manifest,
    slot: n,
    path: `/Users/user/${manifest === OWNER ? "armada" : manifest}/.armada/slots/slot-${n}`,
    base: "main",
    warm: true,
    behind: 0,
    held,
    ...rest,
  });
  const job = (fixture: JobFixture, branch: string, minutes: number, manifest = OWNER) => bay(manifest, 0, { state: "job", job_id: fixture.job.id, job_title: fixture.job.title }, { branch, since: ago(minutes) });
  const at = (n: number, slot: WorktreeSlot) => ({ ...slot, slot: n, path: slot.path.replace("slot-0", `slot-${n}`) });
  return {
    worktrees: [],
    slots: [
      at(1, job(debounce, "fleet/gate-policy-every-run", 41)),
      at(2, job(cache, "fleet/read-in-cluster-membership", 17)),
      bay(OWNER, 3, { state: "session", holder: "claude (pid 5120)" }, { branch: "fix/flaky-store", since: ago(6), behind: 2 }),
      at(4, job(migrate, "armada/83-migrate", 3)),
      at(5, job(check, "fleet/reconnect-wait", 33)),
      at(6, job(pin, "fix/pin-store-clock", 2)),
      at(7, job(mainChecks, "fix/components-test-on-main", 9)),
      bay(OWNER, 8, { state: "free" }, { behind: 3 }),
      bay(OWNER, 9, { state: "free" }, { closed: true, warm: false, behind: 12 }),
      at(1, job(pwa, "pocket/pairing-code", 22, POCKET)),
      at(2, job(shell, "pocket/offline-shell", 11, POCKET)),
    ],
  };
}

/** The forge's open pull requests: two in the merge queue, three not. One of those three is running its checks, or failing them. */
function withPulls(lines: MergeLines, failing = false): MergeLines {
  const pull = (number: number, title: string, branch: string, more: Record<string, unknown> = {}) => ({ number, title, branch, url: `${FORGE}${number}`, ...more });
  return {
    lines: lines.lines.map((one) => ({
      ...one,
      hub: {
        pull_requests: [
          pull(1893, "Bump the lockfile", "chore/bump-the-lockfile", { ci: "passed" }),
          pull(1891, "Pin the store clock", "fix/pin-store-clock", { ci: "passed", queue: { state: "queued", position: 2 } }),
          pull(1894, "Fix the reconnect wait", "fleet/reconnect-wait", { ci: failing ? "failed" : "running" }),
          pull(1890, "Gate policy on every run", "fleet/gate-policy-every-run", { ci: "passed", queue: { state: "awaiting_checks", position: 1 } }),
          pull(1895, "Read the pairing code", "pocket/pairing-code", { ci: "passed" }),
        ],
      },
    })),
  };
}

function build(): Scenario {
  const board = [debounce, cache, mainChecks, migrate, pause, pin, pwa, code, shell, landed, broke];
  const pocket = { ...repository().manifest!, id: POCKET, repository: POCKET, path: "/Users/user/pocket", records_root: "/Users/user/pocket/.armada" };
  const base = holding("dashboard-cockpit", "The Dashboard with Jobs and Sessions at work in two repositories, and three calls that arrive one at a time", board, {
    alsoServed: [{ root: "/Users/user/pocket", records_root: "/Users/user/pocket/.armada", manifest: pocket }],
  });
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  const jobs = board.map((one) => one.job);
  const state = { ...base.state, repository: null, mergeLines: withPulls(mergeLines()), jobs };
  return {
    ...base,
    state,
    reads: { ...base.reads, [plan.job.id]: plan, [check.job.id]: check },
    held: pool(),
    later: [{ jobs: [...jobs, plan.job] }, { jobs: [...jobs, plan.job, check.job] }, {}, { mergeLines: withPulls(mergeLines(), true) }],
    draft: { calls: now, now: nows, sessions: (control) => asking(sessionsStore([none, none], none, [], control, [], SESSIONS)) },
  };
}

export const s206DashboardCockpit: Scenario = build();
