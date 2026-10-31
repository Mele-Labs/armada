// A screenshot pasted into Dispatch waits as a chip and goes to the Job as a file, as it does in a Job's own composer. Over `session-after-another`, whose repository is chosen.

import { button, inside, role, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const attached = inside(frame, role("group", "Attached"));

const dispatchPasteScreenshot = walk("session-after-another", [
  { key: "n", on: role("region", "Jobs", { exact: true }), say: "n opens Dispatch on the repository the rail is on" },
  { type: "The save button sits under the footer", into: request, say: "The problem, in the person's own words" },
  { paste: request, say: "A screenshot copied from the screen" },
  { look: attached, say: "It waits as a chip under the request" },
  { look: inside(frame, button("Start", { exact: true })), say: "Start sends the words with the screenshot as a file" },
]);

export { dispatchPasteScreenshot as "dispatch-paste-screenshot" };
