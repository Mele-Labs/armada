// Six Sessions, each in the state Fleet read it to be in, and the Cockpit and the Sessions page
// agreeing about every one. Two replied to him and ask nothing (one in a terminal, not stuck), one said it is done (not stuck),
// one went quiet mid-turn (stuck), one is at work, and one asked a question he will open. The
// timeline's last event for the replied and the done is a live turn, the case that used to read as
// moving and then as stuck. The walk `session-standing` plays it.

import type { TimelineEvent } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const said = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "agent" }, text }) as const;
const asked = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "you" }, text }) as const;

/** Its turn ended on a message to him, 6 minutes ago. */
const REPLIED_AT = ago(6);
/** It said nothing is left, 4 minutes ago. */
const DONE_AT = ago(4);

const replied: Session = {
  id: "r1",
  title: "Pin the store clock",
  turn: { state: "idle" },
  lastTurnAt: REPLIED_AT,
  standing: "replied",
  rows: [
    asked("r1-u", "14:02:00", "Pin the clock in the store tests."),
    said("r1-a", "14:12:09", "The clock is pinned in all four store tests and the fixture is gone.\nTwo files changed, and the suite passes."),
  ],
  attachments: [],
};

const done: Session = {
  id: "d1",
  title: "Changelog script",
  turn: { state: "idle" },
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

/** Replied 12 minutes ago in a terminal session; the person answers it there, which the Cockpit is never told. */
const TERMINAL_AT = ago(12);
const terminal: Session = {
  id: "t1",
  title: "Retry cap",
  terminal: true,
  turn: { state: "idle" },
  lastTurnAt: TERMINAL_AT,
  standing: "replied",
  rows: [
    asked("t1-u", "13:58:00", "Cap the retries at five."),
    said("t1-a", "14:06:12", "The cap is five and the backoff stops there."),
  ],
  attachments: [],
};

const stuck: Session = {
  id: "k1",
  title: "Theme token audit",
  turn: { state: "working" },
  lastTurnAt: ago(25),
  standing: "stuck",
  rows: [
    asked("k1-u", "13:40:00", "List the tokens no stylesheet reads."),
    { id: "k1-t", at: "13:50:11", kind: "tool", text: "Grep packages/tokens/src -e --status" },
  ],
  attachments: [],
};

const working: Session = {
  id: "w1",
  title: "Migration notes",
  turn: { state: "working" },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [
    asked("w1-u", "14:10:00", "Summarise the migrations since 0042."),
    { id: "w1-t", at: "14:17:02", kind: "tool", text: "Read crates/store/migrations/0043.sql" },
  ],
  attachments: [],
};

const asking: Session = {
  id: "o1",
  title: "Cache key",
  turn: { state: "idle" },
  lastTurnAt: ago(9),
  standing: "idle",
  rows: [
    asked("o1-u", "14:00:00", "Cache the manifest read."),
    said("o1-a", "14:09:20", "I can key the cache on the manifest hash or on its path."),
  ],
  attachments: [],
  waitingFor: [{ id: "o1-q", text: "Key the cache on the manifest hash or on its path?", since: ago(9), source: "agent" }],
};

let count = 0;
const event = (minutes: number, session: Session, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, live: boolean): TimelineEvent => {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: { kind: "session", id: session.id }, title: session.title ?? session.id, tone, says, manifest_id: OWNER, ...(live ? { live: true } : {}) };
};

/** Each Session's last event is a live turn, as Fleet writes one when a turn ends. */
const EVENTS: TimelineEvent[] = [
  event(24, replied, "session.started", "run", "Started", true),
  event(6, replied, "session.turn", "run", "Pinned the clock in the store tests", true),
  event(22, done, "session.started", "run", "Started", true),
  event(4, done, "session.turn", "run", "Merged the script", true),
  event(30, terminal, "session.started", "run", "Started", true),
  event(12, terminal, "session.turn", "run", "Capped the retries", true),
  event(40, stuck, "session.started", "run", "Started", true),
  event(25, stuck, "session.turn", "run", "Searching the tokens", true),
  event(8, working, "session.started", "run", "Started", true),
  event(1, working, "session.turn", "run", "Reading migration 0043", true),
  event(18, asking, "session.started", "run", "Started", true),
  event(9, asking, "session.asked", "ask", "Asked: manifest hash or path?", false),
];

function build(): Scenario {
  const base = holding("session-standing", "Six Sessions each in the state Fleet read them to be in: replied twice, done, stuck, working and asking");
  return {
    ...base,
    state: { ...base.state, repository: null },
    later: [{}],
    draft: {
      events: EVENTS,
      sessions: () => {
        const store = sessionsStore([terminal, replied, done, stuck, working, asking]);
        return {
          ...store,
          // He answers Retry cap in its terminal: its turn begins again, and Fleet reads it working and drops the reply.
          later: () => store.edit("t1", (one) => ({ ...one, turn: { state: "working" }, lastTurnAt: new Date().toISOString(), standing: "working", waitingFor: [] })),
        };
      },
    },
  };
}

export const s242SessionStanding: Scenario = build();
