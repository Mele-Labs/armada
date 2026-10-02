// Until the proposer's title lands, a Job's title is its request, and the
// request was typed in markdown. The row and the page's header draw its words
// with none of the asterisks or backticks; the Brief card still draws it as
// markdown, which `markdownFromTheProposer` looks at.

import { role, text, walk } from "../walk";

export const plainWordsInARequestTitle = walk("arc/proposing-workflow-landed", [
  {
    look: text("the worktree alone the branch as well"),
    say: "The Board row's title: the request's words, with no ** or backticks",
  },
  { press: text("the worktree alone the branch as well"), say: "Open the Job" },
  {
    look: role("heading", "the worktree alone the branch as well"),
    say: "The page's header, in the same plain words",
  },
]);
