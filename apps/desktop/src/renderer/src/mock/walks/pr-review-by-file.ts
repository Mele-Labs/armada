// The Review tab, which reads a pull request one file at a time with the findings pinned beside their lines. With no tool tab
// open it runs Risks from its head, steps between files with Previous and Next beside the file's place, counts the findings marked
// as they are marked, and starts a review from lines picked in its gutter. A path and line named in a finding opens those lines in
// place, with Jump; t opens Go to file; the submit bar says once that findings are unmarked; and Ask, docked under the panes,
// takes a question. Over the `pr-review-by-file` scenario.

import { button, inside, place, region, role, tab, text, walk } from "../walk";

const list = region("Pull requests");
const reader = region("Pull request #2417");
const toolsPane = inside(reader, region("Empty pane"));
const reviewTab = inside(
  reader,
  tab("Review", {
    exact: true,
  }),
);
const reviewPane = inside(reader, region("Pane: Review"));
const askPane = inside(reader, region("Pane: Ask"));
const head = inside(reviewPane, role("group", "Tools"));
const files = inside(reviewPane, role("group", "Files"));
const findings = inside(reviewPane, role("complementary", "Findings"));
const marks = (title: string) => inside(findings, role("group", `Mark ${title}`));
const gutter = (line: RegExp) => inside(reviewPane, button(line));
const yours = inside(reviewPane, region("Your review"));
const PEEKED = "app/models/payment.rb:17-19";
const goTo = role("dialog", "Go to file");

const prReviewByFile = walk("pr-review-by-file", [
  {
    press: place("Reviews"),
    say: "Reviews",
  },
  {
    press: inside(list, button(/#2417/)),
    say: "A pull request of eight files, asked of him",
  },

  // 1. Review, with no tool tab open.
  {
    press: reviewTab,
    say: "Review is a tab of the main pane, after Files",
  },
  {
    look: toolsPane,
    say: "No tool tab is open",
  },

  // 2. Risks, run from Review's head.
  {
    hover: inside(head, button("Run Risks")),
    say: "Each tool has a button in the head. Its glyph is its state: not run",
  },
  {
    press: inside(head, button("Run Risks")),
    say: "Run Risks",
  },
  {
    press: inside(role("dialog", "Risks"), button("Run")),
    say: "It asks first, as every run does",
  },
  {
    hover: inside(head, button("Run Risks again")),
    say: "Done, with when and on which commit",
  },
  {
    look: inside(findings, role("article", /Two refunds can pass the same check/)),
    say: "Its findings land beside their lines, with no Risks tab open",
  },

  // 3. Previous and Next, beside where the file is.
  {
    look: inside(files, text("file 1 of 8")),
    say: "Previous and Next sit either side of where the file is, files with findings first",
  },
  {
    press: inside(files, button("Next file")),
    say: "Next",
  },
  {
    look: inside(files, text("file 2 of 8")),
    say: "The next file",
  },
  {
    press: inside(files, button("Previous file")),
    say: "Previous, back to the first",
  },

  // 4. The findings marked, counted as they are marked.
  {
    hover: inside(reviewPane, text("0 of 4 findings marked")),
    say: "How many findings are marked, across every file. It is the one count Review keeps",
  },
  {
    key: "j",
    on: reviewTab,
    say: "j brings the first finding into focus, tied to its lines",
  },
  {
    key: "j",
    on: reviewTab,
    say: "j again, the next one down",
  },
  {
    press: inside(marks("Two refunds can pass the same check"), button("Expected")),
    say: "Expected puts the first away, dimmed",
  },
  {
    look: inside(reviewPane, text("1 of 4 findings marked")),
    say: "The count moves",
  },
  {
    press: inside(
      marks("A timeout leaves the refund pending until the job runs"),
      button("Not valid"),
    ),
    say: "Not valid puts the second away",
  },
  {
    look: inside(reviewPane, text("2 of 4 findings marked")),
    say: "And again",
  },

  // 5. A range of lines, a review started, and a second comment added to it.
  {
    press: gutter(/^Comment on line 8: raise Refund::TooLarge/),
    say: "A press in the gutter picks a line",
  },
  {
    press: gutter(/^Comment on line 10: refund = /),
    shift: true,
    say: "Shift and a press picks the lines between",
  },
  {
    type: "Lock the payment row before reading what is left.",
    into: inside(reviewPane, role("textbox", "Comment")),
    say: "His comment, under the lines",
  },
  {
    press: inside(reviewPane, button("Start a review")),
    say: "Start a review",
  },
  {
    look: inside(reviewPane, role("article", "Pending comment")),
    say: "Pending, under its lines",
  },
  {
    press: gutter(/^Comment on line 15: Gateway\.refund/),
    say: "Another line",
  },
  {
    type: "Pass an idempotency key, so a retry cannot refund twice.",
    into: inside(reviewPane, role("textbox", "Comment")),
    say: "A second comment",
  },
  {
    press: inside(reviewPane, button("Add to review (1)")),
    say: "Add to review, with the one already pending",
  },

  // A path and line in a finding.
  {
    press: inside(findings, role("link", PEEKED)),
    say: "A path and lines in a finding's words open those lines under it",
  },
  {
    look: inside(findings, region(`Lines of ${PEEKED}`)),
    say: "The lines, cut from the diff",
  },
  {
    press: inside(findings, button(`Jump to ${PEEKED}`)),
    say: "Jump",
  },
  {
    look: inside(files, text("file 2 of 8")),
    say: "Review reads that file, at those lines",
  },
  {
    look: inside(findings, text("Line 6 is not in the diff.")),
    say: "A finding on a line the diff does not hold says so",
  },

  // Go to file.
  {
    key: "t",
    on: reviewTab,
    say: "t opens Go to file",
  },
  {
    type: "refcon",
    into: inside(goTo, role("textbox", "File")),
    say: "A few letters of the file",
  },
  {
    press: inside(goTo, role("option", "app/controllers/refunds_controller.rb")),
    say: "The file they rank first",
  },
  {
    look: inside(files, text("file 3 of 8")),
    say: "Review reads it",
  },

  // 6. The submit bar.
  {
    press: inside(reviewPane, button("Review changes")),
    say: "Review changes opens his review under the file: the two comments, a summary and Submit",
  },
  {
    look: inside(yours, role("list", "Pending comments")),
    say: "The two pending comments",
  },
  {
    press: inside(yours, tab("Approve")),
    say: "Comment, Approve or Request changes",
  },
  {
    press: inside(yours, button("Submit review")),
    say: "Submit review",
  },
  {
    look: inside(yours, role("alert")),
    say: "Two findings are unmarked, so it says so once and waits for a second press",
  },

  // 7. Ask, docked under the panes.
  {
    look: askPane,
    say: "Ask is a tab of its own, docked under the panes and as wide as them",
  },
  {
    type: "Can a retry refund twice?",
    into: inside(askPane, role("textbox", "Ask about this PR")),
    say: "His question",
  },
  {
    key: "Enter",
    on: inside(askPane, role("textbox", "Ask about this PR")),
    say: "↵ sends",
  },
  {
    press: inside(role("dialog", "Ask"), button("Run")),
    say: "An answer costs, so it asks first",
  },
  {
    look: inside(askPane, role("region", "Answer")),
    say: "The question and its answer stay in the tab",
  },
]);

export { prReviewByFile as "pr-review-by-file" };
