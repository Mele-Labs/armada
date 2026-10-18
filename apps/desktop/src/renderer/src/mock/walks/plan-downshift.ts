// The weekly window nearing its end: past 80% Bridge offers the Job model a step down until the window
// resets, in a notification and on the plan limits card, and offers the way back once it has. The Session's
// model list says which Session default it follows throughout. `planDownshiftChoosing` presses Choose… instead.
// Over `plan-downshift`.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

/** The ring, and not the notification whose sentence also starts with the limit's name. */
const weekly = role("button", /Weekly limit \d+%$/);
const card = inside(dialog("Plan limits"), region("Weekly limit"));
const bell = (words: string) => inside(weekly, role("img", words));
const following = (model: string) => role("option", `${model} · default`);

/** To the Session whose model list shows the default, and past 80%. */
const crossing: Step[] = [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Cart totals rounding/)), say: "A Job with no model chosen" },
  { look: following("opus"), say: "Its model list shows the default" },
  { look: weekly, say: "The weekly limit at 75%: nothing offered" },
  { later: weekly, say: "The week passes 80%" },
  { look: text("Weekly limit 83% used"), say: "A notification, with the offer" },
  { look: bell("Default to sonnet offered"), say: "The ring wears a bell while the offer stands" },
  { hover: bell("Default to sonnet offered"), say: "Hover for what it is" },
  { press: weekly, say: "A press keeps the card open" },
  { look: inside(card, text("Default model")), say: "The default now, and the two acts" },
];

const planDownshift = walk("plan-downshift", [
  ...crossing,
  { press: inside(card, button("Default to sonnet")), say: "Approve" },
  { look: inside(card, text("sonnet until")), say: "The card states it, to the window's reset" },
  { look: following("sonnet"), say: "The Job's model list follows" },
  { key: "Escape", on: weekly, say: "Esc closes the card" },
  { later: weekly, say: "The window resets" },
  { look: text("Weekly limit reset"), say: "A second notification, with the way back" },
  { look: bell("Back to opus offered"), say: "The bell is back" },
  { press: weekly, say: "The card" },
  { look: inside(card, button("Keep sonnet")), say: "Back, keep, or choose" },
  { press: inside(card, button("Back to opus")), say: "Back to opus" },
  { look: following("opus"), say: "The default is opus again" },
]);

const planDownshiftChoosing = walk("plan-downshift", [
  ...crossing,
  { press: inside(card, button("Choose…")), say: "Choose…" },
  { look: text("Job model"), say: "Settings opens at the models" },
  { type: "haiku", into: role("textbox", "Job model", { exact: true }), say: "Set it by hand" },
  { press: button("Save Job model"), say: "Save" },
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Cart totals rounding/)), say: "The Job" },
  { look: following("haiku"), say: "Its model list follows" },
]);

export { planDownshift, planDownshiftChoosing };
