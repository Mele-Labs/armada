// A message sent to a Session is in its thread from the press, with a mark that Fleet has not taken it yet. The mark goes when Fleet
// takes it, and Fleet's own row takes the message's place without the message drawing again. A message Fleet did not take stays,
// marked, with a Retry, and he is told. Over `session-message-sending`, where Fleet waits on each step to let it go.

import { button, inside, region, role, text, walk } from "../walk";

const list = region("Jobs", { exact: true });
const session = region(/^Job: Pin the store clock/);
const thread = inside(session, region("Thread"));
const box = inside(session, role("textbox", "Message"));

const sessionMessageSending = walk("session-message-sending", [
  { press: inside(list, button("Pin the store clock")), say: "A Job open on the stage, with its own box at the foot of the thread" },
  { type: "Also tidy the retry cap.", into: box, say: "A message to the Job" },
  { press: inside(session, button("Send")), say: "Send. The message is in the thread at once and the box is clear, while Fleet has not yet taken it" },
  { look: inside(thread, text("Also tidy the retry cap.")), say: "The message, where it will stay" },
  { hover: inside(thread, role("img", "Sending")), say: "A turning mark on its band says Sending. It is the same mark an open ring means everywhere else: under way" },
  { later: inside(thread, role("img", "Sending")), say: "Fleet takes the message a moment later" },
  { look: inside(thread, text("Also tidy the retry cap.")), say: "The mark has gone. The message has not moved, and it is not there twice: Fleet's own row took its place" },
  { type: "Raise it to five.", into: box, say: "A second message" },
  { press: inside(session, button("Send")), say: "Send again. Fleet will refuse this one" },
  { look: inside(thread, role("img", "Sending")), say: "Sending, as before" },
  { later: inside(thread, text("Raise it to five.")), say: "Fleet does not take it" },
  { look: inside(thread, role("img", "Not sent")), say: "A cross on the band says Not sent, and the message stays in the thread" },
  { look: inside(thread, button("Retry")), say: "Retry sits under it. The box has nothing to take back, so a message he has already moved on from is not pulled back into it" },
  { look: text(/^Message to Pin the store clock not sent/), say: "He is told, in a toast" },
  { press: inside(thread, button("Retry")), say: "Retry sends the same message again" },
  { look: inside(thread, role("img", "Sending")), say: "Sending, and Retry is gone while it goes" },
  { later: inside(thread, text("Raise it to five.")), say: "Fleet takes it this time" },
  { look: inside(thread, text("Raise it to five.")), say: "The mark is gone and the message is in the thread once" },
]);

export { sessionMessageSending as "session-message-sending" };
