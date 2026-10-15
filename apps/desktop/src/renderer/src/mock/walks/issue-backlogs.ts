// The composer's open issues for two repositories on All: storefront keeps them on the forge and each row
// is drawn as #412; ops keeps them in Jira and each row is drawn as its key, ABC-123 or OPS-7.
// The `#` picker in Helm's box takes a key too. Over `issue-backlogs`.

import { button, inside, role, text, walk } from "../walk";

const bar = role("combobox");
const helm = role("complementary", "Helm");
const search = role("button", /^Search, or describe work/);

const issueBacklogs = walk("issue-backlogs", [
  // (a) A repository whose issues are on the forge.
  { press: search, say: "The title bar's field" },
  { key: "Shift+Enter", on: bar, say: "Press ⇧↵ for the full composer" },
  { look: text("Pick the repository this Job is for"), say: "On All it asks which repository" },
  { press: button("Choose a repository", { exact: true }), say: "Choose a repository" },
  { press: role("menuitem", /storefront/), say: "storefront" },
  { look: text("Open issues"), say: "Its open issues are listed under the request" },
  { look: text("412"), say: "A forge issue is drawn as its number" },
  { look: text("398"), say: "Another" },
  { key: "Escape", on: text("Open issues"), say: "Escape leaves the composer" },

  // (b) A repository whose issues are in Jira.
  { press: search, say: "The title bar's field again" },
  { key: "Shift+Enter", on: bar, say: "Press ⇧↵" },
  { press: button("Choose a repository", { exact: true }), say: "Choose a repository" },
  { press: role("menuitem", /ops/), say: "ops" },
  { look: text("Open issues"), say: "Its open issues are listed under the request" },
  { look: text("ABC-123"), say: "A Jira issue is drawn as its key" },
  { look: text("OPS-7"), say: "From another project of the same site" },
  { key: "Escape", on: text("Open issues"), say: "Escape leaves the composer" },

  // (c) The # picker in a message box, on the Jira repository.
  { press: button("Helm"), say: "Helm's dock" },
  { press: inside(helm, button("Point Helm at a repository")), say: "Helm is pointed at none" },
  { press: role("menuitem", /ops/), say: "ops" },
  { type: "#OPS", into: inside(helm, role("textbox", "Ask Helm")), say: "A hash and the start of a key" },
  { look: inside(helm, role("listbox", "Tag an issue or pull request")), say: "The issues whose key starts so" },
  { look: inside(helm, role("option", /OPS-7/)), say: "OPS-7" },
  { look: inside(helm, role("option", /OPS-12/)), say: "OPS-12" },
]);

export { issueBacklogs as "issue-backlogs" };
