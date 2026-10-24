// Tidy now on the Logbook page: Fleet runs the pass in the background, so the page says Tidying until it ends. Near-duplicate
// commands in use then fold into the one confirmed most, the folded ones move to Archived, and the survivor keeps their words
// under Earlier as Merged. Leaving the page mid-pass and coming back still shows the result. Over the `logbook-tidy` scenario.
// `docs/concepts/logbook.md`.

import { button, inside, place, region, role, text, walk } from "../walk";

const ENTRIES = role("listbox", "Entries");
const ENTRY = region("Entry");

const logbookTidy = walk("logbook-tidy", [
  { press: place("Logbook"), say: "In use holds the same two commands over and over in different words" },
  { hover: button("Tidy now"), say: "Tidy now folds near-duplicates together" },
  { press: button("Tidy now"), say: "Fleet runs the pass in the background" },
  { look: text("Tidying"), say: "The page says Tidying until the pass ends" },
  { press: place("System"), say: "The pass does not need the page, so he can leave" },
  { later: place("Logbook"), say: "It ends while he is away" },
  { press: place("Logbook"), say: "Back on the Logbook" },
  { look: text("Merged 8 entries"), say: "How many it merged, and nothing else" },
  { look: inside(ENTRIES, text(/Never the whole desktop suite/)), say: "In use is down to the entries that were kept" },
  { look: inside(ENTRY, text(/^Merged/)), say: "The open one keeps what it absorbed under Earlier, as Merged" },
  { press: role("tab", "Archived"), say: "The merged ones are under Archived, kept and searchable" },
]);

export { logbookTidy as "logbook-tidy" };
