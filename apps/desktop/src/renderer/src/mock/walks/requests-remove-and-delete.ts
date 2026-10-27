// The owner adds a `test` environment with a plain `base` and a secret `token`, and saves it: token shows only as set. He
// presses Remove on token's row and saves again, and test keeps base alone. Delete on test asks to be pressed again; pressed
// again, test is out of his environments and the repository's `local` is still there. Over the `session-requests` scenario.

import { button, dialog, inside, region, role, tab, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
const panel = dialog("Requests");
const within = (target: ReturnType<typeof role>) => inside(panel, target);
const fresh = within(role("group", "New environment", { exact: true }));
const added = within(role("group", "test", { exact: true }));

const steps: Step[] = [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Order confirmation shows no order/)), say: "Opened" },
  { press: inside(thread, button("Open in Requests", { exact: true })), say: "Open in Requests" },
  { press: within(tab("Environments")), say: "Environments" },
  { look: within(role("group", "local", { exact: true })), say: "local, which the repository commits" },
  { press: within(button("Add environment", { exact: true })), say: "Add environment" },
  { type: "test", into: inside(fresh, role("textbox", "Environment name")), say: "Named test" },
  { type: "base", into: inside(fresh, role("textbox", "Variable 1", { exact: true })), say: "base" },
  { type: "https://api.test.example.test", into: inside(fresh, role("textbox", "Value 1", { exact: true })), say: "Its value" },
  { type: "token", into: inside(fresh, role("textbox", "Variable 2", { exact: true })), say: "token" },
  { press: inside(fresh, role("checkbox", "Secret 2", { exact: true })), say: "Secret" },
  { type: "walk-only-token", into: inside(fresh, role("textbox", "Value 2", { exact: true })), say: "Typed into a password field" },
  { press: inside(fresh, button("Save", { exact: true })), say: "Saved" },
  { look: inside(added, role("img", "Set, in the Keychain")), say: "token shows only as set" },
  { look: inside(added, button("Remove variable 2", { exact: true })), say: "Remove, at the end of each variable's row" },
  { press: inside(added, button("Remove variable 2", { exact: true })), say: "Remove on token" },
  { press: inside(added, button("Save", { exact: true })), say: "Saved without token" },
  { look: inside(added, role("textbox", "Variable 1", { exact: true })), say: "test keeps base alone" },
  { press: inside(added, button("Delete", { exact: true })), say: "Delete on test" },
  { look: inside(added, button("Press again to delete", { exact: true })), say: "Press again to delete" },
  { press: inside(added, button("Press again to delete", { exact: true })), say: "Pressed again" },
  { look: within(role("group", "local", { exact: true })), say: "test is gone and local is still there" },
];

const requestsRemoveAndDelete = walk("session-requests", steps);

export { requestsRemoveAndDelete as "requests-remove-and-delete" };
