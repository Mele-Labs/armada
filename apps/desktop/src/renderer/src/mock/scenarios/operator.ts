// Helm's dock as the Operator panel, for the walks `operatorWide` and `operatorFull`: the same window in the panel's two takes.
// Ten Jobs: one whose question the Operator raised with its read and pick, one whose question it is still reading, two it kept in
// touch with a Job they wait on, one it nudged after Fleet marked it Stopped, one another Job was merged into, two kept in touch
// from a recorded plan, and two at work. The Log, the Line, Permissions, the standing orders and the rows' ties are the scenario's
// `operator` draft, which no wire carries yet (`operator/model.ts`).

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import type { OperatorSeed, OperatorTake } from "../../operator/model";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const ago = (seconds: number) => new Date(Date.now() - seconds * 1000).toISOString();

const BASE = {
  manifestId: OWNER,
  cwd: ROOT,
  work: {
    proposal: {
      workflow: null,
    },
  },
  attachments: [],
} as const;

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

const question = (
  call: string,
  text: string,
  header: string,
  options: [
    string,
    string,
  ][],
) => ({
  command: text,
  call,
  offers: [
    "allow_once" as const,
    "refuse" as const,
  ],
  questions: [
    {
      question: text,
      header,
      multi_select: false,
      options: options.map(([label, description]) => ({
        label,
        description,
      })),
    },
  ],
});

// Raised: its group is seen, so the call could not answer it. Its read and pick are on the question.
const deaths: Session = {
  ...BASE,
  id: "op-deaths",
  title: "Stop Silent Job Deaths",
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(180),
  rows: [
    asked("op-1-u", "09:10:00", "A Job whose agent process died stays listed as idle. Mark it."),
    said("op-1-a", "09:41:12", "The detector is in. One choice is left before the row draws it."),
  ],
  asked: question("op-q-1", "Which icon marks a dead process?", "Icon", [
    [
      "circle-x",
      "Reads apart from Stopped's circle-pause",
    ],
    [
      "triangle-alert",
      "The warning glyph the Doctor uses",
    ],
  ]),
};

// Being read: the Operator glyph is on it, and it does not pulse until it is ruled.
const filters: Session = {
  ...BASE,
  id: "op-filters",
  title: "Tidy the Logbook filters",
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(8),
  rows: [
    asked("op-2-u", "09:30:00", "The Logbook's filters take two rows. Get them onto one."),
    said("op-2-a", "09:52:40", "They fit on one row either way."),
  ],
  asked: question("op-q-2", "Group the filters under one menu?", "Filters", [
    [
      "One menu",
      "One control, the kinds as its choices",
    ],
    [
      "Keep them apart",
      "Each kind a chip of its own, as now",
    ],
  ]),
};

// Dispatched and held: its header says so, naming the Job it waits for.
const tests: Session = {
  ...BASE,
  id: "op-tests",
  title: "Tests Build Only What They Test",
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(480),
  rows: [
    asked("op-3-u", "09:45:00", "Build only the crates a test reaches, not the workspace."),
  ],
};

const slotPool: Session = {
  ...BASE,
  id: "op-pool",
  title: "Show the slot pool on the Map",
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(300),
  rows: [
    asked("op-4-u", "09:48:00", "Draw each slot on the Map, held or free."),
  ],
};

const legend: Session = {
  ...BASE,
  id: "op-legend",
  title: "Rework the Map legend",
  turn: {
    state: "working",
  },
  lastTurnAt: ago(20),
  rows: [
    asked("op-5-u", "09:20:00", "The legend is illegible at narrow widths. Rework it."),
  ],
};

const console_: Session = {
  ...BASE,
  id: "op-console",
  title: "Cockpit Console",
  turn: {
    state: "working",
  },
  lastTurnAt: ago(40),
  rows: [
    asked("op-6-u", "08:55:00", "Move the bays' acts into the console."),
    said("op-6-a", "09:50:00", "Carrying on: the bay acts, then the tests."),
  ],
};

const home: Session = {
  ...BASE,
  id: "op-home",
  title: "Home Draws Desktop States",
  turn: {
    state: "working",
  },
  lastTurnAt: ago(60),
  rows: [
    asked("op-7-u", "09:00:00", "Home draws a desktop's states as the Cockpit does."),
  ],
};

// Merged: Time each build step had pushed nothing, so it ended and its brief, plan and branch came here. Its row is gone.
const buildTimes: Session = {
  ...BASE,
  id: "op-build-times",
  title: "Show build times in the Record",
  turn: {
    state: "working",
  },
  lastTurnAt: ago(30),
  rows: [
    asked("op-8-u", "09:05:00", "The Record shows a build but not how long each step took."),
    said("op-8-a", "09:51:00", "Took in Time each build step's plan. Recording the plan again."),
  ],
};

// Kept in touch from a recorded plan: the Doctor calls what the store's size group builds, so its checks group waits on that push.
const doctorStore: Session = {
  ...BASE,
  id: "op-doctor-store",
  title: "Doctor Checks the Store",
  turn: {
    state: "working",
  },
  lastTurnAt: ago(50),
  rows: [
    asked("op-9-u", "09:15:00", "The Doctor warns when the store passes its size limit."),
  ],
};

const storeSize: Session = {
  ...BASE,
  id: "op-store-size",
  title: "Store Reports Its Size",
  turn: {
    state: "working",
  },
  lastTurnAt: ago(15),
  rows: [
    asked("op-10-u", "09:25:00", "The store reports its size on disk."),
  ],
};

const seed = (take: OperatorTake): OperatorSeed => ({
  take,
  on: true,
  positions: {
    dispatch: "on",
    delegate: "on",
    answer: "on",
    file: "on",
  },
  ceiling: 300,
  log: [
    {
      id: "log-bug",
      at: ago(12),
      from: "Bug report",
      item: "A Job whose agent process died stays listed as idle",
      did: "existing",
      why: "Stop Silent Job Deaths: its detect group covers process death. The words went to it",
      ways: [
        "A new Job",
        "File it as an issue",
        "Raise it to me",
      ],
    },
    {
      id: "log-late",
      at: ago(20),
      from: "Rework the Map legend",
      item: "Keep the legend collapsed by default?",
      did: "passed_on",
      why: "No ruling in 60 seconds. It pulses in Needs you as today",
    },
    {
      id: "log-plan",
      at: ago(30),
      from: "Store Reports Its Size",
      item: "Plan recorded: the size group",
      did: "in_touch",
      why: "Doctor Checks the Store calls what it builds, in crates/store/src/size.rs. Its checks group waits on this push",
      ways: [
        "Merge them",
        "Raise it to me",
      ],
    },
    {
      id: "log-nudge",
      at: ago(40),
      from: "Cockpit Console",
      item: "Fleet marked it Stopped with 3 tasks open",
      did: "delegated",
      why: "Told it to carry on. Its plan is approved and nothing waits on you",
      ways: [
        "Raise it to me",
      ],
    },
    {
      id: "log-answer",
      at: ago(60),
      from: "Home Draws Desktop States",
      item: "Keep the old test name?",
      did: "answered",
      why: "Keep it, the recommended one. Its group is not seen",
      ways: [
        "Rename it",
        "Raise it to me",
      ],
    },
    {
      id: "log-merge",
      at: ago(90),
      from: "Show build times in the Record",
      item: "Time each build step",
      did: "merged",
      why: "Same work in apps/desktop/src/renderer/src/record. Time each build step had pushed nothing, so it ended and its brief, plan and branch came here",
      ways: [
        "Keep them in touch",
        "Raise it to me",
      ],
    },
    {
      id: "log-meter",
      at: ago(120),
      from: "You dispatched",
      item: "Tighten the review list poll",
      did: "passed_on",
      why: "Weekly meter is out. Started as today",
    },
    {
      id: "log-raise",
      at: ago(180),
      from: "Stop Silent Job Deaths",
      item: "Which icon marks a dead process?",
      did: "raised",
      why: "Its group is seen",
    },
    {
      id: "log-filed",
      at: ago(240),
      from: "Bug report",
      item: "The Logbook search drops the first letter typed",
      did: "filed",
      why: "No open issue matches it. Filed in armada",
      ways: [
        "Raise it to me",
      ],
    },
    {
      id: "log-pool",
      at: ago(300),
      from: "You dispatched",
      item: "Show the slot pool on the Map",
      did: "in_touch",
      why: "Rework the Map legend shares apps/desktop/src/renderer/src/map/slot.tsx. Its map group waits on that push",
      line: "line-pool",
      ways: [
        "A new Job",
        "Send it to Rework the Map legend",
        "Merge it into Rework the Map legend",
      ],
    },
    {
      id: "log-released",
      at: ago(360),
      from: "Line",
      item: "Wire the Doctor's new checks",
      did: "released",
      why: "Store Migration Spike ended. Started now",
    },
    {
      id: "log-tests",
      at: ago(480),
      from: "You dispatched",
      item: "Tests Build Only What They Test",
      did: "in_touch",
      why: "Stop Silent Job Deaths shares crates/testkit/src/run.rs. Its tests group waits on that push",
      line: "line-tests",
      ways: [
        "A new Job",
        "Send it to Stop Silent Job Deaths",
        "Merge it into Stop Silent Job Deaths",
      ],
    },
    {
      id: "log-returned",
      at: ago(540),
      from: "Home Draws Desktop States",
      item: "A Job whose agent process died stays listed as idle",
      did: "returned",
      why: "The Job ended its turn without touching it. Ruled again above",
    },
    {
      id: "log-resumed",
      at: ago(720),
      from: "Fleet restarted",
      item: "Rename the slot pool setting?",
      did: "resumed",
      why: "Kept across the restart and ruled: answered, its group is not seen",
    },
  ],
  line: [
    {
      id: "line-tests",
      title: "Tests Build Only What They Test",
      waits: {
        kind: "job",
        job: "Stop Silent Job Deaths",
        event: "pushes",
        group: "tests",
      },
      why: "Kept in touch on crates/testkit/src/run.rs",
      session: "op-tests",
    },
    {
      id: "line-pool",
      title: "Show the slot pool on the Map",
      waits: {
        kind: "job",
        job: "Rework the Map legend",
        event: "pushes",
        group: "map",
      },
      why: "Kept in touch on apps/desktop/src/renderer/src/map/slot.tsx",
      session: "op-pool",
    },
    {
      id: "line-key",
      title: "Redraw the Map key",
      waits: {
        kind: "meter",
        meter: "five_hour",
        under: 80,
      },
    },
    {
      id: "line-sounds",
      title: "Idea: sounds for raised items",
      waits: {
        kind: "you",
      },
    },
  ],
  orders: [
    {
      id: "order-billing",
      kind: "role_off",
      role: "answer",
      repository: "billing",
    },
    {
      id: "order-components",
      kind: "hold_paths",
      paths: "packages/components",
      until: {
        job: "Cockpit Console",
        event: "pushes",
      },
    },
    {
      id: "order-walks",
      kind: "always_raise",
      what: "walks",
    },
  ],
  reads: [
    {
      session: "op-filters",
      state: "reading",
    },
    {
      session: "op-deaths",
      state: "raised",
      read: "Its group is seen. The Stopped state already uses circle-pause, so circle-x reads apart from it.",
      pick: "circle-x",
    },
  ],
  ties: [
    {
      session: "op-build-times",
      kind: "merged",
      other: "Time each build step",
      paths: "apps/desktop/src/renderer/src/record",
    },
    {
      session: "op-doctor-store",
      kind: "in_touch",
      other: "Store Reports Its Size",
      paths: "crates/store/src/size.rs",
      waits: "checks",
    },
    {
      session: "op-store-size",
      kind: "in_touch",
      other: "Doctor Checks the Store",
      paths: "crates/store/src/size.rs",
    },
    {
      session: "op-pool",
      kind: "in_touch",
      other: "Rework the Map legend",
      paths: "apps/desktop/src/renderer/src/map/slot.tsx",
      waits: "map",
    },
    {
      session: "op-legend",
      kind: "in_touch",
      other: "Show the slot pool on the Map",
      paths: "apps/desktop/src/renderer/src/map/slot.tsx",
    },
    {
      session: "op-tests",
      kind: "in_touch",
      other: "Stop Silent Job Deaths",
      paths: "crates/testkit/src/run.rs",
      waits: "tests",
    },
    {
      session: "op-deaths",
      kind: "in_touch",
      other: "Tests Build Only What They Test",
      paths: "crates/testkit/src/run.rs",
    },
  ],
  talk: [
    {
      asked: "why did you hold the tests",
      said: "It is kept in touch with Stop Silent Job Deaths. Both plans change crates/testkit/src/run.rs, and that Job builds it first. Its tests group starts when that Job pushes.",
    },
    {
      asked: "start it anyway",
      said: "Started Tests Build Only What They Test. Logged as your correction.",
      starts: "line-tests",
    },
  ],
});

function scenario(name: string, says: string, take: OperatorTake): Scenario {
  const base = holding(name, says);
  return {
    ...base,
    state: {
      ...base.state,
      repository: null,
    },
    draft: {
      operator: seed(take),
      sessions: () =>
        sessionsStore(
          [
            deaths,
            filters,
            tests,
            slotPool,
            legend,
            console_,
            home,
            buildTimes,
            doctorStore,
            storeSize,
          ],
          [],
          new Map(),
          [],
        ),
    },
  };
}

export const operatorWide = scenario(
  "operator-wide",
  "Helm's dock as the Operator panel, as wide as the window lets it be: the Log, the Line, Permissions and the box, over ten Jobs",
  "wide",
);

export const operatorFull = scenario(
  "operator-full",
  "Helm's dock as the Operator panel over the whole work: the Line, the Log and Permissions side by side, over ten Jobs",
  "full",
);
