// The Job's views are a strip of icons pinned at the top of its spine, and each opens in the spine, not over the stage. The strip stays while a view is read; the same press or Esc
// brings the story back. Over `spine-top-brief`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const views = role("toolbar", "Views", { exact: true });

export const spineViews = walk("spine-top-brief", [
  { press: deckRow("Issues tab beside Reviews"), say: "A Job at its fourth step" },
  { look: inside(views, button("Artifacts", { exact: true })), say: "The views are icons pinned at the top of the spine: Artifacts, Subagents, Checks, Bays. The head no longer carries them" },
  { look: region("Brief", { exact: true }), say: "Under the strip is the story, as it was" },
  { press: inside(views, button("Checks", { exact: true })), say: "Checks opens in the spine, in place of the story. The strip stays" },
  { press: inside(views, button("Bays", { exact: true })), say: "Bays swaps in; the strip's lit icon follows" },
  { press: inside(views, button("Bays", { exact: true })), say: "The same press brings the story back" },
]);

export { spineViews as "spine-views" };
