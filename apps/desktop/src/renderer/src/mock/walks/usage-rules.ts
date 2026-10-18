// The Usage page's rules: open the page, key to the rules, pick Opus reading, and read a trip's burn before and after.
// Keys on the page: 1 to 4 pick the rule at that place, Esc clears the pick; with a Job picked (↓ ↵) only its trips
// show. The other keys are the page's own: ↓ j ↑ k ↵ o Esc, m t b y d, ← → h l, [ ].

import { inside, region, role, viaPalette, walk } from "../walk";

const SESSIONS = role("listbox", "Jobs", { exact: true });

export const usageRules = walk("usage/week", [
  ...viaPalette("Usage", "Usage, by name"),
  { look: region("Usage rules"), say: "Four rules, each with its limit and whether it is on" },
  { key: "4", on: SESSIONS, say: "4 picks Opus reading and lists its trips" },
  { look: inside(region("Usage rules"), role("list", "Opus reading, trips")), say: "Each trip: the Job, when, the figure, the burn per hour before and after the tell" },
]);
