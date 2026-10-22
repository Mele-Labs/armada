// From a worktree to the Job that holds it, and back to the worktree from the Job. Starts on Cleanup, where a slot
// a Job holds names it, opens the Job, finds the slot in its Bays layer, and ends on the Cockpit's Jobs list,
// searched by a pull request number. Over the `sessions` scenario. Told twice, wide and below the breakpoint.

import { kit, NARROW } from "../sessions/walk-kit";
import { button, inside, role, region, viaPalette, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { close, key } = kit(narrow);
  const bays = region("Bays", { exact: true });
  const bay = role("dialog", "Bay slot-5");
  const owner = role("dialog", "Owned by Release notes script");
  const sessions = region("Jobs", { exact: true });
  return [
    ...viaPalette("Worktree Slots", "Cleanup holds the worktree slots"),
    { look: role("button", "slot-5", { exact: true }), say: "Slot 5 is held by a Job, and says so" },
    { hover: button("Branch rel/notes-script"), say: "Its branch chip names its owner" },
    { press: button("Branch rel/notes-script"), say: "A press keeps the card up" },
    { look: owner, say: "Title, state, slot, pull requests and last turn" },
    { press: inside(owner, button("Open Job")), say: "It opens the Job" },
    { press: key("Bays"), say: "Bays: the slot the Job holds" },
    { press: inside(bays, button("Show slot-5 in Bays")), say: "A press opens the slot in the side panel" },
    { look: bay, say: "What the worktree holds, over the Job" },
    { press: close(bay), say: "Back to the Job" },
    { press: inside(sessions, button(/^Filter/)), say: "The Cockpit's list searches every Job" },
    { type: "#1849", into: role("searchbox", "Search Jobs"), say: "A pull request number" },
    { look: inside(sessions, button("Store migration spike")), say: "It finds the Job that owns it" },
  ];
}

const wide = walk("sessions", steps(false));
const narrow = walk("sessions", steps(true), NARROW);

export { wide as "worktree-to-sessions", narrow as "worktree-to-sessions-narrow" };
