// What changed on the Logbook page: Retros folded in as Armada fixes below the entries, a name on each panel's header,
// Lock in place of Pin, and one Show all switch in place of the three tabs. Over the `logbook` scenario.
// `docs/concepts/logbook.md`.

import { button, inside, place, region, role, text, walk } from "../walk";

const ENTRY = region("Entry");
const ENTRIES = role("listbox", "Entries");

const logbookChanges = walk("logbook", [
  { press: place("Logbook"), say: "One Logbook tab. The retro items are Armada fixes, under the entries" },
  { hover: inside(role("group", "Habits"), text("Habits")), say: "Each panel's header says what it is" },
  { press: inside(ENTRY, button(/^Lock/)), say: "Lock, where Pin was" },
  { hover: inside(ENTRY, role("img", /^Locked/)), say: "Locked, and what it means" },
  { press: role("switch", "Show all"), say: "Show all adds candidates and faded entries, dimmed" },
  { hover: inside(ENTRIES, role("img", /^Candidate/)), say: "Each one names its state" },
]);

export { logbookChanges as "logbook-changes" };
