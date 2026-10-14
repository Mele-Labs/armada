// The Cockpit as a flight deck, in the real window: the places as tabs in the title bar, every Job and Session in a column on
// the left, the stage in the middle, the merge queue under it, and the bays, incoming calls, flight controls and Autopilot
// beside. The preview switch is on for this scenario. The battlefield, a Job's mission, the replay and the map's call markers
// are #2301 and are not in this walk.

import { button, inside, region, role, tab, text, walk } from "../walk";

const places = role("navigation", "Places");
const list = region("Jobs and Sessions");
const controls = region("Flight controls");
const incoming = region("Incoming calls");
const stage = (name: string | RegExp) => region(name);

const cockpitFlightDeck = walk("cockpit-ship-flightdeck", [
  { look: inside(places, tab("Cockpit")), say: "The Cockpit tab is raised: filled and weighed. Jobs, Sessions and System are quiet beside it" },
  { look: region("Fleet timeline"), say: "Nothing picked, so the stage is the fleet's event timeline. The battlefield is not built yet" },
  { press: inside(list, button("Shorten the reconnect wait")), say: "Pick a Job from the column" },
  { look: inside(stage(/^Job: Shorten the reconnect wait/), role("group", "Canvas")), say: "A Job opens as its run on the canvas, opened on the step at work" },
  { look: inside(stage(/^Job: Shorten the reconnect wait/), role("group", "Running now")), say: "And what it is running now, beside it" },
  { press: inside(stage(/^Job: Shorten the reconnect wait/), button("Open the Job page")), say: "Open the Job page opens the page itself, as a layer over the deck" },
  { press: inside(places, tab("Cockpit")), say: "The Cockpit tab in the title bar returns to the deck as it was left, with the Job still open on the stage" },
  { look: inside(region("Activity"), text(/Shorten the reconnect wait/)), say: "Activity is the Cockpit's feed, newest first, in the column under the list" },
  { later: places, say: "A Session replies" },
  { look: inside(incoming, button(/Pin the store clock/)), say: "A call arrives in Incoming calls" },
  { press: inside(incoming, button(/Pin the store clock/)), say: "Pressing a call opens the Session it is about, as a row of the column does. That is the visit, so the call leaves Incoming calls" },
  { look: stage(/^Session: Pin the store clock/), say: "The Session, with its thread and its box" },
  { press: inside(stage(/^Session: Pin the store clock/), button("Back")), say: "Back to the stage as it was" },
  { look: inside(region("Bays"), button(/^slot-5, armada, Free/)), say: "Each bay is two lines: the bay and its state as a glyph with how long it has been held, then who holds it and its branch. A tooltip names each glyph" },
  { press: inside(region("Bays"), button(/^slot-6, armada, Stranded/)), say: "A stranded bay" },
  { look: stage("Bay: slot-6"), say: "The pane has the same facts in full and the acts the Bays page offers for it: Pick up, Stash and Scrap" },
  { press: inside(region("Bays"), button(/^slot-2, armada, Held/)), say: "A leased bay" },
  { look: inside(stage("Bay: slot-2"), region("History")), say: "Under the facts, the bay's history from its own events: each holder, its branch, from when to when, and how it ended" },
  { press: inside(region("Merge queue"), button(/^#1896/)), say: "The merge queue is one row under the stage: a mark for each pull request, one light for main" },
  { look: stage("Pull request #1896"), say: "What the forge says about it" },
  { press: inside(controls, button("Checks")), say: "Checks, from the flight controls, is a plain filter like Jobs, Sessions and Calls. It opens nothing" },
  { look: inside(list, button("Add the pause marker column")), say: "The column, the feed and the timeline narrow to the Jobs and Sessions with a Check running now" },
  { look: stage("Pull request #1896"), say: "The pull request that was open stays" },
  { press: inside(region("Autopilot"), role("switch", "Autopilot")), say: "Autopilot is a compact card beside the controls. Its switch takes the stage for its lane" },
  { look: stage("Autopilot lane"), say: "The lane, in the stage. The controls and the merge queue stay" },
  { press: inside(region("Autopilot"), role("switch", "Autopilot")), say: "Switched off, the stage is as it was left" },
  { press: inside(controls, button("Checks")), say: "Pressed again, the filter is put away and the column is whole" },
  { press: inside(controls, button("Needs me")), say: "Needs me narrows the column to what holds a call" },
  { press: inside(controls, button("Jobs only")), say: "Jobs only narrows it again" },
  { press: inside(controls, button("Timeline")), say: "Timeline puts the stage back on the event timeline" },
  { look: inside(region("Fleet timeline"), text(/Shorten the reconnect wait/)), say: "The timeline keeps the same filters" },
  { key: "c", on: list, say: "c goes to Incoming calls: the panel is lit, its heading and a hairline frame, and the first call is focused" },
  { key: "ArrowDown", on: incoming, say: "Arrows, or j and k, move inside the lit panel by where things are drawn" },
  { key: "Enter", on: incoming, say: "Enter opens it on the stage" },
  { key: "Escape", on: stage(/^(Job|Session|Call|Pull request)/), say: "Esc puts it away, and focus is still in the panel" },
  { key: "1", on: incoming, say: "1 to 6 press the flight tiles in order: this is Jobs only" },
  { key: "Shift+?", on: incoming, say: "? opens the keys" },
  { look: role("dialog", "Keys"), say: "Every key the deck answers" },
  { key: "Escape", on: role("dialog", "Keys"), say: "Esc closes it" },
]);

export { cockpitFlightDeck };
