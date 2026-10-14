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
  { look: role("listbox", "Map"), say: "Nothing picked, so the area is a battlefield. Each repository is a territory: its base with a pad for each bay, and its main as the flag. A Job is the large drone, a Session the small one. Rotors turn on what is working, a ring goes out from what holds a call, amber is owed to him, red is a Check failing" },
  { press: inside(list, button("Shorten the reconnect wait")), say: "Pick a Job from the column" },
  { look: inside(stage(/^Job: Shorten the reconnect wait/), role("group", "Canvas")), say: "A small Job canvas: the step it is on is live, the rest are quiet" },
  { look: inside(stage(/^Job: Shorten the reconnect wait/), role("group", "Running now")), say: "And what it is running now, beside it" },
  { look: inside(region("Activity"), text(/Shorten the reconnect wait/)), say: "Activity is the Cockpit's feed, newest first, in the column under the list" },
  { later: places, say: "A Session replies" },
  { look: inside(incoming, button(/Pin the store clock/)), say: "A call arrives in the panel under the Bays and rings until he opens it" },
  { press: inside(incoming, button(/Pin the store clock/)), say: "Pressing a call opens the Session it is about, as a row of the column does. That is the visit, so the call leaves Incoming calls" },
  { type: "Good. Merge it.\n", into: inside(stage(/^Session: Pin the store clock/), role("textbox", "Message")), say: "Answered in the Session's own box" },
  { press: inside(stage(/^Session: Pin the store clock/), button("Back to the map")), say: "Back to the map" },
  { look: role("listbox", "Map"), say: "The Session he answered is locked on: the reticle is on it, the camera has eased to it and the others are dimmed" },
  { later: places, say: "A Job is dispatched and a pull request merges: a drone lifts from the base, and the pull request flies to the flag, which flashes" },
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
  { press: inside(controls, button("Checks")), say: "Checks, from the flight controls" },
  { look: inside(stage("Checks"), region("Running")), say: "What is running now is in one band, what has finished under it" },
  { press: inside(region("Autopilot"), role("switch", "Autopilot")), say: "Autopilot is a compact card beside the controls. Its switch takes the middle area for its configuration" },
  { look: stage("Autopilot lane"), say: "The lane, in the area the Checks were in. The controls and the merge queue stay" },
  { press: inside(region("Autopilot"), role("switch", "Autopilot")), say: "Switched off, the area is as it was left" },
  { look: stage("Checks"), say: "Back on Checks" },
  { press: inside(controls, button("Needs me")), say: "Needs me narrows the column to what holds a call" },
  { press: inside(controls, button("Jobs only")), say: "Jobs only narrows it again" },
  { press: inside(controls, button("Timeline")), say: "The map and the timeline are one switch" },
  { look: inside(region("Fleet timeline"), text(/Shorten the reconnect wait/)), say: "The timeline keeps the same filters" },
  { press: inside(list, button("Shorten the reconnect wait")), say: "Pick the Job from the column" },
  { key: "Escape", on: stage(/^Job: Shorten the reconnect wait/), say: "Esc returns to the timeline, as it was left" },
  { look: inside(region("Fleet timeline"), text(/Shorten the reconnect wait/)), say: "Still the timeline" },
]);

export { cockpitFlightDeck };
