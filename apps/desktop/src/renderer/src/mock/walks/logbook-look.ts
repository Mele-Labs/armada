// The Logbook and its Retros on the pages' common look: controls in the list's strip, panels to the window's foot, a 36px head,
// flat records, the one list card and ghost acts. Organize's cards keep their own head. Over the `logbook` scenario.

import { button, inside, place, region, role, walk } from "../walk";

const ENTRY = region("Entry");
const PROPOSAL = region("Proposal");
const ITEM = region("Retro item");

const logbookLook = walk("logbook", [
  { press: place("Logbook"), say: "No row above the panels. Both start at the top edge and run to the foot" },
  { look: button("State"), say: "State, Topic and Organize sit in the list's strip" },
  { look: inside(ENTRY, button(/^Edit/)), say: "The title in a 36px head, with the acts beside it. Hold Command to see their keys" },
  { look: inside(ENTRY, role("table", "Record")), say: "Record and Evidence are flat, with no column headings. The id is now the Record's first row" },
  { press: button("Organize"), say: "Organize keeps its menu" },
  { press: role("menuitem", "Find Armada bugs"), say: "Its cards stay as they were" },
  { look: inside(PROPOSAL, button("Report all")), say: "This head is still the taller one. Note it if it should take the 36px head" },
  { press: inside(PROPOSAL, button("Close")), say: "Back to the entry" },
  { press: button("State"), say: "Retros is still in the State menu" },
  { press: role("menuitem", "Retros"), say: "The same list card as the entries, with the same picked row and no column headings" },
  { look: button("Show"), say: "Open and Accepted are now a dropdown, beside State" },
  { look: button("Review"), say: "Review is in the strip" },
  { look: inside(ITEM, button("Start Job")), say: "The item's title and acts in the same 36px head. Who it got in the way of and where the fix lands are in the Record" },
]);

export { logbookLook as "logbook-look" };
