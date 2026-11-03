// Two Jobs whose process ended with a turn open. Fleet is starting the first again, so its item says so and offers nothing to press.
// Nothing is coming for the second, so its item offers Start again, and the press moves the row to the starting form before it clears.
// The walk `process-ended` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const FLEET = {
  kind: "session",
  id: "fleet",
  title: "Fleet",
} as const;

const jobs: Session[] = [
  {
    id: "pe-export",
    title: "Add invoice export",
    manifestId: OWNER,
    cwd: ROOT,
    turn: {
      state: "idle",
    },
    standing: "idle",
    lastTurnAt: ago(2),
    rows: [
      {
        id: "pe-1",
        at: "09:12:00",
        kind: "message",
        from: {
          kind: "you",
        },
        text: "Add an invoice export to the billing page.",
      },
      {
        id: "pe-2",
        at: "09:47:10",
        kind: "message",
        from: FLEET,
        text: "The job's process ended while Fleet was down",
      },
      {
        id: "pe-3",
        at: "09:47:12",
        kind: "message",
        from: FLEET,
        text: "Your process ended during a turn and Fleet started it again. Check what your last calls did, then carry on.",
      },
    ],
    attachments: [
      {
        kind: "branch",
        name: "feat/invoice-export",
        slot: 3,
      },
    ],
    waitingFor: [
      {
        id: `process_ended:${ago(2)}`,
        text: "Process ended. Starting again.",
        since: ago(2),
        source: "process_ended",
        act: {
          kind: "message",
          target: "pe-export",
        },
      },
    ],
  },
  {
    id: "pe-login",
    title: "Fix the login redirect",
    manifestId: OWNER,
    cwd: ROOT,
    turn: {
      state: "idle",
    },
    standing: "idle",
    lastTurnAt: ago(9),
    rows: [
      {
        id: "pe-4",
        at: "08:30:00",
        kind: "message",
        from: {
          kind: "you",
        },
        text: "Fix the login redirect that drops the return address.",
      },
      {
        id: "pe-5",
        at: "08:58:40",
        kind: "message",
        from: FLEET,
        text: "The job's process ended while Fleet was down",
      },
    ],
    attachments: [
      {
        kind: "branch",
        name: "fix/login-redirect",
        slot: 5,
      },
    ],
    waitingFor: [
      {
        id: `process_ended:${ago(9)}`,
        text: "Process ended. Not started again.",
        since: ago(9),
        source: "process_ended",
        act: {
          kind: "answer",
          target: "",
        },
        options: [
          {
            label: "Start again",
          },
        ],
      },
    ],
  },
];

const base = holding(
  "process-ended",
  "Two Jobs whose process ended in a turn, one Fleet is starting again and one it is not",
);

export const s346ProcessEnded: Scenario = {
  ...base,
  state: {
    ...base.state,
    repository: null,
  },
  draft: {
    sessions: () =>
      sessionsStore(
        jobs,
        [],
        new Map(),
        [],
      ),
  },
};
