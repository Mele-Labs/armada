// Requests from the palette with no Job open. The Repository picker starts on storefront, where the Cockpit is narrowed, and
// reads its collections. billing is picked and its own staging environment shows its token as set. handbook is picked: its own
// requests file would not read, Fleet's why is shown with Save off, and the collection the repository commits is still listed.
// Over the `requests-by-repository` scenario.

import { button, dialog, inside, role, tab, text, viaPalette, walk } from "../walk";
import type { Step } from "../walk";

const panel = dialog("Requests");
const within = (target: ReturnType<typeof role>) => inside(panel, target);
const collections = inside(dialog("Collections"), role("list", "Collections"));
const picker = within(role("combobox", "Repository", { exact: true }));

const steps: Step[] = [
  ...viaPalette("Requests", "Requests, from the palette with no Job open"),
  { look: picker, say: "Repository, set to storefront, where the Cockpit is narrowed" },
  { look: inside(collections, button("Store API", { exact: true })), say: "storefront's own Store API" },
  { look: inside(collections, inside(button("Health", { exact: true }), role("img", "The repository's"))), say: "Health, which storefront commits" },
  { type: "/Users/user/code/billing", into: picker, say: "billing" },
  { look: inside(collections, button("Invoices", { exact: true })), say: "billing's own Invoices" },
  { press: within(tab("Environments")), say: "Environments" },
  { look: inside(within(role("group", "staging", { exact: true })), role("img", "Set, in the Keychain")), say: "staging, its token set" },
  { type: "/Users/user/code/handbook", into: picker, say: "handbook" },
  { look: within(text("Your own requests file would not read")), say: "Your own requests file would not read" },
  { look: within(text(/TOML parse error at line 3, column 7.* Save is off until it reads\./)), say: "Fleet's why, naming the file, and Save off" },
  { press: within(tab("Request")), say: "Request" },
  { look: inside(collections, button("Search", { exact: true })), say: "Search, which handbook commits, still listed" },
];

const requestsRepositoryPicker = walk("requests-by-repository", steps);

export { requestsRepositoryPicker as "requests-repository-picker" };
