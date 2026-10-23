// What changed on the Logbook page: Retros folded in as Armada fixes below the entries, a name on each panel's header,
// Lock in place of Pin, and the tabs In use, New and Archived. Over the `logbook` scenario.
// `docs/concepts/logbook.md`.

import { button, inside, place, region, role, text, walk } from "../walk";

const ENTRY = region("Entry");

const logbookChanges = walk("logbook", [
  { press: place("Logbook"), say: "One Logbook tab. The retro items are Armada fixes, under the entries" },
  { hover: inside(role("group", "Habits"), text("Habits")), say: "Each panel's header says what it is" },
  { press: inside(ENTRY, button(/^Lock/)), say: "Lock, where Pin was" },
  { hover: inside(ENTRY, role("img", /^Locked/)), say: "Locked, and what it means" },
  { press: role("tab", "New"), say: "New holds the candidates" },
  { hover: role("tab", "New"), say: "Seen in one Job. A second Job puts it in use" },
  { press: role("tab", "Archived"), say: "Archived holds the faded entries" },
  { hover: role("tab", "Archived"), say: "Unused for 30 days. Recall still finds it" },
]);

export { logbookChanges as "logbook-changes" };
