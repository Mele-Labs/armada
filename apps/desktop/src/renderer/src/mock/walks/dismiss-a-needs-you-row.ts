// A walk or a pull request in a Job's Needs you panel is put away in one press, with no reason box. The row goes,
// and the agent is told it was dismissed. Over `dismiss-a-needs-you-row`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const needs = region("Needs you");
const row = (name: RegExp) => inside(needs, role("listitem", name));

const dismissARow = walk("dismiss-a-needs-you-row", [
  { press: deckRow(/Docs for the pairing screen/), say: "A Job waits on a page to look at and a pull request" },
  { press: inside(needs, button(/^Needs you/)), say: "They are in its Needs you panel" },
  { look: inside(row(/ready to look at/), button(/^Dismiss/)), say: "Under Walks, the page has Dismiss at its end, shown on hover or focus" },
  { look: inside(row(/won't merge/), button(/^Dismiss/)), say: "Under PRs, so does the pull request" },
  { press: inside(row(/ready to look at/), button(/^Dismiss/)), say: "One press puts the walk away, and the agent is told" },
  { press: inside(row(/won't merge/), button(/^Dismiss/)), say: "The same for the pull request" },
]);

export { dismissARow as "dismiss-a-needs-you-row" };
