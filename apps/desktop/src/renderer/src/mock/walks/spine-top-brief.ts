// The top of the Job's story: the header band with the workflow's name and picker, the Brief as the first beat, then one zone on the spine for each of the workflow's steps. A
// finished step is folded to a check and opens to what was made during it, the current step is open, and the last step holds the landing. Over `spine-top-brief`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const brief = region("Brief", { exact: true });
const approvePlan = region("Approve plan", { exact: true });
const deliver = region("Deliver", { exact: true });

export const spineTopBrief = walk("spine-top-brief", [
  { press: deckRow("Issues tab beside Reviews"), say: "A Job at its fourth step" },
  { look: button(/^Workflow Product/), say: "The header holds the workflow's name, its picker and the mark for the proposer's pick. The steps are the zones below" },
  { look: brief, say: "The Brief is the first beat on the spine, shown as one line" },
  { press: inside(approvePlan, button(/Approve plan/)), say: "A finished step is a check on the line, folded. Press it to open what was made during it" },
  { look: inside(approvePlan, role("list", "Done")), say: "The plan's finished groups and their pull requests, under the step they were made in" },
  { look: region("Walk", { exact: true }), say: "The current step is open, with what is happening in it" },
  { look: inside(deliver, role("list", "Ahead")), say: "The last step holds the landing, still to come" },
]);
