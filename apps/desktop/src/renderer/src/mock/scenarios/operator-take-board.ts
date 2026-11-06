// The Operator's Queue drawn as a board of lanes, for the walk `operatorTakeBoard`. The same moment as `operator-queue`.

import type { Scenario } from "../moment";
import { operatorQueue } from "./operator-queue";

export const operatorTakeBoard: Scenario = {
  ...operatorQueue,
  name: "operator-take-board",
  says: "The Operator's Queue as a board: Up next, Operator on it, Needs you and Done, with cards that move between lanes",
  draft: {
    ...operatorQueue.draft,
    operator: {
      ...operatorQueue.draft!.operator!,
      take: "board",
      // As the Fleet-backed Operator does: Discuss brings an item up at once.
      discussesAtOnce: true,
      // Each card names the Job it is about, with what it asks under, as Fleet's board and a Job's Needs you items arrive.
      turns: operatorQueue.draft!.operator!.turns?.map((one) =>
        one.id === "turn-timings"
          ? {
              ...one,
              text: "Where should it keep the timings?",
              job: {
                session: "op-timings",
                title: "Time each build step",
              },
            }
          : one.id === "turn-legend"
            ? {
                ...one,
                text: "Should it keep the old key?",
                job: {
                  session: "op-legend",
                  title: "Rework the Map legend",
                },
              }
            : one,
      ),
      // The Job's question with its own options, the Operator's pick marked, as a Fleet Job's item arrives.
      raises: operatorQueue.draft!.operator!.raises?.map((one) =>
        one.id === "raise-console"
          ? {
              ...one,
              item: "Cockpit Console",
              read: "Which way should it draw the bays?",
              ways: [
                "Keep the order",
                "Sort by state",
              ],
              recommends: "Keep the order",
              results: {
                "Keep the order": "Answered: Keep the order",
                "Sort by state": "Answered: Sort by state",
              },
            }
          : one,
      ),
    },
  },
};
