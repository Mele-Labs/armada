// An Epic Job on the approval canvas past its gate (prototype, 4 Oct 2026):
// the Jobs its plan dispatched stand where it dispatched them, one row per
// depth of the wave, each marked with its own status and opening that Job,
// and the run goes on to the roll-up. The wave graph is not drawn beside it.

import { card, inside, region, role, text, walk } from "../walk";

const job = (title: string, status?: string) =>
  role("button", new RegExp(`^${title}, ${status ?? ""}`));

export const anEpicJobOnItsCanvas = walk("epic/wave", [
  { look: region("This Job's run"), say: "An Epic Job's run, in its three lanes" },
  { look: card("Plan the wave"), say: "Its plan split the work" },
  { look: job("Refuse an unknown code at the seam", "done"), say: "into Jobs: the seam first, done" },
  { look: job("Name the fault in the toast", "done"), say: "two waiting on it, side by side" },
  { hover: inside(job("Carry the code into the journal"), text("awaiting review")), say: "one waiting on you" },
  { hover: inside(job("Say which half refused"), text("needs you")), say: "one escalated" },
  { look: job("Drop the second error shape", "running"), say: "the last, waiting on both, running" },
  { look: card("Roll up the wave"), say: "then the roll-up, in delivery" },
  { press: job("Say which half refused"), say: "A Job opens itself" },
  { look: role("heading", "Say which half refused"), say: "Say which half refused" },
]);
