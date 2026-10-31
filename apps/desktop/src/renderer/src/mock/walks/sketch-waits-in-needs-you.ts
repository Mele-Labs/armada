// A sketch the Job showed and you have not approved marks its row as needing you while the turn runs, and stays pinned above
// the composer as a plan to approve does, with the canvas's zoom and fit and Approve and Send back, and folds to its header line
// with the keys on it. Over
// `sketch-waits-in-needs-you`.

import { button, deckRow, inside, role, walk } from "../walk";

const pinned = role("article", "Sketch to approve: Pause on the row", { exact: true });
const card = role("article", "Sketch: Pause on the row", { exact: true });

const sketchWaitsInNeedsYou = walk("sketch-waits-in-needs-you", [
  { look: deckRow("Pause a Job from its row"), say: "The turn is running and the row needs you" },
  { press: deckRow("Pause a Job from its row"), say: "The Job" },
  { look: inside(pinned, button("Approve", { exact: true })), say: "The sketch is pinned above the composer, with Approve and Send back" },
  { look: button("Open sketch Pause on the row, first take", { exact: true }), say: "The first take, drawn under another title, was taken back when the newest was drawn" },
  { look: inside(pinned, button("Approve", { exact: true })), say: "Only the pin carries Approve and Send back" },
  { press: inside(pinned, button("Zoom in", { exact: true })), say: "Zoom it in place" },
  { press: inside(pinned, button("Fold", { exact: true })), say: "Fold it to one line" },
  { look: inside(pinned, button("Send back", { exact: true })), say: "Approve and Send back stay on the line" },
  { press: inside(pinned, button("Approve", { exact: true })), say: "Approve it from there" },
  { look: inside(card, role("img", "Approved")), say: "The card in the thread reads Approved" },
  { look: button("Open sketch Pause on the row, first take", { exact: true }), say: "Nothing is pinned now: the first take does not come back" },
]);

export { sketchWaitsInNeedsYou as "sketch-waits-in-needs-you" };
