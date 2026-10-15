// A Session's thread names other Sessions by address, in backticks as an agent writes one and in plain text.
// Each is a link, an ended Session's too, and opens that Session. Code that is more than an address stays code.
// Over the `session-id-links` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { thread } = kit(false);
const link = (name: string) => inside(thread, role("link", name, { exact: true }));

const wide = walk("session-id-links", [
  toSessions,
  { press: inside(region("Sessions"), button(/Hand-off thread/)), say: "A thread that names two Sessions" },
  { look: link("s-e165b4df"), say: "An address in backticks is a link, still drawn as code" },
  { hover: link("s-e165b4df"), say: "On hover, the Session it opens" },
  { look: inside(thread, text("s-e165b4df --fix")), say: "Code that is more than an address stays code" },
  { look: link("s-b7d20c9e"), say: "An address in plain text links too, though that Session ended" },
  { press: link("s-b7d20c9e"), say: "He opens the ended Session" },
  { look: inside(thread, text("Nothing more to add.")), say: "Its page opens" },
  { press: button("Sessions"), say: "Back to the list" },
  { press: inside(region("Sessions"), button(/Hand-off thread/)), say: "The first thread again" },
  { press: link("s-e165b4df"), say: "He opens the one in backticks" },
  { look: inside(thread, text("Picking this up from")), say: "That Session's page" },
  { press: button("Sessions"), say: "Back to the list" },
  { press: inside(region("Sessions"), button(/Cart rounding hand-off/)), say: "A Session in another repository, naming pull requests and an issue" },
  { look: link("#401"), say: "A merged pull request links, looked up in that Session's own repository" },
  { hover: link("#401"), say: "On hover, its title" },
  { look: link("https://forge.invalid/pull/401"), say: "The forge's address for it is the same reference" },
  { look: link("#412"), say: "An issue links, and opens the composer's open issues" },
  { look: link("#2301"), say: "So does the pull request this Session opened" },
  { press: link("#2301"), say: "It opens in the Session's own panel" },
  { look: role("dialog", "Pull request #2301"), say: "The panel" },
  { look: inside(thread, role("link", "https://example.com/health")), say: "An address inside inline code is a link" },
  { look: inside(thread, role("link", "https://example.com/ready")), say: "An address inside a code block is a link, and the #412 in its comment is not" },
  { press: button("Sessions"), say: "Back to the list" },
  { press: inside(region("Sessions"), button(/Walk links/)), say: "A Session that sent walk links" },
  { hover: inside(thread, role("link", "http://localhost:47331/?walk=session-id-links")), say: "A walk link says it opens in the walk window" },
  { press: inside(thread, role("link", "http://localhost:47331/?walk=session-id-links")), say: "It opens there, not in the browser" },
  { look: role("dialog", "Bridge's window on Walk links"), say: "The Session's walk window" },
]);

export { wide as "session-id-links" };
