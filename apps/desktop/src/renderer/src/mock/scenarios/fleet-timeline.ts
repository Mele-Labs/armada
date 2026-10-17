// The Cockpit's timeline over `dashboard-cockpit`'s Jobs and Sessions, with the last two hours of what
// Fleet did to them: a Job needing an answer, one that went quiet, a Check red on a pull request, main
// red with a fix on it, work that landed. Nothing arrives later, so a walk reads one frame. The walks
// `fleet-timeline-feed`, `fleet-timeline-lanes` and `fleet-timeline-split` play it, and `jobs-page`
// plays the Jobs rail row over it.

import type { TimelineEvent, TimelineSubject } from "@armada/protocol";
import type { NowView } from "@armada/screens/src/draft/now";

import type { Scenario } from "../moment";
import { s206DashboardCockpit as base } from "./dashboard-cockpit";

const jobs = base.state.jobs;
const FORGE = "https://git.example/armada/pull/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** What an entry happened to, and what it was called then. */
type Who = { subject: TimelineSubject; title?: string };

const job = (number: number): Who => {
  const one = jobs.find((row) => row.handle.startsWith(`${number}-`))!;
  return { subject: { kind: "job", id: one.id }, title: one.title };
};
const session = (id: string, title: string): Who => ({ subject: { kind: "session", id }, title });
const pull = (number: number, title: string): Who => ({ subject: { kind: "pull_request", number, url: `${FORGE}${number}` }, title });
const MAIN: Who = { subject: { kind: "main" } };

let count = 0;
/** One entry, `minutes` ago, as Fleet would number and serve it. */
function event(minutes: number, who: Who, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, rest: { detail?: string; live?: boolean; over?: boolean; repo?: string } = {}): TimelineEvent {
  count += 1;
  const { repo, ...more } = rest;
  return { id: count, at: ago(minutes), kind, subject: who.subject, ...(who.title === undefined ? {} : { title: who.title }), tone, says, manifest_id: repo ?? "armada", ...more };
}

const landed = job(91);
const broke = job(92);
const debounce = job(80);
const cache = job(81);
const fixMain = job(82);
const migrate = job(83);
const pause = job(84);
const pin = job(87);
const pwa = job(88);
const code = job(89);
const shell = job(90);
const POCKET = { repo: "pocket" };

const s9 = session("s9", "Flaky store test");
const s10 = session("s10", "Migration notes");
const s11 = session("s11", "Release script");
const s12 = session("s12", "Theme token audit");

const EVENTS: TimelineEvent[] = [
  // Work that is over.
  event(168, broke, "job.created", "idle", "Dispatched"),
  event(167, broke, "job.started", "run", "Started", { live: true }),
  event(150, broke, "job.step", "ok", "Scope passed", { live: true }),
  event(133, broke, "job.step", "bad", "Tests failed", { detail: "store::reads::roundtrip", live: true }),
  event(130, broke, "job.failed", "bad", "Failed at Tests", { over: true }),
  event(118, landed, "job.created", "idle", "Dispatched"),
  event(117, landed, "job.started", "run", "Started", { live: true }),
  event(96, landed, "job.step", "ok", "Scope passed", { live: true }),
  event(84, landed, "job.step", "ok", "Implement passed", { live: true }),
  event(62, landed, "job.step", "ok", "Review approved", { live: true }),
  event(55, landed, "job.landed", "ok", "Landed on main", { over: true }),
  event(57, pull(1889, "Fold the two notification routes into one"), "pr.opened", "idle", "Opened", { detail: "armada/91-landed" }),
  event(56, pull(1889, "Fold the two notification routes into one"), "pr.checks", "ok", "Checks passed"),
  event(55, pull(1889, "Fold the two notification routes into one"), "pr.merged", "ok", "Merged", { over: true }),
  event(152, s11, "session.started", "run", "Started", { live: true }),
  event(150, s11, "session.ended", "idle", "Ended", { over: true }),

  // A Job at work, sent back once and answered once.
  event(44.5, debounce, "job.created", "idle", "Dispatched"),
  event(43, debounce, "slot.leased", "idle", "Slot 1 leased", { detail: "slot-1" }),
  event(41, debounce, "job.started", "run", "Started", { live: true }),
  event(40, debounce, "job.step", "ok", "Scope passed", { live: true }),
  event(39, debounce, "job.step", "run", "Implement started", { live: true }),
  event(35, debounce, "job.asked", "ask", "Asked: split or wrap?", { detail: "Plan question" }),
  event(34, debounce, "job.answered", "run", "Answered: wrap it in place", { live: true }),
  event(29, debounce, "job.check", "bad", "Check lint failed", { detail: "lint", live: true }),
  event(27, debounce, "job.check", "ok", "Check lint passed", { detail: "lint", live: true }),
  event(24, debounce, "job.judged", "bad", "Implement sent back, 1 of 2", { live: true }),
  event(23, debounce, "job.step", "run", "Implement started again", { live: true }),
  event(6, debounce, "job.files", "run", "3 files changed", { detail: "resize-handler.ts", live: true }),
  event(19, pull(1890, "Gate policy on every run"), "pr.opened", "idle", "Opened", { detail: "fleet/gate-policy-every-run" }),
  event(14, pull(1890, "Gate policy on every run"), "pr.checks", "ok", "Checks passed"),
  event(13, pull(1890, "Gate policy on every run"), "pr.queued", "idle", "Queued, place 1"),
  event(1.2, pause, "job.queued", "idle", "Queued, waits on Job 80"),

  // A Job that asked and has not been answered.
  event(19, cache, "job.created", "idle", "Dispatched"),
  event(18, cache, "slot.leased", "idle", "Slot 2 leased", { detail: "slot-2" }),
  event(17, cache, "job.started", "run", "Started", { live: true }),
  event(14, cache, "job.step", "ok", "Scope passed", { live: true }),
  event(12, cache, "job.asked", "ask", "Asked to run a command", { detail: "cargo test -p manifest read_again" }),

  // Main is red and a fix is on it.
  event(70, MAIN, "main.green", "ok", "Main green"),
  event(12, MAIN, "main.red", "bad", "Main red", { detail: "components_test" }),
  event(10, fixMain, "job.created", "idle", "Dispatched to fix main"),
  event(9.5, fixMain, "slot.leased", "idle", "Slot 7 leased", { detail: "slot-7" }),
  event(9, fixMain, "job.started", "run", "Started", { live: true }),
  event(4, fixMain, "job.judged", "run", "Judge reads the plan", { live: true }),

  // A Job that went quiet.
  event(12, shell, "job.created", "idle", "Dispatched", POCKET),
  event(11.4, shell, "slot.leased", "idle", "Slot 2 leased", { detail: "slot-2", ...POCKET }),
  event(11, shell, "job.started", "run", "Started", { live: true, ...POCKET }),

  // One Job dispatching another, and the Session behind it.
  event(7, s10, "session.started", "run", "Started", { live: true }),
  event(3.5, s10, "session.turn", "run", "Dispatched Job 83", { detail: "Order the store migrations", live: true }),
  event(3.4, migrate, "job.created", "idle", "Dispatched by Migration notes"),
  event(3.2, migrate, "slot.leased", "idle", "Slot 4 leased", { detail: "slot-4" }),
  event(3, migrate, "job.started", "run", "Started", { live: true }),
  event(1.5, migrate, "job.step", "run", "Scope started", { live: true }),
  event(1, s10, "session.turn", "run", "Reading the migrations since 0042", { live: true }),
  event(2.2, pin, "job.created", "idle", "Dispatched by Job 83"),
  event(1.9, pin, "job.started", "run", "Started", { live: true }),
  event(0.5, pin, "job.step", "run", "Scope started", { live: true }),

  // A Session that asked for a command and was allowed.
  event(21, s9, "session.started", "run", "Started", { live: true }),
  event(20, s9, "slot.leased", "idle", "Slot 3 leased", { detail: "slot-3" }),
  event(14, s9, "session.asked", "ask", "Asked to run a command", { detail: "git push --force-with-lease origin fix/flaky-store" }),
  event(12, s9, "session.answered", "run", "Answered: allow", { live: true }),
  event(2, s9, "session.turn", "run", "Replacing the sleep with a wait on the channel", { live: true }),
  event(30, s12, "session.started", "run", "Started", { live: true }),
  event(14, s12, "session.turn", "idle", "Eleven tokens have no caller"),

  // A pull request with a Check red, and two that are waiting.
  event(52, pull(1893, "Bump the lockfile"), "pr.opened", "idle", "Opened", { detail: "chore/bump-the-lockfile" }),
  event(49, pull(1893, "Bump the lockfile"), "pr.checks", "ok", "Checks passed"),
  event(25, pull(1894, "Fix the reconnect wait"), "pr.opened", "idle", "Opened", { detail: "fleet/reconnect-wait" }),
  event(21, pull(1894, "Fix the reconnect wait"), "pr.checks", "run", "Checks running", { live: true }),
  event(6, pull(1894, "Fix the reconnect wait"), "pr.checks", "bad", "Check store failed", { detail: "store: 2 failed" }),
  event(1.2, pull(1891, "Pin the store clock"), "pr.opened", "idle", "Opened", { detail: "fix/pin-store-clock" }),
  event(1.1, pull(1891, "Pin the store clock"), "pr.checks", "run", "Checks running", { live: true }),

  // The second repository.
  event(25, pwa, "job.created", "idle", "Dispatched", POCKET),
  event(24, pwa, "slot.leased", "idle", "Slot 1 leased", { detail: "slot-1", ...POCKET }),
  event(22, pwa, "job.started", "run", "Started", { live: true, ...POCKET }),
  event(18, pwa, "job.step", "ok", "Scope passed", { live: true, ...POCKET }),
  event(17, pwa, "job.step", "run", "Implement started", { live: true, ...POCKET }),
  event(9, pwa, "job.check", "ok", "Check typecheck passed", { detail: "typecheck", live: true, ...POCKET }),
  event(10, pull(1895, "Read the pairing code"), "pr.opened", "idle", "Opened", { detail: "pocket/pairing-code", ...POCKET }),
  event(8, pull(1895, "Read the pairing code"), "pr.checks", "ok", "Checks passed", POCKET),
  event(4, code, "job.queued", "idle", "Queued, waits on Job 88", POCKET),
];

/** What each running Job's Drone did last, and how many seconds ago: the Jobs page's Now line. */
const DOING: Record<number, readonly [string, number]> = {
  80: ["Edit apps/desktop/src/renderer/src/resize-handler.ts", 12],
  81: ["cargo test -p manifest read_again", 40],
  82: ["Read crates/store/tests/snapshots/store.snap", 4],
  83: ["Grep migrations/ -e CREATE", 9],
  87: ["Read crates/store/tests/store_test.rs", 2],
  88: ["Write apps/pocket/src/pairing.tsx", 21],
  90: ["Write apps/pocket/src/service-worker.ts", 11 * 60],
};

/** The scenario's `now` reads, each running Drone given the line it last wrote. */
function nowing(): Record<string, NowView> {
  const held = base.draft?.now ?? {};
  return Object.fromEntries(
    Object.entries(held).map(([id, view]) => {
      const number = Number(jobs.find((row) => row.id === id)?.handle.split("-")[0]);
      const said = DOING[number];
      if (said === undefined) return [id, view];
      const [line, seconds] = said;
      const running = (view.running ?? []).map((one) => (one.of === "drone" ? { ...one, tail: [line], since: ago(seconds / 60) } : one));
      return [id, { ...view, running }];
    }),
  );
}

/** How far each running Job's plan has come: done, working, open. The base gives every Job the same counts. */
const TASKS: Record<number, readonly [number, number, number]> = { 80: [3, 1, 1], 81: [1, 1, 3], 82: [0, 1, 2], 83: [0, 1, 4], 87: [0, 0, 3], 88: [2, 1, 1], 90: [3, 1, 2] };

export const s207FleetTimeline: Scenario = {
  ...base,
  name: "fleet-timeline",
  says: "The Cockpit's timeline: two hours of what Fleet did to Jobs, Sessions and pull requests in two repositories",
  later: [],
  state: {
    ...base.state,
    jobs: base.state.jobs.map((one) => {
      const counts = TASKS[Number(one.handle.split("-")[0])];
      return counts === undefined ? one : { ...one, tasks: { done: counts[0], working: counts[1], open: counts[2], dropped: 0 } };
    }),
  },
  draft: { ...base.draft, events: EVENTS, now: nowing() },
};
