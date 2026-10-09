// Until the proposer's title lands, a Job's title is its request, and the
// request was typed in markdown. The Dashboard and the page's header draw its words
// with none of the asterisks or backticks; the Brief card still draws it as
// markdown, which `markdownFromTheProposer` looks at.

import { button, role, tab, text, walk } from "../walk";

export const plainWordsInARequestTitle = walk("arc/proposing-workflow-landed", [
  { press: tab("Active"), say: "A Job being proposed is under way" },
  {
    look: text("the worktree alone the branch as well"),
    say: "The Dashboard's title: the request's words, with no ** or backticks",
  },
  { press: role("option", /the worktree alone the branch as well/), say: "Picked" },
  { press: button(/^(Open|Review|Redirect|Attest)$/), say: "Open the Job" },
  {
    look: role("heading", "the worktree alone the branch as well"),
    say: "The page's header, in the same plain words",
  },
]);
