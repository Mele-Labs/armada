// The Cockpit as a flight deck: the places as tabs in the title bar, every Job and Session in a column on the left,
// the map until something is picked and then that thing, the merge queue as dots, the bays with the incoming calls under them, and the
// flight controls under the merge queue. A throwaway prototype on proto/cockpit-flight-deck.

import { button, inside, region, role, tab, text, walk } from "../walk";

const places = role("navigation", "Places");
const list = region("Jobs and Sessions");
const controls = region("Flight controls");
const incoming = region("Incoming calls");
const stage = (name: string | RegExp) => region(name);

const cockpitFlightDeck = walk("cockpit-ship-flightdeck", [
  { look: inside(places, tab("Cockpit")), say: "The Cockpit tab is raised: filled and weighed. Jobs, Sessions and System are quiet beside it" },
  { look: role("listbox", "Map"), say: "Nothing picked, so the area is the map. Gray is calm; amber is a star that needs him; red is a Check failing now" },
  { press: inside(list, button("Shorten the reconnect wait")), say: "Pick a Job from the column" },
  { look: inside(stage(/^Job: Shorten the reconnect wait/), role("group", "Canvas")), say: "A small Job canvas: the step it is on is live, the rest are quiet" },
  { look: inside(stage(/^Job: Shorten the reconnect wait/), role("group", "Running now")), say: "And what it is running now, beside it" },
  { look: inside(region("Activity"), text(/Shorten the reconnect wait/)), say: "Activity is the Cockpit's feed, newest first, in the column under the list" },
  { later: places, say: "A Session replies" },
  { look: inside(incoming, button(/Pin the store clock/)), say: "A call arrives in the panel under the Bays and rings until he opens it" },
  { press: inside(incoming, button(/Pin the store clock/)), say: "Open it" },
  { type: "Good. Merge it.\n", into: inside(stage(/^Call: Pin the store clock/), role("textbox", "Message")), say: "The real call card, answered here" },
  { look: role("listbox", "Map"), say: "Answered: the call is gone from Incoming calls, and the area is back on the map" },
  { key: "ArrowDown", on: list, say: "Down moves through the column" },
  { key: "Enter", on: list, say: "Enter opens the row in the area" },
  { look: stage(/^(Job|Session): /), say: "A Job or a Session, in the same place" },
  { key: "Escape", on: stage(/^(Job|Session): /), say: "Esc is back to the map" },
  { press: inside(list, button("Migration notes")), say: "A Session from the column" },
  { look: inside(stage(/^Session: Migration notes/), role("textbox", "Message")), say: "Its thread and its box, as the Session's page has them" },
  { press: inside(region("Bays"), button(/^slot-2/)), say: "A bay" },
  { look: stage("Bay: slot-2"), say: "What holds it, and since when" },
  { press: inside(region("Merge queue"), button(/^#1896/)), say: "The merge queue is one row: a dot for each pull request, one light for main" },
  { look: stage("Pull request #1896"), say: "What the forge says about it" },
  { press: inside(controls, button("Checks")), say: "Checks, from the flight controls" },
  { look: inside(stage("Checks"), region("Running")), say: "What is running now is in one band, what has finished under it" },
  { press: inside(controls, button("Needs me")), say: "Needs me narrows the column to what holds a call" },
  { press: inside(controls, button("Jobs only")), say: "Jobs only narrows it again" },
  { press: inside(controls, button("Timeline")), say: "The map and the timeline are one switch" },
  { look: inside(region("Fleet timeline"), text(/Shorten the reconnect wait/)), say: "The timeline keeps the same filters" },
  { press: inside(list, button("Shorten the reconnect wait")), say: "Pick the Job from the column" },
  { key: "Escape", on: stage(/^Job: Shorten the reconnect wait/), say: "Esc returns to the timeline, as it was left" },
  { look: inside(region("Fleet timeline"), text(/Shorten the reconnect wait/)), say: "Still the timeline" },
]);

export { cockpitFlightDeck };
