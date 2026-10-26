// What the Ledger held, on the Cockpit: a Session has no Ledger panel. Its pull requests, pages, files and the commands it ran are the Artifacts
// layer, a key from Flight control, and what waits on the person is in the Needs you panel on the composer. Over the
// `ledger-in-cockpit` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const artifacts = region("Artifacts", { exact: true });
const stage = region(/^Job: /);

const cockpit = walk("ledger-in-cockpit", [
  { press: deckRow("Queue session for later"), say: "A Job selected on the Cockpit has no Ledger. The Blueprint and the Field share its column" },
  { press: inside(region("Flight controls"), button("Artifacts")), say: "Artifacts (8) holds what the Ledger did, over the thread" },
  { look: inside(artifacts, region("Commands", { exact: true })), say: "Commands: two commands and a monitor running" },
  { look: inside(artifacts, region("Pull requests", { exact: true })), say: "Pull requests, with their state and checks" },
  { type: "walk", into: inside(artifacts, role("searchbox", "Search artifacts")), say: "Typing narrows every group at once" },
  { press: inside(artifacts, button("Back")), say: "Back returns to the thread where it was" },
  { look: inside(stage, region("Needs you")), say: "What waits on him is in the Needs you panel on the composer, not in Artifacts" },
]);

export { cockpit as "ledger-in-cockpit" };
