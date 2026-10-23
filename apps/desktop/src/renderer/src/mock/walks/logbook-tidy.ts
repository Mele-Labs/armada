// Tidy now on the Logbook page: near-duplicate commands in use fold into the one confirmed most, the folded ones move
// to Archived, and the survivor keeps their words under Earlier as Merged. Over the `logbook-tidy` scenario.
// `docs/concepts/logbook.md`.

import { button, inside, place, region, role, text, walk } from "../walk";

const ENTRIES = role("listbox", "Entries");
const ENTRY = region("Entry");

const logbookTidy = walk("logbook-tidy", [
  { press: place("Logbook"), say: "In use holds the same two commands over and over in different words" },
  { hover: button("Tidy now"), say: "Tidy now folds near-duplicates together" },
  { press: button("Tidy now"), say: "Fleet is on it" },
  { look: text("Merged 8 entries"), say: "How many it merged, and nothing else" },
  { look: inside(ENTRIES, text(/Never the whole desktop suite/)), say: "In use is down to the entries that were kept" },
  { look: inside(ENTRY, text(/^Merged/)), say: "The open one keeps what it absorbed under Earlier, as Merged" },
  { press: role("tab", "Archived"), say: "The merged ones are under Archived, kept and searchable" },
]);

export { logbookTidy as "logbook-tidy" };
