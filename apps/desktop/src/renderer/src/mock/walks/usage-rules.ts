// The Usage page's rules: open the page, key to the rules, pick Context, and read a trip's burn before and after.
// Keys on the page: 1 picks the rule, Esc clears the pick; with a Job picked (↓ ↵) only its trips
// show. The other keys are the page's own: ↓ j ↑ k ↵ o Esc, m t b y d, ← → h l, [ ].

import { inside, region, role, viaPalette, walk } from "../walk";

const SESSIONS = role("listbox", "Jobs", {
  exact: true,
});

export const usageRules = walk("usage/week", [
  ...viaPalette("Usage", "Usage, by name"),
  {
    look: region("Usage rules"),
    say: "The rule, with its limit and whether it is on",
  },
  {
    key: "1",
    on: SESSIONS,
    say: "1 picks Context and lists its trips",
  },
  {
    look: inside(region("Usage rules"), role("option", /Context/)),
    say: "Under the picked rule, what the Job is told",
  },
  {
    look: inside(region("Usage rules"), role("list", "Context, trips")),
    say: "Each trip: the Job, when, the figure, the burn per hour before and after the tell",
  },
]);
