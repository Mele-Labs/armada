// A Session's agent draws a sketch with `show_sketch`: a card in the thread where it did, the sketch open in the
// panel and read-only with its steps, a second one with the same title that replaces the first, and a later one
// read as a change to it. Over the `agent-sketch` scenario.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { kit, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const { ledger, message, thread, close, opened, row } = kit(false);
  const panel = (title: string) => dialog(title);
  const drawn = (title: string) => inside(thread, button(`Open sketch ${title}`));
  return [
    { press: inside(region("Sessions"), button("Store clock sketches")), say: "A Session whose agent can draw" },
    { type: "Show me how the clock is read\n", into: message, say: "Asked how the clock is read" },
    { look: drawn("Store clock reads"), say: "It drew a sketch: a card in the thread, where it did, with the drawing small" },
    ...opened([{ look: inside(ledger, button("Sketch Store clock reads")), say: "And a row on the ledger" }]),
    { press: drawn("Store clock reads"), say: "The card opens it beside the thread" },
    { look: inside(panel("Sketch Store clock reads"), role("group", "Sketch", { exact: true })), say: "Drawn by the sketch scene, read-only" },
    { look: inside(panel("Sketch Store clock reads"), text("fails when the second turns")), say: "Pan and zoom are the canvas's" },
    { press: inside(panel("Sketch Store clock reads"), button("Play the steps")), say: "The steps play in the order the agent gave them" },
    { look: inside(panel("Sketch Store clock reads"), button(/Stop the steps|Next step/)), say: "One part lit at a time, the rest stand back" },
    { press: close(panel("Sketch Store clock reads")), say: "Close" },
    { type: "Add the sleep you tried\n", into: message, say: "Asked again" },
    { look: inside(thread, text("Replaced")), say: "A sketch with the same title takes the first one's place: the first card is dashed and says so" },
    ...row("Sketch Store clock reads", "The ledger row opens the new one"),
    { look: inside(panel("Sketch Store clock reads"), text("fails 4 of 30 runs")), say: "Only the new one is on the ledger" },
    { press: close(panel("Sketch Store clock reads")), say: "Close" },
    { type: "Now draw the fix against that\n", into: message, say: "Asked for the change" },
    { look: inside(thread, text("Against Store clock reads")), say: "A later sketch names the one it changes" },
    { press: drawn("Pinned clock"), say: "Opened" },
    { look: inside(panel("Sketch Pinned clock"), text("Against Store clock reads")), say: "A band over the drawing says what it is read against" },
    { look: inside(panel("Sketch Pinned clock"), role("group", "Added, FixedClock")), say: "What it adds is green" },
    { look: inside(panel("Sketch Pinned clock"), role("group", "Changed, Clock")), say: "What it alters is amber" },
    { look: inside(panel("Sketch Pinned clock"), role("group", "Removed, sleep(1s)")), say: "What it drops is red, where it was" },
    { press: inside(panel("Sketch Pinned clock"), button("Whole")), say: "Whole shows the drawing alone" },
    { look: inside(panel("Sketch Pinned clock"), role("group", "FixedClock")), say: "No change marks" },
  ];
}

const sketch = walk("agent-sketch", [toSessions, ...steps()]);

export { sketch as "agent-sketch" };
