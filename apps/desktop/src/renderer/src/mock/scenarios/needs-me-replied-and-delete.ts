// Needs you holds what is asked of him and nothing else. Three Sessions: one replied to him, which asks
// nothing, so it is filed under Idle with an unread mark and raises no call; one is done, which stays under
// Needs you with its call; and one has two items its agent stated, each of which he can remove with a
// reason, the agent then told in its thread. The walk `needs-me-replied-and-delete` plays it.

import type { TimelineEvent } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import { DONE_AT, DONE_ITEM, REPLIED_AT } from "../needs-me-ids";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const said = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "agent" }, text }) as const;
const asked = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "you" }, text }) as const;

const replied: Session = {
  id: "r1",
  title: "Pin the store clock",
  turn: { state: "idle" },
  lastTurn: "14:12",
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
  lastTurn: "14:14",
  lastTurnAt: DONE_AT,
  standing: "done",
  done: { since: DONE_AT },
  rows: [
    asked("d1-u", "14:01:00", "Write the changelog script and merge it."),
    said("d1-a", "14:14:30", "The script is merged. There is nothing left to do."),
  ],
  attachments: [],
  waitingFor: [{ id: DONE_ITEM, text: "Done", since: DONE_AT, source: "done" }],
};

const asking: Session = {
  id: "o1",
  title: "Cache key",
  turn: { state: "idle" },
  lastTurn: "14:09",
  lastTurnAt: ago(9),
  standing: "idle",
  rows: [
    asked("o1-u", "14:00:00", "Cache the manifest read."),
    said("o1-a", "14:09:20", "The cache is in. Two things are left for you."),
  ],
  attachments: [],
  waitingFor: [
    { id: "o1-a1", text: "Confirm the old fixture can go", since: ago(14), source: "agent" },
    { id: "o1-a2", text: "Pick a name for the cache table", since: ago(11), source: "agent" },
  ],
};

const idle: Session = {
  id: "i1",
  title: "Migration notes",
  turn: { state: "idle" },
  lastTurn: "13:58",
  lastTurnAt: ago(20),
  standing: "idle",
  rows: [asked("i1-u", "13:40:00", "Summarise the migrations since 0042."), said("i1-a", "13:58:12", "Four migrations since 0042, summarised in the notes file.")],
  attachments: [],
};

let count = 0;
const event = (minutes: number, session: Session, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string): TimelineEvent => {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: { kind: "session", id: session.id }, title: session.title ?? session.id, tone, says, manifest_id: OWNER, live: true };
};

const EVENTS: TimelineEvent[] = [
  event(24, replied, "session.started", "run", "Started"),
  event(6, replied, "session.turn", "run", "Pinned the clock in the store tests"),
  event(22, done, "session.started", "run", "Started"),
  event(4, done, "session.turn", "run", "Merged the script"),
  event(18, asking, "session.started", "run", "Started"),
  event(9, asking, "session.turn", "run", "Cached the manifest read"),
  event(40, idle, "session.started", "run", "Started"),
  event(20, idle, "session.turn", "run", "Summarised the migrations"),
];

function build(): Scenario {
  const base = holding("needs-me-replied-and-delete", "A reply under Idle with an unread mark, a done under Needs you, and a Session with two items its agent stated");
  return {
    ...base,
    state: { ...base.state, repository: null },
    later: [{}],
    draft: {
      events: EVENTS,
      sessions: () => sessionsStore([replied, done, asking, idle]),
    },
  };
}

export const needsMeRepliedAndDelete: Scenario = build();
