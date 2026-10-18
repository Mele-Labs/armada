// A Session whose agent asks the person questions, one at a time as a deck: a single choice and a
// multi-select, answered with a typed Other, and a second Session whose question is skipped. Over the `session-question`
// scenario. Told twice: wide, and below the breakpoint.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { rail } = kit(narrow);
  const sessions = region("Jobs", { exact: true });
  const ask = role("article", "Waiting on you");
  const answer = inside(ask, button("Answer"));
  return [
    { press: rail("Sessions"), say: "The Jobs page" },
    { press: inside(sessions, button(/Order the lunch/)), say: "A Job that is waiting on an answer" },
    { press: inside(region("Needs you"), button("Open Which size?")), say: "The panel lists the question, and a press opens it" },
    { look: inside(ask, text("Size")), say: "The agent's questions, in the panel over the thread" },
    { look: inside(ask, text("Which size?")), say: "One question at a time, the next behind it" },
    { look: inside(ask, role("radio", "Large")), say: "Each option on its row, with what it means" },
    { press: inside(ask, role("radio", "Large")), say: "Large, and the next comes forward" },
    { look: inside(ask, text("Which toppings?")), say: "A multi-select takes any number" },
    { press: inside(ask, role("checkbox", "Cheese")), say: "Cheese" },
    { press: inside(ask, role("checkbox", "Olives")), say: "And olives" },
    { press: inside(ask, role("checkbox", "Other")), say: "Other is the person's own words" },
    { type: "extra napkins", into: inside(ask, role("textbox", "Other")), say: "Typed in" },
    { press: answer, say: "Answered" },
    { look: text(/Going with Large/), say: "The agent carries on from what was chosen" },
    { press: rail("Sessions"), say: "Back to the list" },
    { press: inside(sessions, button(/Name the branch/)), say: "A Job with one question" },
    { press: inside(region("Needs you"), button("Open Which prefix?")), say: "Opened from the list" },
    { look: inside(ask, role("radio", "fix")), say: "Two options" },
    { press: inside(ask, button("Skip")), say: "Skipped" },
    { look: text("Skipped, so I will use the defaults."), say: "It was told, and took the defaults" },
  ];
}

const wide = walk("session-question", [toSessions, ...steps(false)]);
const narrow = walk("session-question", [toSessions, ...steps(true)], NARROW);

export { wide as "session-question", narrow as "session-question-narrow" };
