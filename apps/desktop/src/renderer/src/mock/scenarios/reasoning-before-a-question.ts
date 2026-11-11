// A Job's thread where each turn's reasoning is a folded row, and the reasoning before a question it asked with nothing
// written to him since his message is drawn open above it. Job 20b4e00a's Queue for later turn, 10 Nov 2026. The walk
// `reasoning-before-a-question` plays it.

import type { CallInFlight, RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = {
  ...repository(),
  manifest: {
    ...repository().manifest!,
    id: "armada",
  },
};

const JOB = "01REASONINGBEFOREQUESTION";

const ASKED: CallInFlight = {
  call: "reasoned-q1",
  manifest_id: "armada",
  asked_at: "2026-11-11T02:34:17.000Z",
  tool: "AskUserQuestion",
  detail: "",
  truncated: false,
  rule: "AskUserQuestion",
  offers: [
    "allow_once",
    "refuse",
  ],
  holding_for_seconds: 120,
  questions: [
    {
      question: "Where should native show the ideas saved with Queue for later?",
      header: "Saved ideas",
      multi_select: false,
      options: [
        {
          label: "Jobs list (Recommended)",
          description: "A not-started row under its repository's group, started from the row",
        },
        {
          label: "In Dispatch",
          description: "A list inside the dispatch sheet, seen when you press N",
        },
        {
          label: "Leave it out",
          description: "No Queue for later in native until the Cockpit mod exists",
        },
      ],
    },
  ],
};

const records = () => [
  hosted(JOB, {
    title: "Native Jobs screens",
    attachments: [
      held("slot", "10"),
      held("branch", "armada/session-20b4e00a"),
    ],
    hosted: {
      turn: {
        state: "idle",
      },
      mode: "auto",
      running: false,
      asked: ASKED,
    },
  }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [JOB]: [
    {
      kind: "message",
      id: "y1",
      at: "2026-11-11T02:20:00.000Z",
      from: {
        kind: "you",
      },
      text: "Is the merge queue strip done?",
    },
    {
      kind: "tool",
      id: "t1",
      at: "2026-11-11T02:20:04.000Z",
      text: "Bash git log --oneline -3 origin/armada/session-ad34a1b8",
    },
    {
      kind: "reasoning",
      id: "r1",
      at: "2026-11-11T02:20:11.000Z",
      text: "The last commit on the Jobs screens branch is the strip's grace period, and its walk is shown, so the strip is done and waits on his look.",
    },
    {
      kind: "message",
      id: "m1",
      at: "2026-11-11T02:20:13.000Z",
      from: {
        kind: "agent",
      },
      text: "Yes. The strip is pushed and its walk is waiting for you.",
    },
    {
      kind: "message",
      id: "y2",
      at: "2026-11-11T02:33:57.000Z",
      from: {
        kind: "you",
      },
      text: "Re: Choose where native shows Queue for later ideas\nPlease explain",
    },
    {
      kind: "tool",
      id: "t2",
      at: "2026-11-11T02:34:03.000Z",
      text: 'Bash git grep -n -i "queue for later" -- apps packages crates/ipc',
    },
    {
      kind: "tool",
      id: "t3",
      at: "2026-11-11T02:34:09.000Z",
      text: "Bash sed -n '1,60p' apps/desktop/src/renderer/src/mock/walks/queue-for-later.ts",
    },
    {
      kind: "reasoning",
      id: "r2",
      at: "2026-11-11T02:34:16.000Z",
      text: "Native has no Cockpit to list saved ideas, so I'm weighing three options: showing them in the Jobs list, in the dispatch sheet, or leaving Queue for later out for now. I recommend the Jobs list, since it matches Electron's repository grouping and sits where you already look.",
    },
    {
      kind: "tool",
      id: "t4",
      at: "2026-11-11T02:34:17.000Z",
      text: "AskUserQuestion",
    },
    {
      kind: "ask",
      id: "ask-reasoned-q1",
      at: "2026-11-11T02:34:17.000Z",
      ask: ASKED,
      state: "waiting",
    },
  ],
});

function build(): Scenario {
  const board = onBoard({
    repositories: [
      ARMADA,
    ],
    picked: ARMADA.root,
  });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "reasoning-before-a-question",
    says: "A Job's thread with each turn's reasoning folded, and the reasoning before a question it asked without writing anything open above it",
    behaves: (handle) =>
      new FakeSessionsFleet(records(), threads()).scenario(board).behaves!(handle),
  };
}

export const s325ReasoningBeforeAQuestion: Scenario = build();
