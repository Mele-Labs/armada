// Command Central as a cockpit: five Jobs and four Sessions under way and nothing asking, then three
// calls that arrive one at a time — a Plan question, a failed Check, a Session waiting on a command —
// each as a walk's `later` step lets time pass. A call's Job is on the Board only from the moment it
// asks, since a call is read off a Job's own view and the mock holds those from the start. The walk
// `dashboard-cockpit` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";
import { mergeLines } from "@armada/screens/src/fixtures/build/merge-line";
import type { MergeLines, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { featureRunning } from "@armada/screens/src/fixtures/jobs/feature-running";
import { completedFailed, completedSuccess, queued, running } from "@armada/screens/src/fixtures/build/index";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { COCKPIT_SESSIONS } from "../cockpit-sessions";
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

/** The Judges agreed `step` needs redoing and sent it back to its Drone once, of the two times they may. #2105. */
function sentBack(fixture: JobFixture, step: string): JobFixture {
  if (fixture.watched.state !== "read") return fixture;
  const steps = fixture.watched.detail.steps.map((one) => (one.step_id === step ? { ...one, sent_back: { times: 1, of: 2 } } : one));
  return { ...fixture, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, steps } } };
}

const debounce = sentBack(live(asRow(featureRunning(), 80, "debounce", "Debounce the Job Board's resize handler"), 41, "implement", false, { branch: "fleet/gate-policy-every-run" }), "implement");
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



const SESSIONS = COCKPIT_SESSIONS;
const quiet = { state: "idle" } as const;

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
      // Hosted Sessions: this one is mid-turn, which Fleet will not release the bay of.
      bay(OWNER, 3, { state: "hosted_session", session_id: "s9", session_title: "Flaky store test" }, { branch: "fix/flaky-store", since: ago(6), behind: 2 }),
      at(4, job(migrate, "armada/83-migrate", 3)),
      at(5, job(check, "fleet/reconnect-wait", 33)),
      at(6, job(pin, "fix/pin-store-clock", 2)),
      at(7, job(mainChecks, "fix/components-test-on-main", 9)),
      bay(OWNER, 8, { state: "free" }, { behind: 3 }),
      bay(OWNER, 9, { state: "free" }, { behind: 5 }),
      bay(OWNER, 10, { state: "free" }, { warm: false, behind: 9 }),
      bay(OWNER, 11, { state: "free" }, { closed: true, warm: false, behind: 12 }),
      bay(OWNER, 12, { state: "free" }, { closed: true, warm: false, behind: 12 }),
      bay(OWNER, 13, { state: "unmade" }, { closed: true, warm: false }),
      // Between turns, so Release frees the bay.
      bay(OWNER, 14, { state: "hosted_session", session_id: "s12", session_title: "Theme token audit" }, { branch: "fix/theme-token-audit", since: ago(14), behind: 1 }),
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
    draft: { sessions: (control) => asking(sessionsStore([none, none], none, control, [], SESSIONS)) },
  };
}

export const s206DashboardCockpit: Scenario = build();
