// A Job's agent disputed a notice of Fleet's. Fleet's card carries the Disputed mark, the line under the agent's reply says why
// and how it was spotted, and its link opens the review Job Fleet started. Over the `fleet-notice-disputed` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { thread } = kit(false);
const jobs = region("Jobs", { exact: true });
const card = inside(thread, region("Message from Fleet"));
const line = inside(thread, region("Notice disputed"));

const fleetNoticeDisputed = walk("fleet-notice-disputed", [
  toSessions,
  { press: inside(jobs, button("Port the store to the new clock")), say: "A Job Fleet told about its subagents' spend" },
  { look: inside(card, text("Subagents burned 5.0 units in your last hour")), say: "Fleet's notice, on its own card" },
  { hover: inside(card, role("img", "Disputed")), say: "The card carries the Disputed mark" },
  { look: inside(thread, text("The opus spend came from building agents, which write the code.")), say: "The agent's reply" },
  { look: inside(line, text("Said by the agent")), say: "Under it, one line from Fleet: the agent's reason, and that the agent said so itself" },
  { press: inside(line, button("Open the review job")), say: "The link opens the review Job Fleet started in the armada repository" },
  { look: inside(thread, text("Port the store to the new clock disputed this notice.")), say: "The review Job, with the notice and the reason it was given" },
]);

export { fleetNoticeDisputed as "fleet-notice-disputed" };
