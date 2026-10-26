// A Job's Artifacts as a graph of how it grew. The walk opens the Job's Artifacts, switches to Graph, reads the request leading to the sketch
// and the plan, the plan leading to its groups, the pull request's versions, and the later note leading to a redraw, walks the nodes with the
// arrows, and opens the first drawing of the sketch from its mark. One take over `artifact-graph`.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { GRAPH_NOTE, GRAPH_SKETCH, GRAPH_TITLE } from "../sessions/artifact-graph";
import { kit } from "../sessions/walk-kit";

function steps(): Step[] {
  const { artifacts, key } = kit(false);
  const graph = inside(artifacts, role("group", "Artifact graph", { exact: true }));
  const node = (name: RegExp | string) => inside(graph, button(name));
  return [
    { press: inside(region("Jobs", { exact: true }), button(GRAPH_TITLE)), say: "A Job two hours in, with links, an issue, a pull request, a sketch, a page and a child Job" },
    { press: key("Artifacts"), say: "Artifacts" },
    { press: inside(artifacts, button("Graph", { exact: true })), say: "Graph" },
    { look: node(/^Message, Read /), say: "The request, and what hangs off it: its two links, the issue, the sketch drawn from it and the plan" },
    { look: node("Plan, version 1"), say: "The plan, and the three groups that hang off it" },
    { look: inside(graph, role("group", /^Versions of #1913/)), say: "The pull request is one node: opened, checks failed, checks passed, merged. The issue it closes is joined to it" },
    { look: node(`Message, ${GRAPH_NOTE}`), say: "The later note leads to the sketch drawn again and the page shown again, by the thin lines" },
    { key: "ArrowRight", on: node(`Message, ${GRAPH_NOTE}`), say: "Tab enters the graph and the arrows move to the nearest node. Enter opens one, a full stop steps through its versions and V switches back to the list" },
    { press: inside(graph, button(new RegExp(`^Drawn, ${GRAPH_SKETCH}`))), say: "The first version of the sketch opens as it was drawn, not the latest" },
  ];
}

const graph = walk("artifact-graph", steps());

export { graph as "artifact-graph" };
