// The Logbook page, driven by its keys: Enter opens an entry, E edits its text in place, L locks it, the arrows move, F
// forgets one, and H adds what is not handed out. Over the `logbook` scenario. Mock only: no Fleet route serves the
// Logbook yet. `docs/concepts/logbook.md`.

import { button, dialog, inside, place, region, role, walk } from "../walk";

const ENTRIES = role("listbox", "Entries");
const ENTRY = region("Entry");

export const logbook = walk("logbook", [
  { press: place("Logbook"), say: "Logbook: each repository, then Habits and This Mac, then Armada fixes" },
  { key: "Enter", on: ENTRIES, say: "Enter opens the first entry. It was tripped on, so it leads" },
  { key: "e", on: ENTRY, say: "E opens its text in place" },
  {
    type: "Call lease_slot before the first Edit or Write. Writes before it are refused.",
    into: role("textbox", "Entry text"),
    say: "Reworded. A draft stays here if you leave the page",
  },
  { press: inside(ENTRY, button("Save", { exact: true })), say: "Saved. The old text moves under Earlier" },
  { key: "l", on: ENTRY, say: "L locks it, so it never fades and Armada never rewrites it" },
  { key: "ArrowDown", on: ENTRIES, say: "↓ moves to the next entry" },
  { key: "f", on: ENTRIES, say: "F forgets it, after asking" },
  { press: inside(dialog("Forget this entry"), button("Forget", { exact: true })), say: "Forgotten. No later Job brings it back" },
  { key: "h", on: ENTRIES, say: "H adds what is not handed out, dimmed" },
]);
