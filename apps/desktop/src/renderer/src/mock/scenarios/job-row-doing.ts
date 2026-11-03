// Five Jobs, one for each thing a row now says a Job is doing: a turn running, a background command
// running, a subagent at work, stopped twice over and calling for him with no item, and one that only replied.
// The walk `jobRowDoing` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const said = (id: string, at: string, text: string) =>
  ({
    id,
    at,
    kind: "message",
    from: {
      kind: "agent",
    },
    text,
  }) as const;
const asked = (id: string, at: string, text: string) =>
  ({
    id,
    at,
    kind: "message",
    from: {
      kind: "you",
    },
    text,
  }) as const;
const fleet = (id: string, at: string, text: string) =>
  ({
    id,
    at,
    kind: "message",
    from: {
      kind: "session",
      id: "fleet",
      title: "Fleet",
    },
    notice: "stop_nudge",
    text,
  }) as const;

const working: Session = {
  id: "w1",
  title: "Migration notes",
  turn: {
    state: "working",
  },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [
    asked("w1-u", "14:10:00", "Summarise the migrations since 0042."),
    {
      id: "w1-t",
      at: "14:17:02",
      kind: "tool",
      text: "Read crates/store/migrations/0043.sql",
    },
  ],
  attachments: [],
};

const running: Session = {
  id: "c1",
  title: "Retry cap",
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(3),
  standing: "working",
  rows: [
    asked("c1-u", "14:02:00", "Cap the retries at five."),
    said("c1-a", "14:14:12", "The cap is in. The store tests are running."),
  ],
  attachments: [
    {
      kind: "background",
      id: "bg-1",
      form: "command",
      task: "Run the store tests",
      command: "armada check test store_retry",
      started: ago(3),
      state: "running",
    },
  ],
};

const subagent: Session = {
  id: "s1",
  title: "Changelog script",
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(2),
  standing: "working",
  rows: [
    asked("s1-u", "14:01:00", "Write the changelog script."),
    said("s1-a", "14:15:30", "A subagent is writing the script."),
  ],
  attachments: [
    {
      kind: "subagent",
      id: "sub-1",
      task: "Write the changelog script",
      state: "running",
      agentType: "armada:implementer",
    },
  ],
};

const STOPPED_AT = ago(5);
const stopped: Session = {
  id: "k1",
  title: "Theme token audit",
  turn: {
    state: "idle",
  },
  lastTurnAt: STOPPED_AT,
  standing: "stopped",
  rows: [
    asked("k1-u", "13:40:00", "List the tokens no stylesheet reads."),
    said("k1-a", "13:58:40", "The list is written. The Tests step comes next."),
    fleet(
      "k1-f",
      "13:59:40",
      "Your turn ended with nothing running and nothing asked. Carry on now. If you wait on the person, name it with waiting_for. If nothing is left, call finish_up and then all_done. A second stop like this one calls the person and marks this Job Stopped.",
    ),
    said("k1-b", "14:00:10", "The Tests step comes next."),
  ],
  attachments: [],
};

const replied: Session = {
  id: "r1",
  title: "Pin the store clock",
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(6),
  standing: "replied",
  rows: [
    asked("r1-u", "14:02:00", "Pin the clock in the store tests."),
    said("r1-a", "14:12:09", "The clock is pinned in all four store tests."),
  ],
  attachments: [],
};

function build(): Scenario {
  const base = holding(
    "job-row-doing",
    "Five Jobs, one working, one running a command, one on a subagent, one stopped and one that replied",
  );
  return {
    ...base,
    state: {
      ...base.state,
      repository: null,
    },
    draft: {
      sessions: () =>
        sessionsStore([
          stopped,
          working,
          running,
          subagent,
          replied,
        ]),
    },
  };
}

export const jobRowDoing: Scenario = build();
