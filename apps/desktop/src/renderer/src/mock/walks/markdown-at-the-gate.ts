// What a reviewer, a Drone and the Judge write is drawn as the markdown they
// wrote it in: weight, code spans and lists, not asterisks and backticks. The
// owner's note of 1 Oct 2026. Three walks, because the sites sit on three
// scenarios and a walk plays one.

import { button, role, tab, text, walk } from "../walk";

export const markdownInTheReview = walk("job/review", [
  { press: button("Shape of the change"), say: "Armada's review, at the gate" },
  { look: text("reads column order from its own module"), say: "What an area changed: a list, with code spans" },
  { look: text("re-runs only the selectors that read it"), say: "What the tests prove: bold, not its asterisks" },
  { look: text("the reviewer's to explain"), say: "Why a finding needs you: bold, not its asterisks" },
  { press: button("Dismissed"), say: "The dismissed findings" },
  { look: text("which owns the tests"), say: "Why it was dismissed: bold, not its asterisks" },
  { look: text("2034 of 2034 tests pass"), say: "The Job's record, open under the review. What was done: bold, then a list of code" },
  { look: text("runs after this gate"), say: "What was not checked: a list, with code spans" },
  { look: text("against a filled store"), say: "What was skipped: code and bold" },
]);

export const markdownInAPlanRefusal = walk("arc/plan-revision-refused", [
  { press: tab("Plan"), say: "The plan, where a revision was refused" },
  { press: tab("List"), say: "Its list, with what you asked under it" },
  { look: text("the panel opens empty"), say: "What the Judge says the revision produced: code, bold and a list" },
]);

export const markdownInAJudgeQuestion = walk("epic/wave", [
  { press: role("option", "Say which half refused"), say: "A Judge question, from Needs you" },
  { look: text("not Fleet, which refused it"), say: "What the Judge says it produced: code, bold and a list" },
]);
