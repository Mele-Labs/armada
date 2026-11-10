// The Review tab, which reads a pull request with the findings pinned beside their lines. Opening it asks which tools to run, Tour
// first and ticked, from the repository's default, and keeps a new default, which System, Repository shows with Ask before running.
// Once Tour has run, Previous and Next step through its stops, each its lines with the findings and threads on them, and the last
// gathers what no stop holds; a file Go to file reaches off the tour says so. Each finding keeps its own answer on its card, and a fix
// asked for one goes to the review as a suggestion on its lines. It starts a review from lines picked in its gutter, and opens the
// lines a finding names in place, with Jump. The submit bar says once that
// findings have no answer. An explanation runs from the head and opens in a tab beside Comments, whose Conversation holds what sits
// under no line. Ask, docked under the panes, takes a question. Over the `pr-review-by-file` scenario.

import { button, inside, place, region, role, tab, text, toSystemPage, walk } from "../walk";

const list = region("Pull requests");
const reader = region("Pull request #2417");
const commentsPane = inside(reader, region("Pane: Comments"));
const comments = inside(
  reader,
  region("Comments", {
    exact: true,
  }),
);
const conversation = inside(comments, region("Conversation"));
const reviewTab = inside(
  reader,
  tab("Review", {
    exact: true,
  }),
);
const reviewPane = inside(reader, region("Pane: Review"));
const askPane = inside(reader, region("Pane: Ask"));
const head = inside(reviewPane, role("group", "Tools"));
const stops = inside(reviewPane, role("group", "Stops"));
const findings = inside(reviewPane, role("complementary", "Findings"));
const marks = (title: string) => inside(findings, role("group", `Mark ${title}`));
const RACE = "Two refunds can pass the same check";
const fix = inside(findings, region(`Fix for ${RACE}`));
const gutter = (line: RegExp) => inside(reviewPane, button(line));
const yours = inside(reviewPane, region("Your review"));
const PEEKED = "app/models/payment.rb:17-19";
const goTo = role("dialog", "Go to file");
const opening = role("dialog", "Run on opening");
const tallyDefaults = inside(region("Run on opening"), region("fernhill/tally"));

const prReviewByFile = walk("pr-review-by-file", [
  {
    press: place("Reviews"),
    say: "Reviews",
  },
  {
    press: inside(list, button(/#2417/)),
    say: "A pull request of eight files, asked of him",
  },

  // 1. Opening Review asks which tools to run, ticked from the repository's default.
  {
    look: commentsPane,
    say: "To the right of the main pane, Comments alone. Every tool runs from Review",
  },
  {
    press: reviewTab,
    say: "Review is a tab of the main pane, after Files",
  },
  {
    look: inside(opening, role("list", "Tools to run")),
    say: "Opening it asks what to run. Every tool and explanation of the head has a box, ticked from this repository's default: Tour and the four tools",
  },
  {
    look: inside(opening, role("checkbox", "Tour")),
    say: "Tour is first, and ticked. Review steps through the change by its stops",
  },
  {
    press: inside(opening, role("checkbox", "Test gaps")),
    say: "He unticks Test gaps",
  },
  {
    press: inside(opening, role("checkbox", "Use these for fernhill/tally from now on")),
    say: "And keeps the four as this repository's default, for the next pull request he opens here",
  },
  {
    press: inside(opening, button("Run")),
    say: "Run starts the ticked ones",
  },
  ...toSystemPage("Repository", "The default is kept in System, Repository, under Run on opening"),
  {
    look: tallyDefaults,
    say: "fernhill/tally runs four, with Test gaps unticked. Ask before running is on: Don't ask again in the ask turns it off, and this turns it back on",
  },
  {
    press: place("Reviews"),
    say: "Back to the pull request, where the ticked tools are running",
  },

  // 2. The tools, each a button in Review's head, Tour first.
  {
    hover: inside(head, button("Run Tour again")),
    say: "Each tool has a button in the head, Tour first. Its glyph is its state: done, with when and on which commit",
  },
  {
    hover: inside(head, button("Run Test gaps")),
    say: "Test gaps was unticked, so it has not run",
  },
  {
    press: inside(reviewPane, button("Add tab")),
    say: "+ lists the tools, each with its mark",
  },
  {
    look: role("menuitem", "Tour"),
    say: "Tour's mark is a folded map",
  },
  {
    key: "Escape",
    on: role("menu"),
    say: "Esc closes the menu",
  },

  // 3. Previous and Next, through Tour's stops.
  {
    look: inside(stops, text("Stop 1 of 6")),
    say: "Previous and Next step through Tour's stops: five in the order to read the change, then one for what they leave out",
  },
  {
    look: inside(reviewPane, text("The payment's page asks the issuer for a refund")),
    say: "Beside the count, what this stop does, then its file",
  },
  {
    look: inside(
      reviewPane,
      role("article", /^Thread on app\/controllers\/refunds_controller\.rb:\d+ by tamsin/),
    ),
    say: "The stop draws its lines and a few around them. Its open thread hangs under its line, with Reply and Resolve",
  },
  {
    press: inside(stops, button("Next stop")),
    say: "Next",
  },
  {
    look: inside(stops, text("Stop 2 of 6")),
    say: "The issuer, the next piece to read",
  },
  {
    look: inside(findings, role("article", /Two refunds can pass the same check/)),
    say: "The stop's findings sit beside its lines, with no Risks tab open",
  },
  {
    press: inside(reviewPane, button("Whole file")),
    say: "Whole file draws the rest of the file around the stop",
  },
  {
    look: gutter(/^Comment on line 1: class RefundIssuer/),
    say: "The lines above the stop",
  },
  {
    press: inside(reviewPane, button("Whole file")),
    say: "And back to the stop's lines",
  },

  // 4. Each finding answered on its own card.
  {
    key: "j",
    on: reviewTab,
    say: "j brings the stop's first finding into focus, tied to its lines",
  },
  {
    key: "j",
    on: reviewTab,
    say: "j again, the next one down",
  },
  {
    press: inside(marks("Two refunds can pass the same check"), button("Expected")),
    say: "Expected puts the first away",
  },
  {
    look: inside(marks("Two refunds can pass the same check"), button("Expected")),
    say: "The card keeps its own answer: Expected stays pressed, and the card dims",
  },
  {
    press: inside(
      marks("A timeout leaves the refund pending until the job runs"),
      button("Not valid"),
    ),
    say: "Not valid puts the second away",
  },

  // 5. A fix for a finding, added to the review as a suggestion.
  {
    press: inside(findings, button(`Ask for a fix: ${RACE}`)),
    say: "Ask for a fix, on the first finding",
  },
  {
    press: inside(role("dialog", "Ask for a fix"), button("Run")),
    say: "It asks first",
  },
  {
    look: inside(fix, role("group", "app/services/refund_issuer.rb")),
    say: "Why, on one line. Then lines 8 to 10 as they are, in red, and as the fix writes them, in green",
  },
  {
    press: inside(fix, button("Add as suggestion")),
    say: "Add as suggestion",
  },
  {
    look: inside(reviewPane, role("textbox", "Comment")),
    say: "The comment box opens under lines 8 to 10, holding the fix as a suggestion. Words can go above it",
  },
  {
    press: inside(reviewPane, button("Start a review")),
    say: "Start a review",
  },
  {
    look: inside(reviewPane, text("Lines 8 to 10")),
    say: "Pending, under its lines, on the range the finding names",
  },

  // 6. A range of lines picked in the gutter, and a second comment added to the review.
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
    press: inside(reviewPane, button("Add to review (1)")),
    say: "Add to review, with the suggestion already pending",
  },
  {
    look: inside(reviewPane, role("article", "Pending comment")),
    say: "Pending, under its lines. Its head reads Lines 8 to 10",
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
    press: inside(reviewPane, button("Add to review (2)")),
    say: "Add to review, with the two already pending",
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
    look: inside(stops, text("Stop 3 of 6")),
    say: "Review goes to the stop that holds those lines: the payment's cap",
  },
  {
    look: inside(findings, role("article", /refundable_amount counts only succeeded refunds/)),
    say: "With its finding beside them",
  },
  {
    press: inside(stops, button("Next stop")),
    say: "Next",
  },
  {
    look: inside(stops, text("Stop 4 of 6")),
    say: "The retry job",
  },
  {
    look: inside(reviewPane, role("article", /^Thread on app\/jobs\/refund_retry_job\.rb/)),
    say: "A thread on the whole file sits over the stop's lines",
  },
  {
    key: "]",
    on: reviewTab,
    say: "] steps to the next stop too",
  },
  {
    look: inside(stops, text("Stop 5 of 6")),
    say: "The spec, the last stop of the tour",
  },
  {
    look: inside(
      reviewPane,
      role("article", /^Thread on spec\/services\/refund_issuer_spec\.rb:\d+ by tamsin/),
    ),
    say: "Its resolved thread is drawn folded under its line",
  },
  {
    press: inside(stops, button("Next stop")),
    say: "Next",
  },
  {
    look: inside(reviewPane, text("Also in app/models/payment.rb")),
    say: "Stop 6 gathers what no stop of the tour holds: a finding on the payment",
  },
  {
    look: inside(findings, text("Line 6 is not in the diff.")),
    say: "On a line the diff does not hold, which it says",
  },

  // Go to file, to one no stop is on.
  {
    key: "t",
    on: reviewTab,
    say: "t opens Go to file, which finds any changed file",
  },
  {
    type: "routes",
    into: inside(goTo, role("textbox", "File")),
    say: "A few letters of a file no stop is on",
  },
  {
    press: inside(goTo, role("option", "config/routes.rb")),
    say: "The file they rank first",
  },
  {
    hover: inside(stops, text("Not on the tour")),
    say: "Review reads it whole, and says it is not on the tour, so Previous and Next pass it",
  },
  {
    press: inside(stops, button("Next stop")),
    say: "Next goes to the first stop",
  },
  {
    look: inside(stops, text("Stop 1 of 6")),
    say: "The first stop",
  },

  // 7. The submit bar.
  {
    hover: inside(reviewPane, button("Submit review")),
    say: "Submit review, at the foot: comment, approve or request changes on GitHub",
  },
  {
    press: inside(reviewPane, button("Submit review")),
    say: "It opens his review under the file: the three comments, a summary, and the button that sends it",
  },
  {
    look: inside(yours, role("list", "Pending comments")),
    say: "The three pending comments",
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
    say: "Two findings have no answer yet, so it says so once and waits for a second press",
  },

  // 8. An explanation, run from the head.
  {
    hover: inside(head, button("Run User Impact")),
    say: "The explanations have buttons in the head too, with the same glyphs",
  },
  {
    press: inside(head, button("Run User Impact")),
    say: "Run User Impact",
  },
  {
    press: inside(role("dialog", "User Impact"), button("Run")),
    say: "It asks first",
  },
  {
    look: inside(reader, region("Pane: User Impact")),
    say: "When it is done, its write-up opens in a tab of its own, beside Comments",
  },
  {
    hover: inside(head, button("Open User Impact")),
    say: "Done. A press opens its tab again",
  },

  // 9. Comments, with what sits under no line.
  {
    press: inside(
      reader,
      tab("Comments", {
        exact: true,
      }),
    ),
    say: "Comments",
  },
  {
    look: conversation,
    say: "Conversation comes first: what was said on the pull request under no line",
  },
  {
    look: inside(conversation, role("list", "Threads on whole files")),
    say: "And the threads on a whole file. A press goes to the file in Files",
  },
  {
    look: inside(comments, role("list", "Open threads")),
    say: "The threads on lines follow",
  },

  // 10. Ask, docked under the panes.
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
