// The Cockpit as a flight deck, in the real window: the places as tabs in the title bar, every Session in a column on
// the left, the map until something is picked and then that thing, the merge queue as one row under it, and the bays,
// flight controls and Autopilot beside. The battlefield draws from the Fleet's own data: a drone for each
// Session, resting by its pad until a path is known. The preview switch is on for this scenario. #2301.

import { button, inside, region, role, tab, text, walk } from "../walk";

const places = role("navigation", "Places");
const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const stage = (name: string | RegExp) => region(name);

const cockpitFlightDeck = walk("cockpit-ship-flightdeck", [
  { look: inside(places, tab("Cockpit")), say: "The Cockpit tab is raised: filled and weighed. Studios, Retros and System are quiet beside it" },
  { look: role("listbox", "Map"), say: "Nothing picked, so the area is a battlefield seen on a tilted plane. Each repository is a territory laid out as its own directories, a raised slab for each that stands taller with recent activity, its base along the near edge with a pad for each bay and its main as the flag. A drone flies above the plane with its shadow below, over the directories it is changing, and a dotted line runs from a bay to the drone that holds it. Where two drones change the same directory the slab is the tallest, amber and hatched. A flag is the main of one repository, green while it is green and red when it is red. It rests zoomed out to take the whole field in, with every territory, base, flag and drone inside the pane. A drone that holds a call he has not opened is the brightest thing on it: a badge over it at one size however far he stands, a pillar of light up from its ground, amber when the call is owed and red when its Check is failing. A badge at the pane's border points to one that is out of view, and pressing it eases the camera there. Drag pans the field and the wheel or a pinch zooms it, and a double click on empty ground comes back to the whole field" },
  { press: inside(list, button("Fix the reconnect wait")), say: "Pick a Job from the column" },
  { look: inside(stage(/^Job: Fix the reconnect wait/), role("textbox", "Message")), say: "It opens in the area, with its thread and its box" },
  { press: inside(places, tab("Cockpit")), say: "The Cockpit tab in the title bar keeps the deck as it was left, with the Job still open in the area" },
  { look: inside(region("Activity"), button(/Fix the reconnect wait/)), say: "Activity is a dot for each event over the last hour, the newest at the right, in the column under the list" },
  { later: places, say: "A Job replies, and the drones that are working take up what their tools are doing: one reads its directory under a scan grid, one writes and the slab rises under a laser, one runs and circles its Check on a ring" },
  { look: inside(list, button(/Pin the store clock/)), say: "A call arrives as its row in Jobs, yellow and pulsing until he opens it" },
  { press: inside(list, button(/Pin the store clock/)), say: "Pressing the row opens the Job it is about. That is the visit, so the row stops pulsing" },
  { type: "Good. Merge it.\n", into: inside(stage(/^Job: Pin the store clock/), role("textbox", "Message")), say: "Answered in the Job's own box" },
  { press: inside(stage(/^Job: Pin the store clock/), button("Back")), say: "Back, to the map as it was" },
  { look: role("listbox", "Map"), say: "The Job he answered is locked on: the reticle is on it, the camera has eased to it and the others are dimmed" },
  { later: places, say: "A pull request merges, and the working drones change what they do, read to write to run: the camera backs off a little, a drone climbs from the base on an arc and settles over the directories it touches, the slab rises, and the pull request arcs to the flag, which flashes and sends a ripple across the plane and a column of light up" },
  { press: inside(region("Activity"), button(/Gate policy on every run/)), say: "A dot of Activity is an event. Pressing it focuses the drone that did it: a ring pings under it, the slabs it touched pulse, the rest dim, and a line names what moved and how long ago. Pressing the dot again, or Esc, lets go" },
  { look: region("Event"), say: "The line over the map says what the event was and when" },
  { press: button("Replay from here"), say: "Replay from here puts the field back as it was at that event, and plays it on to now" },
  { look: role("region", "Replay"), say: "Play and pause, 1x, 4x and 16x, and a track from that event to now with a tick for each event. Drones, slabs and pull requests follow the events up to the playhead; the column and the panels stay live. Space plays, the arrows step an event, Home starts over" },
  { press: inside(role("region", "Replay"), button("Live")), say: "Live is the field as it is now. Esc does the same" },
  { key: "ArrowDown", on: list, say: "Down moves through the column" },
  { key: "Enter", on: list, say: "Enter opens the row in the area" },
  { look: stage(/^Job: /), say: "A Job, in the same place" },
  { key: "Escape", on: stage(/^Job: /), say: "Esc is back to the map" },
  { press: inside(list, button("Migration notes")), say: "A Job from the column" },
  { look: inside(stage(/^Job: Migration notes/), role("textbox", "Message")), say: "Its thread and its box, as the Job's page has them" },
  { key: "Escape", on: stage(/^Job: Migration notes/), say: "Esc is back to the map, and the Bays are back in the column in place of the Job's own panels" },
  { look: inside(region("Bays"), button(/^slot-5, armada, Free/)), say: "Each bay is two lines. The first is the bay and its state as a glyph, in its own colour: a key lit blue where it is leased, with a small Job glyph for whoever holds it, an open door in grey where it is free, an amber ghost that slowly pulses where it is stranded, a locked door where it is closed. The last figure is how long it has been held, or how long since it was given back. The second line is the facts behind it. Leased: the holder, its branch. Free: warm or cold. Stranded: who held it last, and small glyphs for what is left on it and whether the holder was told. A tooltip names each" },
  { press: inside(region("Bays"), button(/^slot-6, armada, Stranded/)), say: "A stranded bay" },
  { look: stage("Bay: slot-6"), say: "The pane has the same facts in full: who held it last, whether they were told, what is uncommitted and unpushed, what stranded means, and every act the Worktree Slots panel offers for it, with the same confirms: Close slot, Pick up, Stash and Scrap. Scrap asks first, naming what would go" },
  { press: inside(region("Bays"), button(/^slot-2, armada, Held/)), say: "A leased bay" },
  { look: stage("Bay: slot-2"), say: "What holds it, and since when" },
  { look: inside(stage("Bay: slot-2"), region("History")), say: "Under the facts, the bay's history: a strip of the last day in a block for each holder, so quiet and busy stretches read at a glance, and below it the holders newest first on a rail. Each row is the holder, from when to when and for how long, with its branch and the branch checkouts between where Fleet serves them. A holder the ship still has opens on a press, and Esc comes back to the bay. j and k move through the rows" },
  { press: inside(region("Merge queue"), button(/^#1896/)), say: "The merge queue is one row under the map: a mark for each pull request, one light for main" },
  { look: stage("Pull request #1896"), say: "What the forge says about it" },
  { press: inside(controls, button("Checks")), say: "Checks, from the flight controls, is a plain filter like Needs me. It opens nothing and the area stays on the map" },
  { look: inside(list, button("Migration notes")), say: "The column, the feed, the map and the timeline narrow to the Jobs with a Check running now" },
  { look: stage("Pull request #1896"), say: "The pull request that was open stays. Back on the map every drone that is not running a Check is dimmed, and the Activity feed is narrowed to the same" },
  { press: inside(region("Autopilot"), role("switch", "Autopilot")), say: "Autopilot is a compact card beside the controls. Its switch takes the middle area for its configuration" },
  { look: stage("Autopilot lane"), say: "The lane, in the area the map was in. The controls and the merge queue stay" },
  { press: inside(region("Autopilot"), role("switch", "Autopilot")), say: "Switched off, the area is as it was left" },
  { press: inside(controls, button("Checks")), say: "Pressed again, the filter is put away and the column is whole" },
  { press: inside(controls, button("Needs me")), say: "Needs me narrows the column to what holds a call" },
  { press: inside(controls, button("Timeline")), say: "The map and the timeline are one switch" },
  { look: inside(region("Fleet timeline"), text(/Fix the reconnect wait/)), say: "The timeline keeps the same filters" },
  { press: inside(list, button("Fix the reconnect wait")), say: "Pick the Job from the column" },
  { key: "Escape", on: stage(/^Job: Fix the reconnect wait/), say: "Esc returns to the timeline, as it was left" },
  { look: inside(region("Fleet timeline"), text(/Fix the reconnect wait/)), say: "Still the timeline" },
  { key: "c", on: list, say: "c goes to the first row that needs him, and focuses it" },
  { key: "ArrowDown", on: list, say: "Arrows, or j and k, move along the list by where things are drawn" },
  { key: "Enter", on: list, say: "Enter opens it in the area" },
  { key: "Escape", on: stage(/^(Job|Call|Pull request)/), say: "Esc puts it away, and focus is still in the list" },
  { key: "1", on: list, say: "1 to 4 press the flight tiles in order: this is Checks" },
  { key: "Shift+?", on: list, say: "? opens the keys" },
  { look: role("dialog", "Keys"), say: "Every key the deck answers, in six groups" },
  { key: "Escape", on: role("dialog", "Keys"), say: "Esc closes it" },
]);

export { cockpitFlightDeck };
