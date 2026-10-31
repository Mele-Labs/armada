// Organize on the Logbook page: one menu with three looks through the entries, each ending in a short list with one act on all
// of it and a Dismiss on each card. Over the `logbook-organize` scenario. `docs/concepts/logbook.md`.

import { button, inside, place, region, role, text, walk } from "../walk";

const PROPOSAL = region("Proposal");

const logbookOrganize = walk("logbook-organize", [
  { press: place("Logbook"), say: "The Logbook" },
  { look: button("Organize"), say: "Organize replaces Tidy, with three looks through the entries" },
  { press: button("Organize"), say: "Open the menu, or press O" },
  { press: role("menuitem", "Merge duplicates"), say: "Near-duplicate entries, folded into the one confirmed most" },
  { look: inside(PROPOSAL, button("Apply all")), say: "Each card names the entry it keeps and the ones it replaces, with Apply for that card and Apply all for the lot" },
  { press: button("Organize"), say: "Next look" },
  { press: role("menuitem", "Find stale entries"), say: "Entries no recent Job has confirmed or tripped" },
  { look: inside(PROPOSAL, button("Archive all")), say: "Stale entries, each with when it was last confirmed" },
  { press: button("Organize"), say: "Last look" },
  { press: role("menuitem", "Find Armada bugs"), say: "Entries that work around a fault in Armada itself" },
  { look: inside(PROPOSAL, button("Report all")), say: "Each names the fault, so it can be fixed at the source" },
  { look: inside(PROPOSAL, text(/start_server/)), say: "A finding he does not want is dismissed with its own Dismiss" },
]);

export { logbookOrganize as "logbook-organize" };
