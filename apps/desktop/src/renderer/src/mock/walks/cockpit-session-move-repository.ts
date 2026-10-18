// The same move from a Job open on the flight deck's stage: its repository in the head, the pick and its confirm, and the new
// Session opens on the stage where the old one was, with a row back. Over `session-move-repository`.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";

const stage = region(/^Job: /);
const ask = dialog("Move to another repository");

const cockpitSessionMoveRepository = walk("session-move-repository", [
  { press: deckRow("Pin the store clock"), say: "A Job open on the stage" },
  { look: inside(stage, button("Repository shop")), say: "Its repository is in the head" },
  { press: inside(stage, button("Repository shop")), say: "Pressed, it lists the other repositories" },
  { press: inside(ask, role("menuitem", "armada")), say: "armada" },
  { look: inside(ask, text("fix/pin-store-clock and its files stay in shop")), say: "The branch and its files stay in the old repository" },
  { press: inside(ask, button("Move to armada")), say: "Move" },
  { look: inside(stage, button("Repository armada")), say: "The new Job is on the stage, in armada" },
  { look: inside(stage, button("Moved from shop")), say: "Its thread starts with a row naming where it came from" },
  { press: inside(stage, button("Moved from shop")), say: "The row opens the old Job, which ended and is not on the deck, so on its own page" },
  { look: button("Moved to armada"), say: "Its thread ends with where it went" },
  { look: role("img", "Repository shop"), say: "Its repository is text now: it cannot be moved twice" },
]);

export { cockpitSessionMoveRepository as "cockpit-session-move-repository" };
