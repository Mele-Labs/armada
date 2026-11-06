// Focus, a tab that reads a pull request one file at a time with the findings pinned beside their lines. Opened beside Risks, it
// steps through the findings with j, marks them Expected or Not valid, and starts a review from lines picked in its gutter. A
// path and line named in a finding opens those lines in place, with Jump into Focus; t opens Go to file; and the review's
// submit says once that findings are unmarked. Over the `pr-focus-review` scenario.

import { button, inside, place, region, role, tab, text, walk } from "../walk";

const list = region("Pull requests");
const reader = region("Pull request #2417");
const toolsPane = inside(reader, region("Empty pane"));
const overview = inside(reader, region("Pane: Overview"));
const focusPane = inside(reader, region("Pane: Focus"));
const risks = inside(reader, region("Risks results"));
const findings = inside(focusPane, role("complementary", "Findings"));
const marks = (title: string) => inside(findings, role("group", `Mark ${title}`));
const gutter = (line: RegExp) => inside(focusPane, button(line));
const PEEKED = "app/models/payment.rb:17-19";
const goTo = role("dialog", "Go to file");

const prFocusReview = walk("pr-focus-review", [
  {
    press: place("Reviews"),
    say: "Reviews",
  },
  {
    press: inside(list, button(/#2417/)),
    say: "A pull request of eight files, asked of him",
  },
  {
    press: inside(toolsPane, button("Add tab")),
    say: "The tools pane's +",
  },
  {
    press: role("menuitem", "Risks"),
    say: "Risks",
  },
  {
    press: inside(reader, button("Run")),
    say: "Run",
  },
  {
    look: risks,
    say: "Its findings, by change",
  },

  // 1. Focus, beside Risks.
  {
    press: inside(overview, button("Add tab")),
    say: "The main pane's + lists Focus after Files",
  },
  {
    press: role("menuitem", "Focus"),
    say: "Focus",
  },
  {
    look: inside(focusPane, text("file 1 of 8")),
    say: "One file at a time, files with findings first",
  },
  {
    look: inside(focusPane, text("0 of 4 handled")),
    say: "How many findings are marked, the one count it keeps",
  },

  // 2. Stepping through the findings.
  {
    key: "j",
    on: inside(reader, tab("Focus")),
    say: "j brings the first finding into focus, tied to its lines",
  },
  {
    key: "j",
    on: inside(reader, tab("Focus")),
    say: "j again, the next one down",
  },

  // 3. Marking two.
  {
    press: inside(marks("Two refunds can pass the same check"), button("Expected")),
    say: "Expected puts the first away, dimmed",
  },
  {
    press: inside(
      marks("A timeout leaves the refund pending until the job runs"),
      button("Not valid"),
    ),
    say: "Not valid puts the second away",
  },
  {
    look: inside(focusPane, text("2 of 4 handled")),
    say: "The count moves",
  },

  // 4. A range of lines, and a review started.
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
    into: inside(focusPane, role("textbox", "Comment")),
    say: "His comment, under the lines",
  },
  {
    press: inside(focusPane, button("Start a review")),
    say: "Start a review",
  },
  {
    look: inside(focusPane, role("article", "Pending comment")),
    say: "Pending, under its lines",
  },

  // 5. A second comment joins it.
  {
    press: gutter(/^Comment on line 15: Gateway\.refund/),
    say: "Another line",
  },
  {
    type: "Pass an idempotency key, so a retry cannot refund twice.",
    into: inside(focusPane, role("textbox", "Comment")),
    say: "A second comment",
  },
  {
    press: inside(focusPane, button("Add to review (1)")),
    say: "Add to review, with the one already pending",
  },

  // 6. A path and line in a finding.
  {
    press: inside(risks, role("link", PEEKED)),
    say: "A path and lines in a finding's words open those lines under it",
  },
  {
    look: inside(risks, region(`Lines of ${PEEKED}`)),
    say: "The lines, cut from the diff",
  },
  {
    press: inside(risks, button(`Jump to ${PEEKED}`)),
    say: "Jump",
  },
  {
    look: inside(focusPane, text("file 2 of 8")),
    say: "Focus reads that file, at those lines",
  },
  {
    look: inside(findings, text("Line 6 is not in the diff.")),
    say: "A finding on a line the diff does not hold says so",
  },

  // 7. Go to file.
  {
    key: "t",
    on: inside(reader, tab("Focus")),
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
    look: inside(focusPane, text("file 3 of 8")),
    say: "Focus reads it",
  },

  // 8. Submitting with findings unmarked.
  {
    press: inside(focusPane, button("Review changes")),
    say: "Review changes opens the review under the file",
  },
  {
    press: inside(focusPane, button("Submit review")),
    say: "Submit review",
  },
  {
    look: inside(focusPane, role("alert")),
    say: "Two findings are unmarked, so it says so once and waits for a second press",
  },
]);

export { prFocusReview as "pr-focus-review" };
