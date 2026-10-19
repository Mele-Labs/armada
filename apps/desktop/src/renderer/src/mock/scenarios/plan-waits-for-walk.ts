// A Job that showed a page, is changing it, and has recorded its plan. Fleet holds the plan until the page is ready, so the thread and Needs you show
// none. When the page is ready the `plan:` item arrives and the plan is asked. Over the `sessions` scenario with one more Session open beside its own.
// The walk `plan-waits-for-walk` plays it.

import type { Session, WorkGroup } from "@armada/screens/src/draft/sessions";
import { markWalkWindow, mockPage, openWalkWindow } from "@armada/screens/src/fixtures/walk-window";

import type { Scenario } from "../moment";
import { planCall } from "../plan-call";
import { s200Sessions } from "./sessions";

const HELD = "plan-held";
const TITLE = "Hold the plan";
const PAGE = "Bay page";

const GROUPS: readonly WorkGroup[] = [
  {
    id: "h1",
    agent: "bridge-engineer",
    title: "Show the bay's warm time",
    paths: ["apps/desktop/src/renderer/src/cockpit/deck"],
    tasks: [
      { id: "H1", text: "Draw the warm time on the bay tile", state: "open" },
      { id: "H2", text: "Keep the tile still while it warms", state: "open" },
    ],
  },
];

const held: Session = {
  id: HELD,
  title: TITLE,
  turn: { state: "working" },
  standing: "working",
  lastTurn: "14:12",
  lastTurnAt: new Date().toISOString(),
  rows: [
    { id: "h-1", at: "14:05:00", kind: "message", from: { kind: "you" }, text: "Show how long a bay took to warm." },
    { id: "h-2", at: "14:08:10", kind: "window", title: PAGE, url: mockPage() },
    { id: "h-3", at: "14:10:30", kind: "message", from: { kind: "agent" }, text: "Changing the page. The plan is recorded." },
  ],
  attachments: [{ kind: "artifact", form: "window", id: mockPage(), title: PAGE }],
  work: { plan: { scope: ["apps/desktop/src/renderer/src/cockpit/deck"], approved: false, after: "h-3", groups: GROUPS } },
};

export const s291PlanWaitsForWalk: Scenario = {
  ...s200Sessions,
  name: "plan-waits-for-walk",
  says: "A Job changing a page it showed, with its plan recorded and held: the plan is asked once the page is ready",
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board, [held]);
      let ready = false;
      return {
        ...store,
        // The window opens still changing until the page is ready, and ready after.
        openWindow: (id, url) => {
          if (id !== HELD) return store.openWindow?.(id, url);
          openWalkWindow(TITLE, url, { title: PAGE, job: TITLE, into: "Session", onNote: () => undefined, readiness: ready ? "ready" : "working" });
        },
        // The page is ready: Fleet puts the plan to him, and the Job is done with its turn.
        later: () => {
          if (ready) return;
          ready = true;
          markWalkWindow("ready");
          store.edit(HELD, (one) => ({
            ...one,
            turn: { state: "idle" },
            standing: "replied",
            lastTurnAt: new Date().toISOString(),
            rows: [...one.rows, { id: "h-4", at: "14:13:00", kind: "message", from: { kind: "agent" }, text: "The page is ready. Approve the plan, or tell me what to change." }],
            waitingFor: [...(one.waitingFor ?? []), planCall(undefined, new Date().toISOString())],
          }));
        },
      };
    },
  },
};
