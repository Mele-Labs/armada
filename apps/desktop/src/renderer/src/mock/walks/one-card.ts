// One card on four lists: the same places for the state, the Job, the linked item and the time. A design mock: nothing here asserts a colour or a size. Over the `one-card` scenario.

import { place, region, role, walk } from "../walk";

const oneCard = walk("one-card", [
  { look: region("Jobs", { exact: true }), say: "Jobs on the new Cockpit. The console's strip, with its lamp and ground, on the one card. The time is plain text before the state glyph, which is last in the corner" },
  { press: place("Issues"), say: "Issues. The same card: the linked pull request, then the state of the Job working it. A worked issue has its bar and ground, and one that asks you also pulses. Titles keep two lines here" },
  { press: place("Reviews"), say: "Pull requests. Review and checks are the state, so they stand where a Job's state stands, and the checks are the bar on the edge" },
  { press: role("button", "List"), say: "The lists" },
  { press: role("menuitem", "My PRs"), say: "A pull request a Job holds takes the bot glyph beside its state" },
  { press: place("Logbook"), say: "Logbook. The same card in place of the table. The figure beside the state is how many Jobs confirmed it" },
]);

export { oneCard as "one-card" };
