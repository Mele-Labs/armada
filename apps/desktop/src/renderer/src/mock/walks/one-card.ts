// One card on four lists: the same places for the state, the Job, the linked item and the time. A design mock: nothing here asserts a colour or a size. Over the `one-card` scenario.

import { place, region, role, walk } from "../walk";

const oneCard = walk("one-card", [
  { look: region("Jobs", { exact: true }), say: "Jobs. Title first, the state glyph last in the top right corner, the number under the title, the time plain at the end of that line" },
  { press: place("Issues"), say: "Issues. The same card: the linked pull request, then the state of the Job working it, the number under the title, the time in the same place" },
  { press: place("Reviews"), say: "Pull requests. Review and checks are the state, so they stand where a Job's state stands" },
  { press: role("button", "List"), say: "The lists" },
  { press: role("menuitem", "My PRs"), say: "A pull request a Job holds takes the bot glyph beside its state" },
  { press: place("Logbook"), say: "Logbook. The same card in place of the table. The figure beside the state is how many Jobs confirmed it" },
]);

export { oneCard as "one-card" };
