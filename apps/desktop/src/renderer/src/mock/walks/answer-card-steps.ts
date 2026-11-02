// A question with choices that also carries steps draws the steps in its Needs you card, numbered, under the
// question and above the choices, each command with its own copy and Run. The choices answer as they did. Over `answer-card-steps`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const needs = region("Needs you");

const answerCardSteps = walk("answer-card-steps", [
  {
    press: deckRow(/Trace the missing session id/),
    say: "A Job waits on a question that comes with steps",
  },
  {
    press: inside(needs, button(/^Needs you/)),
    say: "Its card is in Needs you",
  },
  {
    look: inside(needs, role("list")),
    say: "The steps are numbered under the question",
  },
  {
    look: inside(needs, button("Copy step 3 command")),
    say: "A command in a step has its own copy and Run",
  },
  {
    look: inside(
      needs,
      role("radio", "Sent", {
        exact: true,
      }),
    ),
    say: "The choices are under the steps",
  },
]);

export { answerCardSteps as "answer-card-steps" };
