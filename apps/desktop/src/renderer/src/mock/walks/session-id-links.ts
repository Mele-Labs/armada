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
]);

export { wide as "session-id-links" };
