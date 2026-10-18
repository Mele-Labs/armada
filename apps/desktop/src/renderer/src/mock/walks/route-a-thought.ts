// Helm routes a thought into running work. Every message typed in the dock is in the thread at once and read: a question
// gets Helm's reply, and a thought meant for other work gets a card. A match offers Send (↵), No
// (N) and Ask Helm (H); nothing matched, or No, offers a new Session (S) in the Dispatch
// panel, holding the words. Sessions of every repository are searched. Thoughts sent while
// another is showing queue behind it, one card at a time in the order typed. Over `route-a-thought`.

import { button, dialog, inside, role, tab, text, walk } from "../walk";

const helm = role("complementary", "Helm");
const box = inside(helm, role("textbox", "Ask Helm"));
const card = (name: string) => inside(helm, role("group", name, { exact: true }));
const panel = (target: ReturnType<typeof role>) => inside(dialog("Dispatch"), target);

const routeAThought = walk("route-a-thought", [
  { press: button("Helm"), say: "Helm's dock" },

  // (a) A thought with a match: the Session doing the Sessions page.
  { type: "I wish that the sessions page would move sessions into a new status that says they are just waiting for work to land", into: box, say: "A thought, typed like any other message" },
  { key: "Enter", on: box, say: "Sent with ↵. Helm reads it before it answers" },
  { look: inside(helm, role("img", "Sending")), say: "The words are in the thread at once, going, while Helm finds where they belong" },
  { look: card("Send to"), say: "A running Job fits. Send is ↵, No is N, Ask Helm is H" },
  { hover: inside(card("Send to"), role("img", "Job")), say: "The kind, named on hover" },
  { key: "Enter", on: box, say: "Press ↵" },
  { look: inside(helm, button("Sent to Job Sessions list views")), say: "The words stay in the thread, with a chip for where they went. Pressing it opens the Job" },

  // (b) A thought nothing running is about: start something new.
  { type: "the merge line page should group PRs by repository", into: box, say: "A thought about the merge line, which nothing running is about" },
  { key: "Enter", on: box, say: "Sent" },
  { look: card("Start new"), say: "No match, so the choice is a new Job (S). Esc dismisses it" },
  { key: "s", on: box, say: "Press S" },
  { look: panel(tab("Job")), say: "The Dispatch panel opens holding the words, with Job picked" },
  { look: panel(role("textbox", "Request")), say: "↵ there starts it" },
  { key: "Escape", on: panel(role("textbox", "Request")), say: "Esc puts the panel away" },

  // (c) A thought with a match, declined: No, then a Session.
  { type: "retries should back off longer after the third failed charge", into: box, say: "A thought a running Job in billing is about" },
  { key: "Enter", on: box, say: "Sent" },
  { look: card("Send to"), say: "The match is in another repository, and the card names it" },
  { key: "n", on: box, say: "Press N" },
  { look: card("Start new"), say: "No asks for a new Job instead" },
  { key: "s", on: box, say: "Press S" },
  { look: panel(tab("Job")), say: "The Dispatch panel opens holding the words, with Job picked" },
  { key: "Escape", on: panel(role("textbox", "Request")), say: "Esc puts the panel away" },

  // (d) Two thoughts back to back: the second waits behind the first, and shows once the first is answered.
  { type: "retries should back off longer after the third failed charge", into: box, say: "A thought a running Job in billing is about" },
  { key: "Enter", on: box, say: "Sent" },
  { type: "the merge line page should group PRs by repository", into: box, say: "A second thought, typed while the first is still being read" },
  { key: "Enter", on: box, say: "Sent. It is read as well, and waits its turn" },
  { look: card("Send to"), say: "The first card shows first, in the order typed" },
  { key: "Escape", on: box, say: "Esc puts the first away" },
  { look: card("Start new"), say: "The second card shows now" },
  { key: "Escape", on: box, say: "Esc puts it away too" },

  // (e) A question: Helm answers, and no card is drawn.
  { type: "why did #12 fail?", into: box, say: "A question for Helm" },
  { key: "Enter", on: box, say: "Sent" },
  { look: inside(helm, text("stopped on step 4 of 6")), say: "Helm's reply, as for any ask. No card" },
]);

export { routeAThought as "route-a-thought" };
