// The Cockpit as a flight deck: the places as tabs in the title bar, every Job and Session in a column on the left,
// the map, a battlefield of drones, until something is picked and then that thing, the merge queue as one row of dots or chips under it, the bays with the
// incoming calls under them, and the flight control tiles and Autopilot under it. A throwaway prototype on proto/cockpit-flight-deck.

import { button, inside, region, role, tab, text, walk } from "../walk";

const places = role("navigation", "Places");
const list = region("Jobs and Sessions");
const controls = region("Flight controls");
const incoming = region("Incoming calls");
const stage = (name: string | RegExp) => region(name);

const cockpitFlightDeck = walk("cockpit-ship-flightdeck", [
  { look: inside(places, tab("Cockpit")), say: "The Cockpit tab is raised: filled and weighed. Jobs, Sessions and System are quiet beside it" },
  { look: role("listbox", "Map"), say: "Nothing picked, so the area is a battlefield seen on a tilted plane. Each repository is a territory laid out as its own directories, a raised slab for each that stands taller with recent activity, its base along the near edge with a pad for each bay and its main as the flag. A drone flies above the plane with its shadow below, over the directories it is changing, a Job large and a Session small, and a dotted line runs from a bay to the drone that holds it. Where two drones change the same directory the slab is the tallest, amber and hatched. A flag is the main of one repository, green while it is green and red when it is red. Drag pans the field and the wheel or a pinch zooms it, and a double click on empty ground comes back to the whole field" },
  { press: inside(list, button("Shorten the reconnect wait")), say: "Pick a Job from the column" },
  { look: inside(stage(/^Job: Shorten the reconnect wait/), role("group", "Canvas")), say: "The Job page's own canvas, read only, opened on the step at work with the cards readable. That step is live, the rest are quiet" },
  { look: inside(stage(/^Job: Shorten the reconnect wait/), role("group", "Running now")), say: "And what it is running now, beside it" },
  { key: "0", on: list, say: "0 is the canvas's own Fit, pressed for him: the whole run in the frame" },
  { press: inside(stage(/^Job: Shorten the reconnect wait/), button("Open the Job page")), say: "The canvas is the one the Job page draws, read only. Open the Job page opens the page itself, as a layer over the deck" },
  { press: button("Back to the Cockpit"), say: "Back returns to the deck with the Job still open in the area" },
  { look: inside(region("Activity"), text(/Shorten the reconnect wait/)), say: "Activity is the Cockpit's feed, newest first, in the column under the list" },
  { later: places, say: "A Session replies, and the drones that are working take up what their tools are doing: one reads its directory under a scan grid, one writes and the slab rises under a laser, one runs and circles its Check on a ring" },
  { look: inside(incoming, button(/Pin the store clock/)), say: "A call arrives in the panel under the Bays and rings until he opens it" },
  { press: inside(incoming, button(/Pin the store clock/)), say: "Pressing a call opens the Session it is about, as a row of the column does. That is the visit, so the call leaves Incoming calls" },
  { type: "Good. Merge it.\n", into: inside(stage(/^Session: Pin the store clock/), role("textbox", "Message")), say: "Answered in the Session's own box" },
  { press: inside(stage(/^Session: Pin the store clock/), button("Back to the map")), say: "Back to the map" },
  { look: role("listbox", "Map"), say: "The Session he answered is locked on: the reticle is on it, the camera has eased to it and the others are dimmed" },
  { later: places, say: "A Job is dispatched and a pull request merges, and the working drones change what they do, read to write to run: the camera backs off a little, a drone climbs from the base on an arc and settles over the directories it touches, the slab rises, and the pull request arcs to the flag, which flashes and sends a ripple across the plane and a column of light up" },
  { press: inside(region("Activity"), button("Job: Read the pairing code from the runtime file")), say: "A row of Activity is an event. Pressing it focuses the drone that did it: a ring pings under it, the slabs it touched pulse, the rest dim, and a line names what moved and how long ago. Pressing the row again, or Esc, lets go" },
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
  { press: inside(region("Bays"), button(/^slot-2/)), say: "A bay" },
  { look: stage("Bay: slot-2"), say: "What holds it, and since when" },
  { press: inside(region("Merge queue"), button(/^#1896/)), say: "The merge queue is one row under the map: a mark for each pull request, one light for main" },
  { look: stage("Pull request #1896"), say: "What the forge says about it" },
  { press: inside(controls, button("Checks")), say: "Checks, from the flight controls. It is a filter like Jobs, Sessions and Calls" },
  { look: inside(list, button("Add the pause marker column")), say: "The column, the feed, the map and the timeline narrow to the Jobs and Sessions with a Check running now" },
  { look: inside(stage("Checks"), region("Running")), say: "What is running now is in one band, what has finished under it" },
  { press: inside(region("Autopilot"), role("switch", "Autopilot")), say: "Autopilot is a compact card beside the controls. Its switch takes the middle area for its configuration" },
  { look: stage("Autopilot lane"), say: "The lane, in the area the Checks were in. The controls and the merge queue stay" },
  { press: inside(region("Autopilot"), role("switch", "Autopilot")), say: "Switched off, the area is as it was left" },
  { look: stage("Checks"), say: "Back on Checks" },
  { press: inside(controls, button("Checks")), say: "Pressed again, Checks is put away and the column is whole" },
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
