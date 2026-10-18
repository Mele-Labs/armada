// A Session's retro (`docs/concepts/retro.md`), written from the Session's own header. Pressing Retro
// does not take you anywhere: a card appears in the thread where you pressed, says which stage
// Fleet is at and for how long, and settles the retro's items as the model writes them. When it is
// written the card folds to one line and its `Open retro` link opens the kept retro on the Retros
// page, where agreeing with an item starts a Session.

import { toSessions } from "../sessions/walk-kit";
import { button, dialog, inside, region, role, text, walk } from "../walk";

const ROW = inside(region("Jobs", { exact: true }), button("Fix the flaky store test"));
const MORE = inside(role("region", /^Job: /), button("More"));
const CARD = inside(region("Thread"), region("Retro"));
const RETRO = dialog("Retro");
const item = (words: string) => inside(RETRO, role("listitem", words));
const RESET = item("A reset waited 40 minutes for an answer");
const drawn = (words: string) => inside(CARD, role("listitem", words));

const retro = walk("session-retro-live", [
  toSessions,
  { press: ROW, say: "Open the Job" },
  { press: MORE, say: "Retro is in the head's More menu" },
  { press: role("menuitem", "Retro"), say: "It writes the retro of what happened since the last one" },
  { look: inside(CARD, text("Asking the agent what got in the way")), say: "The retro is a card in the thread, and you stay here" },
  { look: inside(CARD, role("timer")), say: "With the time it has been at this stage" },
  { later: inside(CARD, text("Asking the agent what got in the way")), say: "Fleet has the agent's answer" },
  { look: inside(CARD, text("Reading what Fleet kept of the Job")), say: "It reads what it kept of the Job" },
  { later: inside(CARD, text("Reading what Fleet kept of the Job")), say: "The model starts writing" },
  { look: inside(drawn("A reset waited 40 minutes for an answer"), text("Tell you when an ask has waited ten minutes, and let the agent carry on with other work meanwhile.")), say: "Items settle in a field at a time: whose way, where it lands, the headline, what happened, what would change" },
  { look: inside(CARD, text(/characters$/)), say: "A quiet count of how much has been written" },
  { later: inside(CARD, text("Writing the retro")), say: "The next item" },
  { look: inside(CARD, text("A search of the whole disk set off privacy prompts")), say: "The last item is marked as still being written, its other fields held open" },
  { look: inside(CARD, text("Still being written")), say: "So nothing moves when they arrive" },
  { later: inside(CARD, text("Writing the retro")), say: "The model finishes" },
  { look: inside(drawn("Walks opened in your browser"), text("Tell the agent to show a walk with show_window and never to run open.")), say: "The last item, whole" },
  { later: inside(CARD, text("Writing the retro")), say: "Fleet keeps the retro" },
  { look: inside(CARD, text("Retro 1 written · 5 items")), say: "It folds to one line, and stays in the thread" },
  { look: inside(CARD, text("Walks opened in your browser")), say: "The items by title" },
  { press: inside(CARD, button("Open retro")), say: "Open retro is the one way it goes to the Retros page" },
  { look: RETRO, say: "That retro, on the Retros page" },
  { look: inside(RETRO, text(/Fix the flaky store test · s-01IDLECC/)), say: "Headed by the Job's title and address" },
  { look: inside(RESET, text(/^Agent$/)), say: "An agent's way" },
  { look: inside(RESET, text(/^Armada$/)), say: "And where the fix lands" },
  { look: item("Walks opened in your browser"), say: "Five items, from today's pains" },
  { press: inside(RESET, button("Evidence")), say: "The record rows it cites" },
  { look: inside(RESET, text("Allow git reset --hard origin/main?")), say: "The ask, and how long it waited" },
  { press: inside(RESET, button("Start Job", { exact: true })), say: "Start Job agrees with it" },
]);

export { retro as "session-retro" };
