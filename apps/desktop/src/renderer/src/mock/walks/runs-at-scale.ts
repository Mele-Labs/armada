// How a repository with workspaces runs, in Dispatch. Only what is new is drawn, each line with its workspace; `x` drops a guess; all of it is a layer
// of workspaces, each with its commands. Over the `runs-at-scale` scenario.

import { button, inside, region, role, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const dispatch = role("region", "Dispatch", { exact: true });
const runs = inside(dispatch, role("group", "How my-app runs"));
const layer = inside(dispatch, role("dialog", "All of how my-app runs"));

const runsAtScale = walk("runs-at-scale", [
  { key: "n", on: jobs, say: "N opens Dispatch for my-app" },
  { look: runs, say: "Two lines are new, each in the workspace it ran in. The rest are kept already" },
  { key: "x", on: inside(runs, role("listitem", /packages\/ui\s+Build/)), say: "x drops a guess. It is not kept, and not offered again" },
  { press: inside(dispatch, button(/^All of how my-app runs/)), say: "All of how my-app runs opens as a layer" },
  { press: inside(layer, button(/^packages\/api/)), say: "A workspace lists its commands" },
  { key: "Escape", on: layer, say: "Esc goes back to the workspaces" },
  { key: "Escape", on: layer, say: "Esc returns to Dispatch as it was" },
]);

export { runsAtScale as "runs-at-scale" };
