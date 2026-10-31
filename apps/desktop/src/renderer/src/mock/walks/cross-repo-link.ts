// A Job that started one in its own repository and one in another: the first hangs under it with the line, the second stands in
// its own repository's group with the link glyph alone. Over the `cross-repo-link` scenario.

import { deckRow, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });

export const crossRepoLink = walk("cross-repo-link", [
  { look: deckRow("Trim the retry loop"), say: "Plan the checkout rework started this Job in armada, so it hangs under it with the line" },
  { look: deckRow("Round the cart totals"), say: "It also started this one in storefront. It stays in storefront's group, with no indent and no line" },
  { hover: inside(list, role("img", "Linked Job, started by")), say: "The link glyph names the Job that started it and that Job's repository" },
]);
