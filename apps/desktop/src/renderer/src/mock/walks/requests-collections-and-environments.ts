// The owner asked for the query, and the agent saved Create order to his Store API collection. He opens it in Requests from the
// Saved row: the request under its collection, Store API's shared headers apart from its own, and the address it sends to. He adds
// a `test` environment with a secret token, picks it over the tabs and sends: 200 under Response, a row of his in the thread, and
// the agent acting on it. The picker also offers the Job's server, and the repository's Billing API is hidden by his own. Then
// Requests from the palette with no Job open. Over the `session-requests` scenario. **Proposed**: the members it calls are served
// by the mock alone.

import { button, dialog, inside, region, role, tab, text, viaPalette, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
const controls = region("Flight controls");
const panel = dialog("Requests");
const within = (target: ReturnType<typeof role>) => inside(panel, target);
const collections = within(role("list", "Collections"));
const fresh = within(role("group", "New environment", { exact: true }));
const added = within(role("group", "test", { exact: true }));
const inherited = within(role("group", "From Store API", { exact: true }));

const steps: Step[] = [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Order confirmation shows no order/)), say: "Opened" },
  { look: inside(thread, role("listitem", "Saved Create order to Store API", { exact: true })), say: "The agent saved the query to Store API" },
  { press: inside(thread, button("Open in Requests", { exact: true })), say: "Open in Requests" },
  { look: inside(collections, button("Store API", { exact: true })), say: "The Store API collection" },
  { look: inside(collections, button("POST Create order", { exact: true })), say: "Create order, open in the editor" },
  { look: within(role("heading", "Create order", { exact: true })), say: "Create order, under Store API, with Save beside it" },
  { look: within(text("has no value with no environment picked")), say: "{{base}} has no value with no environment picked" },
  { look: inside(inherited, text("Bearer {{token}}")), say: "From Store API, read only: Authorization: Bearer {{token}}" },
  { look: inside(within(role("group", "This request", { exact: true })), role("textbox", "Header 1 name", { exact: true })), say: "The request's own headers, a Name and a Value each" },
  { press: within(tab("Environments")), say: "Environments" },
  { look: within(role("group", "local", { exact: true })), say: "local, which the repository commits" },
  { press: within(button("Add environment", { exact: true })), say: "Add environment" },
  { type: "test", into: inside(fresh, role("textbox", "Environment name")), say: "Named test" },
  { type: "base", into: inside(fresh, role("textbox", "Variable 1", { exact: true })), say: "base" },
  { type: "https://api.test.example.test", into: inside(fresh, role("textbox", "Value 1", { exact: true })), say: "Its value" },
  { press: inside(fresh, button("Add variable", { exact: true })), say: "Another variable" },
  { type: "token", into: inside(fresh, role("textbox", "Variable 2", { exact: true })), say: "token" },
  { press: inside(fresh, role("checkbox", "Secret")), say: "Secret" },
  { type: "walk-only-token", into: inside(fresh, role("textbox", "Value 2", { exact: true })), say: "Typed into a password field" },
  { press: inside(fresh, button("Save", { exact: true })), say: "Saved" },
  { look: inside(added, role("img", "Set, in the Keychain")), say: "token shows only as set" },
  { press: within(tab("Collections")), say: "Back to Create order" },
  { type: "test", into: within(role("combobox", "Environment")), say: "The test environment, over the tabs, for every send" },
  { look: within(text("https://api.test.example.test/orders")), say: "Sends to https://api.test.example.test/orders" },
  { press: within(button("Send", { exact: true })), say: "Send" },
  { look: inside(panel, inside(region("Response"), role("img", "200"))), say: "Response: 200" },
  { look: inside(panel, inside(region("Response"), role("log", "Body, JSON"))), say: "The order test made" },
  { look: inside(thread, button(/^You: 200 POST https:\/\/api\.test\.example\.test\/orders/)), say: "His request is a row in the thread" },
  { look: inside(thread, text(/Create order on test answered 200/)), say: "The agent acts on it" },
  { type: "server:web", into: within(role("combobox", "Environment")), say: "The Job's server, offered as an environment" },
  { look: within(text("http://127.0.0.1:5173/orders")), say: "Sends to the Job's server" },
  { look: inside(collections, inside(button("Billing API", { exact: true }), text("Hides the repository's"))), say: "His Billing API hides the repository's" },
  { look: inside(collections, inside(button("Health", { exact: true }), role("img", "The repository's"))), say: "Health, which the repository commits" },
  { press: within(role("button", /^Close( Esc)?$/)), say: "Closed" },
  { press: inside(controls, button("Map", { exact: true })), say: "The Job closed, for the map" },
  ...viaPalette("Requests", "Requests, from the palette with no Job open"),
  { look: within(tab("Collections")), say: "Collections" },
  { look: within(tab("Environments")), say: "Environments. No Job, so no History" },
];

const requestsCollectionsAndEnvironments = walk("session-requests", steps);

export { requestsCollectionsAndEnvironments as "requests-collections-and-environments" };
