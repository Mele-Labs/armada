// Sleep mode, reviewed from the Cockpit. Sleep is still on when the owner comes back, and the moon says
// so. The first key or press in the window turns it off. What Sleep decided is a call in the deck: Done,
// or Open the Session it was decided in, or a correction in words, or Dismiss; the card leaves each
// time. Over `sleep-cockpit`; the night itself is the mock's (`mock/sleep.ts`).

import { button, inside, region, role, text, walk } from "../walk";

const moon = button("Sleep mode", { exact: true });
const cockpit = region("Dashboard");
const lunch = region(/^Sleep: Order the lunch/);
const branch = region(/^Sleep: Name the branch/);
const job = region(/^Sleep: Job 44/);

const sleepCockpit = walk("sleep-cockpit", [
  { look: moon, say: "Sleep is on: a filled moon on the accent, plain from across the room" },
  { look: region(/^Check failed/), say: "What blocks a Job comes first, as it always does" },
  { key: "l", on: cockpit, say: "The first key in the window turns Sleep off, and puts the blocked Job at the back" },
  { look: lunch, say: "A decision Sleep made, as a call: who, what was asked, what it chose" },
  { look: inside(lunch, text("Large")), say: "What Sleep picked" },
  { look: inside(lunch, role("radio", /Done/)), say: "Done is first and already picked, so Enter finishes it" },
  { look: inside(lunch, role("radio", /Open the Session/)), say: "Open the Session goes to where it was decided" },
  { look: inside(lunch, button(/Dismiss/)), say: "Dismiss marks it reviewed for good" },
  { key: "Enter", on: cockpit, say: "Enter is Done, and the card leaves" },
  { look: branch, say: "The next decision" },
  { type: "feat\n", into: inside(branch, role("textbox", "Change")), say: "Changing it: the words go to the agent, and the card leaves" },
  { look: job, say: "A decision made in a Job has no Session to open" },
  { press: inside(job, button(/Dismiss/)), say: "Dismissed: reviewed, and it never comes back" },
]);

const opening = walk("sleep-cockpit", [
  { key: "l", on: cockpit, say: "Any key turns Sleep off" },
  { look: lunch, say: "A decision made in a Session" },
  { press: inside(lunch, role("radio", /Open the Session/)), say: "Open the Session picked" },
  { key: "Enter", on: cockpit, say: "Enter opens the Session the decision was made in" },
  { look: inside(region("Thread"), text("Which size?")), say: "The Session, on the question Sleep answered" },
]);

export { sleepCockpit, opening as sleepCockpitOpensTheSession };
