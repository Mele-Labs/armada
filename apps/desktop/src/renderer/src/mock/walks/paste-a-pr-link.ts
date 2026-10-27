// A link to a pull request, pasted anywhere in Bridge, opens its page in Reviews: no panel to open first, no box to find.
// Over the `paste-a-pr-link` scenario.

import { place, region, walk } from "../walk";

const LINK = "https://forge.example/mele-labs/armada/pull/1899";
const JOBS = region("Jobs", { exact: true });

const pasteAPrLink = walk("paste-a-pr-link", [
  { look: place("Cockpit"), say: "The window is on the Cockpit, and Reviews has not been opened" },
  { paste: JOBS, text: LINK, say: "A pull request's link someone shared, pasted with nothing focused" },
  { look: region("Pull request #1899"), say: "Reviews opens on that pull request" },
]);

export { pasteAPrLink as "paste-a-pr-link" };
