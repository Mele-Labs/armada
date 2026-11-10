// The Operator's page over the whole work, which the Operator scenarios after it build on. Ten live Jobs: one whose question the Operator
// raised with its read and pick, one whose question it is still reading, two kept in touch with a Job they wait on, one it nudged after
// Fleet marked it Stopped, one another Job was merged into, two kept in touch from a recorded plan, and two at work. One ended Job, the
// one merged in. The Jobs held to start and the rows' ties are the scenario's `operator` draft, which no wire carries yet
// (`operator/model.ts`).

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import type { OperatorSeed } from "../../operator/model";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const ago = (seconds: number) => new Date(Date.now() - seconds * 1000).toISOString();

export const BASE = {
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

export const question = (
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
    {
      id: "op-1-o",
      at: "09:10:00",
      kind: "operator",
      text: "The Operator started this Job. No live Job plans it.",
    },
    asked("op-1-u", "09:10:00", "A Job whose agent process died stays listed as idle. Mark it."),
    {
      id: "op-1-s",
      at: "09:32:00",
      kind: "operator",
      text: "The Operator sent: Job says Idle, but its agent crashed. A bug report from Home Draws Desktop States. Your detect group covers it.",
    },
    {
      id: "op-1-b",
      at: "09:38:00",
      kind: "operator",
      text: 'The Operator answered "Keep the existing column": no one sees it, and the migration is smaller.',
    },
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

// The Job merged in: dispatched, ruled the same work, and ended before it pushed.
const stepTimes: Session = {
  ...BASE,
  id: "op-step-times",
  title: "Time each build step",
  dead: "ended",
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(90),
  rows: [
    asked("op-11-u", "09:49:00", "Time each step of a build in the Record."),
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
  // Delegate stands at Suggest: what it would send to this Job waits on one press.
  waitingFor: [
    {
      id: "suggestion:7",
      text: "Send this Job: Doctor needs the size in bytes, not a label. Its checks group calls it",
      since: ago(40),
      source: "operator_suggestion",
      act: {
        kind: "answer",
        target: "",
      },
      options: [
        {
          label: "Do it",
        },
        {
          label: "Skip",
        },
      ],
    },
  ],
};

export const operatorJobs: readonly Session[] = [
  deaths,
  filters,
  tests,
  slotPool,
  legend,
  console_,
  home,
  buildTimes,
  stepTimes,
  doctorStore,
  storeSize,
];

const job = (session: Session) => ({
  session: session.id,
  title: session.title!,
});

const seed: OperatorSeed = {
  on: true,
  line: [
    {
      id: "line-tests",
      title: "Tests take minutes, but build the whole workspace",
      waits: {
        kind: "job",
        job: job(deaths),
        event: "pushes",
        group: "tests",
      },
      why: "Kept in touch on crates/testkit/src/run.rs",
      session: "op-tests",
    },
    {
      id: "line-pool",
      title: "Map shows Jobs, but not the slots they hold",
      waits: {
        kind: "job",
        job: job(legend),
        event: "pushes",
        group: "map",
      },
      why: "Kept in touch on apps/desktop/src/renderer/src/map/slot.tsx",
      session: "op-pool",
    },
    {
      id: "line-key",
      title: "Map key lists colours the Map no longer uses",
      waits: {
        kind: "meter",
        meter: "five_hour",
        under: 80,
      },
    },
    {
      id: "line-sounds",
      title: "Raised items make no sound",
      waits: {
        kind: "you",
      },
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
  ],
};

function scenario(name: string, says: string): Scenario {
  const base = holding(name, says);
  return {
    ...base,
    state: {
      ...base.state,
      repository: null,
    },
    draft: {
      operator: seed,
      sessions: () => sessionsStore(operatorJobs, [], new Map(), []),
    },
  };
}

export const operatorFull = scenario(
  "operator-full",
  "The Operator page over the whole work: its conversation alone, over ten live Jobs and one merged in",
);
