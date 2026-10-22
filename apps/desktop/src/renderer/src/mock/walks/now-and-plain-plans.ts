// What a Session is on, under its head and on its row; the plan in his words with the technical groups under Details; Not yet
// beside Approve, which sets the plan aside; and Pause, Resume and Auto land drawn at the press while Fleet answers two seconds
// on. Three walks over the `now-and-plain-plans` scenarios, one a take: this one; `now-and-plain-plans-said`, where the agent's
// last sentence is a third line under the Now line; and `now-and-plain-plans-refused`, where Auto land is refused and the switch
// reverts with the refusal's words in the toast. The first scenario takes the same two takes from its url, `now=said` and
// `press=refused`, for a look without a walk.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { NOT_YET_WORDS, REFUSAL_WORDS, TASK, TITLE } from "../scenarios/now-and-plain-plans";

const jobs = region("Jobs", { exact: true });
const frame = region(/^Job /);
const now = role("group", "Now");
const card = role("article", "Plan to approve");
const controls = region("Flight controls");

const open = { press: deckRow(TITLE), say: "A Session mid-turn, its plan waiting" };
const nowLine = { look: now, say: "Under the head: the task it is on, then the action Fleet detected" };
const subagents = { look: inside(now, role("list", "Subagents")), say: "One line a running subagent: what it was sent to do, then its own action" };
const row = { look: inside(jobs, text(TASK)), say: "The row in the list carries the first line" };

const plan = [
  { look: inside(card, text("A plan reads in plain words first")), say: "The plan in his words first" },
  { look: inside(card, role("list", "What each group gives")), say: "One line a group: what it gives" },
  { press: inside(card, button("Details")), say: "Details opens the groups" },
  { look: inside(card, role("img", /^Subagent type/)), say: "Subagent types, paths and task marks" },
  { press: inside(card, button("Show less")), say: "Show less folds them" },
  { press: inside(card, button("Not yet")), say: "Not yet, beside Approve" },
  { look: button("Plan set aside"), say: "The card folds to a chip" },
  { look: role("textbox", "Message"), say: "Nothing waits in Needs you" },
  { look: text(NOT_YET_WORDS), say: "The words the agent was sent" },
];

const pause = [
  { press: inside(controls, button("Pause")), say: "Pause" },
  { look: inside(controls, role("img", "Sending")), say: "The going mark, until Fleet answers" },
  { look: inside(jobs, role("img", "Paused")), say: "Paused at the press" },
  { look: inside(frame, role("img", /^Idle/)), say: "Fleet's answer lands two seconds on: the going mark goes" },
  { press: inside(controls, button("Resume")), say: "Resume" },
  { look: inside(frame, role("img", /^Working/)), say: "Working again" },
];

const nowAndPlainPlans = walk("now-and-plain-plans", [
  open,
  nowLine,
  subagents,
  row,
  ...plan,
  ...pause,
  { press: inside(controls, button("Auto land")), say: "Auto land" },
  { look: inside(controls, role("img", "Auto land on")), say: "On at the press" },
]);

const nowAndPlainPlansSaid = walk("now-and-plain-plans-said", [
  open,
  nowLine,
  subagents,
  { look: inside(now, text("Folding the technical groups under Details")), say: "The agent's last sentence, faintest, under the subagents" },
  row,
]);

const nowAndPlainPlansRefused = walk("now-and-plain-plans-refused", [
  open,
  { press: inside(controls, button("Auto land")), say: "Auto land" },
  { look: inside(controls, role("img", "Auto land on")), say: "On at the press" },
  { look: text(REFUSAL_WORDS), say: "Fleet refuses: the switch reverts, the words are in the toast" },
  { look: inside(controls, role("img", "Auto land off")), say: "Off again" },
]);

export { nowAndPlainPlans as "now-and-plain-plans", nowAndPlainPlansSaid as "now-and-plain-plans-said", nowAndPlainPlansRefused as "now-and-plain-plans-refused" };
