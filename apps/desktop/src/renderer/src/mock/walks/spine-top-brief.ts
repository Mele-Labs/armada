// The top of the Job's story: the header band with the workflow and its steps, then the Brief as the first beat above Completed, pressed open to the whole text and the settings the
// Job started with. Over `spine-top-brief`; `?take=labelled` and `?take=glyphs` draw the steps two ways.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const brief = region("Brief", { exact: true });

export const spineTopBrief = walk("spine-top-brief", [
  { press: deckRow("Issues tab beside Reviews"), say: "A Job at its fourth step" },
  { look: button(/^Workflow Product/), say: "The workflow's name at reading size, its picker, and the mark for the proposer's pick" },
  { look: role("list", "Workflow"), say: "The steps under it: the finished ones checked, the current one lit, the ones to come dim" },
  { look: brief, say: "The brief is the first beat on the spine, above Completed, shown as one line" },
  { press: inside(brief, button("Brief")), say: "Press it to open the brief in place" },
  { look: inside(brief, text("Started by")), say: "The whole text, then how the Job started. A press on Started by opens that Job" },
]);
