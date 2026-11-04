// The Usage page weighed by model and cache lifetime, then its Spent on view: what 14 days of tokens went on and who spent
// them, then one Job's own split. A sketch for the owner before Fleet computes either. Over the `usage/where-it-went` scenario.

import { inside, region, role, tab, text, viaPalette, walk } from "../walk";

const JOBS = role("listbox", "Jobs", {
  exact: true,
});
const TILES = role("list", "Headline figures");

const usageWhereItWent = walk("usage/where-it-went", [
  ...viaPalette("Usage", "Usage, by name"),
  {
    look: inside(TILES, role("listitem", "Today's burn")),
    say: "Burn now weighs each model: Haiku 1, Sonnet 3, Opus and Fable 5",
  },
  {
    look: inside(inside(TILES, role("listitem", "Burn in subagents")), text("27%")),
    say: "Subagents: 27% of the weighed week, counted as Spent on counts them",
  },
  {
    look: inside(region("Breakdown"), role("listitem", /^opus, /)),
    say: "Opus rises to a third of the burn",
  },
  {
    key: "y",
    on: JOBS,
    say: "y sets the breakdown to token type",
  },
  {
    look: inside(region("Breakdown"), role("listitem", /^cache write, 1 hour, /)),
    say: "A cache write held an hour counts 2, and has its own row",
  },
  {
    press: tab("Spent on"),
    say: "Spent on: what the tokens went on",
  },
  {
    look: inside(
      region("Spent on", {
        exact: true,
      }),
      role("listitem", /^Opening text, /),
    ),
    say: "Over 14 days, the text a thread opens with comes first",
  },
  {
    look: inside(
      region("Spent by", {
        exact: true,
      }),
      role("listitem", /^Main threads, /),
    ),
    say: "Who spent it: main threads, subagents, background",
  },
  {
    press: inside(role("listbox", "Jobs by burn"), role("option", /^Debug Job 14, /)),
    say: "He picks one Job",
  },
  {
    look: inside(
      role("list", "Debug Job 14, spent on"),
      role("listitem", /^Cold cache rewrites, /),
    ),
    say: "Its own split: most went on rewrites after a cold cache",
  },
]);

export { usageWhereItWent as "usage-where-it-went" };
