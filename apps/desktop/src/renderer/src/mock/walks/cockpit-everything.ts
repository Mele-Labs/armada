// Every change to the Cockpit, in the order he asked for them: it opens on the timeline with a pull request
// on the track of the Job or Session that owns it and no lane of its own; no dispatch bar, and n reaches the
// title bar's; Autopilot is a control in the header band that asks for a repository and opens its panel at
// the top; the filters are Active, Done and Bays; a call arrives as a transmission in the tray, rings in its
// hue and waits; w opens it and Enter answers it; Grid shows the tiles on the ruled background, with the tray
// on every filter. Nothing is remembered. Over `cockpit-everything`.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, tab, walk } from "../walk";

const cockpit = region("Dashboard");
const timeline = region("Timeline");
const autopilot = button("Autopilot", { exact: true });
const plan = region(/^Plan question/);
const sessions = region("Sessions");
const tray = role("group", "Calls waiting");
const call = (title: RegExp) => region(title);
const row = (title: string) => inside(sessions, role("listitem", title));
const toCockpit = { press: inside(role("navigation", "Work"), button("Cockpit")), say: "Back to the Cockpit" } as const;

const cockpitEverything = walk("cockpit-everything", [
  { look: timeline, say: "The Cockpit opens on the timeline, with nothing remembered" },
  { look: inside(timeline, role("list", "Moving")), say: "A track for each Job and each Session. No track is a pull request" },
  { hover: inside(timeline, button(/Queued, place 1/)), say: "A Job's track carries its pull request: #1890 queued, in the square shape of a pull request" },
  { hover: inside(timeline, button(/Check store passed/)), say: "A Session's track carries its own: #1896, on the branch the Session works on" },
  { look: autopilot, say: "No dispatch bar under the title bar. Autopilot is a control in the header band" },
  { key: "n", on: cockpit, say: "n reaches the title bar's dispatch, where the work is typed" },
  { look: role("combobox"), say: "The title bar's field, ready for the work" },
  { key: "Escape", on: role("combobox"), say: "Esc puts it away" },
  { press: autopilot, say: "No repository is picked, so Autopilot asks for one in the title row's repository menu" },
  { look: role("searchbox", "Find a repository"), say: "The same menu, filter field and all" },
  { press: role("menuitem", /armada/), say: "Pick armada" },
  { look: region("Autopilot"), say: "Its panel opens at the very top of the Cockpit, above the filters" },
  { look: tab("Active"), say: "The filters are Active, Done and Bays. There is no Your move" },
  { look: tab("Done"), say: "Done" },
  { look: tab("Bays"), say: "Bays" },
  { later: cockpit, say: "Time passes, and nothing is new" },
  { later: cockpit, say: "A Drone reaches a question" },
  { look: button(/^Plan question/), say: "It arrives as a transmission in the tray along the foot: it slides up and its hue rings out, then it waits, and the timeline is not covered" },
  { key: "w", on: cockpit, say: "w opens it" },
  { look: plan, say: "The whole card, with the context to answer it" },
  { key: "2", on: cockpit, say: "A number picks an answer" },
  { key: "Enter", on: cockpit, say: "Enter sends it, and the card goes" },
  { later: cockpit, say: "A Check fails on another Job" },
  { look: button(/^Check failed/), say: "The next call arrives the same way, and waits" },
  { press: button("Grid"), say: "Grid shows the tiles" },
  { look: role("listbox", "Tiles"), say: "The tiles, on the ruled background that runs the width of the panel" },
  { look: button(/^Check failed/), say: "The call still waits in the tray" },
  { press: tab("Done"), say: "Done" },
  { look: button(/^Check failed/), say: "The tray is there on every filter" },
  { press: tab("Bays"), say: "Bays" },
  { look: button(/^Check failed/), say: "And here" },

  // What a Session is doing is one answer everywhere: `session-standing`.
  { press: tab("Active"), say: "Active again" },
  { press: button("Timeline"), say: "The timeline again" },
  { later: cockpit, say: "Four Sessions report in: one replied, one is done, one went quiet mid-turn, one asked a question" },
  { look: inside(timeline, role("list", "Needs you")), say: "The Session that replied and the one that is done stand in Needs you, beside the one that asked. Neither is moving and neither is stuck, though each ended on a live turn" },
  { hover: inside(timeline, role("img", "Replied")), say: "A reply: its turn ended on a message to him" },
  { hover: inside(timeline, role("img", "Done")), say: "Done: the agent said nothing is left" },
  { look: inside(inside(timeline, role("list", "Stuck")), button(/^Session: Unused token audit/)), say: "Stuck holds the one Session that went quiet mid-turn, and neither of the two that ended their turns" },
  toSessions,
  { look: inside(row("Trim the fixture clock"), role("img", "Replied")), say: "The Sessions page shows the same Sessions with the same marks: a reply is the Reply glyph" },
  { look: inside(row("Changelog script"), role("img", "Done")), say: "Done is the power glyph" },
  { look: inside(row("Unused token audit"), role("img", "Stuck")), say: "And the quiet one reads Stuck here as it does on the timeline" },
  toCockpit,
  { look: inside(tray, button(/^Session: Cache key/)), say: "The question the agent asked waits in the tray. The reply and the done wait with it, raised from what happened and not from a tool call" },
  { press: inside(tray, button(/^Session: Cache key/)), say: "Open the question" },
  { key: "o", on: cockpit, say: "o opens the Session, to see what it asked" },
  toCockpit,
  { look: inside(tray, button(/^Session: Trim the fixture clock/)), say: "His question is gone from the calls: he was in the Session after it began waiting. The reply waits" },
  { press: inside(tray, button(/^Session: Trim the fixture clock/)), say: "A press opens the reply: the agent's first line, with a box to answer in" },
  { type: "Good. Merge it.\n", into: inside(call(/^Session: Trim the fixture clock/), role("textbox", "Message")), say: "Answered in words, and the card clears" },
  { press: inside(tray, button(/^Session: Changelog script/)), say: "The done: end the Session, or keep going" },
  { look: button(/Keep going/), say: "Keep going is the other act" },
  { press: button(/End session/), say: "End session, and the card clears" },
]);

export { cockpitEverything as "cockpit-everything" };
