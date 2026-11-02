// A real Job's thread, drawn in the new thread look: the three `real-thread-*` mocks differ only in the Job they hold.
// The look, the quick bar and the Aa panel are `livelier-thread`'s. The walk over them is `real-thread-walk.ts`, kept
// apart so the app's scenarios never load the walk engine.

import { DEFAULT_RESPONSES } from "@armada/components";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "../scenarios/sessions";

type RealThread = {
  /** The scenario's name, which the walk is over. */
  name: string;
  says: string;
  /** The Job's id as Fleet kept it. */
  id: string;
  title: string;
  branch: string;
  slot: number;
  rows: readonly SessionRow[];
  /** Short replies to the agent's last message. */
  asked: readonly string[];
  /** A question the agent is held on, put in the Needs you band. */
  ask?: Session["asked"];
};

export function realThreadScenario(one: RealThread): Scenario {
  const last = one.rows[one.rows.length - 1]!.at;
  const job = (): Session => ({
    id: one.id,
    title: one.title,
    turn: {
      state: "idle",
    },
    lastTurnAt: last,
    rows: [
      ...one.rows,
    ],
    attachments: [
      {
        kind: "slot",
        slot: one.slot,
      },
      {
        kind: "branch",
        name: one.branch,
        slot: one.slot,
      },
    ],
    ...(one.ask === undefined
      ? {}
      : {
          asked: one.ask,
        }),
  });
  return {
    ...s200Sessions,
    name: one.name,
    says: one.says,
    draft: {
      sessions: (board) =>
        s200Sessions.draft!.sessions!(board, [
          job(),
        ]),
      threadLook: {
        look: {
          agentName: "",
          showTimes: true,
          showAgentName: true,
          textSize: "base",
          spacing: "default",
          quick: [
            {
              id: "learnt:done",
              text: "Just get it done",
              kind: "learnt",
            },
            {
              id: "learnt:walk",
              text: "Walk me through it",
              kind: "learnt",
            },
            {
              id: "saved:land",
              text: "Land it",
              kind: "saved",
            },
          ],
          learn: true,
          forgotten: [],
          voice: "apple",
          quickOff: false,
          quickOffFor: [],
          responses: DEFAULT_RESPONSES,
        },
        asked: {
          [one.id]: one.asked,
        },
        heard: "Go ahead",
      },
    },
  };
}
