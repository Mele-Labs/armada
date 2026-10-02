// The palette's greyed Open the log and Open the diff rows name where each act
// is, now the story's chapters are gone. The owner's decision, "Reword them";
// `palette.test.tsx` holds the claim.

import { button, role, walk } from "../walk";

const query = role("combobox", "Search actions");

export const whereTheLogAndTheDiffAre = walk("job/running", [
  { press: button("Search jobs"), say: "The palette, as ⌘K opens it" },
  { type: "log", into: query, say: "Search for the log" },
  { look: role("option", /^Open the log/), say: "Greyed, and it says where: L on a job's Overview" },
  { type: "diff", into: query, say: "Search for the diff" },
  { look: role("option", /^Open the diff/), say: "Greyed, and it says where: f on a job's Overview" },
]);
