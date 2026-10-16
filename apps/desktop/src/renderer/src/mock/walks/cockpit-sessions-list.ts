// What the Sessions page held, found in the Cockpit's list: no Sessions tab, a search, Ended (and Quiet and All) beside Active, fifty
// rows at a time, an ended Session opened on the stage with its Retro refused in the open, and a Session reached from ⌘K selected there.
// Over the `cockpit-sessions-list` scenario.

import { deckRow, inside, region, role, tab, text, walk } from "../walk";

const places = role("navigation", "Places");
const list = region("Jobs and Sessions");
const search = inside(list, role("searchbox", "Search Jobs and Sessions"));

export const cockpitSessionsList = walk("cockpit-sessions-list", [
  { look: places, say: "There is no Sessions tab. A Session is viewed on the Cockpit, and its list is the one on the left. ⌘3 and a link to a Session arrive here" },
  { press: inside(list, tab("Ended")), say: "Active, Quiet, Ended and All. Ended lists the Sessions that ended in the last week, newest first. Quiet lists the ones Fleet cannot reach, and All adds both to the live ones" },
  { look: deckRow("Trim the retry loop 49"), say: "Fifty rows are drawn at first, and this is the last of them" },
  { look: deckRow("Draft the release notes 63"), say: "Fifty more are drawn each time the end of the list comes into view. j and k walk past a page and draw what they need" },
  { type: "1702", into: search, say: "The search finds a Session by its title, id, pull request, branch, slot or Job. An ended Session older than a week is listed only by a search" },
  { key: "ArrowDown", on: search, say: "↓ or Enter goes to the first row, and ↑ from the first row comes back to the field. The row says what matched in the place of its age" },
  { press: deckRow("Backfill the audit log"), say: "An ended Session opens on the stage like a live one: its thread and its Ledger, with no box to answer in" },
  { press: role("button", "More"), say: "Fork and Retro are behind the three dots, as they were on the page" },
  { press: role("menuitem", "Retro"), say: "Fleet refuses it" },
  { look: text("A retro is already being written for this Session"), say: "What Fleet refused is said at the top of the thread, where the Session is" },
  { key: "Meta+k", on: places, say: "⌘K, for a Session" },
  { type: "Draft the migration notes\n", into: inside(role("dialog", "Command palette"), role("combobox")), say: "A Session reached from the palette is selected in the Cockpit" },
  { look: region("Session: Draft the migration notes"), say: "On the stage, with the list as it was left" },
]);
