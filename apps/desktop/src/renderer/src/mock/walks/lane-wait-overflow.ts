// A step that waits on several Jobs names them all, which is longer than its node. The node shows two lines of it and its tooltip the whole. Over `workspaces-lanes`.

import { button, deckRow, inside, role, walk } from "../walk";

const views = role("toolbar", "Views", { exact: true });
const lanes = role("group", "Workflow", { exact: true });
const oversee = inside(lanes, button("Step: Oversee", { exact: true }));

const laneWaitOverflow = walk("workspaces-lanes", [
  { press: deckRow("Go through app feedback"), say: "A Job whose Oversee step waits on the Jobs it started" },
  { press: inside(views, button("Workflow", { exact: true })), say: "Its Workflow" },
  { later: lanes, say: "The agent adds Dispatch and Oversee to its copy" },
  { later: oversee, say: "Dispatch starts two Jobs. Oversee waits on both, and the wait stays inside the node, cut at two lines" },
  { hover: oversee, say: "The whole wait is in the tooltip" },
]);

export { laneWaitOverflow as "lane-wait-overflow" };
