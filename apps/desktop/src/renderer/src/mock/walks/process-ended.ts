// A Job whose process ended in a turn lists it in Needs you. While Fleet starts the process again the item has nothing to press;
// with nothing coming it offers Start again, and the press hands the Job its turn back. Over `process-ended`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const needs = region("Needs you");

const processEnded = walk("process-ended", [
  {
    look: deckRow("Add invoice export"),
    say: "Fleet is starting this one again",
  },
  {
    look: deckRow("Fix the login redirect"),
    say: "Nothing is starting this one",
  },
  {
    press: deckRow("Fix the login redirect"),
    say: "Open it",
  },
  {
    press: inside(needs, button(/^Needs you/)),
    say: "Needs you",
  },
  {
    look: inside(needs, text("Process ended. Not started again.")),
    say: "The process ended and was not started again",
  },
  {
    press: inside(
      needs,
      role("radio", "Start again", {
        exact: true,
      }),
    ),
    say: "Start again",
  },
  {
    press: inside(
      needs,
      button("Answer", {
        exact: true,
      }),
    ),
    say: "Answer",
  },
  {
    look: inside(needs, text("Process ended. Starting again.")),
    say: "Fleet starts it, and the item has nothing left to press",
  },
]);

export { processEnded as "process-ended" };
