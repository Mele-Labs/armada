// A plan recorded while the page the Job showed is still changing is not asked: the thread and Needs you show none. When the page is ready
// the plan arrives with Approve. Over the `plan-waits-for-walk` scenario.

import { button, dialog, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const window = dialog("Bridge's window on Hold the plan");
const thread = region("Thread");
const needs = region("Needs you");

const planWaitsForWalk = walk("plan-waits-for-walk", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Hold the plan")), say: "A Job with its plan recorded" },
  { press: inside(thread, button("Open window Bay page")), say: "Its page is open" },
  { later: inside(window, role("img", "Still changing")), say: "The page is still changing. The thread and Needs you hold no plan" },
  { look: inside(window, role("img", "Ready to walk")), say: "The page is ready" },
  { look: inside(thread, role("article", "Plan to approve")), say: "The plan is in the thread, with Approve" },
  { look: inside(needs, button("Open Plan to approve")), say: "Needs you lists it" },
]);

export { planWaitsForWalk as "plan-waits-for-walk" };
