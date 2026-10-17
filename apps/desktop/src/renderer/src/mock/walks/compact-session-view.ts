// A Session open with less on screen: the Work list, the thread, the Field, Progress and Flight control, and a key for the rest. Over the
// `compact-session-view` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const controls = region("Flight controls");
const key = (name: string) => inside(controls, button(name));

const compactSessionView = walk("compact-session-view", [
  { press: deckRow("Retry the slot lease"), say: "A Session open keeps the Work list, the thread, the Field, Progress and Flight control. Nothing else" },
  { hover: inside(region("Progress"), role("img", "Build · current")), say: "Progress is the workflow as a line of glyphs and the plan's groups and tasks under it. Hovering a step names it and its state" },
  { hover: deckRow("Prune the stale branches"), say: "Another Session is calling him, so its row pulses. Hovering the row names the call" },
  { look: role("button", "Attempts"), say: "What waits on him is a pill over the composer, and its sheet is answered where it stands" },
  { press: key("Artifacts"), say: "Artifacts (8) opens everything the Session made or used, over the thread: pull requests, commits, pages, files, docs, sketches, subagents, commands and the Sessions it started" },
  { type: "lease", into: inside(region("Artifacts"), role("searchbox", "Search artifacts")), say: "Typing narrows the groups at once. Esc is Back" },
  { press: key("Subagents"), say: "Subagents (9) lists every one. The Field draws only those at work" },
  { later: inside(region("Subagents"), button("Back")), say: "Time passes. Slot 3 is left stranded" },
  { press: inside(region("Subagents"), button("Back")), say: "Back returns to the thread where it was. The Slot key is lit and the thread has the call" },
]);

export { compactSessionView };
