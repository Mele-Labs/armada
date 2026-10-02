// The palette's greyed Expand and collapse row is gone, with the `h` `l` and
// `j` `k` keys that worked only inside the activity log. The owner's decision
// on 2 Oct 2026, "Remove them"; `palette.test.tsx` holds the claim.

import { button, role, text, walk } from "../walk";

const query = role("combobox", "Search actions");

export const nothingToDisclose = walk("job/running", [
  { press: button("Search jobs"), say: "The palette, as ⌘K opens it" },
  { type: "disclose", into: query, say: "Search for disclose" },
  { look: text("Nothing matches “disclose”."), say: "Nothing matches" },
  { type: "expand and", into: query, say: "Search for the act's own verb" },
  { look: text("Nothing matches “expand and”."), say: "No Expand and collapse: the act is gone, with its keys" },
]);
