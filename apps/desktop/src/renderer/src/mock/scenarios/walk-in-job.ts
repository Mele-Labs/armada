// A Job's walk as a tab of the Job, and no window. Three Jobs on the Cockpit: one with two walks, Billing with three takes and five steps and Settings with none and three, one with a single
// walk, and one running with none. A walk's tab is waiting in its Job's panes before he opens it, the Job's row pulses while it owes him a walk, and each tab pulses until it has been in front
// of him. Draft data, nothing here reaches Fleet. The walks `walk-in-job` and `walk-in-job-notes` play it.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { forgetPanes } from "../../job-panes/store";
import { forgetWalks, showAgain, workingJobs } from "../../job-walk/store";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const you = (id: string, at: string, text: string): SessionRow => ({
  id,
  at,
  kind: "message",
  from: {
    kind: "you",
  },
  text,
});
const agent = (id: string, at: string, text: string): SessionRow => ({
  id,
  at,
  kind: "message",
  from: {
    kind: "agent",
  },
  text,
});

const account: Session = {
  id: "wj-account",
  title: "Tidy the account pages",
  manifestId: OWNER,
  cwd: ROOT,
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(5),
  standing: "idle",
  rows: [
    you("a-1", "13:40:00", "Tighten the spacing on Billing and Settings."),
    agent("a-2", "14:02:10", "Both pages are in walks. Billing has three takes."),
  ],
  attachments: [
    {
      kind: "branch",
      name: "bridge/account-spacing",
      slot: 2,
    },
    {
      kind: "artifact",
      form: "window",
      id: "walk:settings",
      title: "Settings",
      walk: {
        steps: [
          {
            say: "The name field",
            target: "name",
          },
          {
            say: "The email switch",
            target: "notifications",
          },
          {
            say: "Save",
            target: "save",
          },
        ],
      },
    },
    {
      kind: "artifact",
      form: "window",
      id: "walk:billing",
      title: "Billing",
      walk: {
        takes: [
          "Compact",
          "Roomy",
          "With table",
        ],
        steps: [
          {
            say: "The plan card",
            target: "plan",
          },
          {
            say: "The invoice list",
            target: "invoices",
          },
          {
            say: "The state of an invoice",
            target: "status",
          },
          {
            say: "A receipt",
            target: "receipt",
          },
          {
            say: "Export all",
            target: "export",
          },
        ],
      },
    },
  ],
  work: {},
};

const clock: Session = {
  id: "wj-clock",
  title: "Pin the store clock",
  manifestId: OWNER,
  cwd: ROOT,
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(4),
  standing: "idle",
  rows: [
    you("c-1", "13:30:00", "Pin the clock in the store tests."),
    agent("c-2", "14:04:30", "Every test reads the pinned clock. The report is in a walk."),
  ],
  attachments: [
    {
      kind: "branch",
      name: "fix/pin-store-clock",
      slot: 4,
    },
    {
      kind: "artifact",
      form: "window",
      id: "walk:clock",
      title: "Clock report",
      walk: {
        steps: [
          {
            say: "The pinned time",
            target: "clock",
          },
          {
            say: "The tests that read it",
            target: "tests",
          },
        ],
      },
    },
  ],
  work: {},
};

const fold: Session = {
  id: "wj-fold",
  title: "Fold Jobs into Sessions",
  manifestId: OWNER,
  cwd: ROOT,
  turn: {
    state: "working",
  },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [
    you("f-1", "13:20:00", "Fold Jobs into Sessions."),
    {
      id: "f-2",
      at: "14:05:02",
      kind: "tool",
      text: "Edit docs/concepts/session.md",
    },
  ],
  attachments: [
    {
      kind: "branch",
      name: "fleet/fold-jobs",
      slot: 5,
    },
  ],
  work: {},
};

function sessions() {
  // Each mount starts with no tab kept and no walk touched.
  if (typeof window !== "undefined") {
    forgetPanes();
    forgetWalks();
  }
  const store = sessionsStore(
    [
      account,
      clock,
      fold,
    ],
    [],
    new Map([
      [
        ROOT,
        OWNER,
      ],
    ]),
    [],
  );
  // Time passing is the Job done with the notes it was sent: it says so and shows the page again.
  const passes = store.later;
  store.later = () => {
    passes();
    for (const id of workingJobs()) {
      store.edit(id, (one) => ({
        ...one,
        turn: {
          state: "idle",
        },
        lastTurnAt: new Date().toISOString(),
        rows: [
          ...one.rows,
          agent(`reply-${one.rows.length}`, "14:20:00", "The notes are in and the page is up again."),
        ],
      }));
      showAgain(id);
    }
  };
  return store;
}

export const walkInJob: Scenario = {
  ...holding(
    "walk-in-job",
    "A Job's walk as a tab beside its thread: the row and the tab pulse until opened, and approving closes the tab",
  ),
  draft: {
    sessions,
  },
};
