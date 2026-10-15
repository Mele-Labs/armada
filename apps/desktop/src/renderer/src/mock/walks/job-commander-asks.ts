// What a Job's Commander cannot do, put to the owner as one card: Group 3 is red on a Check no change of
// the Job's can turn green, and a Check is the owner's to skip or edit. The card is where a Job's cards
// already are, the Dashboard's Your move and the Job's Now panel. His answer wakes it, and it carries on.
// Over `job-commander-asks`.

import { button, inside, place, region, role, text, walk } from "../walk";

const commander = role("region", "Commander", { exact: true });
const now = region("Now");
const card = role("region", /^Commander asks: desktop_test/);

const w = walk("job-commander-asks", [
  { press: place("Cockpit"), say: "The Dashboard" },
  { look: role("region", /^Commander asks/), say: "The card is a call like any other, from the Commander" },
  { press: button("Open Job"), say: "Open the Job" },
  { look: inside(now, card), say: "The same card, in the Now panel's asks" },
  { look: inside(commander, text(/not mine to do/)), say: "The thread says why it is out of its reach" },
  { press: inside(card, role("radio", "Skip desktop_test on Group 3")), say: "The owner picks" },
  { press: inside(card, button("Answer")), say: "Answer" },
  { look: inside(commander, text("The owner answered your card")), say: "His answer wakes it" },
  { look: inside(commander, text("T9 restarted")), say: "It restarts what it may" },
  { look: inside(now, role("img", "Commander")), say: "The card is gone and the Drone on the task is marked" },
]);

export { w as "job-commander-asks" };
