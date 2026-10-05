// A stranded slot's Finding gets a third act: Pick up stashes the work, frees the
// slot and proposes a Job that continues from its branch. The proposal is a row
// on the Board, as a dispatched request is, and waits at its gate.

import { button, dialog, inside, role, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const ACT = (n: number, said: string) => inside(BAY(n), role("button", said, { exact: true }));
const FINDING = dialog("Finding");
const IN_FINDING = (name: string) => inside(FINDING, role("button", name, { exact: true }));

export const pickingUpASlot = walk("cleanup/slots", [
  { press: role("button", "Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { press: ACT(4, "Rescue"), say: "A Scout starts reading the stranded slot" },
  { press: ACT(4, "Finding"), say: "Its Finding opens in the side panel" },
  { look: inside(FINDING, role("heading", "Unfinished", { exact: true })), say: "Unfinished, with what is left" },
  { hover: IN_FINDING("Pick up"), say: "Pick up is beside Stash and Scrap" },
  { press: IN_FINDING("Pick up"), say: "Picked up at once, and the panel closes" },
  { look: inside(BAY(4), role("status")), say: "The slot is free, and the commit is named" },
  { press: button("Overview", { exact: true }), say: "Back to the Board" },
  { look: role("option", "Continue the work on branch fleet/an-old-try."), say: "The proposal is a row, titled with its request" },
]);
