// A Session that started in the wrong repository, moved. Its repository in the head is the control: pressed, it lists the others,
// a pick shows what stays behind, and Move opens the new Session in armada with the whole conversation and a row naming shop.
// That row opens the old one, which ended and ends with a row naming armada. A terminal that is still listening is drawn off.
// Over `session-move-repository`.

import { toSessions } from "../sessions/walk-kit";
import { button, dialog, inside, region, role, text, walk } from "../walk";

const sessions = region("Jobs", { exact: true });
const ask = dialog("Move to another repository");

const sessionMoveRepository = walk("session-move-repository", [
  toSessions,
  { press: inside(sessions, button("Pin the store clock")), say: "A Job that started in shop, holding a branch there" },
  { look: button("Repository shop"), say: "Its repository is in the head, with the repository glyph" },
  { press: button("Repository shop"), say: "Pressed, it lists the other repositories" },
  { look: inside(ask, role("menuitem", "armada")), say: "armada" },
  { look: inside(ask, role("menuitem", "billing")), say: "billing. The one it is in is not listed" },
  { press: inside(ask, role("menuitem", "armada")), say: "armada" },
  { look: inside(ask, text("fix/pin-store-clock and its files stay in shop")), say: "The confirm names the branch and where it stays" },
  { look: inside(ask, text("Ends this Job")), say: "It is live, so moving ends it" },
  { press: inside(ask, button("Move to armada")), say: "Move" },
  { look: button("Repository armada"), say: "The new Job opens, in armada, with the same title" },
  { look: text("There are no store tests in this repository"), say: "The whole conversation came with it" },
  { look: button("Moved from shop"), say: "Its thread starts with a row naming where it came from" },
  { press: button("Moved from shop"), say: "The row opens the old Job" },
  { look: button("Moved to armada"), say: "The old thread ends with a row naming where it went" },
  { look: role("img", "Repository shop"), say: "Its repository is text now: it cannot be moved twice" },
  { look: role("img", "Ended"), say: "It ended, and nothing more can be said to it" },
  { press: button("Moved to armada"), say: "The row opens the new Job again" },
  { look: button("Repository armada"), say: "Back in armada" },
  toSessions,
  { press: inside(sessions, button("Why the reader drops lines")), say: "A Job in a terminal that is still listening" },
  { press: button("Repository shop"), say: "Its repository lists the others" },
  { hover: inside(ask, role("menuitem", "armada")), say: "Each is drawn off, and the tooltip says why: Fleet cannot move the person's terminal" },
]);

export { sessionMoveRepository as "session-move-repository" };
