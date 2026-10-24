// A question whose options carry a preview: the checks an option stands for are drawn as a card
// beside it while it is pointed at, as the agent wrote them, and nothing is drawn for an option with
// none. Over the `session-question-preview` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { rail } = kit(false);
const sessions = region("Jobs", { exact: true });
const ask = role("article", "Waiting on you");

const preview = walk("session-question-preview", [
  toSessions,
  { press: rail("Sessions"), say: "The Jobs page" },
  { press: inside(sessions, button(/Pick the checks/)), say: "A Job asking which checks to use" },
  { look: inside(ask, text("Use these checks for ops, consumer and api-gateway?")), say: "The question, and its options" },
  { hover: inside(ask, role("radio", "The shared set")), say: "Pointing at an option opens a card beside it with the checks it means, as the agent wrote them" },
  { hover: inside(ask, role("radio", "None yet")), say: "An option the agent attached nothing to shows no card, and the options have not moved" },
  { press: inside(ask, role("radio", "Per service")), say: "Choosing one" },
  { press: inside(ask, button("Answer")), say: "Sends it" },
]);

export { preview as "session-question-preview" };
