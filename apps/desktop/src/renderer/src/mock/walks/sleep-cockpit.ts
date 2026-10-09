// Sleep mode, reviewed from the Cockpit: what Sleep decided overnight is a call in the deck, kept with
// Enter or changed in words, and the card leaves. Over `sleep-cockpit`; the night itself is the mock's
// (`mock/sleep.ts`).

import { button, inside, region, role, tab, text, walk } from "../walk";

const moon = button("Sleep mode", { exact: true });
const cockpit = region("Dashboard");
const lunch = region(/^Sleep: Order the lunch/);
const branch = region(/^Sleep: Name the branch/);

const sleepCockpit = walk("sleep-cockpit", [
  { look: tab("Your move"), say: "Your move: a Job blocked on a Check, and nothing from Sleep yet" },
  { press: moon, say: "Sleep on, the work carries on" },
  { press: moon, say: "Sleep off: the night's decisions arrive in the deck" },
  { look: region(/^Check failed/), say: "What blocks a Job comes first, as it always does" },
  { key: "l", on: cockpit, say: "l puts it at the back" },
  { look: lunch, say: "A decision Sleep made, as a call: who, what was asked, what it chose" },
  { look: inside(lunch, text("Large")), say: "What Sleep picked" },
  { key: "Enter", on: cockpit, say: "Enter keeps it, and the card leaves" },
  { look: branch, say: "The next decision" },
  { type: "feat\n", into: inside(branch, role("textbox", "Change")), say: "Changing it: the words go to the agent, and the card leaves" },
  { look: region(/^Sleep: Job 44/), say: "The next decision comes forward" },
]);

export { sleepCockpit };
