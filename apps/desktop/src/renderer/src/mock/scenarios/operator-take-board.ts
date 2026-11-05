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
    },
  },
};
