// Two Jobs over the `sessions` scenario. "Retry backoff cap" waits on the person for seven things, one of each kind: a question, a command, a plan, a sketch,
// a draft, a document and a walk. The question and the command are on its Needs you card; the Review row there, or ⇧⌘Y, opens the sheet with the other five.
// "Name the cache key" waits on a question and on a Job its agent started, both on the card, and has nothing to review, so its card has no Review row.
// Draft data, so an answer goes where the thread's own card sends it and nothing reaches Fleet. The walks `job-review-sheet` and `job-review-sheet-wired` play it.

import type { PlanMark, Session } from "@armada/screens/src/draft/sessions";
import { mockPage } from "@armada/screens/src/fixtures/walk-window";

import type { Scenario } from "../moment";
import { planCall, withoutPlanCall, walkCall } from "../plan-call";
import { s200Sessions } from "./sessions";

const ID = "review-backoff";
const TITLE = "Retry backoff cap";
const ASK_ID = "review-question";
const ASK_TITLE = "Name the cache key";
const CHILD_ID = "review-child";
const CHILD_TITLE = "Add the retry metrics";
const READ_ID = "review-only";
const READ_TITLE = "Draw the retry curve";
const READ_SKETCH = "sketch:Retry curve";
const COMMAND = "scripts/rebuild-index";
const DESIGN_PATH = "documents/retry-design/r1.html";
const SKETCH = "sketch:Backoff curve";
const PULL = "pull-text";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const walkUrl = () => {
  const url = new URL(mockPage());
  url.search = "?scenario=default&walked";
  return url.toString();
};

const DESIGN = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Retry design</title></head>
<body>
<h1>Retry design</h1>
<p>A retry waits twice as long as the one before it, starting at one second. The wait stops growing at the cap.</p>
<table>
<thead><tr><th>Retry</th><th>Wait</th></tr></thead>
<tbody>
<tr><td>1</td><td>1 second</td></tr>
<tr><td>2</td><td>2 seconds</td></tr>
<tr><td>5</td><td>16 seconds</td></tr>
<tr><td>6 and after</td><td>30 seconds</td></tr>
</tbody>
</table>
<p>The cap is read once, when Fleet starts.</p>
</body></html>`;

const CURVE = {
  nodes: [
    {
      id: "n1",
      kind: "box",
      x: 0,
      y: 0,
      title: "First retry",
      body: "1 second",
    },
    {
      id: "n2",
      kind: "box",
      x: 220,
      y: 0,
      title: "Second retry",
      body: "2 seconds",
    },
    {
      id: "n3",
      kind: "box",
      x: 440,
      y: 0,
      title: "Third retry",
      body: "4 seconds",
    },
    {
      id: "n4",
      kind: "box",
      x: 660,
      y: 0,
      title: "Capped",
      body: "30 seconds",
      color: "amber",
    },
  ],
  edges: [
    {
      id: "e1",
      from: "n1",
      to: "n2",
    },
    {
      id: "e2",
      from: "n2",
      to: "n3",
    },
    {
      id: "e3",
      from: "n3",
      to: "n4",
      label: "doubles to the cap",
    },
  ],
} as const;

const job = (): Session => ({
  id: ID,
  title: TITLE,
  turn: {
    state: "idle",
  },
  lastTurnAt: minutesAgo(1),
  rows: [
    {
      id: "j1",
      at: "13:50:00",
      kind: "message",
      from: {
        kind: "you",
      },
      text: "Cap the retry backoff at thirty seconds.",
    },
    {
      id: "j2",
      at: "14:02:10",
      kind: "message",
      from: {
        kind: "agent",
      },
      text: "The cap is in. I need a few things from you.",
    },
    {
      id: PULL,
      at: "14:12:03",
      kind: "draft",
      title: "Pull request text",
      text: "Cap the retry backoff at thirty seconds.\n\nThe delay doubled with no limit, so a Job that failed ten times waited seventeen minutes. It now stops growing at thirty seconds. The cap is a setting, and zero is refused.",
      before: "Fix the retry backoff.",
    },
  ],
  attachments: [
    {
      kind: "slot",
      slot: 2,
    },
    {
      kind: "branch",
      name: "fleet/retry-backoff-cap",
      slot: 2,
    },
    {
      kind: "sketch",
      id: SKETCH,
      title: "Backoff curve",
      by: "agent",
      drawing: CURVE,
      asked: true,
      drawn: "drawn-1",
    },
    {
      kind: "artifact",
      form: "document",
      id: DESIGN_PATH,
      title: "Retry design",
      revisions: [
        DESIGN_PATH,
      ],
      asked: true,
      revision: "1",
      kept: minutesAgo(3),
    },
    {
      kind: "artifact",
      form: "window",
      id: walkUrl(),
      title: "Backoff table",
      window: {
        takes: [],
        steps: [],
      },
    },
  ],
  asked: {
    command: "AskUserQuestion",
    call: "q-fixture",
    questions: [
      {
        question: "Keep the old fixture?",
        header: "Fixture",
        multi_select: false,
        options: [
          {
            label: "Keep it",
            description: "The retry tests read it as it is",
          },
          {
            label: "Drop it",
            description: "Each test builds the delays it needs",
          },
        ],
      },
    ],
  },
  work: {
    plan: {
      version: 2,
      summary: "Cap the retry delay at thirty seconds, and test the curve up to the cap.",
      scope: [
        "crates/fleet/src/retry",
      ],
      approved: false,
      groups: [
        {
          id: "p1",
          agent: "rust-engineer",
          title: "Cap the delay",
          paths: [
            "crates/fleet/src/retry",
          ],
          tasks: [
            {
              id: "T1",
              text: "Add the cap to the retry policy",
              state: "open",
            },
            {
              id: "T2",
              text: "Read the cap from settings",
              state: "open",
            },
          ],
        },
        {
          id: "p2",
          agent: "test-writer",
          title: "Hold the curve",
          paths: [
            "crates/fleet/src/retry",
          ],
          tasks: [
            {
              id: "T3",
              text: "Test each delay up to the cap",
              state: "open",
            },
            {
              id: "T4",
              text: "Test that a cap of zero is refused",
              state: "open",
            },
          ],
        },
      ],
    },
  },
  waitingFor: [
    {
      id: "reindex",
      text: "Rebuild the search index",
      since: minutesAgo(7),
      source: "agent",
      act: {
        kind: "run",
        target: COMMAND,
      },
    },
    {
      ...planCall(2, minutesAgo(6)),
      text: "Plan v2",
    },
    {
      id: SKETCH,
      text: "Backoff curve",
      since: minutesAgo(5),
      source: "sketch",
      act: {
        kind: "message",
        target: ID,
      },
    },
    {
      id: `draft:${PULL}`,
      text: "Pull request text",
      since: minutesAgo(4),
      source: "draft",
    },
    {
      id: `document:${DESIGN_PATH}`,
      text: "Retry design",
      since: minutesAgo(3),
      source: "document",
      act: {
        kind: "message",
        target: ID,
      },
    },
    walkCall(walkUrl(), "Backoff table", minutesAgo(2)),
  ],
});

/** A Job with nothing to review: a question, and a Job its agent started that waits for Start. Both are on the card. */
const askJob = (): Session => ({
  id: ASK_ID,
  title: ASK_TITLE,
  turn: {
    state: "idle",
  },
  lastTurnAt: minutesAgo(1),
  rows: [
    {
      id: "a1",
      at: "13:40:00",
      kind: "message",
      from: {
        kind: "you",
      },
      text: "Cache the parsed manifests.",
    },
    {
      id: "a2",
      at: "13:52:30",
      kind: "message",
      from: {
        kind: "agent",
      },
      text: "The cache is in. I started a Job for the retry metrics, and I need one answer.",
    },
  ],
  attachments: [
    {
      kind: "started_to",
      id: CHILD_ID,
    },
  ],
  asked: {
    command: "AskUserQuestion",
    call: "q-cache",
    questions: [
      {
        question: "What should the cache key include?",
        header: "Cache key",
        multi_select: false,
        options: [
          {
            label: "The path",
            description: "Two checkouts of one repository share an entry",
          },
          {
            label: "The path and the commit",
            description: "An entry goes stale as soon as the commit moves",
          },
        ],
      },
    ],
  },
  waitingFor: [
    {
      id: `start:${CHILD_ID}`,
      text: `Press Start on "${CHILD_TITLE}"`,
      since: minutesAgo(2),
      source: "held_child",
      act: {
        kind: "message",
        target: CHILD_ID,
      },
    },
  ],
});

/** A Job with only something to review: the card has nothing, so a press on the band opens the sheet. */
const readJob = (): Session => ({
  id: READ_ID,
  title: READ_TITLE,
  turn: {
    state: "idle",
  },
  lastTurnAt: minutesAgo(1),
  rows: [
    {
      id: "r1",
      at: "13:30:00",
      kind: "message",
      from: {
        kind: "agent",
      },
      text: "The curve is drawn.",
    },
  ],
  attachments: [
    {
      kind: "sketch",
      id: READ_SKETCH,
      title: "Retry curve",
      by: "agent",
      drawing: CURVE,
      asked: true,
      drawn: "drawn-1",
    },
  ],
  waitingFor: [
    {
      id: READ_SKETCH,
      text: "Retry curve",
      since: minutesAgo(2),
      source: "sketch",
      act: {
        kind: "message",
        target: READ_ID,
      },
    },
  ],
});

/** The Job the agent started, held on its start card. */
const childJob = (): Session => ({
  id: CHILD_ID,
  title: CHILD_TITLE,
  turn: {
    state: "idle",
  },
  rows: [],
  attachments: [
    {
      kind: "started_by",
      id: ASK_ID,
    },
  ],
  start: {
    asksDelivery: true,
  },
});

/** What the sheet sent to the draft, for the test that holds it carries his words. Cleared by the test before each run. */
export const SENT = {
  suggest: [] as {
    marks: readonly PlanMark[];
    approving: boolean | undefined;
  }[],
  sketch: [] as {
    how: string;
    words: string | undefined;
  }[],
  document: [] as {
    how: string;
    words: string | undefined;
  }[],
  /** What reached a waiting item's own answer, as the draft's card sends it. */
  waiting: [] as {
    item: string;
    given: object;
  }[],
};

const settled = (how: "approved" | "sent_back", words: string | undefined) => {
  const at = new Date().toISOString();
  return how === "sent_back"
    ? {
        how,
        words: words ?? "",
        at,
      }
    : {
        how,
        at,
      };
};

export const s310JobReviewSheet: Scenario = {
  ...s200Sessions,
  name: "job-review-sheet",
  says: "A Job's questions and commands on its Needs you card, and its plan, sketch, draft, document and walk on one sheet the Review row opens",
  draft: {
    sessions: (board) => {
      const base = s200Sessions.draft!.sessions!(board, [
        job(),
        askJob(),
        childJob(),
        readJob(),
      ]);
      return {
        ...base,
        // Start begins the Job the agent started, which leaves the card's row.
        begin: (id) =>
          base.edit(id, ({ start: _start, ...one }) => ({
            ...one,
            turn: {
              state: "working",
            },
          })),
        answerWaiting: (id, item, given) => {
          SENT.waiting.push({
            item,
            given,
          });
          return base.answerWaiting?.(id, item, given);
        },
        readArtifact: async (id, path) =>
          path === DESIGN_PATH
            ? {
                ok: true as const,
                bytes: new TextEncoder().encode(DESIGN),
                type: "text/html",
              }
            : (base.readArtifact?.(id, path) ?? {
                ok: false as const,
                why: "unreadable" as const,
              }),
        // Fleet settles the sketch on the ledger and takes its item off, as it does for the thread's own card.
        answerSketch: ({ session_id: id, sketch, how, words }) => {
          SENT.sketch.push({
            how,
            words,
          });
          return base.edit(id, (one) => ({
            ...one,
            attachments: one.attachments.map((a) =>
              a.kind === "sketch" && a.id === sketch
                ? {
                    ...a,
                    settled: settled(how, words),
                  }
                : a,
            ),
            waitingFor: (one.waitingFor ?? []).filter((item) => item.id !== sketch),
          }));
        },
        answerDocument: ({ session_id: id, document, revision, how, words }) => {
          SENT.document.push({
            how,
            words,
          });
          return base.edit(id, (one) => ({
            ...one,
            attachments: one.attachments.map((a) =>
              a.kind === "artifact" && `document:${a.id}` === document && a.revision === revision
                ? {
                    ...a,
                    settled: settled(how, words),
                  }
                : a,
            ),
            waitingFor: (one.waitingFor ?? []).filter((item) => item.id !== document),
          }));
        },
        work: {
          workflows: [],
          choose: () => undefined,
          unpick: () => undefined,
          approve: (id) =>
            base.edit(id, (one) => ({
              ...one,
              waitingFor: withoutPlanCall(one.waitingFor),
              work: {
                ...one.work,
                ...(one.work?.plan === undefined
                  ? {}
                  : {
                      plan: {
                        ...one.work.plan,
                        approved: true,
                      },
                    }),
              },
            })),
          // What he sent back goes to the agent as his changes, and the plan stops waiting on him until it is recorded again.
          suggest: (id, marks, approving) => {
            SENT.suggest.push({
              marks,
              approving,
            });
            base.edit(id, (one) => ({
              ...one,
              waitingFor: withoutPlanCall(one.waitingFor),
            }));
          },
        },
      };
    },
  },
};
