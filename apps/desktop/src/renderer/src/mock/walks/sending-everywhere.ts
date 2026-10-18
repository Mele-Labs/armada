// Words sent to any agent behave the same way. In Helm's dock the words are in the thread at once, with the
// mark a Session's message has: the whole row fades while Fleet has not taken it, the mark turns and says Sending, and a refusal
// leaves a cross that says Not sent, a Retry and a toast. The first message of a repository has no Session and so no thread: the
// words stand where the thread will be. Over `sending-everywhere`, where Fleet waits on each step to let it go.

import { button, inside, role, text, walk } from "../walk";

const helm = role("complementary", "Helm");
const box = inside(helm, role("textbox", "Ask Helm"));
const thread = inside(helm, role("region", "Thread"));

const sendingEverywhere = walk("sending-everywhere", [
  // The first message of a repository.
  { press: button("Helm"), say: "Helm's dock. This repository has no Helm Job yet, so there is no thread" },
  { type: "why did the store migration spike stop?", into: box, say: "The first message here" },
  { key: "Enter", on: box, say: "Sent. The box is clear" },
  { look: inside(thread, role("region", "Your message")), say: "The words are where the thread will be, as a Job's message looks" },
  { hover: inside(thread, role("img", "Sending")), say: "A turning mark says Sending, and the message fades and returns while Fleet has not taken it" },
  { later: inside(thread, role("img", "Sending")), say: "Fleet takes it. Helm's Job starts" },
  { look: inside(thread, text(/Store migration spike is idle/)), say: "Helm's reply. The message has not moved and is there once" },

  // A message Fleet refuses.
  { type: "what is it waiting on?", into: box, say: "A second message, which Fleet will refuse" },
  { key: "Enter", on: box, say: "Sent" },
  { look: inside(thread, role("img", "Sending")), say: "Sending, in the thread of Helm's Job this time" },
  { later: inside(thread, role("img", "Sending")), say: "Fleet does not take it" },
  { look: inside(thread, role("img", "Not sent")), say: "A cross says Not sent. The message stays" },
  { look: inside(thread, button("Retry")), say: "Retry sits under it" },
  { look: text(/^Message to Helm not sent/), say: "He is told, in a toast" },
  { press: inside(thread, button("Retry")), say: "Retry sends the same words again" },
  { look: inside(thread, role("img", "Sending")), say: "Sending, and Retry is gone while it goes" },
  { later: inside(thread, role("img", "Sending")), say: "Fleet takes it this time" },

  // Words Helm sends on to a Session.
  { type: "I wish that the sessions page would move sessions into a new status that says they are just waiting for work to land", into: box, say: "A thought for a running Job" },
  { key: "Enter", on: box, say: "Sent. Helm reads where it belongs" },
  { look: inside(thread, role("img", "Sending")), say: "The words are in the thread at once, going, and no card stands in for them" },
  { look: inside(helm, role("group", "Send to", { exact: true })), say: "A Job fits. The words stay going until they are sent" },
  { key: "Enter", on: box, say: "Press Enter" },
  { later: inside(thread, role("img", "Sending")), say: "Fleet takes the words to the Job" },
  { look: inside(thread, button("Sent to Job Sessions list views")), say: "The words have not moved. A line under them says where they went, and pressing it opens the Job" },
]);

export { sendingEverywhere as "sending-everywhere" };
