// A Job open with less on screen: the Work list, the thread, the Job's story in the right column (Workflow, Blueprint, Field, Delivery), Flight control, and a key for the rest. Over the
// `compact-session-view` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const key = (name: string) => button(name, { exact: true });

const compactSessionView = walk("compact-session-view", [
  { press: deckRow("Retry the slot lease"), say: "A Job open keeps the Jobs list, the thread, the right column's Workflow, Blueprint, Field and Delivery, and Flight control. Nothing else" },
  { hover: inside(region("Workflow"), role("img", "Build · current")), say: "The Workflow is the workflow as a line of glyphs, the plan under it in the Blueprint. Hovering a step names it and its state" },
  { hover: deckRow("Prune the stale branches"), say: "Another Job is calling him, so its row pulses. Hovering the row names the call" },
  { look: button("Needs you", { exact: true }), say: "What waits on him is the Needs you panel on the composer, the question answered where it stands" },
  { press: key("Artifacts"), say: "Artifacts (8) opens everything the Job made or used, over the thread: pull requests, commits, pages, files, docs, sketches, subagents, commands and the Jobs it started" },
  { type: "lease", into: inside(region("Artifacts"), role("searchbox", "Search artifacts")), say: "Typing narrows the groups at once. Esc is Back" },
  { press: key("Subagents"), say: "Subagents (9) lists every one. The Field draws only those at work" },
  { later: inside(region("Subagents"), button("Back")), say: "Time passes. Slot 3 is left stranded" },
  { press: inside(region("Subagents"), button("Back")), say: "Back returns to the thread where it was. The Bays key is lit and the thread has the call" },
]);

export { compactSessionView };
