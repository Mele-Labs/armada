// What a Session is doing for his message, three takes on where it sits: in the thread under the message, in a panel beside the thread,
// or as a band on the composer. Each walks the account and the page being built as the turn moves on, and the turn's end; the first
// goes on to the plan in his words, its groups under Details and Not yet. A fourth, `now-and-plain-plans-refused`, has Fleet refuse
// Auto land, so the switch reverts with the refusal's words in the toast.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { LIVE_TITLE, NOT_YET_WORDS, REFUSAL_WORDS, TITLE } from "../scenarios/now-and-plain-plans";

const thread = region("Thread");
const account = role("list", "Account");
const live = role("figure", LIVE_TITLE);
const now = region("Now", { exact: true });
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

const nowAndPlainPlansThread = walk("now-and-plain-plans-thread", [
  open,
  { look: inside(thread, account), say: "Under your message: done, on now, next" },
  { look: inside(thread, live), say: "The page it is building" },
  { later: inside(thread, account), say: "It moves on" },
  { later: inside(thread, live), say: "The page moves with it" },
  { later: inside(thread, account), say: "The last part" },
  { look: inside(thread, button(/Checked it reads in a narrow window/)), say: "Turn over: one line above the reply" },
  reply,
  ...plan,
]);

const nowAndPlainPlansSide = walk("now-and-plain-plans-side", [
  open,
  { look: inside(now, account), say: "Beside the thread: done, on now, next" },
  { look: inside(now, live), say: "The page it is building, under it" },
  { later: inside(now, account), say: "It moves on" },
  { later: inside(now, live), say: "The page moves with it" },
  { later: inside(now, account), say: "Turn over: all done, the page stays" },
  reply,
]);

const nowAndPlainPlansBand = walk("now-and-plain-plans-band", [
  open,
  { press: button(/^Now: /), say: "On the composer: what it is on now. Open it" },
  { look: inside(now, live), say: "Done, next, and the page it is building" },
  { later: inside(now, live), say: "Both move on" },
  { key: "Escape", on: button(/^Now: /), say: "Esc folds it" },
  { later: button(/^Now: /), say: "It moves on" },
  { later: button(/^Now: /), say: "The last part" },
  { look: button("Needs you"), say: "Turn over: the plan waits on you, and Needs you takes the band's place" },
]);

const nowAndPlainPlansRefused = walk("now-and-plain-plans-refused", [
  open,
  { press: inside(controls, button("Auto land")), say: "Auto land" },
  { look: inside(controls, role("img", "Auto land on")), say: "On at the press" },
  { look: text(REFUSAL_WORDS), say: "Fleet refuses: the switch reverts, the words are in the toast" },
  { look: inside(controls, role("img", "Auto land off")), say: "Off again" },
]);

export {
  nowAndPlainPlansThread as "now-and-plain-plans-thread",
  nowAndPlainPlansSide as "now-and-plain-plans-side",
  nowAndPlainPlansBand as "now-and-plain-plans-band",
  nowAndPlainPlansRefused as "now-and-plain-plans-refused",
};
