// A Session whose agent asks the person questions: a single choice and a multi-select, answered
// with a typed Other, and a second Session whose question is skipped. Over the `session-question`
// scenario. Told twice: wide, and below the breakpoint.

import { kit, NARROW } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { rail } = kit(narrow);
  const sessions = region("Sessions");
  const ask = role("article", "Waiting on you");
  const answer = inside(ask, button("Answer"));
  return [
    { press: rail("Sessions"), say: "The Sessions page" },
    { press: inside(sessions, button(/Order the lunch/)), say: "A Session that is waiting on an answer" },
    { look: inside(ask, text("Question")), say: "The agent's questions, under the thread" },
    { look: inside(ask, text("Which size?")), say: "A single choice, one option at a time" },
    { hover: inside(ask, role("radio", "Large")), say: "What each option means is on hover" },
    { look: inside(ask, text("Which toppings?")), say: "A multi-select takes any number" },
    { look: answer, say: "Answer waits until each question has something" },
    { press: inside(ask, role("radio", "Large")), say: "Large" },
    { press: inside(ask, role("checkbox", "Cheese")), say: "Cheese" },
    { press: inside(ask, role("checkbox", "Olives")), say: "And olives" },
    { press: inside(ask, role("checkbox", "Other")), say: "Other is the person's own words" },
    { type: "extra napkins", into: inside(ask, role("textbox", "Other")), say: "Typed in" },
    { press: answer, say: "Answered" },
    { look: text(/Going with Large/), say: "The agent carries on from what was chosen" },
    { press: rail("Sessions"), say: "Back to the list" },
    { press: inside(sessions, button(/Name the branch/)), say: "A Session with one question" },
    { look: inside(ask, role("radio", "fix")), say: "Two options" },
    { press: inside(ask, button("Skip")), say: "Skipped" },
    { look: text("Skipped, so I will use the defaults."), say: "It was told, and took the defaults" },
  ];
}

const wide = walk("session-question", steps(false));
const narrow = walk("session-question", steps(true), NARROW);

export { wide as "session-question", narrow as "session-question-narrow" };
