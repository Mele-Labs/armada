// Set up with a job, from the Manifest's Set up workspaces tab. The picker holds a storefront
// in every state it draws; the act on the tab row starts a Session in that repository, whose first
// message asks for onboard-repo and carries the reading, a line a workspace, and opens it.

import { button, inside, region, role, tab, text, walk } from "../walk";

const workspaces = region("Workspaces");
const thread = region("Thread");

const setupWithASession = walk("setup-with-a-session", [
  { press: button("Open the Manifest", { exact: true }), say: "The Manifest" },
  { press: tab("Set up workspaces"), say: "Set up workspaces, the picker over every workspace Scan found" },
  { look: inside(role("listitem", "apps/web", { exact: true }), text("ready to write")), say: "A workspace with a proposal" },
  { look: inside(role("listitem", "docs", { exact: true }), text("no checks proposed")), say: "One with no Checks proposed" },
  { look: inside(workspaces, text("Nothing here could be read.")), say: "And one Scan could not read" },
  { look: button("Set up with a job"), say: "Instead of opening each one, a Job can ask about them" },
  { press: button("Set up with a job"), say: "It starts in this repository and opens" },
  { look: inside(thread, text(/using the onboard-repo skill/)), say: "Its first message asks for onboard-repo, with the skill's rules spelled out" },
  { look: inside(thread, text(/Bridge has already scanned this checkout/)), say: "And hands over the reading, so it does not scan again" },
  { look: inside(thread, text("Nothing here could be read.")), say: "Every workspace is on a line, the unreadable one too" },
]);

export { setupWithASession as "setup-with-a-session" };
