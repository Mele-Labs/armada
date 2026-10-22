// A Job's agent disputed a notice of Fleet's. Fleet's card carries the Disputed mark, the line under the agent's reply says why
// and how it was spotted, and its link opens the review Job Fleet started, linked under the disputing Job in the list. The
// review Job's thread ends on its verdict and a question with options, and Settings, Features holds the switch for it.
// Over the `fleet-notice-disputed` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, deckRow, inside, region, role, text, toSystemPage, walk } from "../walk";

const { thread } = kit(false);
const jobs = region("Jobs", { exact: true });
const card = inside(thread, region("Message from Fleet"));
const review = deckRow("Disputed: opus spend notice");
const ask = role("article", "Waiting on you");

const fleetNoticeDisputed = walk("fleet-notice-disputed", [
  toSessions,
  { press: inside(jobs, button("Port the store to the new clock")), say: "A Job Fleet told about its subagents' spend" },
  { look: inside(thread, text("Subagents burned 5.0 units in your last hour")), say: "Fleet's notice, on its own card" },
  { hover: inside(card, role("img", "Disputed")), say: "The card carries the Disputed mark" },
  { look: inside(thread, text("The opus spend came from building agents, which write the code.")), say: "The agent's reply" },
  { look: inside(thread, text("Said by the agent")), say: "Under it, one line from Fleet: the agent's reason, and that the agent said so itself" },
  { look: review, say: "The review Job Fleet started, named for the notice, under the Job that disputed it" },
  { hover: inside(review, role("img", "Linked Job")), say: "Its link mark: the disputing Job started it" },
  { look: inside(thread, text(/^The agent had already compacted at 120k/)), say: "A second notice, read from the agent's replies: the reason is on top, with how it was spotted and the time under it, and with no review started there is no press" },
  { press: inside(thread, button("Open the review job")), say: "Review job opens it" },
  { look: inside(thread, text("Port the store to the new clock disputed this notice.")), say: "Fleet's first message: the notice and the reason it was given" },
  { look: inside(thread, text(/^Fleet was wrong\. The rule counts every opus subagent turn/)), say: "The review agent's verdict" },
  { look: inside(ask, text("What should change about the opus spend notice?")), say: "Its question to you, before it changes anything" },
  { look: inside(ask, role("radio", "Fix the rule to count only reading agents")), say: "Fix the rule" },
  { look: inside(ask, role("radio", "Send the notice less often")), say: "Send it less often" },
  { look: inside(ask, role("radio", "Leave it")), say: "Or leave it" },
  ...toSystemPage("Features", "Settings, Features"),
  { look: role("switch", /^Dispute reviews/), say: "Dispute reviews: off, and no review Job is started for a disputed notice" },
]);

export { fleetNoticeDisputed as "fleet-notice-disputed" };
