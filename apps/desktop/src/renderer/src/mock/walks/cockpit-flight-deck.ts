// The Cockpit as a flight deck, in the real window: the places as tabs in the title bar, every Job and Session in a column on
// the left, the map until something is picked and then that thing, the merge queue as one row under it, and the bays, incoming
// calls, flight controls and Autopilot beside. The battlefield draws from the Fleet's own data: a drone for each Job and
// Session, resting by its pad until a path is known. The preview switch is on for this scenario. #2301.

import { button, inside, region, role, tab, text, walk } from "../walk";

const places = role("navigation", "Places");
const list = region("Jobs and Sessions");
const controls = region("Flight controls");
const incoming = region("Incoming calls");
const stage = (name: string | RegExp) => region(name);

const cockpitFlightDeck = walk("cockpit-ship-flightdeck", [
  { look: inside(places, tab("Cockpit")), say: "The Cockpit tab is raised: filled and weighed. Jobs, Sessions and System are quiet beside it" },
  { look: role("listbox", "Map"), say: "Nothing picked, so the area is a battlefield seen on a tilted plane. Each repository is a territory laid out as its own directories, a raised slab for each that stands taller with recent activity, its base along the near edge with a pad for each bay and its main as the flag. A drone flies above the plane with its shadow below, over the directories it is changing, a Job large and a Session small, and a dotted line runs from a bay to the drone that holds it. Where two drones change the same directory the slab is the tallest, amber and hatched. A flag is the main of one repository, green while it is green and red when it is red. It rests zoomed out to take the whole field in, with every territory, base, flag and drone inside the pane. A drone that holds a call he has not opened is the brightest thing on it: a badge over it at one size however far he stands, a pillar of light up from its ground, amber when the call is owed and red when its Check is failing. A badge at the pane's border points to one that is out of view, and pressing it eases the camera there. Drag pans the field and the wheel or a pinch zooms it, and a double click on empty ground comes back to the whole field" },
  { press: inside(list, button("Shorten the reconnect wait")), say: "Pick a Job from the column" },
  { look: inside(stage(/^Job: Shorten the reconnect wait/), role("listbox", "Route")), say: "A Job opens as its mission, the default: the battlefield's own plane cut down to this one Job. Its workflow is a dotted route of pins, green and lit behind it, an orange pin under its drone at the step at work, grey ahead, red with a flare where a step failed and amber with a beacon where one waits on him. The plots it touches stand beside the route, its Sessions and Drones near the steps they serve, its bay at the start and its pull request at the end. The camera has eased to the step at work. Drag pans, the wheel zooms, 0 is the whole route, and pressing a pin opens that step's own card" },
  { look: inside(stage(/^Job: Shorten the reconnect wait/), role("group", "Running now")), say: "And what it is running now, beside it" },
  { key: "v", on: stage(/^Job: Shorten the reconnect wait/), say: "v swaps the head's Mission for Canvas, pressed for him: the same run as the Job page's own canvas, read only" },
  { look: inside(stage(/^Job: Shorten the reconnect wait/), role("group", "Canvas")), say: "The canvas, opened on the step at work with the cards readable. The choice is kept for the window" },
  { key: "0", on: list, say: "0 is the canvas's own Fit, pressed for him: the whole run in the frame. In the mission it is the whole route" },
  { key: "v", on: stage(/^Job: Shorten the reconnect wait/), say: "And v again is the mission" },
  { press: inside(stage(/^Job: Shorten the reconnect wait/), button("Open the Job page")), say: "Open the Job page opens the page itself, as a layer over the deck" },
  { press: inside(places, tab("Cockpit")), say: "The Cockpit tab in the title bar returns to the deck as it was left, with the Job still open in the area" },
  { look: inside(region("Activity"), text(/Shorten the reconnect wait/)), say: "Activity is the Cockpit's feed, newest first, in the column under the list" },
  { later: places, say: "A Session replies, and the drones that are working take up what their tools are doing: one reads its directory under a scan grid, one writes and the slab rises under a laser, one runs and circles its Check on a ring" },
  { look: inside(incoming, button(/Pin the store clock/)), say: "A call arrives in the panel under the Bays and rings until he opens it" },
  { press: inside(incoming, button(/Pin the store clock/)), say: "Pressing a call opens the Session it is about, as a row of the column does. That is the visit, so the call leaves Incoming calls" },
  { type: "Good. Merge it.\n", into: inside(stage(/^Session: Pin the store clock/), role("textbox", "Message")), say: "Answered in the Session's own box" },
  { press: inside(stage(/^Session: Pin the store clock/), button("Back")), say: "Back, to the map as it was" },
  { look: role("listbox", "Map"), say: "The Session he answered is locked on: the reticle is on it, the camera has eased to it and the others are dimmed" },
  { later: places, say: "A Job is dispatched and a pull request merges, and the working drones change what they do, read to write to run: the camera backs off a little, a drone climbs from the base on an arc and settles over the directories it touches, the slab rises, and the pull request arcs to the flag, which flashes and sends a ripple across the plane and a column of light up" },
  { press: inside(region("Activity"), button(/Gate policy on every run/)), say: "A row of Activity is an event. Pressing it focuses the drone that did it: a ring pings under it, the slabs it touched pulse, the rest dim, and a line names what moved and how long ago. Pressing the row again, or Esc, lets go" },
  { look: region("Event"), say: "The line over the map says what the event was and when" },
  { press: button("Replay from here"), say: "Replay from here puts the field back as it was at that event, and plays it on to now" },
  { look: role("region", "Replay"), say: "Play and pause, 1x, 4x and 16x, and a track from that event to now with a tick for each event. Drones, slabs and pull requests follow the events up to the playhead; the column and the panels stay live. Space plays, the arrows step an event, Home starts over" },
  { press: inside(role("region", "Replay"), button("Live")), say: "Live is the field as it is now. Esc does the same" },
  { key: "ArrowDown", on: list, say: "Down moves through the column" },
  { key: "Enter", on: list, say: "Enter opens the row in the area" },
  { look: stage(/^(Job|Session): /), say: "A Job or a Session, in the same place" },
  { key: "Escape", on: stage(/^(Job|Session): /), say: "Esc is back to the map" },
  { press: inside(list, button("Migration notes")), say: "A Session from the column" },
  { look: inside(stage(/^Session: Migration notes/), role("textbox", "Message")), say: "Its thread and its box, as the Session's page has them" },
  { look: inside(region("Bays"), button(/^slot-5, armada, Free/)), say: "Each bay is two lines. The first is the bay and its state as a glyph, in its own colour: a key lit blue where it is leased, with a small Job or Session glyph for whoever holds it, an open door in grey where it is free, an amber ghost that slowly pulses where it is stranded, a locked door where it is closed. The last figure is how long it has been held, or how long since it was given back. The second line is the facts behind it. Leased: the holder, its branch. Free: warm or cold. Stranded: who held it last, and small glyphs for what is left on it and whether the holder was told. A tooltip names each" },
  { press: inside(region("Bays"), button(/^slot-6, armada, Stranded/)), say: "A stranded bay" },
  { look: stage("Bay: slot-6"), say: "The pane has the same facts in full: who held it last, whether they were told, what is uncommitted and unpushed, what stranded means, and the acts the Bays page offers for it: Pick up, Stash and Scrap. Scrap asks first, naming what would go" },
  { press: inside(region("Bays"), button(/^slot-2, armada, Held/)), say: "A leased bay" },
  { look: stage("Bay: slot-2"), say: "What holds it, and since when" },
  { look: inside(stage("Bay: slot-2"), region("History")), say: "Under the facts, the bay's history: a strip of the last day in a block for each holder, so quiet and busy stretches read at a glance, and below it the holders newest first on a rail. Each row is the holder, its branch, from when to when and for how long, and how it ended as a glyph: still held, given back clean, given back with its branch kept, stranded, or taken by the owner. A holder the ship still has opens on a press, and Esc comes back to the bay. j and k move through the rows. In a replay it is the history as of the playhead" },
  { press: inside(region("Merge queue"), button(/^#1896/)), say: "The merge queue is one row under the map: a mark for each pull request, one light for main" },
  { look: stage("Pull request #1896"), say: "What the forge says about it" },
  { press: inside(controls, button("Checks")), say: "Checks, from the flight controls, is a plain filter like Jobs, Sessions and Calls. It opens nothing and the area stays on the map" },
  { look: inside(list, button("Add the pause marker column")), say: "The column, the feed, the map and the timeline narrow to the Jobs and Sessions with a Check running now" },
  { look: stage("Pull request #1896"), say: "The pull request that was open stays. Back on the map every drone that is not running a Check is dimmed, and the Activity feed is narrowed to the same" },
  { press: inside(region("Autopilot"), role("switch", "Autopilot")), say: "Autopilot is a compact card beside the controls. Its switch takes the middle area for its configuration" },
  { look: stage("Autopilot lane"), say: "The lane, in the area the map was in. The controls and the merge queue stay" },
  { press: inside(region("Autopilot"), role("switch", "Autopilot")), say: "Switched off, the area is as it was left" },
  { press: inside(controls, button("Checks")), say: "Pressed again, the filter is put away and the column is whole" },
  { press: inside(controls, button("Needs me")), say: "Needs me narrows the column to what holds a call" },
  { press: inside(controls, button("Jobs only")), say: "Jobs only narrows it again" },
  { press: inside(controls, button("Timeline")), say: "The map and the timeline are one switch" },
  { look: inside(region("Fleet timeline"), text(/Shorten the reconnect wait/)), say: "The timeline keeps the same filters" },
  { press: inside(list, button("Shorten the reconnect wait")), say: "Pick the Job from the column" },
  { key: "Escape", on: stage(/^Job: Shorten the reconnect wait/), say: "Esc returns to the timeline, as it was left" },
  { look: inside(region("Fleet timeline"), text(/Shorten the reconnect wait/)), say: "Still the timeline" },
  { key: "c", on: list, say: "c goes to Incoming calls: the panel is lit, its heading and a hairline frame, and the first call is focused" },
  { key: "ArrowDown", on: incoming, say: "Arrows, or j and k, move inside the lit panel by where things are drawn" },
  { key: "Enter", on: incoming, say: "Enter opens it in the area" },
  { key: "Escape", on: stage(/^(Job|Session|Call|Pull request)/), say: "Esc puts it away, and focus is still in the panel" },
  { key: "1", on: incoming, say: "1 to 6 press the flight tiles in order: this is Jobs only" },
  { key: "Shift+?", on: incoming, say: "? opens the keys" },
  { look: role("dialog", "Keys"), say: "Every key the deck answers, in six groups" },
  { key: "Escape", on: role("dialog", "Keys"), say: "Esc closes it" },
]);

export { cockpitFlightDeck };
