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
      // The Job's question with its own options, the Operator's pick marked, as a Fleet Job's item arrives.
      raises: operatorQueue.draft!.operator!.raises?.map((one) =>
        one.id === "raise-console"
          ? {
              ...one,
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
