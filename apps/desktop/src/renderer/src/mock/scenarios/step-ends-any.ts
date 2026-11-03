// One proposal per Job. Feature Jobs: one put a plan with a sketch inside it, one a sketch alone, one a rough walk he approved, and
// one on Walk with three walks, one approved so far. Approving the sketch settles the card and the Job moves on to Walk with its tasks
// written, no plan to approve. The rough walk was the walk, so that Job is on Refine. The Job with three walks stays on Walk until he
// has approved all three. The press settles the sketch in the store, as Fleet's
// `POST /sessions/sketch/answer` settles it on the ledger. The Workflows page's feature Propose ends on any of six
// (`workflows-managed.ts`). The walk `step-ends-any` plays it.

import type {
  Session,
  WorkGroup,
  WorkPlan,
  WorkTaskState,
} from "@armada/screens/src/draft/sessions";

import { approved, shown, waitingOn, walkPage } from "../audit-job";
import type { Scenario } from "../moment";
import { planCall } from "../plan-call";
import { arrow, box, scene } from "../scenes";
import { s200Sessions } from "./sessions";

const STEPS = [
  "Propose",
  "Walk",
  "Refine",
  "Deliver",
];
const SKETCH_JOB = "pause-from-row";
const TITLE = "Pause on the row";
const SKETCH = `sketch:${TITLE}`;

const DRAWING = scene(
  [
    box("row", 0, 60, "Job row", "a Pause press at its end", {
      icon: "layers",
      step: 1,
    }),
    box("fleet", 320, 60, "Fleet", "holds the Drone's next turn", {
      icon: "server",
      step: 2,
    }),
    box("row-paused", 640, 60, "Job row", "Paused, with Resume", {
      icon: "lock",
      step: 3,
    }),
  ],
  [
    arrow("row", "fleet", "pause"),
    arrow("fleet", "row-paused", "held"),
  ],
);

const tasks = (prefix: string, state: WorkTaskState, ...texts: string[]): WorkGroup => ({
  id: `${prefix}-group`,
  agent: "bridge-engineer",
  title: "The row",
  paths: [
    "apps/desktop/src/renderer/src/cockpit/deck",
  ],
  tasks: texts.map((text, at) => ({
    id: `${prefix}${at + 1}`,
    text,
    state,
    seen: false,
  })),
});

const plan = (approved: boolean, after: string, group: WorkGroup): WorkPlan => ({
  version: 1,
  scope: group.paths,
  approved,
  after,
  groups: [
    group,
  ],
});

const you = (id: string, at: string, text: string): Session["rows"][number] => ({
  id,
  at,
  kind: "message",
  from: {
    kind: "you",
  },
  text,
});

const agent = (id: string, at: string, text: string): Session["rows"][number] => ({
  id,
  at,
  kind: "message",
  from: {
    kind: "agent",
  },
  text,
});

const job = (id: string, title: string, at: number, rest: Partial<Session>): Session => ({
  id,
  title,
  turn: {
    state: "idle",
  },
  standing: "replied",
  lastTurnAt: new Date().toISOString(),
  rows: [],
  attachments: [],
  ...rest,
  work: {
    ...rest.work,
    workflow: {
      name: "Feature",
      steps: STEPS,
      at,
    },
  },
});

const PLAN_SKETCH_TITLE = "Stop on the row";

const planWithSketch = job("stop-from-row", "Stop a Job from its row", 0, {
  rows: [
    you("s-1", "09:02:00", "Let me stop a Job from its row."),
    agent("s-2", "09:04:10", "The plan is recorded, with the sketch inside it."),
  ],
  waitingFor: [
    planCall(1, new Date().toISOString()),
  ],
  work: {
    plan: {
      ...plan(
        false,
        "s-2",
        tasks("S", "open", "Add Stop at the end of the row", "Ask Fleet to end the turn"),
      ),
      sketch: {
        id: `sketch:${PLAN_SKETCH_TITLE}`,
        title: PLAN_SKETCH_TITLE,
        scene: DRAWING,
      },
    },
  },
});

const sketchAlone = job(SKETCH_JOB, "Pause a Job from its row", 0, {
  rows: [
    you("p-1", "09:12:00", "Let me pause a Job from its row on the Board."),
    {
      id: "p-2",
      at: "09:13:20",
      kind: "tool",
      text: `show_sketch ${JSON.stringify({
        title: TITLE,
        scene: DRAWING,
      })}`,
    },
    agent("p-3", "09:13:30", "A Pause press at the end of the row. Fleet holds the next turn."),
  ],
  attachments: [
    {
      kind: "sketch",
      id: SKETCH,
      title: TITLE,
      by: "agent",
      drawing: DRAWING,
      asked: true,
      drawn: "drawn-1",
    },
  ],
  waitingFor: [
    {
      id: SKETCH,
      text: TITLE,
      since: new Date().toISOString(),
      source: "sketch",
      act: {
        kind: "message",
        target: SKETCH_JOB,
      },
    },
  ],
});

const roughWalk = job("resume-from-row", "Resume a Job from its row", 2, {
  rows: [
    you("r-1", "08:40:00", "Let me resume a paused Job from its row."),
    agent("r-2", "08:46:00", "Rough walk approved. The tasks are recorded, and the walk is done."),
  ],
  work: {
    plan: plan(
      true,
      "r-2",
      tasks("R", "working", "Wire Resume to Fleet", "Draw the row as running again"),
    ),
  },
});

const PAGES = {
  board: shown("t-2", "10:02:00", "Mute on the Board", walkPage("step-ends-any", "board"), 1),
  job: shown("t-3", "10:02:05", "Mute on the Job", walkPage("step-ends-any", "job"), 1),
  dock: shown("t-4", "10:02:10", "Mute on the dock", walkPage("step-ends-any", "dock"), 1),
};

const threeWalks = job("mute-a-job", "Mute a Job", 1, {
  rows: [
    you("t-1", "10:00:00", "Let me mute a Job from wherever I see it."),
    approved(PAGES.board, "10:10:00"),
    PAGES.job,
    PAGES.dock,
  ],
  waitingFor: [
    ...waitingOn(PAGES.job),
    ...waitingOn(PAGES.dock),
  ],
  work: {
    plan: plan(true, "t-1", tasks("T", "open", "Add Mute in the three places")),
  },
});

export const stepEndsAny: Scenario = {
  ...s200Sessions,
  name: "step-ends-any",
  says: "Feature Jobs: a plan with a sketch inside, a sketch alone, a rough walk he approved, and three walks with one approved",
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board, [
        planWithSketch,
        sketchAlone,
        roughWalk,
        threeWalks,
      ]);
      return {
        ...store,
        answerSketch: ({ session_id: id, sketch, how, words }) => {
          const at = new Date().toISOString();
          const settled =
            how === "sent_back"
              ? {
                  how,
                  words: words ?? "",
                  at,
                }
              : {
                  how,
                  at,
                };
          store.edit(id, (one) => ({
            ...one,
            waitingFor: (one.waitingFor ?? []).filter((item) => item.id !== sketch),
            attachments: one.attachments.map((a) =>
              a.kind === "sketch" && a.id === sketch
                ? {
                    ...a,
                    settled,
                  }
                : a,
            ),
            rows: [
              ...one.rows,
              agent(
                "p-4",
                "09:15:00",
                settled.how === "approved"
                  ? "Tasks recorded. Building the mock."
                  : "I will redraw it with your note.",
              ),
            ],
            ...(settled.how === "approved"
              ? {
                  work: {
                    ...one.work,
                    plan: plan(
                      true,
                      "p-4",
                      tasks(
                        "P",
                        "open",
                        "Add Pause at the end of the row",
                        "Draw the row as paused",
                      ),
                    ),
                    workflow: {
                      name: "Feature",
                      steps: STEPS,
                      at: 1,
                    },
                  },
                }
              : {}),
          }));
        },
      };
    },
  },
};
