// Words sent to any agent behave the same way. In Helm's dock and under a Drone's log the words are in the thread at once, with the
// mark a Session's message has: the whole row fades while Fleet has not taken it, the mark turns and says Sending, and a refusal
// leaves a cross that says Not sent, a Retry and a toast. The first message of a repository has no Session and so no thread: the
// words stand where the thread will be. Over `sending-everywhere`, where Fleet waits on each step to let it go.

import { button, inside, role, tab, text, walk } from "../walk";

const helm = role("complementary", "Helm");
const box = inside(helm, role("textbox", "Ask Helm"));
const thread = inside(helm, role("region", "Thread"));
const drone = role("dialog", "This Job's Drone");

const sendingEverywhere = walk("sending-everywhere", [
  // The first message of a repository.
  { press: button("Helm"), say: "Helm's dock. This repository has no Helm Session yet, so there is no thread" },
  { type: "why did Job 12 fail?", into: box, say: "The first message here" },
  { key: "Enter", on: box, say: "Sent. The box is clear" },
  { look: inside(thread, role("region", "Your message")), say: "The words are where the thread will be, as a Session's message looks" },
  { hover: inside(thread, role("img", "Sending")), say: "A turning mark says Sending, and the message fades and returns while Fleet has not taken it" },
  { later: inside(thread, role("img", "Sending")), say: "Fleet takes it. Helm's Session starts" },
  { look: inside(thread, text(/stopped on step 4 of 6/)), say: "Helm's reply. The message has not moved and is there once" },

  // A message Fleet refuses.
  { type: "what stopped Job 12?", into: box, say: "A second message, which Fleet will refuse" },
  { key: "Enter", on: box, say: "Sent" },
  { look: inside(thread, role("img", "Sending")), say: "Sending, in the thread of Helm's Session this time" },
  { later: inside(thread, role("img", "Sending")), say: "Fleet does not take it" },
  { look: inside(thread, role("img", "Not sent")), say: "A cross says Not sent. The message stays" },
  { look: inside(thread, button("Retry")), say: "Retry sits under it" },
  { look: text(/^Message to Helm not sent/), say: "He is told, in a toast" },
  { press: inside(thread, button("Retry")), say: "Retry sends the same words again" },
  { look: inside(thread, role("img", "Sending")), say: "Sending, and Retry is gone while it goes" },
  { later: inside(thread, role("img", "Sending")), say: "Fleet takes it this time" },

  // Words Helm sends on to a Session.
  { type: "I wish that the sessions page would move sessions into a new status that says they are just waiting for work to land", into: box, say: "A thought for a running Session" },
  { key: "Enter", on: box, say: "Sent. Helm reads where it belongs" },
  { look: inside(thread, role("img", "Sending")), say: "The words are in the thread at once, going, and no card stands in for them" },
  { look: inside(helm, role("group", "Send to", { exact: true })), say: "A Session fits. The words stay going until they are sent" },
  { key: "Enter", on: box, say: "Press Enter" },
  { later: inside(thread, role("img", "Sending")), say: "Fleet takes the words to the Session" },
  { look: inside(thread, button("Sent to Session Sessions list views")), say: "The words have not moved. A line under them says where they went, and pressing it opens the Session" },

  // A redirect to a Drone.
  { press: inside(helm, button("Close")), say: "Helm put away, on a Job with a Drone working" },
  { press: tab("Drones"), say: "Its Drones" },
  { press: button("This Job's Drone"), say: "The one working now" },
  { type: "Keep the old export so nothing else has to move.", into: inside(drone, role("textbox", "Message the drone")), say: "A redirect to it" },
  { press: inside(drone, button("Send")), say: "Send. The words are above the box at once, where a waiting sentence used to be" },
  { look: inside(drone, role("img", "Sending")), say: "The same row, the same mark: Sending" },
  { later: inside(drone, role("img", "Sending")), say: "Fleet takes it" },
  { look: inside(drone, text("Keep the old export so nothing else has to move.")), say: "Delivered: the row stays, at rest, with no mark" },
]);

export { sendingEverywhere as "sending-everywhere" };
