// The Judge did not answer on the plan step and the Job stopped (owner, Job 3,
// 5 Oct 2026). Four walks over one scenario: what Overview says, the step open
// on its canvas, accepting the step, and asking the Judge again.

import { button, dialog, inside, role, text, walk } from "../walk";

const accepting = dialog("Accept this step");

export const theJudgeDidNotAnswer = walk("judge/undecided", [
  { look: role("heading", "The Judge did not answer"), say: "Plain words for what stopped it" },
  { look: text("The work was not judged. Ask again, or accept the step yourself."), say: "What happened, and what is safe" },
  { look: button("Accept this step"), say: "Go on without the Judge" },
  { look: button("Ask the Judge again"), say: "Run the Judge once more on the same evidence" },
  { look: button("Redirect drone"), say: "Or tell the Drone something first" },
  { look: button("Read what stopped it"), say: "The Record has the detail" },
]);

export const theStepTheJudgeSkipped = walk("judge/undecided", [
  { press: role("button", /^Plan the change/), say: "Open the stopped step on the canvas" },
  { look: button("Accept this step"), say: "The same two acts sit in the step's head" },
  { look: button("Ask the Judge again"), say: "Nothing is redone by asking again" },
  { look: text("plan_recorded"), say: "Its Check passed" },
]);

export const acceptingAStepNobodyJudged = walk("judge/undecided", [
  { press: button("Accept this step"), say: "Accept the plan yourself" },
  { look: inside(accepting, text(/the work was not judged/i)), say: "It says what you are signing for" },
  { look: inside(accepting, button("Accept this step")), say: "No reason is required" },
  { press: inside(accepting, button("Accept this step")), say: "The step advances" },
  { look: text(/Accepted|advanced/i), say: "The Job carries on at the next step" },
]);

export const askingTheJudgeAgain = walk("judge/undecided", [
  { press: button("Ask the Judge again"), say: "Ask again on the evidence already submitted" },
  { look: button("Accept this step"), say: "The Judge is still silent, so the step stays put" },
]);
