// How full a Session's context window is, and how much of the plan is used. The context is a bare
// figure in each Session's header, beside its acts, the live tokens and the percent, opening a card of what fills the window. The 5-hour and weekly limits are one ring in the
// title bar, the 5-hour limit outside and the week inside, opening one card of both on hover or focus, kept by a press and closed by Esc or a press outside. Normal below 80%, amber from 80%, red from 95%.
// A Session held for the plan limit wears a battery-low mark with when it resumes, on its row and in its
// header. Over the `session-usage` scenario.

import { dialog, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Jobs", { exact: true });
const limits = role("button", "5-hour limit");
const card = dialog("Plan limits");
const limitMark = role("img", "Limit reached");

/** Opens a Session from the list, then looks at and hovers its context figure. */
function context(title: string, address: string, said: string): Step[] {
  const figure = inside(region(`Job ${address}`), role("button", "Context"));
  return [
    toSessions,
    { press: inside(sessions, role("button", new RegExp(title))), say: title },
    { look: figure, say: said },
    { hover: figure, say: "Hover for what fills the window" },
    { press: figure, say: "A press keeps the card open" },
    { key: "Escape", on: figure, say: "Esc closes it" },
  ];
}

const usage = walk("session-usage", [
  { look: limits, say: "The plan's limits, one ring: 5 hours outside, the week inside" },
  { hover: limits, say: "Hover for the card: each limit, how much is used and when it resets" },
  { press: limits, say: "A press keeps the card open" },
  { look: inside(card, region("Weekly limit")), say: "The weekly limit" },
  { key: "Escape", on: limits, say: "Esc closes it" },
  { later: limits, say: "Later in the week" },
  { look: limits, say: "Amber from 80%, red from 95%" },
  { hover: limits, say: "The figures in the card take the same tones" },
  toSessions,
  { look: inside(sessions, limitMark), say: "A Job held for the limit wears this on its row" },
  { hover: inside(sessions, limitMark), say: "Hover for when it resumes" },
  { press: inside(sessions, role("button", /Invoice PDF layout/)), say: "Invoice PDF layout" },
  { look: inside(region("Job s-01CTXLIM"), limitMark), say: "The same mark in its header" },
  { hover: inside(region("Job s-01CTXLIM"), limitMark), say: "Same tooltip" },
  { hover: inside(region("Job s-01CTXLIM"), role("button", "Context")), say: "Context at 96% is red" },
  ...context("Release notes script", "s-01CTXLOW", "53.8k · 5%"),
  ...context("Store migration spike", "s-01CTXMID", "142k · 71%"),
  ...context("Cart totals rounding", "s-01CTXHIG", "188k · 94%, amber"),
]);

export { usage as "session-usage" };
