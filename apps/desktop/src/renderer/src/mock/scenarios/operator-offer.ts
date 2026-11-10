// A Job the Operator offers in Discussion, for the walk `operatorOffer`: a reply that needs the code read offers a Job.

import type { Scenario } from "../moment";
import { operatorPanel } from "./operator-panel";

const seed = operatorPanel.draft!.operator!;

export const operatorOffer: Scenario = {
  ...operatorPanel,
  name: "operator-offer",
  says: "The Operator answers that the code needs reading and offers a Job, which he dispatches into a repository from Discussion",
  draft: {
    ...operatorPanel.draft,
    operator: {
      ...seed,
      talk: [
        {
          asked: "error",
          said: "React error #185 is an update loop in the overview. No open issue or live Job covers it, and finding the cause means reading the code, so I'm offering a Job to do that.",
          offers:
            "Find the update loop behind React error #185 in the overview region, and say whether it needs its own fix.",
        },
        ...seed.talk,
      ],
    },
  },
};
