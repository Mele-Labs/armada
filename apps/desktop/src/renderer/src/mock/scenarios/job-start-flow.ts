// A Job he starts runs at once: Start on the Dispatch sheet opens the Job working on his words. Draft beside it is the one way
// his own work waits on its start card. Under the sheet, a card starts a quick Job as ⌘T does. Over `job-start-card`, whose store
// holds a start only when it is asked to. The walk `job-start-flow` plays it.

import type { Scenario } from "../moment";
import { s323JobStartCard } from "./job-start-card";

export const s340JobStartFlow: Scenario = {
  ...s323JobStartCard,
  name: "job-start-flow",
  says: "Start runs a Job at once, Draft holds it on its start card, and the card under the sheet starts a quick Job",
};
