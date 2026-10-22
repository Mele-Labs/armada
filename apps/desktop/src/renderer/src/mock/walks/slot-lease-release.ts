// The two slot rows in a Session's thread: Slot leased where the first write took a slot, Slot released where it was given back. Over `sessions`.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, dialog, inside, role, region, tab, walk } from "../walk";

const { thread } = kit(false);
const request = inside(dialog("Dispatch"), role("textbox", "Request"));

const slotLeaseRelease = walk("sessions", [
  toSessions,
  { press: inside(role("region", "Dispatch controls"), button("Dispatch")), say: "A new Job starts from Dispatch" },
  { press: inside(dialog("Dispatch"), tab("Job")), say: "Job" },
  { type: "Fix the flaky store test and have subagents read the CI history.", into: request, say: "A raw request" },
  { press: role("radio", "Repository armada"), say: "In armada" },
  { key: "Enter", on: request, say: "The Job opens and the agent reads first" },
  { look: region("Slot leased"), say: "Its first write took a slot and cut a branch: Slot leased, with the slot and the branch" },
  { later: thread, say: "The agent carries on" },
  { later: thread, say: "It opens a pull request" },
  { later: thread, say: "Another Job writes to this one, and it replies" },
  { look: region("Slot released"), say: "Slot released: the slot given back, with the branch it was on" },
]);

export { slotLeaseRelease as "slot-lease-release" };
