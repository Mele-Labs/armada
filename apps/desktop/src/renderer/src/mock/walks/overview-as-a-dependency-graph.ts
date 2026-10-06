// Overview over Jobs that wait on each other: dependents nested under the Job they wait on in the
// list, then the same Jobs as a graph on the canvas, with the list's acts on each node.

import { button, role, walk } from "../walk";

export const overviewAsADependencyGraph = walk("overview/dependencies", [
  { look: role("img", "Waits on other jobs"), say: "Waits on more than one: drawn under the first" },
  { press: button("Graph", { exact: true }), say: "The same Jobs as a graph" },
  { look: role("group", /Cut the release notes/), say: "A join: it waits on three. Each dependency an edge, left to right" },
  { look: button("Kill", { exact: true }), say: "The list's acts, on the node" },
  { press: button("Graph", { exact: true }), say: "Back to the list" },
]);
