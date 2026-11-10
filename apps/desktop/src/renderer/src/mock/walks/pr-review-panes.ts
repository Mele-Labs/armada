// An open pull request in the panes a Job's are drawn in: Overview, Files, Review, Checks and Commits in the main pane, Comments to
// its right. A tool opens in a tab of its own beside Comments, from + or by its number. Comments lists the threads by file and
// line, and a press on one brings Files forward at its line. Over the `pr-review-panes` scenario, which is `pr-review` with a commit pushed to this pull request.

import { button, inside, place, region, role, tab, text, walk } from "../walk";

const list = region("Pull requests");
const reader = region("Pull request #1899");
const toolsPane = inside(reader, region("Pane: Comments"));
const comments = inside(
  reader,
  region("Comments", {
    exact: true,
  }),
);
const open = inside(comments, role("list", "Open threads"));
const files = inside(reader, region("Pane: Files"));

const prReviewPanes = walk("pr-review-panes", [
  {
    press: place("Reviews"),
    say: "Reviews",
  },
  {
    press: inside(list, button(/#1899/)),
    say: "A pull request a teammate asked for, with comments on the forge",
  },
  {
    look: inside(reader, tab("Overview")),
    say: "It opens in panes, as a Job does. Overview, Files, Review, Checks and Commits are tabs of the main pane",
  },
  {
    look: comments,
    say: "To the right, Comments. A tool's tab opens beside it",
  },
  {
    press: inside(toolsPane, button("Add tab")),
    say: "+ lists every explanation and tool, each with its number, and Add a tool last",
  },
  {
    look: role("menuitem", "Add a tool"),
    say: "Add a tool is the last entry",
  },
  {
    press: role("menuitem", "Risks"),
    say: "Risks. Nothing is kept for it and running it spends tokens, so its tab waits for Run",
  },
  {
    look: inside(reader, text("Running this uses tokens.")),
    say: "The line beside Run says so, with Don't ask again",
  },
  {
    press: inside(reader, button("Run")),
    say: "Run",
  },
  {
    look: inside(reader, region("Risks results")),
    say: "Its result, in a tab of its own",
  },
  {
    key: "6",
    on: reader,
    say: "6 opens Possible bugs, the sixth entry of +",
  },
  {
    look: inside(reader, text("Running this uses tokens.")),
    say: "It waits for Run too",
  },
  {
    press: inside(reader, role("checkbox", "Don't ask again")),
    say: "Don't ask again, for every tool on this machine",
  },
  {
    press: inside(reader, button("Run")),
    say: "Run",
  },
  {
    look: inside(reader, tab("Possible bugs")),
    say: "A tab beside Risks",
  },
  {
    look: inside(reader, region("Possible bugs results")),
    say: "In front",
  },
  {
    press: inside(inside(reader, region("Pane: Possible bugs")), button("Add tab")),
    say: "Another tool from +",
  },
  {
    press: role("menuitem", "Why"),
    say: "Why. The question is off, so opening its tab runs it",
  },
  {
    look: inside(reader, tab("Why")),
    say: "Its tab opens and runs, with no Run to press",
  },
  {
    press: inside(
      reader,
      tab("Comments", {
        exact: true,
      }),
    ),
    say: "Comments, a tab in the same pane as the tools",
  },
  {
    look: inside(open, button(/^Go to crates\/fleet\/src\/plan_usage\.rs:\d+ in Files$/)),
    say: "Comments lists each open thread by its file and line",
  },
  {
    press: inside(comments, button(/^Resolved/)),
    say: "The resolved ones are folded under their count",
  },
  {
    look: inside(comments, role("list", "Resolved threads")),
    say: "Opened",
  },
  {
    press: inside(open, button(/^Open the thread on crates\/fleet\/src\/plan_usage\.rs:/)),
    say: "A thread opens in place",
  },
  {
    look: inside(comments, role("article", /^Thread on .+ by devon$/)),
    say: "To read, reply and resolve it here",
  },
  {
    press: inside(
      inside(comments, role("article", /^Thread on .+ by devon$/)),
      button("Reply", {
        exact: true,
      }),
    ),
    say: "Reply",
  },
  {
    type: "A const is fine until someone asks for it.",
    into: inside(comments, role("textbox", "Reply")),
    say: "His reply",
  },
  {
    press: inside(comments, button("Add to review")),
    say: "It joins his pending review, posted with Submit",
  },
  {
    look: inside(comments, role("article", "Pending comment")),
    say: "Under the thread",
  },
  {
    press: inside(open, button(/^Go to crates\/fleet\/src\/plan_usage\.rs:\d+ in Files$/)),
    say: "A press on the thread goes to it in Files",
  },
  {
    look: inside(files, role("article", /^Thread on .+ by devon$/)),
    say: "Files comes forward at its line, with the thread under the line",
  },
  {
    press: inside(reader, button(/^Layout: Default/)),
    say: "The Layout control, in the pull request's head",
  },
  {
    look: role("menuitem", "New pull requests open on"),
    say: "Update, Save as, Reset, and the layout new pull requests open on, kept apart from the Jobs' layouts",
  },
  {
    key: "Escape",
    on: role("menuitem", "New pull requests open on"),
    say: "Esc puts it away",
  },
  {
    press: inside(comments, button("Reply")),
    say: "A reply he begins and does not add",
  },
  {
    type: "Check the minute against the clock.",
    into: inside(comments, role("textbox", "Reply")),
    say: "Written under the thread",
  },
  {
    press: inside(list, button(/#212/)),
    say: "He leaves for another pull request",
  },
  {
    look: region("Pull request #212"),
    say: "It opens in the layout new pull requests open on",
  },
  {
    press: inside(list, button(/#1899/)),
    say: "And comes back to the first",
  },
  {
    look: inside(reader, tab("Risks")),
    say: "Its tabs are where he left them, with the results kept for them",
  },
  {
    look: comments,
    say: "The one in front is the one that was: Comments",
  },
  {
    look: inside(files, region("crates/fleet/src/plan_usage.rs")),
    say: "Files is scrolled to the file he was reading",
  },
  {
    look: inside(comments, role("article", /^Thread on .+ by devon$/)),
    say: "The thread he had open in Comments is still open",
  },
  {
    look: inside(comments, role("textbox", "Reply")),
    say: "With the reply he had begun, in its box",
  },
  {
    press: inside(list, button(/#212/)),
    say: "He leaves again",
  },
  {
    press: inside(list, button("Refresh")),
    say: "Refresh reads the lists again, and a commit has been pushed to the first",
  },
  {
    press: inside(list, button(/#1899/)),
    say: "Back to the first",
  },
  {
    look: inside(reader, role("status", "Pull request changed")),
    say: "One line says the pull request has moved since he looked, from one commit to the next, and how many came between",
  },
  {
    press: inside(reader, tab("Possible bugs")),
    say: "A result kept on the older commit",
  },
  {
    look: inside(
      inside(reader, region("Possible bugs results")),
      role("img", /^Out of date\. Ran on/),
    ),
    say: "Says so in its tab, naming the commit it ran on",
  },
  {
    press: inside(reader, button("Re-run")),
    say: "Re-run runs every one of them again. It does not ask, since he pressed it",
  },
]);

export { prReviewPanes as "pr-review-panes" };
