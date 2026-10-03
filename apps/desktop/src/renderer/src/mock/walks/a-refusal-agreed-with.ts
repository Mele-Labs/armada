// A Judge refusal he agreed with: the plan step stopped and the Job escalated.
// Overview's lead says what was refused and offers Fleet's recourse; the
// Workflow step panel says why the step stopped, with the same acts. The
// owner's Job 3, 2 Oct 2026; `judge-refusal-agreed.test.tsx` holds the claims.

import { button, card, inside, region, role, tab, text, walk } from "../walk";

const why = region("Why it stopped");

export const aRefusalAgreedWith = walk("judge/refusal-agreed", [
  { look: role("heading", "A Judge refused 1 of 2 criteria"), say: "What stopped the step" },
  { look: button("Overrule the verdict"), say: "Overrule the Judge and keep the plan" },
  { look: button("Restart step"), say: "Plan again, on the same worktree" },
  { press: tab("Workflow"), say: "The run, top to bottom" },
  { press: card("Plan the change"), say: "The stopped step's panel" },
  { look: inside(why, text(/T4 adds "Document the rule/)), say: "The refused criterion, and the Judge's finding" },
  { look: inside(why, button("Restart step")), say: "The same acts, under the reason" },
]);
