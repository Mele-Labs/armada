// The Usage page: open it by name, key down to the Job that burned most, pick it, step onto its busiest hour on the
// day by hour map, and set its breakdown to tools.
// Keys on the page: ↓ j and ↑ k move down the Jobs, ↵ o picks one, Esc clears the hour cell and then the pick,
// m t b y d set the breakdown to model, tool, Bash use, token type or day, ← → h l step the weekday and [ ] the 3 hour block on the map.

import { inside, region, role, viaPalette, walk } from "../walk";

const SESSIONS = role("listbox", "Jobs", { exact: true });

export const burnBySessionModelTool = walk("usage/week", [
  ...viaPalette("Usage", "Usage, by name"),
  { key: "ArrowDown", on: SESSIONS, say: "↓ moves the cursor to the top Job" },
  { key: "Enter", on: SESSIONS, say: "↵ picks it: the tiles, the day chart and the map are now its own" },
  { key: "l", on: SESSIONS, say: "l steps onto the busiest 3 hours of its week" },
  { look: inside(region("Day and hour"), role("gridcell", /Wed, 21h to 00h/)), say: "Its cell on the map; h l step the weekday, [ ] the hour" },
  { key: "t", on: SESSIONS, say: "t sets the breakdown to tools" },
  { look: inside(region("Breakdown"), region("By tool")), say: "By tool, the turns that called no tool among them" },
]);
