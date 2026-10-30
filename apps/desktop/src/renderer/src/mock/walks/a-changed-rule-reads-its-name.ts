// A rule added to one Job is listed under Changed on this Job by its name, not its id (armada-tzg, 30 Oct 2026). Played wide, as `workspaces-lanes-copy` is.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const RULE_FORM = role("form", "Add a rule to this Job", { exact: true });
const RULE = "version: 1\nrule_id: ask_for_review\nname: Ask for review\nscope: everywhere\nwhen: { step_starts: plan }";
const CHANGES = region("Changed on this Job", { exact: true });

export const aChangedRuleReadsItsName = walk("workspaces-lanes", [
  toSessions,
  { press: deckRow("Add gift cards to the cart"), say: "A Job on the feature workflow" },
  { press: inside(role("toolbar", "Views", { exact: true }), button("Workflow", { exact: true })), say: "Its Workflow" },
  { press: button("Add to this Job", { exact: true }), say: "Add to this Job" },
  { press: role("menuitem", "Rule", { exact: true }), say: "A rule" },
  { type: RULE, into: inside(RULE_FORM, role("textbox", "Rule file", { exact: true })), say: "Its file declares ask_for_review, named Ask for review" },
  { press: inside(RULE_FORM, button("Add", { exact: true })), say: "Added to this Job" },
  { look: inside(CHANGES, text("Ask for review")), say: "The row reads the rule's name" },
  { look: inside(CHANGES, button("Keep for every Job: Ask for review", { exact: true })), say: "Its Keep is named the same way" },
], { width: 2000, height: 1200 });

export { aChangedRuleReadsItsName as "a-changed-rule-reads-its-name" };
