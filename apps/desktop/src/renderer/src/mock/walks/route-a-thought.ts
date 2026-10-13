// Helm routes a thought into running work. Every message typed in the dock is read first: a question
// gets Helm's reply, and a thought meant for other work gets a card. A match offers Send (↵), No
// (N) and Ask Helm (H); nothing matched, or No, offers a new Session (S) or Job (J) in the Dispatch
// panel, holding the words. Sessions and Jobs of every repository are searched. Thoughts sent while
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
  { look: inside(helm, role("status", "Finding where this goes")), say: "Helm is finding where it belongs" },
  { look: card("Send to"), say: "A running Session fits. Send is ↵, No is N, Ask Helm is H" },
  { hover: inside(card("Send to"), role("img", "Session")), say: "The kind, named on hover" },
  { key: "Enter", on: box, say: "Press ↵" },
  { look: inside(helm, button("Sent to Session Sessions list views")), say: "The words stay in the thread, with a chip for where they went. Pressing it opens the Session" },

  // (b) A thought nothing running is about: start something new.
  { type: "the merge line page should group PRs by repository", into: box, say: "A thought about the merge line, which nothing running is about" },
  { key: "Enter", on: box, say: "Sent" },
  { look: card("Start new"), say: "No match, so the choice is a new Session (S) or a new Job (J). Esc dismisses it" },
  { key: "s", on: box, say: "Press S" },
  { look: panel(tab("Session")), say: "The Dispatch panel opens holding the words, with Session picked" },
  { look: panel(role("textbox", "Request")), say: "↵ there starts it" },
  { key: "Escape", on: panel(role("textbox", "Request")), say: "Esc puts the panel away" },

  // (c) A thought with a match, declined: No, then a Job.
  { type: "retries should back off longer after the third failed charge", into: box, say: "A thought a running Job in billing is about" },
  { key: "Enter", on: box, say: "Sent" },
  { look: card("Send to"), say: "The match is in another repository, and the card names it" },
  { key: "n", on: box, say: "Press N" },
  { look: card("Start new"), say: "No asks for a new Session or Job instead" },
  { key: "j", on: box, say: "Press J" },
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
  { type: "why did Job 12 fail?", into: box, say: "A question for Helm" },
  { key: "Enter", on: box, say: "Sent" },
  // "Job 12" is a link to the Job, so the reply's sentence is read from the words after it.
  { look: inside(helm, text("stopped on step 4 of 6")), say: "Helm's reply, as for any ask. No card" },
]);

export { routeAThought as "route-a-thought" };
