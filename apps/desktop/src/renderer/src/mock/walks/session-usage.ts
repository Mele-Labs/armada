// How full a Session's context window is, and how much of the Claude plan is used. The context is a bare
// figure in each Session's header, beside its acts. The 5-hour and weekly limits are two fill meters in the
// title bar, beside Sleep, each opening a card on hover. Normal below 80%, amber from 80%, red from 95%.
// Over the `session-usage` scenario.

import { inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Sessions");
const five = role("img", "5-hour limit");
const weekly = role("img", "Weekly limit");

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
  ...context("Release notes script", "s-01CTXLOW", "5%"),
  ...context("Store migration spike", "s-01CTXMID", "71%"),
  ...context("Cart totals rounding", "s-01CTXHIG", "94%, amber"),
]);

export { usage as "session-usage" };
