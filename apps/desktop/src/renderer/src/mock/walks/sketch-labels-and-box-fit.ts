// An agent's sketch of how data moves through Fleet, where the owner found its arrows' words piled on each other and on a box,
// a box with its last line cut, and arrows crawling on a card that was not being played. Over the `sketch-labels-and-box-fit` scenario.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { kit, toSessions } from "../sessions/walk-kit";

const TITLE = "How data moves to and from the Fleet";

function steps(): Step[] {
  const { message, thread, close } = kit(false);
  const panel = dialog(`Sketch ${TITLE}`);
  const card = inside(thread, button(`Open sketch ${TITLE}`));
  return [
    { press: inside(region("Jobs", { exact: true }), button("How data moves")), say: "A Job whose agent can draw" },
    { type: "Show me how Armada sends data back and forth\n", into: message, say: "Asked for the diagram" },
    { look: card, say: "The card in the thread: arrows still, nothing crawling until it is pointed at" },
    { hover: card, say: "Pointed at, it plays: one dot crosses the step's arrow and the other lines hold still" },
    { press: card, say: "Opened beside the thread" },
    { look: inside(panel, role("img", "WS /jobs/:id/observe and run observe, HTTP for commands")), say: "Each arrow's words sit on a backing of their own, cut to two short lines, with the whole text as the arrow's name" },
    { look: inside(panel, role("img", "MCP: get_events_since(upto), get_job, start_run, show_sketch")), say: "The two arrows into Fleet no longer share a place: this one slid along its line until it cleared the other" },
    { look: inside(panel, role("img", "code: advances, retry, fails")), say: "The short arrow to Check keeps its words in the gap, off both boxes" },
    { look: inside(panel, text(/lifeboat stays up/)), say: "Fleet's body is whole: the box grew to its words, and the height the agent gave it is the least it takes" },
    { look: inside(panel, button(/Play the steps|Stop the steps|Next step/)), say: "The steps play as it opens, and loop" },
    { press: close(panel), say: "Close" },
  ];
}

const sketchLabelsAndBoxFit = walk("sketch-labels-and-box-fit", [toSessions, ...steps()]);

export { sketchLabelsAndBoxFit as "sketch-labels-and-box-fit" };
