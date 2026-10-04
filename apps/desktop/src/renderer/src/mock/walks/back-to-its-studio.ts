// A Job dispatched off a Studio is followed back to it: the header's own
// sentence is a press, and so is the Studio node that opens Overview's canvas;
// each lands on the Studio with the Job's node picked.
// The owner's decision of 2 Oct 2026 on #1674; `job-detail-from-studio.test.tsx`
// holds the claims.

import { button, inside, role, text, walk } from "../walk";

const option = role("option", "Cache the manifest read between dispatches");
const sentence = button("From a Studio, by you", { exact: true });
const studio = role("button", /^Studio, /);
const picked = text("Studios · Every kind of node and edge · Job Cache the manifest read between dispatches selected");

export const backToItsStudio = walk("every-state", [
  { press: option, say: "A Job dispatched off a Studio" },
  { look: sentence, say: "Where it came from is a link now; its tooltip names the Studio" },
  { press: sentence, say: "One press leaves the Job for that Studio" },
  { press: button("Helm"), say: "Helm's footer says where you are" },
  { look: picked, say: "On that Studio, with the Job's own node picked" },
  { press: button("Overview", { exact: true }), say: "Back to the Board" },
  { press: option, say: "The same Job, on Overview" },
  { look: inside(studio, text("Every kind of node and edge")), say: "The canvas opens on the Studio it came from, named" },
  { press: studio, say: "The same one press" },
  { look: picked, say: "The same Studio, the same node picked" },
]);
