// What the Sessions page held, found in the Cockpit's list: no Sessions tab, a Filter menu in the band with a search and Ended (and Quiet and All) beside Active, fifty
// rows at a time, an ended Session opened on the stage with its Retro refused in the open, and a Session reached from ⌘K selected there.
// Over the `cockpit-sessions-list` scenario.

import { deckRow, inside, region, role, text, walk } from "../walk";

const places = role("navigation", "Places");
const list = region("Jobs", { exact: true });
const filter = inside(list, role("button", /^Filter/));
// The menu is drawn over the page and not inside the list.
const search = role("searchbox", "Search Jobs");

export const cockpitSessionsList = walk("cockpit-sessions-list", [
  { look: places, say: "There is no Jobs tab. A Job is viewed on the Cockpit, and its list is the one on the left. ⌘3 and a link to a Job arrive here" },
  { press: filter, say: "Filter is at the right of the Jobs band: a small menu with a search field, the caret in it, and the four scopes under it, Active checked. Esc closes it" },
  { press: role("menuitem", "Ended"), say: "Ended lists the Jobs that ended in the last week, newest first. Quiet lists the ones Fleet cannot reach, and All adds both to the live ones. The band says Ended in place of Filter while the list is narrowed" },
  { look: deckRow("Trim the retry loop 49"), say: "Fifty rows are drawn at first, and this is the last of them" },
  { look: deckRow("Draft the release notes 63"), say: "Fifty more are drawn each time the end of the list comes into view. j and k walk past a page and draw what they need" },
  { key: "/", on: list, say: "/ opens the same menu from anywhere on the Cockpit outside a text box, with the caret in the search" },
  { type: "1702", into: search, say: "The search finds a Job by its title, id, pull request, branch or slot. An ended Job older than a week is listed only by a search. The band carries the words" },
  { key: "Enter", on: search, say: "Enter closes the menu and goes to the first row, and ↑ from the first row opens it again. The row says what matched in the place of its age" },
  { press: deckRow("Backfill the audit log"), say: "An ended Job opens on the stage like a live one: its thread and its Blueprint and Field, with no box to answer in" },
  { press: role("button", "More"), say: "Fork and Retro are behind the three dots, as they were on the page" },
  { press: role("menuitem", "Retro"), say: "Fleet refuses it" },
  { look: text("A retro is already being written for this Session"), say: "What Fleet refused is said at the top of the thread, where the Job is" },
  { key: "Meta+k", on: places, say: "⌘K, for a Job" },
  { type: "Draft the migration notes\n", into: inside(role("dialog", "Command palette"), role("combobox")), say: "A Job reached from the palette is selected in the Cockpit" },
  { look: region("Job: Draft the migration notes"), say: "On the stage, with the list as it was left" },
]);
