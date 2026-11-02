// One card on four lists: the same places for the state, the Job, the linked item and the time. A design mock: nothing here asserts a colour or a size. Over the `one-card` scenario.

import { place, region, role, walk } from "../walk";

const oneCard = walk("one-card", [
  { look: region("Jobs", { exact: true }), say: "Jobs. One line in the Cockpit's own size and spacing, with hairlines and no fill. The time is plain text before the state glyph, which is last in the corner" },
  { press: place("Issues"), say: "Issues. The same card: the linked pull request, then the state of the Job working it. Titles keep two lines here. The number is under the title and the time at the foot" },
  { press: place("Reviews"), say: "Pull requests. Review and checks are the state, so they stand where a Job's state stands" },
  { press: role("button", "List"), say: "The lists" },
  { press: role("menuitem", "My PRs"), say: "A pull request a Job holds takes the bot glyph beside its state" },
  { press: place("Logbook"), say: "Logbook. The same card in place of the table. The figure beside the state is how many Jobs confirmed it" },
]);

// The card with colour's bar on it: what an issue that asks draws, and the checks' bar on a pull request.
const oneCardColour = walk("one-card", [
  { press: place("Issues"), say: "Issues" },
  { look: region("Issues", { exact: true }), say: "An issue a Job works has its bar and its ground. One that asks you also pulses, over the bar" },
  { press: place("Reviews"), say: "Reviews" },
  { look: region("Pull requests", { exact: true }), say: "A pull request's checks are the bar on its edge, on the same card" },
]);

export { oneCard as "one-card", oneCardColour as "one-card-colour" };
