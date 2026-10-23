// A question with long option descriptions in Needs you: Back, the question and Answer stay in view, and the options scroll between them. Over `job-long-question`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const needs = region("Needs you");
const choice = (name: string) => inside(needs, role("radio", name, { exact: true }));

export const jobLongQuestion = walk("job-long-question", [
  { press: deckRow("Retry the slot lease"), say: "The Job is open, and its question is in Needs you" },
  { look: choice("Back off in Fleet"), say: "The first option, with its description in full" },
  { look: inside(needs, button("Answer")), say: "Answer stays in view under the options" },
  { look: choice("Hand the bay to the second Job"), say: "The last option, reached by scrolling the options alone" },
  { look: inside(needs, button("Answer")), say: "Answer has not moved" },
]);
