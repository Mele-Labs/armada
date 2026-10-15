// The composer's open issues for two repositories on All: storefront keeps them on the forge and each row
// is drawn as #412; ops keeps them in Jira and each row is drawn as its key, ABC-123 or OPS-7.
// Over `issue-backlogs`.

import { button, role, text, walk } from "../walk";

const bar = role("combobox");
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
]);

export { issueBacklogs as "issue-backlogs" };
