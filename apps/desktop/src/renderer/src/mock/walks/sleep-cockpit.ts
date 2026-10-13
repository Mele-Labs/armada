// Sleep mode, reviewed from the Cockpit. Sleep is still on when the owner comes back, and the moon says
// so. The first key or press in the window turns it off. What Sleep decided is a call under "Decided
// while you slept", its acts on the Helm rail: Agree, Change, Open the Session or the Job, Later, and
// Dismiss under its cover. A
// decision from a Session ends in that Session's own box, which is how it is changed; one from a Job
// reveals a plain box. Over `sleep-cockpit`; the night itself is the mock's (`mock/sleep.ts`).

import { button, inside, region, role, text, walk } from "../walk";

const moon = button("Sleep mode", { exact: true });
const cockpit = region("Dashboard");
const lunch = region(/^Decided while you slept: Order the lunch/);
const branch = region(/^Decided while you slept: Name the branch/);
const job = region(/^Decided while you slept: Shorten the reconnect wait/);

const sleepCockpit = walk("sleep-cockpit", [
  { look: moon, say: "Sleep is on: a filled moon on the accent, plain from across the room" },
  { look: region(/^Check failed/), say: "What blocks a Job comes first, as it always does" },
  { key: "l", on: cockpit, say: "The first key in the window turns Sleep off, and puts the blocked Job at the back" },
  { look: lunch, say: "The header says it was decided while he slept; under it the question, and set apart, what Sleep chose" },
  { look: inside(lunch, text("Large")), say: "What Sleep chose" },
  { look: button(/Agree/), say: "On the Helm rail: Agree is the primary and Enter runs it; Change is under it" },
  { look: button(/Open the Session/), say: "Under a rule: Open the Session and Later; then Dismiss, under its cover" },
  { look: inside(lunch, role("textbox", "Message")), say: "A decision from a Session ends in the Session's own box, which is how it is changed" },
  { press: button(/Agree/), say: "One press on Agree, and the card leaves" },
  { look: branch, say: "The next decision" },
  { press: button(/Change/), say: "Change goes to the box" },
  { type: "feat\n", into: inside(branch, role("textbox", "Message")), say: "Enter sends the correction to the Session, and the card leaves" },
  { look: job, say: "A decision from a Job opens the Job, and has no Session box" },
  { press: button(/Change/), say: "Change reveals a plain box here" },
  { look: inside(job, role("textbox", "Change")), say: "The box, focused" },
  { key: "Escape", on: inside(job, role("textbox", "Change")), say: "Esc puts it away" },
  { press: button(/Dismiss/), say: "Dismiss is under a cover: one press lifts it" },
  { press: button(/Dismiss/), say: "A second dismisses it for good" },
]);

const opening = walk("sleep-cockpit", [
  { key: "l", on: cockpit, say: "Any key turns Sleep off" },
  { look: lunch, say: "A decision made in a Session" },
  { press: button(/Open the Session/), say: "One press on Open the Session goes to where it was decided" },
  { look: inside(region("Thread"), text("Which size?")), say: "The Session, on the question Sleep answered" },
]);

export { sleepCockpit, opening as sleepCockpitOpensTheSession };
