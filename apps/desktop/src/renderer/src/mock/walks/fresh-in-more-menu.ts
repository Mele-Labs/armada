// A big or old Session's fresh start, in the header's More menu and no longer a mark of its own: New Job
// opens a Job on a short handoff from this one, and Fork starts from its conversation. Over the
// `session-fresh` scenario, on the Session that is 612k tokens deep.

import { button, inside, region, role, walk } from "../walk";

const sessions = region("Jobs", { exact: true });
const head = region(/^Job s-01FRESH3/);
const more = inside(head, button("More"));

const freshInMoreMenu = walk("session-fresh", [
  { press: inside(sessions, button(/Cart totals rounding/)), say: "Big, so it used to wear a mark" },
  { look: head, say: "The header has no mark after the context figure" },
  { press: more, say: "More" },
  { look: role("menu"), say: "New Job and Fork" },
]);

export { freshInMoreMenu };
