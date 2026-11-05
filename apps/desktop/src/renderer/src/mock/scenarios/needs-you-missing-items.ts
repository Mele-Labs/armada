// Three Jobs whose calls Fleet now lists: a feature Job on Deliver whose plan was recorded again (version 6), a design plan Job on Present with two pages
// to approve (one it fetched, one its Draft step wrote), and a Job that drew a second sketch after the first, so only the newest is pinned.
// A fourth workflow, `doc_review` from the Kit, is in the Workflow menu. The walk `needs-you-missing-items` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionWorkActs, WorkflowChoice } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { planCall } from "../plan-call";
import { arrow, box, scene } from "../scenes";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const PLAN_JOB = "nym-1";
const DOCS_JOB = "nym-2";
const SKETCH_JOB = "nym-3";

const FEATURE: WorkflowChoice = {
  id: "feature",
  name: "Feature",
  steps: [
    "Propose",
    "Discuss",
    "Approve plan",
    "Walk",
    "Refine",
    "Deliver",
  ],
  manifestId: OWNER,
};
const DESIGN: WorkflowChoice = {
  id: "design_plan",
  name: "Design plan",
  steps: [
    "Explore",
    "Draft",
    "Present",
  ],
  manifestId: OWNER,
};
const DOC_REVIEW: WorkflowChoice = {
  id: "doc_review",
  name: "doc_review",
  steps: [
    "Read",
    "Comment",
    "Present",
  ],
  manifestId: OWNER,
};

const message = (id: string, at: string, from: "you" | "agent", text: string) => ({
  id,
  at,
  kind: "message" as const,
  from: {
    kind: from,
  },
  text,
});

const task = (id: string, text: string) => ({
  id,
  text,
  state: "open" as const,
});

// ---- The plan recorded again on Deliver -----------------------------------------------------------------------

const planJob: Session = {
  id: PLAN_JOB,
  title: "Cap retries in the store client",
  turn: {
    state: "idle",
  },
  standing: "idle",
  lastTurnAt: ago(2),
  rows: [
    message("p1", "10:02:00", "you", "Cap the retries in the store client."),
    message("p2", "10:41:00", "agent", "The plan is recorded again with one task added."),
  ],
  attachments: [],
  work: {
    proposal: {
      workflow: FEATURE.id,
    },
    workflow: {
      name: FEATURE.name,
      steps: FEATURE.steps,
      at: 5,
    },
    plan: {
      version: 6,
      summary: "Cap retries at three in the store client, and read the cap from settings.",
      scope: [
        "crates/store",
      ],
      approved: false,
      changes: {
        added: [
          "T3",
        ],
        renamed: [],
        removed: [],
      },
      groups: [
        {
          id: "g1",
          agent: "rust-engineer",
          title: "Cap the retries",
          paths: [
            "crates/store",
          ],
          tasks: [
            task("T1", "Add the cap to the client"),
            task("T2", "Read the cap from settings"),
            task("T3", "Refuse a cap of zero"),
          ],
        },
      ],
    },
  },
  waitingFor: [
    {
      ...planCall(6, ago(2)),
      text: "Plan v6",
    },
  ],
};

// ---- Two pages to approve on Present ---------------------------------------------------------------------------

const HANDOFF_PATH = "docs/handoff/operator-design-handoff.html";
const OPERATOR_PATH = "docs/design/the-operator-takes-over.html";

const docsJob: Session = {
  id: DOCS_JOB,
  title: "Design the Operator handoff",
  turn: {
    state: "idle",
  },
  standing: "idle",
  lastTurnAt: ago(3),
  rows: [
    message("d1", "09:10:00", "you", "Design the handoff to the Operator."),
    message("d2", "09:55:00", "agent", "Both pages are ready."),
  ],
  attachments: [
    {
      kind: "artifact",
      form: "document",
      id: HANDOFF_PATH,
      title: "Operator design handoff",
      revisions: [
        HANDOFF_PATH,
      ],
      asked: true,
      revision: "1",
      kept: ago(20),
    },
    {
      kind: "artifact",
      form: "document",
      id: OPERATOR_PATH,
      title: "The Operator takes over",
      revisions: [
        OPERATOR_PATH,
      ],
      asked: true,
      revision: "1",
      kept: ago(8),
    },
  ],
  work: {
    proposal: {
      workflow: DESIGN.id,
    },
    workflow: {
      name: DESIGN.name,
      steps: DESIGN.steps,
      at: 2,
    },
  },
  waitingFor: [
    {
      id: `document:${HANDOFF_PATH}`,
      text: "Operator design handoff",
      since: ago(20),
      source: "document",
      act: {
        kind: "message",
        target: DOCS_JOB,
      },
    },
    {
      id: `document:${OPERATOR_PATH}`,
      text: "The Operator takes over",
      since: ago(8),
      source: "document",
      act: {
        kind: "message",
        target: DOCS_JOB,
      },
    },
  ],
};

// ---- Two sketches, the newer approved -----------------------------------------------------------------------

const DRAWING = scene(
  [
    box("loan", 0, 60, "Loan", "a DSCR below the floor", {
      icon: "layers",
      step: 1,
    }),
    box("guard", 320, 60, "Guardrail", "stops the quote", {
      icon: "server",
      step: 2,
    }),
  ],
  [
    arrow("loan", "guard", "checked"),
  ],
);
const EARLIER = "Pricing guardrails (DSCR only)";
const LATER = "Pricing guardrails (alerts)";

const sketchJob: Session = {
  id: SKETCH_JOB,
  title: "Guard the pricing floor",
  turn: {
    state: "working",
  },
  standing: "working",
  lastTurnAt: ago(1),
  rows: [
    message("s1", "11:00:00", "you", "Stop a quote that falls under the DSCR floor."),
    {
      id: "s1a",
      at: "11:05:00",
      kind: "tool" as const,
      text: `show_sketch ${JSON.stringify({
        title: EARLIER,
        scene: DRAWING,
      })}`,
    },
    {
      id: "s1b",
      at: "11:15:00",
      kind: "tool" as const,
      text: `show_sketch ${JSON.stringify({
        title: LATER,
        scene: DRAWING,
      })}`,
    },
    message("s2", "11:20:00", "agent", "The alerts sketch replaces the DSCR one."),
  ],
  // Fleet sends the ledger newest first. Both stay asked on it; only the newest drawn is put to him.
  attachments: [
    {
      kind: "sketch",
      id: `sketch:${LATER}`,
      title: LATER,
      by: "agent",
      drawing: DRAWING,
      asked: true,
      settled: {
        how: "approved",
        at: ago(5),
      },
      drawn: "job12345-20261104T151426339Z-7",
    },
    {
      kind: "sketch",
      id: `sketch:${EARLIER}`,
      title: EARLIER,
      by: "agent",
      drawing: DRAWING,
      asked: true,
      drawn: "job12345-20261104T150112210Z-3",
    },
  ],
  work: {
    proposal: {
      workflow: FEATURE.id,
    },
    workflow: {
      name: FEATURE.name,
      steps: FEATURE.steps,
      at: 3,
    },
  },
};

function build(): Scenario {
  const base = holding(
    "needs-you-missing-items",
    "A plan recorded again on Deliver, two pages on Present, an older open sketch that does not wait, and a Kit workflow in the Workflow menu",
  );
  return {
    ...base,
    state: {
      ...base.state,
      repository: repository().root,
    },
    draft: {
      sessions: () => {
        const store = sessionsStore(
          [
            planJob,
            docsJob,
            sketchJob,
          ],
          [],
          new Map([
            [
              repository().root,
              OWNER,
            ],
          ]),
        );
        const work: SessionWorkActs = {
          ...store.work!,
          workflows: [
            FEATURE,
            DESIGN,
            DOC_REVIEW,
          ],
        };
        return {
          ...store,
          work,
        };
      },
    },
  };
}

export const needsYouMissingItems: Scenario = build();
