// A response pasted into Dispatch that is too big to send as words waits as a snippet chip and goes to the Job as a file, so the agent reads
// as much of it as it needs. The words stay the person's own. Over `session-after-another`, whose repository is chosen.

import { button, inside, role, text, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const attached = inside(frame, role("group", "Attached"));

/** About 60 000 characters of an API response: far past what is worth sending as words. */
const RESPONSE = JSON.stringify({ data: Array.from({ length: 800 }, (_, at) => ({ id: at, status: "settled", amount: 1250 + at, currency: "USD", memo: "monthly statement line" })) });

const dispatchPasteTooBig = walk("session-after-another", [
  { key: "n", on: role("region", "Jobs", { exact: true }), say: "n opens Dispatch on the repository the rail is on" },
  { type: "Why does this response have settled lines with no memo", into: request, say: "The question, in the person's own words" },
  { paste: request, text: RESPONSE, say: "An API response copied whole, far past 8,000 tokens" },
  { look: inside(attached, text("pasted-snippet-1.txt")), say: "It waits as a snippet chip and the request keeps only the question" },
  { press: inside(attached, button(/^Remove pasted-snippet-1\.txt/)), say: "The chip comes off if it was a mistake" },
  { paste: request, text: RESPONSE, say: "Pasted again" },
  { look: inside(frame, button("Start", { exact: true })), say: "Start sends the question as the first message, with the snippet as a file the agent can open" },
]);

export { dispatchPasteTooBig as "dispatch-paste-too-big" };
