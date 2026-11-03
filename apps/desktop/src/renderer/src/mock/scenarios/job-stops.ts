// What Bridge says about a Job that is working, waiting or stopped, and what Fleet is doing for it. Eight Jobs: three stopped for three reasons (tasks open, its
// process ended, Fleet was down), one at work on a call, one stuck on a call it has not come back from, one held open by its pull request, one paused, and
// one idle whose process Fleet let go. Fleet's own acts stand in Now while they run, in Open while they are to come, and in Done once they are over.
// Draft fields only: nothing in Fleet carries them yet. The walk `jobStops` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { FleetAct, Session, SessionAttachment } from "@armada/screens/src/draft/sessions";
import type { WaitingItem } from "@armada/protocol";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const ahead = (seconds: number) => new Date(Date.now() + seconds * 1000).toISOString();
/** Today at a local clock time: a stretch that has a stated time, such as when Fleet was down. */
const today = (hour: number, minute: number) => {
  const at = new Date();
  at.setHours(hour, minute, 0, 0);
  return at.toISOString();
};
const clock = (at: string) =>
  new Date(at).toLocaleTimeString("en-GB", {
    hour: "2-digit",
    minute: "2-digit",
  });

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

const pr = (number: number, title: string, branch: string): SessionAttachment => ({
  kind: "pull_request",
  number,
  title,
  branch,
  address: `https://git.example/armada/pull/${number}`,
  checks: {
    state: "passed",
  },
  state: "open",
  auto: false,
});

const act = (
  id: string,
  at: string,
  state: FleetAct["state"],
  kind: FleetAct["kind"],
  words: string,
  rest: Pick<FleetAct, "why" | "until" | "step"> = {},
): FleetAct => ({
  id,
  at,
  state,
  kind,
  said: words,
  ...rest,
});

const BASE = {
  manifestId: OWNER,
  cwd: ROOT,
  work: {
    proposal: {
      workflow: null,
    },
  },
} as const;

const NUDGE_AT = ahead(40);

// 1. It ended its turn with two tasks open. Fleet will nudge it; the nudge is in Open until it is sent.
const retry: Session = {
  ...BASE,
  id: "js-retry",
  title: "Fix the merge queue retry",
  turn: {
    state: "idle",
  },
  standing: "idle",
  lastTurnAt: ago(5),
  rows: [
    asked(
      "js-1-u",
      "13:40:00",
      "The merge queue retries a failed merge forever. Cap it and test the cap.",
    ),
    said("js-1-a", "14:31:40", "The cap is in. The regression test and the push are left."),
  ],
  attachments: [
    {
      kind: "branch",
      name: "queue/retry-cap",
      slot: 7,
    },
  ],
  stopped: {
    reason: "tasks_open",
    at: ago(5),
    tasks: [
      "Write the regression test",
      "Push the branch",
    ],
    restart: "none",
  },
  fleetActs: [
    act("js-1-f1", ago(52), "over", "slot", "Moved into slot 7", {
      why: "Slot 7 was free",
    }),
    act("js-1-f2", ago(38), "over", "told", "Told it: slot-13 is 47 commits behind main", {
      why: "Fleet tells a Job when its slot falls behind main",
    }),
    act("js-1-f3", ago(31), "over", "held", "Held back a word you muted: Pull request merged", {
      why: "You muted this notice",
    }),
    act("js-1-f4", ago(17), "over", "restart", "Restarted the agent: process ended", {
      why: "The agent's process ended mid-turn",
    }),
    act("js-1-f5", ago(9), "over", "step", "Moved to Tests", {
      why: "The step's work was done",
    }),
    act("js-1-f6", NUDGE_AT, "coming", "nudge", `Nudge at ${clock(NUDGE_AT)}`, {
      until: NUDGE_AT,
      why: "It ended its turn with tasks open",
    }),
  ],
};

// 2. Its process ended and Fleet could not bring it back. He is asked to carry on.
const PHONE_ASK = "stopped:js-phone";
const phone: Session = {
  ...BASE,
  id: "js-phone",
  title: "Tell the phone what holds a Job",
  turn: {
    state: "idle",
  },
  standing: "idle",
  lastTurnAt: ago(21),
  rows: [
    asked(
      "js-2-u",
      "13:55:00",
      "Have the phone say what is holding a Job: a call, a walk, or the merge.",
    ),
    said("js-2-a", "14:20:12", "Wiring the hold types into the phone's list."),
  ],
  attachments: [
    {
      kind: "branch",
      name: "phone/holds",
      slot: 4,
    },
  ],
  stopped: {
    reason: "process_gone",
    at: ago(20),
    said: "MCP tool mcp__armada-fleet__ask_the_person (passed via --permission-prompt-tool) not found.",
    restart: "failed",
  },
  waitingFor: [
    {
      id: PHONE_ASK,
      text: "Fleet could not restart it",
      since: ago(20),
      source: "stopped" as WaitingItem["source"],
      act: {
        kind: "answer",
        target: PHONE_ASK,
      },
      options: [
        {
          label: "Carry on",
        },
      ],
    },
  ],
  fleetActs: [
    act("js-2-f1", ago(26), "over", "told", "Told it: slot-4 is 12 commits behind main", {
      why: "Fleet tells a Job when its slot falls behind main",
    }),
  ],
};

// 3. Fleet was down when its turn ended. Its restart waits behind one other.
const DOWN_FROM = today(16, 10);
const DOWN_TO = today(16, 57);
const everything: Session = {
  ...BASE,
  id: "js-everything",
  title: "Phone review of everything",
  turn: {
    state: "idle",
  },
  standing: "idle",
  lastTurnAt: today(16, 8),
  rows: [
    asked("js-3-u", "13:10:00", "Let the phone review every pull request, not only mine."),
    said("js-3-a", "16:07:55", "The review list reads from the same queue now."),
  ],
  attachments: [
    {
      kind: "branch",
      name: "phone/review-all",
      slot: 9,
    },
  ],
  work: {
    proposal: {
      workflow: "Feature",
    },
    landing: "pull_request",
    workflow: {
      name: "Feature",
      steps: [
        "Plan",
        "Build",
        "Tests",
        "Pull request",
      ],
      at: 2,
      visits: [
        {
          step: 0,
          since: today(13, 10),
        },
        {
          step: 1,
          since: today(14, 0),
        },
        {
          step: 2,
          since: today(15, 30),
        },
      ],
    },
  },
  stopped: {
    reason: "fleet_down",
    at: DOWN_TO,
    restart: "queued",
    down: {
      from: DOWN_FROM,
      to: DOWN_TO,
    },
  },
  fleetActs: [
    act("js-3-f1", today(13, 14), "over", "slot", "Moved into slot 9", {
      step: "Plan",
      why: "Slot 9 was free",
    }),
    act("js-3-f2", today(14, 20), "over", "process", "Took the process back after a restart", {
      step: "Build",
      why: "The process had been let go while it was idle",
    }),
    act("js-3-f3", today(15, 5), "over", "held", "Held back a word you muted: Pull request merged", {
      step: "Build",
      why: "You muted this notice",
    }),
    act("js-3-f4", today(15, 24), "over", "process", "Let the process go after 55 minutes idle", {
      step: "Build",
      why: "Nothing had been asked of it for 55 minutes",
    }),
    act("js-3-f5", DOWN_TO, "coming", "restart", "Restart queued, 1 ahead", {
      why: "Restarts go one at a time",
    }),
  ],
};

// 4. At work on a call, and Fleet judging the step beside it.
const rate: Session = {
  ...BASE,
  id: "js-rate",
  title: "Rate limit the API",
  turn: {
    state: "working",
  },
  standing: "working",
  turnSince: ago(31),
  lastHeardAt: ago(0.2),
  openCall: {
    text: "Bash armada check test rate_limit",
    since: ago(12),
  },
  lastTurnAt: ago(31),
  rows: [
    asked("js-4-u", "14:05:00", "Limit requests per key on the public API."),
  ],
  attachments: [
    {
      kind: "branch",
      name: "api/rate-limit",
      slot: 2,
    },
  ],
  fleetActs: [
    act("js-4-f1", ago(2), "under_way", "judge", "Judging the step", {
      why: "The Build step's checks passed",
    }),
    act("js-4-f2", ahead(300), "coming", "held", "Word held until its turn ends", {
      why: "A notice for it arrived mid-turn",
    }),
  ],
};

// 5. A turn that has been in one call and silent for 14 minutes.
const importer: Session = {
  ...BASE,
  id: "js-importer",
  title: "Rewrite the importer",
  turn: {
    state: "working",
  },
  standing: "stuck",
  turnSince: ago(40),
  lastHeardAt: ago(14),
  openCall: {
    text: "Bash pnpm build",
    since: ago(14),
  },
  lastTurnAt: ago(40),
  rows: [
    asked("js-5-u", "13:30:00", "Rewrite the importer to stream rows."),
  ],
  attachments: [
    {
      kind: "branch",
      name: "import/stream",
      slot: 5,
    },
  ],
};

// 6. Its turn ended and its pull request is in the merge queue. Reads as it does today: held, not stopped.
const ship: Session = {
  ...BASE,
  id: "js-ship",
  title: "Ship the rate limit to QA",
  turn: {
    state: "idle",
  },
  standing: "idle",
  lastTurnAt: ago(18),
  rows: [
    asked("js-6-u", "13:02:00", "Ship the rate limit and see it through the QA deploy."),
    said("js-6-a", "14:20:30", "#412 is in the merge queue."),
  ],
  attachments: [
    {
      kind: "branch",
      name: "api/rate-limit-qa",
      slot: 3,
    },
    pr(412, "Limit requests per key on the public API", "api/rate-limit-qa"),
  ],
  heldOpenBy: [
    {
      kind: "pull_request",
      number: 412,
      on: {
        kind: "queue",
      },
    },
  ],
};

// 7. Parked by him with tasks open. Reads Paused.
const settings: Session = {
  ...BASE,
  id: "js-settings",
  title: "Sort the settings page",
  turn: {
    state: "idle",
  },
  standing: "idle",
  paused: true,
  lastTurnAt: ago(70),
  rows: [
    asked("js-7-u", "12:40:00", "Sort the settings page by how often each is changed."),
    said("js-7-a", "13:05:00", "The groups are sorted. The page's tests are left."),
  ],
  attachments: [
    {
      kind: "branch",
      name: "settings/sort",
      slot: 6,
    },
  ],
  activity: {
    todos: [
      {
        text: "Sort the groups",
        state: "done",
      },
      {
        text: "Update the page's tests",
        state: "todo",
      },
      {
        text: "Move the rarely used settings down",
        state: "todo",
      },
    ],
  },
};

// 8. Idle with nothing open. Fleet let its process go after a quiet spell.
const store: Session = {
  ...BASE,
  id: "js-clock",
  title: "Answer about the store clock",
  turn: {
    state: "idle",
  },
  standing: "idle",
  lastTurnAt: ago(70),
  rows: [
    asked("js-8-u", "12:50:00", "Where does the store read the clock from?"),
    said("js-8-a", "13:02:15", "From `Clock::now` in the store crate, once per write."),
  ],
  attachments: [],
  fleetActs: [
    act("js-8-f1", ago(15), "over", "process", "Let the process go after 55 minutes idle", {
      why: "Nothing had been asked of it for 55 minutes",
    }),
  ],
};

const base = holding(
  "jobStops",
  "Eight Jobs: three stopped for three reasons, one at work on a call, one stuck on one, one held by its pull request, one paused, one idle; with what Fleet did for each",
);

export const jobStops: Scenario = {
  ...base,
  state: {
    ...base.state,
    repository: null,
  },
  draft: {
    sessions: () =>
      sessionsStore(
        [
          retry,
          phone,
          everything,
          rate,
          importer,
          ship,
          settings,
          store,
        ],
        [],
        new Map(),
        [],
      ),
  },
};
