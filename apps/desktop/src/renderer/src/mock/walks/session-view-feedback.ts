// Three notes on the Cockpit's one Session view. Opening a Session fits the whole Field in the foot, and opening another fits it again;
// the right column is the Job's story, Workflow, Blueprint, Field and Delivery (the Ledger and Status gave way to it); and the Merge line frames the open Session's own pull requests. Over `session-view-feedback`.

import { button, deckRow, inside, region, walk } from "../walk";

const queue = region("Merge queue");

export const sessionViewFeedback = walk("session-view-feedback", [
  { press: deckRow("Rotate the audit log"), say: "A Job is selected" },
  { look: region("Field"), say: "The Field shows the whole floor of the repository, centred, whatever the Cockpit's map was last zoomed to. A zoom or a pan stays until he opens another Job" },
  { look: region("Blueprint"), say: "The Blueprint is above the Field in the right column. There is no Ledger and no Status" },
  { look: inside(queue, button(/^#2388/)), say: "In the Merge line this Job's pull request has the accent's ring round it. The others are as they were" },
  { press: deckRow("Retry the slot lease"), say: "Another Job" },
  { look: inside(queue, button(/^#2390/)), say: "The ring has moved to this Job's pull request, and the Field is fitted to its own floor" },
]);
