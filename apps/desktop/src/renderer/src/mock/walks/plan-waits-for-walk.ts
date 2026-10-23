// A plan recorded while the page the Job showed is changing is not asked, and the page raises nothing. When the page is ready it waits in
// Needs you, window open or closed, and the plan waits behind it until he approves the page. Over the `plan-waits-for-walk` scenario.

import { button, dialog, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const window = dialog("Bridge's window on Hold the plan");
const thread = region("Thread");
const needs = region("Needs you");
const page = inside(needs, button("Open Bay page"));

const planWaitsForWalk = walk("plan-waits-for-walk", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Hold the plan")), say: "A Job with its plan recorded" },
  { press: inside(thread, button("Open window Bay page")), say: "Its page is open" },
  { later: inside(window, role("img", "Still changing")), say: "The page is changing. Needs you holds no page and no plan" },
  { look: inside(window, role("img", "Ready to walk")), say: "The page is ready" },
  { press: inside(needs, button("Needs you")), say: "Needs you opens" },
  { look: page, say: "The page waits here. The plan does not" },
  { press: inside(window, button("Close window")), say: "The window is closed" },
  { look: page, say: "The page still waits" },
  { press: page, say: "A press opens it again" },
  { press: inside(window, button("Approve")), say: "Approve on the page" },
  { look: inside(thread, role("article", "Plan to approve")), say: "The plan is in the thread, with Approve" },
]);

export { planWaitsForWalk as "plan-waits-for-walk" };
