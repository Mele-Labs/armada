// An agent's sketch of how data moves through Fleet, shared with somebody who has no Armada: Share on the sketch's
// rail opens three items, and each result is shown as the recipient has it. Over the `sketch-share` scenario.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { kit, toSessions } from "../sessions/walk-kit";

const TITLE = "How data moves to and from the Fleet";

function steps(): Step[] {
  const { message, thread, close } = kit(false);
  const panel = dialog(`Sketch ${TITLE}`);
  const share = inside(panel, button("Share"));
  const item = (name: string) => role("menuitem", name);
  const opened = (name: string) => region(`What they see when they open ${name}`);
  const closeSaved = button("Close");
  return [
    { press: inside(region("Jobs", { exact: true }), button("How data moves")), say: "A Job whose agent can draw" },
    { type: "Show me how Armada sends data back and forth\n", into: message, say: "Asked for the diagram" },
    { press: inside(thread, button(`Open sketch ${TITLE}`)), say: "Opened beside the thread" },
    { look: share, say: "Share is the last act on the sketch's rail" },
    { press: share, say: "It opens a menu of three" },
    { look: item("Save as web page"), say: "Save as web page" },
    { look: item("Save as image"), say: "Save as image" },
    { look: item("Copy as Mermaid"), say: "Copy as Mermaid" },
    { press: item("Save as web page"), say: "In Bridge a save dialog opens and you choose where the file goes. Here nothing is saved; the next step shows the file as the person you send it to sees it." },
    { look: text(`Saved ${TITLE}.html.`), say: "The save is said in a toast" },
    { look: opened(`${TITLE}.html`), say: "What the person you send it to sees when they open the file, in any browser with no network: the drawing as Bridge draws it, the steps playing in a loop with the dot crossing each arrow, and Play and Restart for them" },
    { press: closeSaved, say: "Closed" },
    { press: share, say: "Share again" },
    { press: item("Save as image"), say: "In Bridge a save dialog opens, you pick PNG or SVG and choose where the file goes. Here nothing is saved; the next step shows the PNG as the person you send it to sees it." },
    { look: opened(`${TITLE}.png`), say: "A PNG at twice the size, framed to the drawing, in the theme's own colours" },
    { press: closeSaved, say: "Closed" },
    { press: share, say: "Share again" },
    { press: item("Copy as Mermaid"), say: "Nothing is saved. The text goes on your clipboard; the next steps show it as someone who pastes it sees it." },
    { look: text("The Mermaid text is on the clipboard."), say: "The copy is confirmed as every copy in Bridge is" },
    { look: region("What they paste"), say: "What they paste into a Mermaid page: a flowchart of the boxes and labelled arrows, every label quoted and escaped" },
    { press: closeSaved, say: "Closed" },
    { press: close(panel), say: "Close" },
  ];
}

const sketchShare = walk("sketch-share", [toSessions, ...steps()]);

export { sketchShare as "sketch-share" };
