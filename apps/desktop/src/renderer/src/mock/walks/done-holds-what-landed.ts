// Done holds a pull request once it merges and an issue once it closes. Until then each stands in Now, under Not landed. Over `job-story-full`,
// whose Job holds two open issues and an open pull request beside three finished groups.

import { deckRow, inside, region, role, tab, walk } from "../walk";

const done = region("Done", {
  exact: true,
});
const now = region("Now", {
  exact: true,
});

export const doneHoldsWhatLanded = walk("job-story-full", [
  {
    press: deckRow("Issues tab beside Reviews"),
    say: "A Job with two issues still open and a pull request not merged",
  },
  {
    look: inside(now, role("list", "Not landed")),
    say: "Now holds the open issues and the pull request not yet merged",
  },
  {
    press: tab("Done", {
      exact: true,
    }),
    say: "Done",
  },
  {
    press: role("radio", "By kind", {
      exact: true,
    }),
    say: "Done by kind",
  },
  {
    look: done,
    say: "Done lists only the merged pull requests and the closed issues",
  },
]);
