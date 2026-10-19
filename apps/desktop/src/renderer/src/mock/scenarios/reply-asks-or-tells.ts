// A reply that asks and replies that only tell. Four Sessions: one whose agent ended its turn on a question, which is
// a call with the asking line; two whose agents ended on a report, which ask nothing and read as unread; and one idle.
// The list is in the order the Sessions are given and nothing sorts it by status: the asking one yellow and pulsing,
// the two that told with a faint mark, the idle one last. The walk `reply-asks-or-tells` plays it.

import type { TimelineEvent } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import { ASKED_AT, ASKED_ITEM } from "../needs-me-ids";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const said = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "agent" }, text }) as const;
const asked = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "you" }, text }) as const;

const ASKING_LINE = "Should the retry cap be five or ten?";

const asking: Session = {
  id: "q1",
  title: "Retry cap",
  turn: { state: "idle" },
  lastTurnAt: ASKED_AT,
  standing: "replied",
  rows: [
    asked("q1-u", "14:00:00", "Cap the retries in the lease client."),
    said("q1-a", "14:11:40", `The retry loop is capped and the backoff is in.\n${ASKING_LINE}`),
  ],
  attachments: [],
  waitingFor: [{ id: ASKED_ITEM, text: ASKING_LINE, since: ASKED_AT, source: "replied" }],
};

const told: Session = {
  id: "t1",
  title: "Pin the store clock",
  turn: { state: "idle" },
  lastTurnAt: ago(10),
  standing: "replied",
  rows: [
    asked("t1-u", "13:55:00", "Pin the clock in the store tests."),
    said("t1-a", "14:08:09", "The clock is pinned in all four store tests and the fixture is gone.\nTwo files changed, and the suite passes."),
  ],
  attachments: [],
};

const toldToo: Session = {
  id: "t2",
  title: "Trim the log lines",
  turn: { state: "idle" },
  lastTurnAt: ago(15),
  standing: "replied",
  rows: [
    asked("t2-u", "13:50:00", "Trim the debug lines from the lease log."),
    said("t2-a", "14:03:30", "Eleven debug lines are gone from the lease log. The info lines are unchanged."),
  ],
  attachments: [],
};

const idle: Session = {
  id: "i1",
  title: "Migration notes",
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  standing: "idle",
  rows: [asked("i1-u", "14:00:00", "Summarise the migrations since 0042."), said("i1-a", "14:14:12", "Four migrations since 0042, summarised in the notes file.")],
  attachments: [],
};

let count = 0;
const event = (minutes: number, session: Session, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string): TimelineEvent => {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: { kind: "session", id: session.id }, title: session.title ?? session.id, tone, says, manifest_id: OWNER, live: true };
};

const EVENTS: TimelineEvent[] = [
  event(30, asking, "session.started", "run", "Started"),
  event(7, asking, "session.turn", "run", "Capped the retry loop"),
  event(28, told, "session.started", "run", "Started"),
  event(10, told, "session.turn", "run", "Pinned the clock in the store tests"),
  event(33, toldToo, "session.started", "run", "Started"),
  event(15, toldToo, "session.turn", "run", "Trimmed the lease log"),
  event(26, idle, "session.started", "run", "Started"),
  event(2, idle, "session.turn", "run", "Summarised the migrations"),
];

function build(): Scenario {
  const base = holding("reply-asks-or-tells", "A reply that asks, two that only tell, and an idle Session, in a fixed order");
  return {
    ...base,
    state: { ...base.state, repository: null },
    later: [{}],
    draft: {
      events: EVENTS,
      sessions: () => sessionsStore([asking, told, toldToo, idle]),
    },
  };
}

export const replyAsksOrTells: Scenario = build();
