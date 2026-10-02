// A Job dispatched off a Studio is followed back to it: the header's own
// sentence is the press, and it lands on the canvas with the Job's node picked.
// The owner's decision of 2 Oct 2026 on #1674; `job-detail-from-studio.test.tsx`
// holds the claims.

import { button, role, text, walk } from "../walk";

const sentence = button("From a Studio, by you", { exact: true });

export const backToItsStudio = walk("every-state", [
  { press: role("option", "Cache the manifest read between dispatches"), say: "A Job dispatched off a Studio" },
  { look: sentence, say: "Where it came from is a link now; its tooltip names the Studio" },
  { press: sentence, say: "One press leaves the Job for that Studio" },
  { press: button("Helm"), say: "Helm's footer says where you are" },
  {
    look: text("Studios · Every kind of node and edge · Job Cache the manifest read between dispatches selected"),
    say: "On that Studio, with the Job's own node picked",
  },
]);
