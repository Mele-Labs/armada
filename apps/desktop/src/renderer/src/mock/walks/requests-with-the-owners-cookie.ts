// The agent's GraphQL POST to staging got 401. From Flight control the owner opens Requests, adds a cookie for the host,
// picks the environment, opens the agent's request from the Job's history and sends it: 200, a row of his in the thread,
// and the agent acting on it. Then Requests from the palette with no Job open. Over the `session-requests` scenario.

import { button, dialog, inside, region, role, tab, text, viaPalette, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
const controls = region("Flight controls");
const panel = dialog("Requests");
const within = (target: ReturnType<typeof role>) => inside(panel, target);

const steps: Step[] = [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Account page shows nobody signed in/)), say: "Opened" },
  { look: inside(thread, button(/^Agent: 401 POST https:\/\/api\.example\.test\/graphql/)), say: "The agent's request, answered 401" },
  { press: inside(thread, button(/^Agent: 401 POST/)), say: "Its answer" },
  { look: inside(thread, role("log", "Response")), say: "The answer's body, as plain text" },
  { press: inside(controls, button("Requests", { exact: true })), say: "Requests, from Flight control" },
  { look: panel, say: "The side panel, for this Job" },
  { press: within(tab("Cookies")), say: "Cookies" },
  { type: "api.example.test", into: within(role("textbox", "Host")), say: "The host" },
  { type: "session=walk-only-example", into: within(role("textbox", "Cookie")), say: "The cookie, typed and never drawn" },
  { press: within(button("Keep cookie")), say: "Kept in the Keychain" },
  { look: inside(panel, inside(role("list", "Hosts holding a cookie"), text("api.example.test"))), say: "The host holds one. Its value is not shown" },
  { press: within(tab("Request")), say: "Request" },
  { type: "staging", into: within(role("combobox", "Environment")), say: "The environment" },
  { press: within(tab("History")), say: "The Job's requests" },
  { press: within(button(/^Agent: 401 POST https:\/\/api\.example\.test\/graphql/)), say: "The agent's request, opened to send again" },
  { look: within(text("https://api.example.test/graphql")), say: "{{base}}/graphql, resolved under the address" },
  { press: within(button("Send", { exact: true })), say: "Send" },
  { look: inside(panel, inside(region("Response"), role("img", "200"))), say: "200, with the cookie for api.example.test" },
  { look: within(role("log", "Body, JSON")), say: "The body, set out as JSON" },
  { look: inside(thread, button(/^You: 200 POST https:\/\/api\.example\.test\/graphql/)), say: "His request is a row in the thread" },
  { look: inside(thread, text(/With your cookie `?me`? is/)), say: "The agent acts on it" },
  { press: within(role("button", /^Close( Esc)?$/)), say: "Closed" },
  { press: inside(controls, button("Map", { exact: true })), say: "The Job closed, for the map" },
  ...viaPalette("Requests", "Requests, from the palette with no Job open"),
  { look: within(text("New request")), say: "No Job, so no History" },
  { press: within(tab("Environments")), say: "The saved environments" },
  { look: within(role("group", "staging")), say: "staging, editable, saved as one file" },
];

const requestsWithTheOwnersCookie = walk("session-requests", steps);

export { requestsWithTheOwnersCookie as "requests-with-the-owners-cookie" };
