// Bridge with no backdrop blur, in two takes on the window bar: `?glass=solid` and `?glass=tint`. A design mock: nothing here asserts a colour or a size.
// Over `cockpit-list-by-repository`.

import { inside, region, role, text, walk } from "../walk";

const list = region("Jobs", { exact: true });

const noGlassBlur = walk("cockpit-list-by-repository", [
  { look: list, say: "A panel with no frost behind it. Swap the takes on the bar to compare" },
  { press: inside(list, text("Prune the stale branches")), say: "A Job open, with its panes beside the list" },
  { press: role("button", "Repository", { exact: true }), say: "A list floating over the panels, also with no blur" },
]);

export { noGlassBlur as "no-glass-blur" };
