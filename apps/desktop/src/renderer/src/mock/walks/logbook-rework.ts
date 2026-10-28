// The Logbook as a mission log: on the left one table of entries, a titled line each coded by kind, under a column head
// and section rows that stay as it scrolls. On the right the open entry under its own head, its words and acts held at
// the top, and its record and evidence as tables whose heads stay. The retro items on their own tab, drawn the same way.
// Over the `logbook-long` scenario. `docs/concepts/logbook.md`.

import { button, inside, place, region, role, tab, text, walk } from "../walk";

const ENTRIES = role("grid", "Entries");
const ENTRY = region("Entry");
const RETROS = role("grid", "Retros");
const ITEM = region("Retro item");

const logbookRework = walk("logbook-long", [
  { press: place("Logbook"), say: "The log: kind, title, flags, Jobs that confirmed it, last confirmed" },
  { look: inside(ENTRIES, text("Habits")), say: "A section row for each repository, Habits and This Mac. It stays as you scroll" },
  { press: inside(ENTRIES, role("row", /Copy to \/tmp before restoring from HEAD/)), say: "Open the entry confirmed by 15 Jobs" },
  { look: inside(ENTRY, text("lb-head-backup")), say: "The entry's own head: kind, id and state" },
  { look: inside(ENTRY, button(/^Edit/)), say: "The title, the rule and Edit, Lock and Forget stay at the top" },
  { look: inside(ENTRY, role("table", "Record")), say: "The record as a table" },
  { look: inside(ENTRY, role("table", "Evidence")), say: "Evidence: a row a Job, its cites, newest first. The head stays as it scrolls" },
  { hover: inside(ENTRY, text("refusal:1")), say: "A cite says what it showed and when" },
  { key: "ArrowDown", on: ENTRIES, say: "↓ moves to the next entry" },
  { press: tab("Retros"), say: "The retro items have their own tab" },
  { look: RETROS, say: "The same log: route, item, Job, written" },
  { key: "ArrowDown", on: RETROS, say: "↓ opens the next item" },
  { look: ITEM, say: "The item under its own head, its answers held at the top and its record below" },
]);

export { logbookRework as "logbook-rework" };
