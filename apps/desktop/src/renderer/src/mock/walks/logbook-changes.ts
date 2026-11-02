// What changed on the Logbook page: Retros folded in as a choice in the Logbook's State menu, a section row naming each group of entries,
// Lock in place of Pin, and the states In use, New and Archived, also in State. Over the `logbook` scenario.
// `docs/concepts/logbook.md`.

import { button, inside, place, region, role, text, walk } from "../walk";

const ENTRY = region("Entry");

const logbookChanges = walk("logbook", [
  { press: place("Logbook"), say: "One Logbook. The retro items are Retros in the State menu, beside In use, New and Archived" },
  { hover: inside(role("rowgroup", "Habits"), text("Habits")), say: "Each section row says what it is" },
  { press: inside(ENTRY, button(/^Lock/)), say: "Lock, where Pin was" },
  { hover: inside(ENTRY, role("img", /^Locked/)), say: "Locked, and what it means" },
  { press: button("State"), say: "State holds the states" },
  { hover: role("menuitem", "New"), say: "Seen in one Job. A second Job puts it in use" },
  { press: role("menuitem", "New"), say: "New holds the candidates" },
  { press: button("State"), say: "State again" },
  { hover: role("menuitem", "Archived"), say: "Unused for 30 days. Recall still finds it" },
  { press: role("menuitem", "Archived"), say: "Archived holds the faded entries" },
]);

export { logbookChanges as "logbook-changes" };
