// A new Job dispatched from the Dispatch sheet, which the agent's first turn ends by asking how its work is delivered: Auto land, Open PR only or
// Keep local. Nothing is pressed to open the question's card, so Needs you is open on it the moment the Job asks. Over `quick-dispatch`, and the
// walk `dispatch-asks-delivery` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s210QuickDispatch } from "./quick-dispatch";
import { others, sessionsStore } from "../sessions/script";

/** The first Job dispatched in the mock takes this id. The usual Sessions are left without it, so the new Job is the only one under it. */
const MINE = "s7";

const DELIVERY: NonNullable<Session["waitingFor"]>[number] = {
  id: "delivery",
  text: "How should this work be delivered?",
  since: "2026-10-19T14:12:00.000Z",
  source: "delivery",
  act: { kind: "answer", target: "delivery" },
  options: [{ label: "Auto land" }, { label: "Open PR only" }, { label: "Keep local" }],
};

export const s299DispatchAsksDelivery: Scenario = {
  ...s210QuickDispatch,
  name: "dispatch-asks-delivery",
  says: "A Job dispatched from the Dispatch sheet whose first turn asks how its work is delivered, so Needs you opens on the question with no click",
  draft: {
    ...s210QuickDispatch.draft,
    sessions: () => {
      const store = sessionsStore([], [], new Map([[repository().root, repository().manifest!.id]]), others().filter((one) => one.id !== MINE));
      return {
        ...store,
        // The Job's first turn ends on the delivery question: it goes idle with the question held, and the agent says it in the thread.
        later: () =>
          store.edit(MINE, (one: Session) => ({
            ...one,
            turn: { state: "idle" },
            lastTurnAt: new Date().toISOString(),
            rows: [
              ...one.rows,
              { id: "d1", at: "14:12:00", kind: "message", from: { kind: "agent" }, text: "The change is in. How should this work be delivered?" },
            ],
            waitingFor: [DELIVERY],
          })),
      };
    },
  },
};
