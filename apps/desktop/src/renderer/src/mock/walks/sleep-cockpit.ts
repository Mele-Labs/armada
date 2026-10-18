// Sleep mode, reviewed from the Cockpit. Sleep is still on when the owner comes back, and the moon on the
// Menu button says so. What Sleep decided is a call, "Decided while you slept", and
// the key that brings the next call forward opens it in the stage: the question, what Sleep chose set apart, and its acts: Agree,
// Change, Open the Session, Later and Dismiss. A decision from a Session ends
// in that Session's own box, which is how it is changed. Over
// `sleep-cockpit`; the night itself is the mock's (`mock/sleep.ts`).

import { button, inside, region, role, text, walk } from "../walk";

const moon = button("Menu", { exact: true });
const list = region("Work", { exact: true });
const lunch = region(/^Decided while you slept: Order the lunch/);
const branch = region(/^Decided while you slept: Name the branch/);

const sleepCockpit = walk("sleep-cockpit", [
  { look: moon, say: "Sleep is on: the moon rides on the Menu button" },
  { key: "w", on: list, say: "w brings the next call forward: a decision Sleep made" },
  { look: lunch, say: "The header says it was decided while he slept; under it the question, and set apart, what Sleep chose" },
  { look: inside(lunch, text("Large")), say: "What Sleep chose" },
  { look: button(/Agree/), say: "Agree is the primary and Enter runs it; Change is beside it" },
  { look: button(/Open the Session/), say: "Open the Session and Later; then Dismiss" },
  { look: inside(lunch, role("textbox", "Message")), say: "A decision from a Session ends in the Session's own box, which is how it is changed" },
  { press: button(/Agree/), say: "One press on Agree, and the card leaves" },
  { key: "w", on: list, say: "w again, for the next decision" },
  { look: branch, say: "A second decision, from another Session" },
  { press: button(/Change/), say: "Change goes to the box" },
  { type: "feat\n", into: inside(branch, role("textbox", "Message")), say: "Enter sends the correction to the Session, and the card leaves" },
]);

const opening = walk("sleep-cockpit", [
  { key: "w", on: list, say: "w brings the next call forward: a decision made in a Session" },
  { look: lunch, say: "The call, in the stage" },
  { press: button(/Open the Session/), say: "One press on Open the Session goes to where it was decided" },
  { look: inside(region("Thread"), text("Which size?")), say: "The Session, on the question Sleep answered" },
]);

export { sleepCockpit, opening as sleepCockpitOpensTheSession };
