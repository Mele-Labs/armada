// The same ask, and the new Job does not come up: the Job is as it was, keeps its row, and a band on its composer says why. No second call is raised.
// Retry (r) runs the hand off again and it fails the same way; Dispatch job (d) hands the failure to a new Job as the failure toast does. Over `job-handoff-fails`.

import { HANDOFF_YES } from "@armada/protocol";

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const list = region("Jobs", { exact: true });
const band = region("Failed to hand off to new job");

export const jobHandoffFails = walk("job-handoff-fails", [
  { press: deckRow("Retry the slot lease"), say: "The same ask" },
  { key: "1", on: inside(region("Needs you"), role("radio", HANDOFF_YES, { exact: true })), say: "1 picks yes" },
  { press: inside(region("Needs you"), button("Answer")), say: "Answer sends it" },
  { look: inside(list, role("img", "Handing off")), say: "The new Job is coming up" },
  { look: inside(region(/^Job: /), button("Failed to hand off to new job")), say: "The hand off failed: a band on the composer, as Needs you is. The Job is as it was, on its row" },
  { look: inside(band, text("Slot lease refused: all 4 bays are held")), say: "The reason Fleet recorded, and two ways on" },
  { press: inside(band, button("Retry")), say: "Retry runs the hand off again" },
  { look: inside(list, role("img", "Handing off")), say: "The new Job is coming up again" },
  { look: inside(band, text("Slot lease refused: all 4 bays are held")), say: "It fails the same way, and the band is back" },
  { press: inside(band, button("Dispatch job")), say: "Dispatch job sends the failure to a new Job, as the failure toast does" },
]);
