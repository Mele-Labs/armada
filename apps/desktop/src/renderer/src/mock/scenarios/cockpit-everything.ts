// Every change to the Cockpit in one sitting: `cockpit-timeline-first`'s timeline, with a pull request on the
// track of the Job or Session that owns it, over `cockpit-calls-arrive`'s two calls, which arrive one after
// the other in the tray. No repository is picked, so Autopilot asks for one. At the fourth moment four
// Sessions report in, each as `session-standing` reads it: one replied and waits, one said it is done, one
// went quiet mid-turn, one asked a question. The walk `cockpit-everything` plays it.

import type { TimelineEvent } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import { cockpitCallsArrive as arriving } from "./cockpit-calls-arrive";
import { s242CockpitTimelineFirst as base } from "./cockpit-timeline-first";

/** The Job that asks in each of `cockpit-calls-arrive`'s later moments, the last Job it names. */
const asking = (moment: number) => {
  const jobs = arriving.later?.[moment];
  if (typeof jobs === "function" || jobs?.jobs === undefined) throw new Error(`cockpit-calls-arrive holds no Job at moment ${moment}`);
  return jobs.jobs[jobs.jobs.length - 1]!;
};
const plan = asking(1);
const check = asking(2);

const NONE = { id: "x", number: 0, title: "", branch: "", slot: 0 };
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const said = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "agent" }, text }) as const;
const asked = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "you" }, text }) as const;

const REPLIED_AT = ago(6);
const DONE_AT = ago(4);

const replied: Session = {
  id: "r1",
  title: "Trim the fixture clock",
  turn: { state: "idle" },
  lastTurn: "14:12",
  lastTurnAt: REPLIED_AT,
  standing: "replied",
  rows: [
    asked("r1-u", "14:02:00", "Trim the clock out of the store fixtures."),
    said("r1-a", "14:12:09", "The clock is gone from all four store fixtures.\nTwo files changed, and the suite passes."),
  ],
  attachments: [],
  waitingFor: [{ id: `replied:${REPLIED_AT}`, text: "The clock is gone from all four store fixtures.", since: REPLIED_AT, source: "replied" }],
};

const done: Session = {
  id: "d1",
  title: "Changelog script",
  turn: { state: "idle" },
  lastTurn: "14:14",
  lastTurnAt: DONE_AT,
  standing: "done",
  done: { since: DONE_AT },
  rows: [
    asked("d1-u", "14:01:00", "Write the changelog script and merge it."),
    said("d1-a", "14:14:30", "The script is merged. There is nothing left to do."),
  ],
  attachments: [],
  waitingFor: [{ id: `done:${DONE_AT}`, text: "Done", since: DONE_AT, source: "done" }],
};

const stuck: Session = {
  id: "k1",
  title: "Unused token audit",
  turn: { state: "working" },
  lastTurn: "13:50",
  lastTurnAt: ago(25),
  standing: "stuck",
  rows: [
    asked("k1-u", "13:40:00", "List the tokens no stylesheet reads."),
    { id: "k1-t", at: "13:50:11", kind: "tool", text: "Grep packages/tokens/src -e --status" },
  ],
  attachments: [],
};

const asking1: Session = {
  id: "o1",
  title: "Cache key",
  turn: { state: "idle" },
  lastTurn: "14:09",
  lastTurnAt: ago(9),
  standing: "idle",
  rows: [
    asked("o1-u", "14:00:00", "Cache the manifest read."),
    said("o1-a", "14:09:20", "I can key the cache on the manifest hash or on its path."),
  ],
  attachments: [],
  waitingFor: [{ id: "o1-q", text: "Key the cache on the manifest hash or on its path?", since: ago(9), source: "agent" }],
};

const REPORTING = [replied, done, stuck, asking1];
const HIDDEN = new Set(REPORTING.map((one) => one.id));

let count = 2000;
const event = (minutes: number, session: Session, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, live: boolean): TimelineEvent => {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: { kind: "session", id: session.id }, title: session.title ?? session.id, tone, says, manifest_id: "armada", ...(live ? { live: true } : {}) };
};

/** Each Session's last event is a live turn, as Fleet writes one when a turn ends. */
const REPORTS: TimelineEvent[] = [
  event(24, replied, "session.started", "run", "Started", true),
  event(6, replied, "session.turn", "run", "Trimmed the fixture clock", true),
  event(22, done, "session.started", "run", "Started", true),
  event(4, done, "session.turn", "run", "Merged the script", true),
  event(40, stuck, "session.started", "run", "Started", true),
  event(25, stuck, "session.turn", "run", "Searching the tokens", true),
  event(18, asking1, "session.started", "run", "Started", true),
  event(9, asking1, "session.asked", "ask", "Asked: manifest hash or path?", false),
];

export const cockpitEverything: Scenario = {
  ...base,
  name: "cockpit-everything",
  says: "The Cockpit on its timeline with pull requests on their tracks, Autopilot in the header band, two calls that arrive in the tray one after the other, and then four Sessions that report in",
  // The first moment is a beat with nothing in it: a call is only news once Bridge has settled on what it holds.
  later: [{}, { jobs: [...base.state.jobs, plan] }, { jobs: [...base.state.jobs, check] }],
  reads: { ...base.reads, [plan.id]: arriving.reads[plan.id]!, [check.id]: arriving.reads[check.id]! },
  draft: {
    ...base.draft,
    events: [...(base.draft?.events ?? []), ...REPORTS],
    calls: { ...base.draft?.calls, ...arriving.draft?.calls },
    now: { ...base.draft?.now, ...arriving.draft?.now },
    // The base's Sessions, and the four that report in at the fourth moment: held from the start, so
    // answering and ending them does what it does for any Session, and shown only once they report.
    sessions: (control) => {
      const held = base.draft!.sessions!(control);
      const usual = held.get();
      held.dispose();
      const store = sessionsStore([NONE, NONE], NONE, [], control, [], [...usual, ...REPORTING]);
      const listeners = new Set<() => void>();
      let moment = 0;
      let cached: { from: readonly Session[]; out: readonly Session[] } | undefined;
      const get = () => {
        const all = store.get();
        if (moment >= 4) return all;
        if (cached?.from !== all) cached = { from: all, out: all.filter((one) => !HIDDEN.has(one.id)) };
        return cached.out;
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
          store.later();
          moment += 1;
          if (moment === 4) listeners.forEach((one) => one());
        },
      };
    },
  },
};
