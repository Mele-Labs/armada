// A pull request the forge refused is drawn open with Fleet's sentence for it: the forge's reason and what to run. A pull request
// the forge answers is unchanged beside it. Over `pr-panel-refused`.

import { button, inside, region, role, tab, walk } from "../walk";

const queue = region("Merge queue");
const refused = role("dialog", "Pull request #1899");
const answered = role("dialog", "Pull request #1892");

const prPanelSaysWhyRefused = walk("pr-panel-refused", [
  { press: inside(queue, button(/^#1899/)), say: "A pull request the forge refused" },
  { look: inside(refused, role("alert")), say: "Fleet's words: the forge's reason, and the step that fixes it" },
  { key: "Escape", on: refused, say: "Esc closes the panel" },
  { press: inside(queue, button(/^#1892/)), say: "One the forge answers" },
  { look: inside(answered, tab("Conversation")), say: "Is drawn whole" },
  { key: "Escape", on: answered, say: "Esc" },
]);

export { prPanelSaysWhyRefused as "pr-panel-says-why-refused" };
