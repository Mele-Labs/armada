// A pull request he opened stays open until he opens another: Esc stops at the list, and one opened by its link keeps a row
// above whichever list is chosen. Review is a tab of the reader, and each pending comment there goes to its line. The
// explanations are the first entries of the tools pane's +.
// Over the `pr-review` scenario.

import { button, inside, region, role, tab, walk } from "../walk";

const list = region("Pull requests");
const reader = (number: number) => region(`Pull request #${number}`);
const yours = region("Your review");
const review = (number: number) => inside(reader(number), region("Pane: Review"));
const lists = inside(list, button("List"));
const plus = inside(inside(reader(1899), region("Pane: Comments")), button("Add tab"));

const prReviewStaysOpen = walk("pr-review", [
  {
    paste: region("Jobs", {
      exact: true,
    }),
    text: "https://forge.example/mele-labs/armada/pull/1905",
    say: "A link to a pull request on none of his lists, pasted. The box on the Reviews list opens one the same way",
  },
  {
    look: reader(1905),
    say: "Reviews opens on it",
  },
  {
    look: inside(list, button(/#1905/)),
    say: "It has a row at the top of the list, marked as opened by its link, so he can come back to it",
  },
  {
    press: inside(list, button(/#1899/)),
    say: "He opens another one from the list",
  },
  {
    key: "Escape",
    on: inside(list, button(/#1899/)),
    say: "Esc in the list",
  },
  {
    key: "Escape",
    on: inside(list, button(/#1899/)),
    say: "And again. Esc never closes the pull request: only opening another one changes it",
  },
  {
    look: reader(1899),
    say: "Still open",
  },
  {
    press: lists,
    say: "Another list",
  },
  {
    press: role("menuitem", "Team"),
    say: "Team",
  },
  {
    look: inside(list, button(/#1905/)),
    say: "The pull request opened by its link is above this list too",
  },
  {
    press: inside(list, button(/#1905/)),
    say: "A press goes back to it. A reload of Bridge opens it again too, and keeps the row",
  },
  {
    look: inside(
      reader(1905),
      tab("Review", {
        exact: true,
      }),
    ),
    say: "Review is a tab of the pull request, after Files",
  },
  {
    press: inside(
      reader(1905),
      tab("Review", {
        exact: true,
      }),
    ),
    say: "It reads the pull request a file at a time",
  },
  {
    press: inside(review(1905), button("Submit review")),
    say: "Submit review opens his review under the file",
  },
  {
    look: inside(yours, tab("Approve")),
    say: "Comment, Approve and Request changes, with a summary",
  },
  {
    look: inside(yours, button("Submit review")),
    say: "Submit posts it as one review, with every comment he added",
  },
  {
    press: inside(list, button(/#1899/)),
    say: "A pull request that already has comments on the forge",
  },
  {
    press: inside(
      reader(1899),
      tab("Review", {
        exact: true,
      }),
    ),
    say: "Review",
  },
  {
    look: inside(inside(reader(1899), region("Comments")), button(/^Open the thread on /)),
    say: "Comments already posted are in the Comments tab beside it, each thread folded to a line",
  },
  {
    press: inside(review(1899), button(/^Comment on line \d+: let mut held/)),
    say: "Comment on a line, from the gutter",
  },
  {
    type: "Take the lock after the call.",
    into: inside(review(1899), role("textbox", "Comment")),
    say: "His comment",
  },
  {
    press: inside(review(1899), button("Start a review")),
    say: "Start a review. Nothing reaches the forge until Submit",
  },
  {
    press: inside(review(1899), button("Submit review")),
    say: "His review",
  },
  {
    look: inside(yours, button(/^Go to your comment on /)),
    say: "It waits in his review with the file, the line and the code it is on. X takes it out",
  },
  {
    press: inside(yours, button(/^Go to your comment on /)),
    say: "A press goes to it in the file",
  },
  {
    look: inside(review(1899), role("article", "Pending comment")),
    say: "His comment, under the line it is on",
  },
  {
    press: plus,
    say: "The tools pane's + holds everything that runs: the four explanations first, then the tools. There is no Comments entry",
  },
  {
    look: role("menuitem", "User Impact"),
    say: "User Impact, How to Test and Why are beside How it works, each with its number",
  },
  {
    press: role("menuitem", "How it works"),
    say: "How it works, from +",
  },
  {
    press: inside(reader(1899), button("Run")),
    say: "It spends tokens, so its tab waits for Run",
  },
  {
    look: inside(reader(1899), role("region", "Theme: Fleet keeps the last read")),
    say: "The explanation is in a tab of its own",
  },
]);

export { prReviewStaysOpen as "pr-review-stays-open" };
