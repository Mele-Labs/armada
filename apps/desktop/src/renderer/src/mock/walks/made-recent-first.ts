// Done keeps the five most recent things a Job made, and Show earlier opens the rest. A walk shown twice stands once, and the pictures a test run or
// a look in the browser left behind are not listed. Over `job-story-full`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const completed = region("Done", { exact: true });

export const madeRecentFirst = walk("job-story-full", [
  { press: deckRow("Issues tab beside Reviews"), say: "A Job that has made a lot" },
  { press: role("radio", "By step", { exact: true }), say: "Done by step" },
  { look: completed, say: "Under its last group, Done shows the five most recent things it made" },
  { press: inside(completed, button("Show earlier", { exact: true })), say: "Show earlier opens the rest" },
  { look: completed, say: "A walk shown twice stands once, where it was shown last. Test screenshots are not listed" },
]);
