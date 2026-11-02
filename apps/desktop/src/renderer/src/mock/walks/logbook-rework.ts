// The Logbook as a list and the open entry: on the left one list of entries, a card each coded by kind, under a strip of
// State, Topic and Organize and section rows that stay as it scrolls. On the right the open entry under its own head, its title
// and acts held at the top, and its record and evidence as flat rows under a legend. Retros, chosen in State, drawn the same way.
// Over the `logbook-long` scenario. `docs/concepts/logbook.md`.

import { button, inside, place, region, role, text, walk } from "../walk";

const ENTRIES = role("grid", "Entries");
const ENTRY = region("Entry");
const RETROS = role("grid", "Retros");
const ITEM = region("Retro item");

const logbookRework = walk("logbook-long", [
  {
    press: place("Logbook"),
    say: "The log: kind, title, flags, Jobs that confirmed it, last confirmed",
  },
  {
    look: inside(ENTRIES, text("Habits")),
    say: "A section row for each repository, Habits and This Mac. It stays as you scroll",
  },
  {
    press: inside(ENTRIES, role("row", /Copy to \/tmp before restoring from HEAD/)),
    say: "Open the entry confirmed by 15 Jobs",
  },
  {
    look: inside(ENTRY, role("heading", /Copy to \/tmp before restoring from HEAD/)),
    say: "The entry's own head: its title on one line, with its flags and acts at the right",
  },
  {
    look: inside(ENTRY, button(/^Edit/)),
    say: "Edit, Lock and Forget in the head. The head stays put as the entry scrolls",
  },
  {
    look: inside(ENTRY, text("lb-head-backup")),
    say: "The record is flat rows, and its first is the id",
  },
  {
    look: inside(ENTRY, role("table", "Evidence")),
    say: "Evidence: a row a Job, its cites, newest first",
  },
  {
    hover: inside(ENTRY, text("refusal:1")),
    say: "A cite says what it showed and when",
  },
  {
    key: "ArrowDown",
    on: ENTRIES,
    say: "↓ moves to the next entry",
  },
  {
    press: inside(ENTRIES, role("row", /Restart through the preview, never onto main/)),
    say: "An entry with a long rule",
  },
  {
    look: inside(ENTRY, text(/^A restart the owner must run/)),
    say: "The rule scrolls with the record and evidence under the title, however long it runs",
  },
  {
    press: button("State"),
    say: "The retro items are in the State menu",
  },
  {
    press: role("menuitem", "Retros"),
    say: "Retros",
  },
  {
    look: RETROS,
    say: "The same list: route, title, Job and age on each line",
  },
  {
    key: "ArrowDown",
    on: RETROS,
    say: "↓ opens the next item",
  },
  {
    look: ITEM,
    say: "The item under its own head, its answers held at the top and its record below",
  },
]);

export { logbookRework as "logbook-rework" };
