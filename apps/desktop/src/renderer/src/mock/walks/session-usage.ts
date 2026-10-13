// How full a Session's context window is, and how much of the Claude plan is used. The context is a bare
// figure in each Session's header, beside its acts. The 5-hour and weekly limits are two fill meters in the
// title bar, beside Sleep, each opening a card on hover. Normal below 80%, amber from 80%, red from 95%.
// A Session or a Job held for the plan limit wears a battery-low mark with when it resumes, on its row and, for
// a Session, in its header. Over the `session-usage` scenario.

import { inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Sessions");
const five = role("img", "5-hour limit");
const weekly = role("img", "Weekly limit");
const limitMark = role("img", "Limit reached");

/** Opens a Session from the list, then looks at and hovers its context figure. */
function context(title: string, address: string, said: string): Step[] {
  const figure = inside(region(`Session ${address}`), role("img", "Context"));
  return [
    toSessions,
    { press: inside(sessions, role("button", new RegExp(title))), say: title },
    { look: figure, say: said },
    { hover: figure, say: "Hover for the tokens" },
  ];
}

const usage = walk("session-usage", [
  { look: five, say: "The 5-hour limit, a fill meter in the title bar" },
  { hover: five, say: "Hover for the card: used, and when it resets" },
  { look: weekly, say: "The weekly limit" },
  { hover: weekly, say: "Hover for its card" },
  { later: weekly, say: "Later in the week" },
  { look: five, say: "Amber from 80%" },
  { hover: weekly, say: "Red from 95%" },
  { press: role("tab", "Active"), say: "A Job's row" },
  { look: limitMark, say: "A Job held for the limit wears this beside its status" },
  { hover: limitMark, say: "Hover for when it resumes" },
  toSessions,
  { look: inside(sessions, limitMark), say: "A Session held for the limit, in place of its live mark" },
  { hover: inside(sessions, limitMark), say: "Hover for when it resumes" },
  { press: inside(sessions, role("button", /Invoice PDF layout/)), say: "Invoice PDF layout" },
  { look: inside(region("Session s-01CTXLIM"), limitMark), say: "The same mark in its header" },
  { hover: inside(region("Session s-01CTXLIM"), limitMark), say: "Same tooltip" },
  { hover: inside(region("Session s-01CTXLIM"), role("img", "Context")), say: "Context at 96% is red" },
  ...context("Release notes script", "s-01CTXLOW", "5%"),
  ...context("Store migration spike", "s-01CTXMID", "71%"),
  ...context("Cart totals rounding", "s-01CTXHIG", "94%, amber"),
]);

export { usage as "session-usage" };
