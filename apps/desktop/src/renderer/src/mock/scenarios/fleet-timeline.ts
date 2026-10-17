// The Cockpit's timeline over `dashboard-cockpit`'s Sessions, with the last two hours of what Fleet did to
// them: one needing an answer, one that went quiet, a Check red on a pull request, main red, work that
// landed. Nothing arrives later, so a walk reads one frame. The walks
// `fleet-timeline-feed`, `fleet-timeline-lanes` and `fleet-timeline-split` play it.

import type { TimelineEvent, TimelineSubject } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s206DashboardCockpit as base } from "./dashboard-cockpit";

const FORGE = "https://git.example/armada/pull/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** What an entry happened to, and what it was called then. */
type Who = { subject: TimelineSubject; title?: string };

const work = (id: string, title: string): Who => ({ subject: { kind: "session", id }, title });
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

const landed = work("s91", "Fold the two notification routes into one");
const broke = work("s92", "Move the store reads behind one trait");
const debounce = work("s80", "Debounce the resize handler");
const cache = work("s81", "Cache the manifest read between dispatches");
const fixMain = work("s82", "Fix components_test on main");
const migrate = work("s83", "Order the store migrations");
const pin = work("s87", "Pin the clock in the store tests");
const pwa = work("s88", "Pair a phone with Fleet");
const shell = work("s90", "Install the PWA shell offline");
const POCKET = { repo: "pocket" };

const s9 = session("s9", "Flaky store test");
const s10 = session("s10", "Migration notes");
const s11 = session("s11", "Release script");
const s12 = session("s12", "Theme token audit");

const EVENTS: TimelineEvent[] = [
  // Work that is over.
  event(168, broke, "session.started", "run", "Started", { live: true }),
  event(150, broke, "session.turn", "ok", "Turn ended", { live: true }),
  event(133, broke, "session.turn", "bad", "Check tests failed", { detail: "store::reads::roundtrip", live: true }),
  event(130, broke, "session.ended", "bad", "Ended", { over: true }),
  event(118, landed, "session.started", "run", "Started", { live: true }),
  event(96, landed, "session.turn", "ok", "Turn ended", { live: true }),
  event(84, landed, "session.turn", "ok", "Turn ended", { live: true }),
  event(62, landed, "session.turn", "ok", "Turn ended", { live: true }),
  event(55, landed, "session.ended", "ok", "Ended", { over: true }),
  event(57, pull(1889, "Fold the two notification routes into one"), "pr.opened", "idle", "Opened", { detail: "armada/91-landed" }),
  event(56, pull(1889, "Fold the two notification routes into one"), "pr.checks", "ok", "Checks passed"),
  event(55, pull(1889, "Fold the two notification routes into one"), "pr.merged", "ok", "Merged", { over: true }),
  event(152, s11, "session.started", "run", "Started", { live: true }),
  event(150, s11, "session.ended", "idle", "Ended", { over: true }),

  // A Session at work, asked once and answered once.
  event(44.5, debounce, "session.started", "run", "Started", { live: true }),
  event(43, debounce, "slot.leased", "idle", "Slot 1 leased", { detail: "slot-1" }),
  event(40, debounce, "session.turn", "ok", "Turn ended", { live: true }),
  event(39, debounce, "session.turn", "run", "Turn started", { live: true }),
  event(35, debounce, "session.asked", "ask", "Asked: split or wrap?", { detail: "Split or wrap" }),
  event(34, debounce, "session.answered", "run", "Answered: wrap it in place", { live: true }),
  event(29, debounce, "session.turn", "bad", "Check lint failed", { detail: "lint", live: true }),
  event(27, debounce, "session.turn", "ok", "Check lint passed", { detail: "lint", live: true }),
  event(6, debounce, "session.edited", "run", "3 files changed", { detail: "resize-handler.ts", live: true }),
  event(19, pull(1890, "Gate policy on every run"), "pr.opened", "idle", "Opened", { detail: "fleet/gate-policy-every-run" }),
  event(14, pull(1890, "Gate policy on every run"), "pr.checks", "ok", "Checks passed"),
  event(13, pull(1890, "Gate policy on every run"), "pr.queued", "idle", "Queued, place 1"),

  // A Session that asked and has not been answered.
  event(19, cache, "session.started", "run", "Started", { live: true }),
  event(18, cache, "slot.leased", "idle", "Slot 2 leased", { detail: "slot-2" }),
  event(14, cache, "session.turn", "ok", "Turn ended", { live: true }),
  event(12, cache, "session.asked", "ask", "Asked to run a command", { detail: "cargo test -p manifest read_again" }),

  // Main is red and a fix is on it.
  event(70, MAIN, "main.green", "ok", "Main green"),
  event(12, MAIN, "main.red", "bad", "Main red", { detail: "components_test" }),
  event(10, fixMain, "session.started", "run", "Started", { live: true }),
  event(9.5, fixMain, "slot.leased", "idle", "Slot 7 leased", { detail: "slot-7" }),
  event(4, fixMain, "session.turn", "run", "Reading components_test", { live: true }),

  // A Session that went quiet.
  event(12, shell, "session.started", "run", "Started", { live: true, ...POCKET }),
  event(11.4, shell, "slot.leased", "idle", "Slot 2 leased", { detail: "slot-2", ...POCKET }),

  // One Session starting another.
  event(7, s10, "session.started", "run", "Started", { live: true }),
  event(3.5, s10, "session.turn", "run", "Started a Session", { detail: "Order the store migrations", live: true }),
  event(3.4, migrate, "session.started", "run", "Started", { live: true }),
  event(3.2, migrate, "slot.leased", "idle", "Slot 4 leased", { detail: "slot-4" }),
  event(1.5, migrate, "session.turn", "run", "Turn started", { live: true }),
  event(1, s10, "session.turn", "run", "Reading the migrations since 0042", { live: true }),
  event(2.2, pin, "session.started", "run", "Started", { live: true }),
  event(0.5, pin, "session.turn", "run", "Turn started", { live: true }),

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
  event(25, pwa, "session.started", "run", "Started", { live: true, ...POCKET }),
  event(24, pwa, "slot.leased", "idle", "Slot 1 leased", { detail: "slot-1", ...POCKET }),
  event(18, pwa, "session.turn", "ok", "Turn ended", { live: true, ...POCKET }),
  event(17, pwa, "session.turn", "run", "Turn started", { live: true, ...POCKET }),
  event(9, pwa, "session.turn", "ok", "Check typecheck passed", { detail: "typecheck", live: true, ...POCKET }),
  event(10, pull(1895, "Read the pairing code"), "pr.opened", "idle", "Opened", { detail: "pocket/pairing-code", ...POCKET }),
  event(8, pull(1895, "Read the pairing code"), "pr.checks", "ok", "Checks passed", POCKET),
];

export const s207FleetTimeline: Scenario = {
  ...base,
  name: "fleet-timeline",
  says: "The Cockpit's timeline: two hours of what Fleet did to Sessions and pull requests in two repositories",
  later: [],
  draft: { ...base.draft, events: EVENTS },
};
