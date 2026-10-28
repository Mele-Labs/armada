// The Logbook reworked: one line an entry, the open one on the right with its text and acts held at the top and its
// evidence one Job a line, scrolling on its own. The retro items on their own Retros tab, laid out the same way.
// Over the `logbook-long` scenario. `docs/concepts/logbook.md`.

import { button, inside, place, region, role, tab, text, walk } from "../walk";

const ENTRIES = role("listbox", "Entries");
const ENTRY = region("Entry");
const RETROS = role("listbox", "Retros");
const ITEM = region("Retro item");

const logbookRework = walk("logbook-long", [
  { press: place("Logbook"), say: "One line an entry. The whole text is on the right" },
  { press: inside(ENTRIES, role("option", /^Putting HEAD's version/)), say: "Open the entry confirmed by 15 Jobs" },
  { look: inside(ENTRY, button(/^Edit/)), say: "The text and Edit, Lock and Forget stay at the top" },
  { look: inside(ENTRY, text("Confirmed by")), say: "The facts on one line" },
  { look: inside(ENTRY, text("Evidence")), say: "Evidence: one line a Job, its cites beside it, newest Job first" },
  { hover: inside(ENTRY, text("refusal:1")), say: "A cite says what it showed and when" },
  { look: inside(ENTRY, text("Annotate sketches, documents and images for jobs")), say: "The evidence scrolls on its own, so every row is reached" },
  { key: "ArrowDown", on: ENTRIES, say: "↓ moves to the next entry" },
  { press: tab("Retros"), say: "The retro items have their own tab" },
  { look: RETROS, say: "One line an item, with whose way it got in and where the fix lands" },
  { key: "ArrowDown", on: RETROS, say: "↓ opens the next item on the right" },
  { look: ITEM, say: "The item whole, with its answers" },
  { press: tab("Open"), say: "Open and Accepted, and Review, as before" },
]);

export { logbookRework as "logbook-rework" };
