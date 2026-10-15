// Until the proposer's title lands, a Job's title is its request, and the
// request was typed in markdown. The page's header draws its words with none of the
// asterisks or backticks; the Brief card still draws it as markdown, which
// `markdownFromTheProposer` looks at. The deck's list row still draws the request raw: owed in src.

import { button, deckRow, inside, region, role, walk } from "../walk";

export const plainWordsInARequestTitle = walk("arc/proposing-workflow-landed", [
  { press: deckRow(/^The Cleared tab keeps rows/), say: "A Job being proposed is under way, its request standing in for the title" },
  { press: inside(region(/^Job: The Cleared tab keeps rows/), button("Open the Job page")), say: "Open the Job page" },
  {
    look: role("heading", "the worktree alone the branch as well"),
    say: "The page's header, in the same plain words",
  },
]);
