// A bay's side panel: every act of the Worktree Slots panel on a held, free, closed and stranded bay, with the same
// confirms, and each bay's history of leases. Over `bay-pane-acts`.

import { button, inside, region, role, text, walk } from "../walk";

const bays = region("Bays");
const bay = (name: string) => role("dialog", `Bay ${name}`);
const acts = (name: string) => inside(bay(name), role("group", "Acts"));
const act = (name: string, label: string) => inside(acts(name), button(label, { exact: true }));
const history = (name: string) => inside(bay(name), role("list", "History"));

const bayPaneActs = walk("bay-pane-acts", [
  { press: inside(bays, button(/^slot-10, armada, Held/)), say: "A bay a terminal's agent holds, from its row in Bays" },
  { look: acts("slot-10"), say: "The acts are in the panel: Release and Close slot, the same two the Worktree Slots panel offers for it" },
  { hover: act("slot-10", "Release"), say: "Each act says what it does in git's words, as it does there" },
  { hover: act("slot-10", "Close slot"), say: "Close slot" },
  { key: "]", on: bay("slot-10"), say: "] is History" },
  { look: history("slot-10"), say: "Every holder newest first, each with its branch and how long it held" },
  { look: inside(history("slot-10"), text("Checks --changed Bug")), say: "A holder from a terminal is named plainly. It is not a Job, so it opens nothing" },
  { look: inside(history("slot-10"), text("armada/session-627d8d8f")), say: "Its branch" },
  { key: "Escape", on: bay("slot-10"), say: "Esc" },
  { press: inside(bays, button(/^slot-5, armada, Free/)), say: "A free bay" },
  { look: acts("slot-5"), say: "Close slot and Remove slot" },
  { hover: act("slot-5", "Remove slot"), say: "Remove slot" },
  { key: "]", on: bay("slot-5"), say: "]" },
  { look: history("slot-5"), say: "Who held it" },
  { key: "Escape", on: bay("slot-5"), say: "Esc" },
  { press: inside(bays, button(/^slot-2, pocket, Closed/)), say: "A closed bay" },
  { look: act("slot-2", "Reopen slot"), say: "Reopen slot takes the place of Close slot" },
  { key: "]", on: bay("slot-2"), say: "]" },
  { look: history("slot-2"), say: "A lease that ended before it was closed" },
  { key: "Escape", on: bay("slot-2"), say: "Esc" },
  { press: inside(bays, button(/^slot-6, armada, Stranded/)), say: "A stranded bay" },
  { look: act("slot-6", "Close slot"), say: "Close slot, with the rescue acts under it" },
  { look: inside(bay("slot-6"), button("Pick up", { exact: true })), say: "Pick up, Stash and Scrap, from the Scout's Finding, as in the panel" },
  { hover: inside(bay("slot-6"), button("Pick up", { exact: true })), say: "Hover says what Pick up does in git's words" },
  { press: inside(bay("slot-6"), button("Scrap", { exact: true })), say: "Scrap asks first" },
  { look: inside(bay("slot-6"), role("group", "Scrap slot-6", { exact: true })), say: "It names the uncommitted files and the unpushed commits, and nothing is sent until it is confirmed" },
  { press: inside(bay("slot-6"), button("Cancel", { exact: true })), say: "Cancel puts it away" },
  { key: "Escape", on: bay("slot-6"), say: "Esc" },
  { press: inside(bays, button(/^slot-10, armada, Held/)), say: "Back to the held bay" },
  { press: act("slot-10", "Release"), say: "Release asks first" },
  { look: inside(bay("slot-10"), role("group", "Release slot-10", { exact: true })), say: "It names the holder and its branch, and says the process keeps running" },
  { press: inside(inside(bay("slot-10"), role("group", "Release slot-10", { exact: true })), button("Release", { exact: true })), say: "Sent" },
  { look: inside(bay("slot-10"), role("status")), say: "The bay is free, with the receipt in the panel" },
  { look: act("slot-10", "Close slot"), say: "With the holder gone, Remove slot is offered beside Close slot" },
]);

export { bayPaneActs as "bay-pane-acts" };
