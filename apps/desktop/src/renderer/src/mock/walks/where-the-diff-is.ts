// The palette's greyed Open the diff row names where the act is, now the
// story's chapters are gone, and Open the log is gone with the log. The owner's
// decisions, "Reword them" and, on 2 Oct 2026, "There is not a single log for
// a job"; `palette.test.tsx` holds both claims.

import { button, role, text, walk } from "../walk";

const query = role("combobox", "Search actions");

export const whereTheDiffIs = walk("job/running", [
  { press: button("Search, or describe work"), say: "The palette, as ⌘K opens it" },
  { type: "diff", into: query, say: "Search for the diff" },
  { look: role("option", /^Open the diff/), say: "Greyed, and it says where: f on a job's Overview" },
  { type: "log", into: query, say: "Search for the log" },
  { look: text("Nothing matches “log”."), say: "No Open the log: a job has no single log" },
]);
