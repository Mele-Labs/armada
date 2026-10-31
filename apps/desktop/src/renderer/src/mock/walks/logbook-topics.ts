// The Logbook sorted by topic: chips above the list, and under each repository's header its entries grouped by topic. Over
// the `logbook-long` scenario. `docs/concepts/logbook.md`.

import { button, inside, place, region, role, text, walk } from "../walk";

const ENTRIES = role("grid", "Entries");
const ENTRY = region("Entry");
const TOPICS = role("toolbar", "Topics");

const logbookTopics = walk("logbook-long", [
  { press: place("Logbook"), say: "The Logbook" },
  { look: TOPICS, say: "All, then each topic that has entries. Armada sorts every entry; one it could not place is under Other" },
  { press: inside(TOPICS, button("Testing and checks")), say: "Press a topic to show only its entries" },
  { look: ENTRIES, say: "Only that topic's entries, in each repository" },
  { press: inside(TOPICS, button("All", { exact: true })), say: "All again" },
  { look: inside(ENTRIES, text("Slots and worktrees")), say: "Within a repository the rows sit under a small heading for their topic" },
  { key: "Enter", on: ENTRIES, say: "Open the first entry" },
  { look: inside(ENTRY, role("table", "Record")), say: "The record names the entry's Topic, after its Kind" },
]);

export { logbookTopics as "logbook-topics" };
