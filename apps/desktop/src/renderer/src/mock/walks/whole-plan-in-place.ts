// Whole plan on an approved plan opens every group where it stands: finished groups list their tasks in Done, the ones being worked stay in Now, and
// the ones to come open in Open. Nothing finished is drawn in Now. Over `job-story-full`.

import { button, deckRow, inside, region, walk } from "../walk";

const now = region("Now", { exact: true });

export const wholePlanInPlace = walk("job-story-full", [
  { press: deckRow("Issues tab beside Reviews"), say: "A Job with finished groups, groups being worked and groups to come" },
  { press: inside(now, button("Whole plan", { exact: true })), say: "Whole plan" },
  { look: region("Done", { exact: true }), say: "Finished groups stay in Done, each with its tasks listed" },
  { look: now, say: "Now keeps only the groups being worked" },
  { look: region("Open", { exact: true }), say: "The groups to come open in Open" },
  { press: inside(now, button("Whole plan", { exact: true })), say: "Press again to fold them back to their bars" },
]);
