// A Session open with less on screen: the Work list, the thread, the Field, Progress and Flight control, and a key for the rest. Over the
// `compact-session-view` scenario, which draws in the new look. A design mock: nothing here asserts a colour or a size.

import { button, deckRow, inside, region, role, walk } from "../walk";

const controls = region("Flight controls");
const key = (name: string) => inside(controls, button(name));

const compactSessionView = walk("compact-session-view", [
  { press: deckRow("Retry the slot lease"), say: "A Session open keeps the Work list, the thread, the Field, Progress and Flight control. Nothing else" },
  { hover: inside(region("Progress"), role("img", "Build · current")), say: "Progress is Status and the Plan as one panel: the workflow runs along its band as glyphs, and hovering a step names it and its state. The plan's groups and tasks sit under it" },
  { hover: deckRow("Prune the stale branches"), say: "Another Session is calling him, so its row pulses. Hovering the row names the call" },
  { press: key("Artifacts"), say: "Artifacts (8) opens everything the Session made or used, over the thread: pull requests, commits, pages, files, docs, sketches, subagents and commands" },
  { type: "lease", into: inside(region("Artifacts"), role("searchbox", "Search artifacts")), say: "Typing narrows the groups at once" },
  { press: key("Subagents"), say: "Subagents (9) lists every one. The Field draws only those at work" },
  { later: inside(region("Subagents"), button("Back")), say: "Time passes. Slot 3 is left stranded" },
  { press: inside(region("Subagents"), button("Back")), say: "Back returns to the thread where it was. The Slot key is lit and the thread has the call" },
]);

export { compactSessionView };
