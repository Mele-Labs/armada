// A Job holding a Jira ticket at `Ready for QA`, a status the Ticket lifecycle lists for no state, so the Job's ticket stands unmapped. The ticket's
// row in Done and in the Issues tab has a menu of the Ticket states: choosing one teaches Armada the word (`slices/lifecycles.ts`), and the Job's own
// copy takes it and its Workflow is drawn again, as Fleet's re-read would. Based on `progress-issues-jira`. The walk `ticket-word` plays it.

import type { AppliedLifecycle, LifecycleDefinition } from "@armada/protocol";
import type { Lifecycle, SessionLanes } from "@armada/components";
import { MANIFEST_ID, repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import type { TicketRead } from "../../issues-tab/model";
import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sharedPanelReads } from "../shared-panel-reads";
import { sessionsStore, type SessionsStore } from "../sessions/script";
import { ticketWords } from "../ticket-words";
import { throughWire } from "../workspaces-lanes-wire";

const ID = "tw-1";
const KEY = "ABC-12";
const WORD = "Ready for QA";
const TRACKER = "https://tracker.invalid/browse/";
const TITLE = "Receipt shows the tax line twice";

/** The Jira words each Ticket state is read from before anything is taught. */
const WORDS: readonly (readonly [
  id: string,
  label: string,
  words: readonly string[],
])[] = [
  [
    "todo",
    "To do",
    [
      "To Do",
      "Backlog",
    ],
  ],
  [
    "in_progress",
    "In progress",
    [
      "In Progress",
    ],
  ],
  [
    "in_review",
    "In review",
    [
      "In Review",
    ],
  ],
  [
    "closed",
    "Closed",
    [
      "Done",
      "Closed",
    ],
  ],
];

const MOVES = [
  [
    "todo",
    "in_progress",
  ],
  [
    "in_progress",
    "in_review",
  ],
  [
    "in_review",
    "closed",
  ],
] as const;

/** Armada's Ticket with the Jira words, kept for every repository, as Fleet serves it before anything is taught. */
const TICKET: LifecycleDefinition = {
  id: "ticket",
  name: "Ticket",
  scope: {
    kind: "everywhere",
  },
  states: WORDS.map(([id, label, words]) => ({
    id,
    label,
    read: {
      source: "tracker",
      words: [
        ...words,
      ],
    },
  })),
  transitions: MOVES.map(([from, to]) => ({
    from,
    to,
  })),
  ends: [
    "closed",
  ],
};

const KEPT: AppliedLifecycle = {
  lifecycle: TICKET,
  kept: {
    rank: {
      reach: "everywhere",
      source: "armada",
    },
  },
};

/** The state the word was taught as, for the life of the window. */
let taught: string | undefined;

/** The Job's own copy of the Ticket, with the word on the state it was taught as. */
const ticketLane = (): Lifecycle => ({
  id: "ticket",
  label: "Ticket",
  from: "Jira",
  kind: "tracker",
  states: WORDS.map(([id, label, words], col) => ({
    id,
    label,
    col,
    reads: [
      {
        source: "Jira",
        values:
          id === taught
            ? [
                ...words,
                WORD,
              ]
            : words,
        scope: "default",
      },
    ],
    ...(id === "closed"
      ? {
          end: true as const,
        }
      : {}),
  })),
  next: MOVES,
});

const lanes = (): SessionLanes =>
  throughWire(ID, {
    workflow: "feature",
    at: 1,
    job: {
      workspace: "checkout",
      copy: {
        steps: [
          {
            id: "build",
            label: "Build",
            col: 0,
          },
          {
            id: "open_pr",
            label: "Open PR",
            col: 1,
          },
          {
            id: "land",
            label: "Land",
            col: 3,
          },
        ],
        lifecycles: [
          ticketLane(),
        ],
        rules: [],
      },
      instances: [
        taught === undefined
          ? {
              id: KEY,
              lifecycle: "ticket",
              label: KEY,
              state: "",
              unmapped: WORD,
              visited: [],
            }
          : {
              id: KEY,
              lifecycle: "ticket",
              label: KEY,
              state: taught,
              visited: [
                taught,
              ],
            },
      ],
      fired: [],
      changes: [],
    },
  });

const ticket: SessionAttachment = {
  kind: "issue",
  number: KEY,
  title: TITLE,
  address: `${TRACKER}${KEY}`,
  state: "open",
  status: WORD,
  pull_requests: [
    2412,
  ],
};

const job = (): Session => ({
  id: ID,
  title: "Fix the doubled tax line",
  manifestId: MANIFEST_ID,
  cwd: repository().root,
  turn: {
    state: "idle",
  },
  lastTurnAt: new Date(Date.now() - 6 * 60_000).toISOString(),
  standing: "idle",
  rows: [
    {
      id: "tw-1-1",
      at: "14:02:00",
      kind: "message",
      from: {
        kind: "you",
      },
      text: `Work ${KEY}.`,
    },
  ],
  attachments: [
    ticket,
    {
      kind: "branch",
      name: "receipt/one-tax-line",
      slot: 4,
    },
    {
      kind: "pull_request",
      number: 2412,
      title: "Draw the tax line once on the receipt",
      branch: "receipt/one-tax-line",
      address: "https://git.example/armada/pull/2412",
      checks: {
        state: "passed",
      },
      state: "open",
      auto: false,
    },
  ],
  work: {
    proposal: {
      workflow: null,
    },
  },
  lanes: lanes(),
});

const read: TicketRead = {
  reference: KEY,
  title: TITLE,
  url: `${TRACKER}${KEY}`,
  state: "open",
  author: "Priya Shah",
  assignees: [
    "Nick Mele",
  ],
  updated_at: new Date(Date.now() - 2 * 3_600_000).toISOString(),
  excerpt: "The receipt draws the tax line once per tax rate and once more for the total.",
  fields: [
    {
      name: "Status",
      values: [
        WORD,
      ],
    },
    {
      name: "Priority",
      values: [
        "High",
      ],
    },
  ],
  linked_pull_request: true,
  holders: [],
  body: "The receipt draws the tax line once per tax rate and once more for the total.",
  linked_pull_requests: [
    {
      number: 2412,
      title: "Draw the tax line once on the receipt",
      address: "https://git.example/armada/pull/2412",
      state: "open",
    },
  ],
};

/** The store on show. A taught word draws its Job again from the copy that took it. */
let live: SessionsStore | undefined;
ticketWords.listen(({ manifestId, word, state }) => {
  if (manifestId !== MANIFEST_ID || word !== WORD || live === undefined) return [];
  taught = state;
  live.edit(ID, (one) => ({
    ...one,
    lanes: lanes(),
  }));
  return [
    ID,
  ];
});

const base = holding(
  "ticket-word",
  "A Job's Jira ticket at Ready for QA, a status no Ticket state lists, taught from its row's menu",
);

export const ticketWord: Scenario = {
  ...base,
  state: {
    ...base.state,
    repository: null,
  },
  draft: {
    sessions: () => {
      taught = undefined;
      live = sessionsStore(
        [
          job(),
        ],
        [],
        new Map(),
        [],
      );
      return live;
    },
    areas: () => mockAreas("/armada"),
    sharedPanel: () =>
      sharedPanelReads({
        lines: [],
      }),
    pieces: {
      workspaces: [
        {
          id: "checkout",
          name: "checkout",
          repositories: [
            MANIFEST_ID,
          ],
          source: "kit",
          path: "/Users/user/.armada/workspaces/checkout.toml",
        },
      ],
      lifecycles: [
        KEPT,
      ],
      rules: [],
    },
    issuesTab: [
      {
        tracker: {
          manifestId: MANIFEST_ID,
          state: "read",
          name: "Jira",
          open: "statusCategory != Done ORDER BY updated DESC",
          assigned: "assignee = currentUser() AND statusCategory != Done",
          saved: [],
        },
        held: {
          [KEY]: [
            ID,
          ],
        },
        tickets: [
          read,
        ],
        me: "Nick Mele",
        language: "jql",
      },
    ],
  },
};
