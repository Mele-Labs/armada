// The Ledger panel on the Cockpit: the branch and the bay are chips in its band, so the filter bar offers
// Pull requests and Artifacts and no Branches and slots, and what waits on the person is in the thread's Needs you
// band. Over the `ledger-in-cockpit` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const ledger = region("Ledger");
const bar = inside(ledger, role("group", "Narrow the ledger"));
const stage = region(/^Session: /);

const cockpit = walk("ledger-in-cockpit", [
  { press: deckRow("Queue session for later"), say: "A Session selected on the Cockpit" },
  { look: inside(ledger, role("img", /^Branch /)), say: "The branch is a chip in the Ledger's band" },
  { look: inside(ledger, role("img", /^Bay /)), say: "And the bay" },
  { look: bar, say: "The filter bar offers Search, Pull requests and Artifacts: Branches and slots would have nothing behind it" },
  { look: inside(ledger, region("Background")), say: "Background: two commands and a monitor running" },
  { press: inside(bar, button("Pull requests")), say: "Pull requests" },
  { look: inside(ledger, role("heading", "Pull requests")), say: "Only the pull requests are left" },
  { look: inside(stage, region("Needs you")), say: "What waits on him is in the thread's Needs you band, not the Ledger" },
]);

export { cockpit as "ledger-in-cockpit" };
