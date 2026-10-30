// Teaching Armada a lifecycle rule from a pull request. Teach is pressed on the pull request a Job holds, the owner says "when this is
// approved, move its tickets to Done", and Armada shows back the rule it made of it: when it fires, what it does, and how far it goes
// alone. Autonomy is changed in the panel before Save keeps it as a rule piece. Over the `teach-rule` scenario.

import { kit } from "../sessions/walk-kit";
import { RULE_PULL, RULE_TITLE } from "../sessions/teach-rule";
import { button, deckRow, dialog, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";

function steps(): Step[] {
  const { artifacts, key } = kit(false);
  const panel = dialog("Teach");
  const instruction = inside(panel, role("textbox", "Instruction"));
  const rule = inside(panel, region("Rule", { exact: true }));
  return [
    { press: deckRow(RULE_TITLE), say: "A Job holding a pull request" },
    { press: key("Artifacts"), say: "Its pull request is in the Artifacts" },
    { context: inside(artifacts, button(new RegExp(`^Pull request #${RULE_PULL} `))), say: "A right press on its pull request" },
    { press: role("menuitem", "Teach"), say: "Teach" },
    { type: "When this is approved, move its tickets to Done", into: instruction, say: "What to do, in his own words" },
    { key: "Meta+Enter", on: instruction, say: "⌘↵ teaches" },
    { look: rule, say: "The rule: when it fires, what it does, and how far it goes alone" },
    { press: inside(rule, role("radio", "Acts")), say: "Asks is how it starts. Acts does it without asking" },
    { press: inside(panel, button("Save")), say: "Save keeps it as a rule" },
  ];
}

const teachRule = walk("teach-rule", steps());

export { teachRule as "teach-rule" };
