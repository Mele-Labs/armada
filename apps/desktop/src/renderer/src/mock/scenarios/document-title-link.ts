// A design plan Job wrote a page with its own title, and its message names that page. Pressing the title in the thread opens the page, ahead of
// the Job of the same name. The page is not approved, so the person is owed one approval, the page's, and no plan card stands in the thread; his
// Approve of the page settles the plan too. Mock only: the page's approval is a walk page, as `plan-waits-for-walk` holds one. The walk
// `document-title-link` plays it.

import type { Session, SessionRow, WorkflowChoice } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";
import { openWalkWindow } from "@armada/screens/src/fixtures/walk-window";

import { walkPage } from "../audit-job";
import { mockAreas } from "../cockpit-areas";
import { ROLLOUT_DOC, ROLLOUT_DOC_PATH, ROLLOUT_DOC_TITLE } from "../document-pages";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { walkCall, withWalkCall } from "../plan-call";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const JOB = "title-link-1";
const URL = walkPage("document-title-link", "rollout");
const DESIGN: WorkflowChoice = { id: "Design plan", name: "Design plan", steps: ["Explore", "Write the design", "Present"] };

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

const job: Session = {
  id: JOB,
  title: ROLLOUT_DOC_TITLE,
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: ago(1),
  standing: "replied",
  rows: [
    you("t-1", "14:00:00", "Plan how Custom Margins reaches lenders, in four phases."),
    agent("t-2", "14:06:00", `I've written the rollout plan as a page in Bridge, "${ROLLOUT_DOC_TITLE}", and recorded the plan.`),
  ],
  attachments: [{ kind: "artifact", form: "document", id: ROLLOUT_DOC_PATH, title: ROLLOUT_DOC_TITLE, revisions: [ROLLOUT_DOC_PATH] }],
  // The page waits, so the plan is not asked: no plan call stands in waitingFor.
  waitingFor: [walkCall(URL, ROLLOUT_DOC_TITLE, ago(1))],
  work: {
    proposal: { workflow: DESIGN.name },
    landing: "document",
    workflow: { ...DESIGN, at: 2 },
    plan: {
      scope: ["docs/plans"],
      approved: false,
      after: "t-2",
      summary: "Four phases: staff, five pilot lenders, half of lenders, then everyone.",
      groups: [{ id: "c1", agent: "general-purpose", title: "Roll Custom Margins out in four phases", paths: ["docs/plans"], tasks: [{ id: "C1", text: "Write the rollout plan", state: "done" }] }],
    },
  },
};

function sessions() {
  const base = sessionsStore([job], [], new Map(), []);
  return {
    ...base,
    readArtifact: async (id: string, path: string) => (path === ROLLOUT_DOC_PATH ? { ok: true as const, bytes: new TextEncoder().encode(ROLLOUT_DOC), type: "text/html" } : (base.readArtifact?.(id, path) ?? { ok: false as const, why: "unreadable" as const })),
    // His Approve of the page takes it off his list and settles the plan with it.
    openWindow: (to: string, at: string) => {
      if (to !== JOB) return base.openWindow?.(to, at);
      openWalkWindow(ROLLOUT_DOC_TITLE, at, {
        title: ROLLOUT_DOC_TITLE,
        job: ROLLOUT_DOC_TITLE,
        into: "Session",
        onNote: () => undefined,
        readiness: "ready",
        onApprove: async () => {
          base.edit(JOB, (one) => ({
            ...one,
            rows: [...one.rows, you("t-3", "14:09:00", `Approved: ${at}`)],
            waitingFor: withWalkCall(one.waitingFor, URL),
            work: one.work?.plan === undefined ? one.work : { ...one.work, plan: { ...one.work.plan, approved: true } },
          }));
          return { ok: true };
        },
      });
    },
  };
}

function build(): Scenario {
  const base = holding("document-title-link", "A design plan Job whose page has its own title: the title in the thread opens the page, and the page's approval is the only one asked");
  return { ...base, state: { ...base.state, repository: null }, draft: { sessions, areas: () => mockAreas("/armada") } };
}

export const sDocumentTitleLink: Scenario = build();
