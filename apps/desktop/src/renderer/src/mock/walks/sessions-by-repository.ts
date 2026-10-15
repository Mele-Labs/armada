// On All repositories the Sessions list has a band for each repository, with the state columns inside it.
// One repository picked draws the list as it was, with no band. Over `sessions-by-repository`.

import { button, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Sessions");
const armada = inside(sessions, region("armada"));

const sessionsByRepository = walk("sessions-by-repository", [
  toSessions,
  { look: armada, say: "On All, armada has a band of its own" },
  { look: inside(armada, role("heading", "Needs you")), say: "The state columns sit inside it" },
  { look: inside(sessions, region("storefront")), say: "storefront has the next band" },
  { press: button("All repositories"), say: "Pick one repository" },
  { press: role("menuitem", "storefront"), say: "storefront" },
  { look: inside(sessions, role("heading", "Needs you")), say: "The columns stay" },
  { look: inside(sessions, text("Round the cart totals")), say: "Only its Sessions are listed, with no band above them" },
]);

export { sessionsByRepository as "sessions-by-repository" };
