// Bridge with no backdrop blur. A design mock: nothing here asserts a colour or a size.
// Over `cockpit-list-by-repository`.

import { inside, region, role, text, walk } from "../walk";

const list = region("Jobs", { exact: true });

const noGlassBlur = walk("cockpit-list-by-repository", [
  { look: list, say: "A solid panel" },
  { press: inside(list, text("Prune the stale branches")), say: "A Job open, with its panes beside the list" },
  { press: role("button", "Repository", { exact: true }), say: "A list floating over the panels, also with no blur" },
]);

export { noGlassBlur as "no-glass-blur" };
