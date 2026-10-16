// Two repositories on All: storefront keeps its issues on the forge and each is drawn as #412; ops keeps
// them in Jira and each is drawn as its key, ABC-123 or OPS-7. The `#` picker in Helm's box takes a key.
// The composer's own issue list is reached from an issue reference, no longer from the one bar. Over `issue-backlogs`.

import { button, inside, role, walk } from "../walk";

const helm = role("complementary", "Helm");

const issueBacklogs = walk("issue-backlogs", [
  // The # picker in a message box, on the Jira repository.
  { press: button("Helm"), say: "Helm's dock" },
  { press: inside(helm, button("Point Helm at a repository")), say: "Helm is pointed at none" },
  { press: role("menuitem", /ops/), say: "ops" },
  { type: "#OPS", into: inside(helm, role("textbox", "Ask Helm")), say: "A hash and the start of a key" },
  { look: inside(helm, role("listbox", "Tag an issue or pull request")), say: "The issues whose key starts so" },
  { look: inside(helm, role("option", /OPS-7/)), say: "OPS-7" },
  { look: inside(helm, role("option", /OPS-12/)), say: "OPS-12" },
]);

export { issueBacklogs as "issue-backlogs" };
