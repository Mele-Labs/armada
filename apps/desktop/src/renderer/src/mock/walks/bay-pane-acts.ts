// A bay's pane on the flight deck: every act of the Worktree Slots panel on a held, free, closed and stranded bay, with the same
// confirms, and each bay's history of leases and checkouts. Over `bay-pane-acts`.

import { button, inside, region, role, text, walk } from "../walk";

const bays = region("Bays");
const bay = (name: string) => region(`Bay: ${name}`);
const acts = (name: string) => inside(bay(name), role("group", "Acts"));
const act = (name: string, label: string) => inside(acts(name), button(label, { exact: true }));
const history = (name: string) => inside(bay(name), region("History"));

const bayPaneActs = walk("bay-pane-acts", [
  { press: inside(bays, button(/^slot-10, armada, Held/)), say: "A bay a terminal's agent holds, from its row in Bays" },
  { look: acts("slot-10"), say: "The acts are in the pane: Release and Close slot, the same two the Worktree Slots panel offers for it" },
  { hover: act("slot-10", "Release"), say: "Each act says what it does in git's words, as it does there" },
  { hover: act("slot-10", "Close slot"), say: "Close slot" },
  { look: history("slot-10"), say: "Its history under the facts: the last day as blocks, then every holder newest first, each with its branch, its span and how long it held" },
  { look: inside(history("slot-10"), text("Checks --changed Bug")), say: "A holder from a terminal is named plainly. It is not a Job, so it opens nothing" },
  { hover: inside(history("slot-10"), role("img", "Agent", { exact: true })), say: "Agent" },
  { look: inside(history("slot-10"), text("fix/checks-changed")), say: "A checkout is a row of its own where it happened: the branch it left, the branch it took, the commit and the time" },
  { hover: inside(history("slot-10"), role("img", "Checkout")), say: "Checkout" },
  { look: inside(history("slot-10"), text("Today")), say: "Days are headed on the rail" },
  { look: inside(history("slot-10"), role("button", "Job Cache key")), say: "A Job the ship holds opens from its row" },
  { press: inside(bays, button(/^slot-5, armada, Free/)), say: "A free bay" },
  { look: acts("slot-5"), say: "Close slot and Remove slot" },
  { hover: act("slot-5", "Remove slot"), say: "Remove slot" },
  { look: history("slot-5"), say: "Who held it, and the branches checked out while they did" },
  { press: inside(bays, button(/^slot-2, pocket, Closed/)), say: "A closed bay" },
  { look: act("slot-2", "Reopen slot"), say: "Reopen slot takes the place of Close slot" },
  { look: history("slot-2"), say: "A lease that ended before it was closed" },
  { press: inside(bays, button(/^slot-6, armada, Stranded/)), say: "A stranded bay" },
  { look: act("slot-6", "Close slot"), say: "Close slot, with the rescue acts under it" },
  { look: inside(bay("slot-6"), button("Pick up", { exact: true })), say: "Pick up, Stash and Scrap, from the Scout's Finding, as in the panel" },
  { hover: inside(bay("slot-6"), button("Pick up", { exact: true })), say: "Hover says what Pick up does in git's words" },
  { press: inside(bay("slot-6"), button("Scrap", { exact: true })), say: "Scrap asks first" },
  { look: inside(bay("slot-6"), role("group", "Scrap slot-6", { exact: true })), say: "It names the uncommitted files and the unpushed commits, and nothing is sent until it is confirmed" },
  { press: inside(bay("slot-6"), button("Cancel", { exact: true })), say: "Cancel puts it away" },
  { press: inside(bays, button(/^slot-10, armada, Held/)), say: "Back to the held bay" },
  { press: act("slot-10", "Release"), say: "Release asks first" },
  { look: inside(bay("slot-10"), role("group", "Release slot-10", { exact: true })), say: "It names the holder and its branch, and says the process keeps running" },
  { press: inside(inside(bay("slot-10"), role("group", "Release slot-10", { exact: true })), button("Release", { exact: true })), say: "Sent" },
  { look: inside(bay("slot-10"), role("status")), say: "The bay is free, with the receipt in the pane" },
  { look: act("slot-10", "Close slot"), say: "With the holder gone, Remove slot is offered beside Close slot" },
]);

export { bayPaneActs as "bay-pane-acts" };
