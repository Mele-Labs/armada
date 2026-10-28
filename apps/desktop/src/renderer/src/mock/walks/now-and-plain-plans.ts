// What a Session is doing for his message: Now and the page being built in Now as the turn moves on, the page
// opened large, and the turn's end; then the plan in his words, its groups under Details and Not yet. A second,
// `now-and-plain-plans-refused`, has Fleet refuse Auto land, so the switch reverts with the refusal's words in the toast.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";
import { LIVE_TITLE, NOT_YET_WORDS, REFUSAL_WORDS, TITLE } from "../scenarios/now-and-plain-plans";

// The Job is at Plan until its turn ends, then at Build: the current step's zone holds Now.
const now = region("Plan", { exact: true });
const built = region("Build", { exact: true });
const account = role("list", "Now");
const live = role("figure", LIVE_TITLE);
const card = role("article", "Plan to approve");
const controls = region("Flight controls");

const open = { press: deckRow(TITLE), say: "A Session working on your message" };
const reply = { look: text(/^The page is up with its three switches/), say: "The reply" };

const plan = [
  { look: inside(card, text("The three switches keep their setting")), say: "The plan in your words first" },
  { look: inside(card, role("list", "What each group gives")), say: "One line a group: what it gives" },
  { press: inside(card, button("Details")), say: "Details opens the groups" },
  { look: inside(card, role("img", /^Subagent type/)), say: "Subagent types, paths and task marks" },
  { press: inside(card, button("Show less")), say: "Show less folds them" },
  { press: inside(card, button("Not yet")), say: "Not yet, beside Approve" },
  { look: button("Plan set aside"), say: "The card folds to a chip" },
  { look: role("textbox", "Message"), say: "Nothing waits in Needs you" },
  { look: text(NOT_YET_WORDS), say: "The words the agent was sent" },
];

const nowAndPlainPlans = walk("now-and-plain-plans", [
  open,
  { look: inside(now, account), say: "Now lists the task finished last, the one being worked on, the one next" },
  { look: inside(now, role("img", "Working on this now")), say: "The task it is on is marked, with what the turn is doing under it" },
  { look: inside(now, live), say: "The page it is building, in Now" },
  { later: inside(now, account), say: "It moves on" },
  { later: inside(now, live), say: "The page moves with it" },
  { press: inside(now, button(`Open ${LIVE_TITLE}`)), say: "Open it large" },
  { key: "Escape", on: dialog(LIVE_TITLE), say: "Esc closes it" },
  { later: inside(now, account), say: "The last part" },
  { look: inside(built, role("button", "Approve")), say: "Turn over: the plan to approve is in Now, with Approve at its foot" },
  reply,
  ...plan,
]);

const nowAndPlainPlansRefused = walk("now-and-plain-plans-refused", [
  open,
  { press: inside(controls, button("Auto land")), say: "Auto land" },
  { look: inside(controls, role("img", "Auto land on")), say: "On at the press" },
  { look: text(REFUSAL_WORDS), say: "Fleet refuses: the switch reverts, the words are in the toast" },
  { look: inside(controls, role("img", "Auto land off")), say: "Off again" },
]);

export { nowAndPlainPlans as "now-and-plain-plans", nowAndPlainPlansRefused as "now-and-plain-plans-refused" };
