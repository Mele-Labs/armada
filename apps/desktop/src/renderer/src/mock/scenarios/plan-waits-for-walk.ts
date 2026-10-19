// A Job that showed a page, is changing it, and has recorded its plan. Fleet puts nothing to him while the page changes. When it is ready the page
// waits in Needs you, closed window or open, and the plan waits behind it. His Approve of the page releases the plan. Over the `sessions` scenario with
// one more Session open beside its own. The walk `plan-waits-for-walk` plays it.

import type { Session, WorkGroup } from "@armada/screens/src/draft/sessions";
import { markWalkWindow, mockPage, openWalkWindow } from "@armada/screens/src/fixtures/walk-window";

import type { Scenario } from "../moment";
import { planCall, walkCall, withoutPlanCall, withWalkCall } from "../plan-call";
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
  says: "A Job changing a page it showed, with its plan recorded: the page waits in Needs you once ready, and the plan once he approves it",
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board, [held]);
      const url = mockPage();
      // Changing: nothing is put to him. Ready: the page waits. Approved: the page is off the list and the plan is asked.
      let phase: "changing" | "ready" | "approved" = "changing";
      return {
        ...store,
        openWindow: (id, at) => {
          if (id !== HELD) return store.openWindow?.(id, at);
          openWalkWindow(TITLE, at, {
            title: PAGE,
            job: TITLE,
            into: "Session",
            onNote: () => undefined,
            readiness: phase === "changing" ? "working" : "ready",
            onApprove: async () => {
              if (phase !== "approved") {
                phase = "approved";
                const now = new Date().toISOString();
                store.edit(HELD, (one) => ({
                  ...one,
                  rows: [
                    ...one.rows,
                    { id: "h-5", at: "14:14:00", kind: "message", from: { kind: "you" }, text: `Approved: ${at}`, instant: now },
                    { id: "h-6", at: "14:14:20", kind: "message", from: { kind: "agent" }, text: "Approve the plan, or tell me what to change." },
                  ],
                  waitingFor: [...withoutPlanCall(withWalkCall(one.waitingFor, url)), planCall(undefined, now)],
                }));
              }
              return { ok: true };
            },
          });
        },
        later: () => {
          if (phase !== "changing") return;
          phase = "ready";
          markWalkWindow("ready");
          store.edit(HELD, (one) => ({
            ...one,
            turn: { state: "idle" },
            standing: "replied",
            lastTurnAt: new Date().toISOString(),
            rows: [...one.rows, { id: "h-4", at: "14:13:00", kind: "message", from: { kind: "agent" }, text: "The page is ready." }],
            waitingFor: withWalkCall(one.waitingFor, url, walkCall(url, PAGE, new Date().toISOString())),
          }));
        },
      };
    },
  },
};
