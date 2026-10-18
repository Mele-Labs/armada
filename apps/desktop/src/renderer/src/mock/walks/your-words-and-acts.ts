// The person's own words and acts in a Session's thread: a message under a band of its own, and
// what they did (an Approve, an answer to a waiting item) as one filled line. Over the
// `session-your-words-and-acts` scenario.

import { button, inside, region, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");

const yourWordsAndActs = walk("session-your-words-and-acts", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Store clock reply/)), say: "A thread with the agent, and then the person" },
  { look: inside(thread, region("Your message")), say: "The person's message, under a band of its own" },
  { look: inside(thread, region("You approved")), say: "An Approve, as one line with the page it was on" },
  { look: inside(thread, region("You answered")), say: "An answer, with the item it answered" },
]);

export { yourWordsAndActs as "your-words-and-acts" };
