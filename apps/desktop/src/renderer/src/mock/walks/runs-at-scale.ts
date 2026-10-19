// How a repository with workspaces runs. Its row in the repository picker wears the place its runs are kept in; all of it is a layer of workspaces,
// each with its commands; `x` drops a command and it is never kept again. Over the `runs-at-scale` scenario.

import { button, inside, role, walk } from "../walk";

const picker = role("combobox", "Repository");
const layer = role("dialog", "Armada configuration for my-app");

const runsAtScale = walk("runs-at-scale", [
  { press: picker, say: "The repository picker opens" },
  { key: "ArrowDown", on: picker, say: "my-app's row wears the laptop, where its Armada configuration is kept" },
  { key: "ArrowRight", on: picker, say: "→ opens Armada configuration for my-app, by workspace" },
  { press: inside(layer, button(/^packages\/api/)), say: "A workspace lists its commands" },
  { key: "x", on: inside(layer, role("listitem", /packages\/api\s+Build/)), say: "x drops a command. It is never kept again" },
  { key: "Escape", on: layer, say: "Esc goes back to the workspaces" },
  { key: "Escape", on: layer, say: "Esc closes the layer" },
]);

export { runsAtScale as "runs-at-scale" };
