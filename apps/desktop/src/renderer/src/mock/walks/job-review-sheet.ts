// One sheet for what a Job has to read: a plan, a sketch, a draft, a document and a walk. The Review row on its Needs you card opens it on the oldest item, the plan. ⌘↵ approves
// that plan alone, and the next open item comes up. ↓ moves to the document, ⌥R sends it back with a note, the draft is approved from the head, where its Approve is named like the rest,
// and the walk plays in the sheet's pane. The question and the command are answered on the card. Over the `job-review-sheet` scenario.

import { button, dialog, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Jobs", {
  exact: true,
});
const sheet = dialog("Retry backoff cap");
const list = inside(sheet, role("group", "Waiting on you"));
const row = (name: string | RegExp) => inside(list, button(name));
const note = inside(sheet, role("textbox", "Note"));
const needs = region("Needs you");
const review = inside(
  needs,
  button("Review", {
    exact: true,
  }),
);

const sheetWalk = walk("job-review-sheet", [
  toSessions,
  {
    press: inside(sessions, button(/Retry backoff cap/)),
    say: "The Job",
  },
  {
    look: inside(needs, button(/Open Keep the old fixture/)),
    say: "The question and the command are on the card",
  },
  {
    press: review,
    say: "Review opens what he reads",
  },
  {
    look: inside(sheet, region("Plan v2")),
    say: "The oldest is the plan",
  },
  {
    press: row(/Plan v2/),
    say: "Plan v2 is read in the sheet",
  },
  {
    look: inside(sheet, button(/^Approve Plan v2/)),
    say: "The button names what it approves",
  },
  {
    key: "Meta+Enter",
    on: row(/Plan v2/),
    say: "⌘↵ approves Plan v2 and nothing else",
  },
  {
    look: inside(sheet, region("Backoff curve")),
    say: "The next open item comes up",
  },
  {
    key: "ArrowDown",
    on: row(/Backoff curve/),
    say: "↓ draws the next item at once",
  },
  {
    look: inside(sheet, button(/^Approve Pull request text/)),
    say: "The draft's Approve is in the head, and names it",
  },
  {
    key: "ArrowDown",
    on: row(/Pull request text/),
    say: "↓ again, to the document",
  },
  {
    look: inside(sheet, region("Retry design")),
    say: "The document",
  },
  {
    key: "Alt+r",
    on: row(/Retry design/),
    say: "⌥R opens the note",
  },
  {
    type: "Say why the cap is thirty seconds.",
    into: note,
    say: "The note",
  },
  {
    key: "Meta+Enter",
    on: note,
    say: "⌘↵ sends it back",
  },
  {
    press: row(/Pull request text/),
    say: "The draft",
  },
  {
    press: inside(sheet, button(/^Approve/)),
    say: "Approved from its own card",
  },
  {
    press: row(/Backoff table/),
    say: "The walk",
  },
  {
    look: inside(sheet, button(/^Approve/)),
    say: "It plays in the sheet's pane",
  },
]);

export { sheetWalk as "job-review-sheet" };
