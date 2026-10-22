// What a Session is doing for his message, at the top of Progress: the account and the page being built as the turn moves on, the page
// opened large, and the turn's end; then the plan in his words, its groups under Details and Not yet. A second,
// `now-and-plain-plans-refused`, has Fleet refuse Auto land, so the switch reverts with the refusal's words in the toast.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";
import { LIVE_TITLE, NOT_YET_WORDS, REFUSAL_WORDS, TITLE } from "../scenarios/now-and-plain-plans";

const progress = region("Progress", { exact: true });
const account = role("list", "Account");
const live = role("figure", LIVE_TITLE);
const card = role("article", "Plan to approve");
const controls = region("Flight controls");

const open = { press: deckRow(TITLE), say: "A Session working on your message" };
const reply = { look: text(/^The plan card now opens on what you get/), say: "The reply" };

const plan = [
  { look: inside(card, text("A plan reads in plain words first")), say: "The plan in your words first" },
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
  { look: inside(progress, account), say: "At the top of Progress: done, on now, next" },
  { look: inside(progress, live), say: "The page it is building" },
  { later: inside(progress, account), say: "It moves on" },
  { later: inside(progress, live), say: "The page moves with it" },
  { press: inside(progress, button(`Open ${LIVE_TITLE}`)), say: "Open it large" },
  { key: "Escape", on: dialog(LIVE_TITLE), say: "Esc closes it" },
  { later: inside(progress, account), say: "The last part" },
  { look: inside(progress, account), say: "Turn over: all done, until your next message" },
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
