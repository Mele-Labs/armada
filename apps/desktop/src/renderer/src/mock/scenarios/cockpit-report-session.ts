// A Session open on the flight deck's stage, with enough on its record that a report about it carries something to read: its
// directory, model, context, branch and slot, a pull request whose Check failed, a question it waits on him for, a
// thread, and the timeline's entries for it. Report starts a Session beside it, as the wired draft does, and the walk lands on it.
// The claims are in `walks/cockpit-report-session.ts`.

import type { TimelineEvent } from "@armada/protocol";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });
const tool = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "tool", text });

const REPORTED: Session = {
  id: "6f2b9c1e-3d47-4a8b-9e10-5c7d2a4b8f31",
  title: "Pin the store clock",
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  resolvedModel: "vendor-opus-5-5",
  effort: "high",
  mode: "auto",
  context: { tokens: 142_000, window: 200_000 },
  turn: { state: "idle" },
  lastTurnAt: ago(9),
  standing: "idle",
  trail: { started: ago(75), bays: [{ slot: 4, from: ago(60) }] },
  rows: [
    you("r-1", "13:05:00", "Pin the clock in the store tests, then open a pull request."),
    tool("r-2", "13:40:12", "Edit crates/store/tests/clock.rs"),
    agent("r-3", "13:41:30", "The clock is pinned in clock.rs. The pool test still reads the wall clock."),
    tool("r-4", "13:52:05", "Bash cargo test -p store"),
    tool("r-5", "13:58:40", "Bash git push origin main"),
    agent("r-6", "14:00:20", "Pushed. The pull request is open and desktop_test is red."),
    you("r-7", "14:02:00", "Why did it go to main?"),
    agent("r-8", "14:11:03", "I used the wrong remote ref. The branch is fix/pin-store-clock."),
  ],
  attachments: [
    { kind: "branch", name: "fix/pin-store-clock", slot: 4 },
    { kind: "pull_request", number: 1861, title: "Pin the store clock", branch: "fix/pin-store-clock", address: "https://git.example/armada/pull/1861", checks: { state: "failed", failing: "desktop_test" }, state: "open", auto: false },
  ],
  waitingFor: [{ id: `q:${ago(8)}`, text: "Revert the push to main, or leave it?", since: ago(8), source: "agent" }],
};

const who = { subject: { kind: "session", id: REPORTED.id }, title: REPORTED.title } as const;
let count = 0;
const event = (minutes: number, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, detail?: string): TimelineEvent => {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: who.subject, title: who.title, tone, says, manifest_id: OWNER, ...(detail === undefined ? {} : { detail }) };
};
const EVENTS: TimelineEvent[] = [
  event(75, "session.started", "run", "Started"),
  event(60, "slot.leased", "idle", "Slot 4 leased", "slot-4"),
  event(24, "session.committed", "run", "Committed", "fix/pin-store-clock"),
  event(11, "session.pushed", "bad", "Pushed to main", "main"),
  event(9, "session.turn", "idle", "Replied"),
];

function build(): Scenario {
  const base = holding("cockpit-report-session", "The flight deck with one Session open that has a branch, a pull request with a red Check and a question waiting on him");
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: {
      events: EVENTS,
      sessions: () => {
        const store = sessionsStore([REPORTED]);
        // The ship holds this Session and the ones a report starts beside it, not the usual fixtures.
        const own = new Set([REPORTED.id]);
        // The mock's first start is a fixture Session with a script of its own; it is spent here and kept off the ship.
        store.start();
        let from: readonly Session[] | undefined;
        let only: readonly Session[] = [];
        return {
          ...store,
          get: () => {
            const all = store.get();
            if (all !== from) {
              from = all;
              only = all.filter((one) => own.has(one.id));
            }
            return only;
          },
          // A Session started in a repository works in it, which the wired draft's `root` says and the mock's does not.
          start: (tag, root) => {
            const id = store.start(tag, root);
            if (typeof id === "string") {
              own.add(id);
              store.edit(id, (one) => ({ ...one, manifestId: OWNER }));
            }
            return id;
          },
        };
      },
    },
  };
}

export const s255CockpitReportSession: Scenario = build();
