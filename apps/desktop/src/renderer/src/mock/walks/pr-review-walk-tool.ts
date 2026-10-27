// The Walk review tool: Armada plays the pull request's change where it runs, in its own walk window, as a walk of a page or
// a run of requests. Where it runs is found in his order, one source at a time: the pull request, what Armada remembers of the
// area, then him, and the answer is remembered for the area. Walk it holds a pulled-down place up, a line at a time, and opens the
// window on it. Over the `pr-review` scenario, after `pr-review-tab`, which it extends nothing of.

import { button, dialog, inside, place, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const list = region("Pull requests");
const flight = region("Flight controls");
const reader = (number: number) => region(`Pull request #${number}`);
const tool = inside(flight, button("Walk"));
const results = region("Walk results");
const fact = (words: string) => inside(inside(results, role("list", "Where it runs")), role("listitem", words));
const setup = (words: RegExp) => inside(inside(results, role("list", "Setup")), role("listitem", words));
const walkIt = (kind: "Web walk" | "API walk") => inside(inside(results, region(kind)), role("article", "Walk it"));
const theWindow = (number: number) => dialog(new RegExp(`^Bridge's window on #${number} `));
const bar = (number: number) => inside(theWindow(number), role("group", /^Walk /));
const sheet = dialog("Where it runs");
const places = dialog("Where changes run");
const remembered = (area: string) => inside(places, role("article", `Remembered: ${area}`));
const lists = inside(list, button("List"));
const pick = (name: string | RegExp) => role("menuitem", name);

const fromThePullRequest: Step[] = [
  { press: lists, say: "The team's list" },
  { press: pick("Team"), say: "Team" },
  { press: inside(list, button(/#1907/)), say: "A pull request with a preview deployed" },
  { look: tool, say: "Walk is a tool of Armada's, in the Flight control with the rest. It has a colour of its own" },
  { hover: inside(tool, role("img", "Armada's tool")), say: "Defined where the other tools are" },
  { key: "5", on: reader(1907), say: "5 presses it. It plays the change, which costs, so it asks first" },
  { look: dialog("Walk"), say: "The same question every tool that spends asks" },
  { key: " ", on: dialog("Walk"), say: "Space turns the question off on this machine" },
  { key: "Enter", on: dialog("Walk"), say: "↵ runs it" },
  { look: fact("From the pull request"), say: "Where the change runs is found first from the pull request itself" },
  { look: inside(fact("From the pull request"), role("img", "Found")), say: "A deploy bot posted a preview link" },
  { look: inside(fact("From the pull request"), text("Preview ready: https://pr-1907.preview.armada.dev")), say: "The line it posted" },
  { hover: inside(fact("From the pull request"), role("img", /^From the pull request/)), say: "Each line names its source. Nothing further is tried once one finds it" },
  { look: region("Web walk"), say: "The walk is written from the description, How to Test and the themes" },
  { look: inside(region("Web walk"), role("list", "Steps")), say: "A preview of its steps: what is said, and the part of the page each acts on, found by role and name" },
  { look: inside(region("Web walk"), role("listitem", "Walk step 2")), say: "The same verbs and targets show_window steps have" },
  { key: "Enter", on: walkIt("Web walk"), say: "↵ on Walk it opens it in Armada's own walk window, the one a Job's show_window opens" },
  { look: theWindow(1907), say: "The same window: the page the change runs at, and the walk's controls" },
  { look: inside(theWindow(1907), text("https://pr-1907.preview.armada.dev")), say: "The preview the bot posted is the page" },
  { look: inside(bar(1907), button("Next")), say: "Next acts on the ringed target and goes on. Back, Autoplay and Replay are beside it" },
  { look: inside(theWindow(1907), button("Capture")), say: "Notes are taken in it as in any walk window. No Job holds this pull request, so they stay in Armada" },
  { press: button("Close window"), say: "Closed" },
  { look: inside(tool, role("img", /^Kept/)), say: "The walk is kept for the commit it was written on, as a tool's result is" },
];

const fromMemory: Step[] = [
  { press: inside(list, button(/#1899/)), say: "A pull request that touches Fleet" },
  { press: tool, say: "Walk. The question is off, so it runs" },
  { look: inside(fact("From the pull request"), role("img", "Nothing found")), say: "The pull request says nothing about where it runs" },
  { look: fact("From what Armada remembers of this area"), say: "So Armada looks at what it remembers of this repository and the area of code the pull request touches" },
  { look: inside(fact("From what Armada remembers of this area"), text(/Pull it down, run armada fleet --dev/)), say: "crates/fleet: pull it down, run it, and the API is on the port Fleet takes in development" },
  { look: region("API walk"), say: "This one is an API walk: requests, in order, against the API there" },
  { look: inside(region("API walk"), role("listitem", /^Request 1: GET \/plan-usage/)), say: "Each request is a method, a path and a body where it has one" },
  { look: inside(region("API walk"), role("listitem", /^Request 3:/)), say: "Read again after a minute has passed" },
  { key: "Enter", on: walkIt("API walk"), say: "↵ on Walk it. A change pulled down is set up first, a line at a time" },
  { look: setup(/Check out marta\/plan-windows-cache into slot 3/), say: "Checked out into a slot" },
  { look: setup(/Run armada fleet --dev/), say: "The command it remembered" },
  { look: inside(setup(/Wait for http:\/\/localhost:7420/), role("img", "Done")), say: "It waits for the address to answer before it walks" },
  { look: theWindow(1899), say: "Then the same walk window as a web walk. Each request is one step, and the page shows the request and its response" },
  { look: inside(theWindow(1899), text("http://localhost:7420")), say: "Against the API Armada pulled down and ran" },
  { look: inside(bar(1899), button("Next")), say: "Next sends the request. The step's words are what to look for in the response" },
  { press: button("Close window"), say: "Closed" },
];

const asked: Step[] = [
  { press: inside(list, button(/#1903/)), say: "A pull request with nothing posted on it, in an area nothing is remembered for" },
  { press: tool, say: "Walk" },
  { look: sheet, say: "The pull request says nothing, and nothing is remembered for apps/desktop, so it asks him. Keyboard first: focus is on the choice" },
  { press: inside(sheet, role("radio", "Pull it down and run")), say: "Deployed at, or pull it down and run" },
  { type: "pnpm --filter @armada/desktop dev", into: inside(sheet, role("textbox", "Command")), say: "What starts it" },
  { type: "http://localhost:5173", into: inside(sheet, role("textbox", "Address", { exact: true })), say: "The address Armada waits on" },
  { look: inside(sheet, role("textbox", "API base address")), say: "An API base address is there when the change has one" },
  { look: inside(sheet, role("checkbox", "Remember for")), say: "Remember is checked: the next pull request in this area does not ask" },
  { look: inside(sheet, role("textbox", "Area")), say: "The area is the directory most of the pull request's files are in, and it can be edited" },
  { key: "Enter", on: inside(sheet, role("textbox", "Address", { exact: true })), say: "↵ sends it from any box" },
  { look: inside(fact("From the pull request"), role("img", "Nothing found")), say: "Each source tried is a line: nothing in the pull request" },
  { look: inside(fact("From what Armada remembers of this area"), role("img", "Nothing found")), say: "Nothing remembered" },
  { look: inside(fact("From you"), text(/Pull it down, run pnpm --filter/)), say: "And his answer, like the others" },
  { look: region("Web walk"), say: "And the walk, a web walk of the bay's history" },
  { look: inside(region("Web walk"), role("listitem", "Walk step 2")), say: "Its steps, like the first" },
];

const theRememberedPlaces: Step[] = [
  { press: inside(reader(1903), button("Where changes run")), say: "Where changes run is in the reader's head, with its key, ⇧W" },
  { look: remembered("apps/desktop/"), say: "The place he just gave, under the area it was remembered for" },
  { look: remembered("crates/fleet/"), say: "And the one Armada already had" },
  { look: remembered("src/"), say: "Another repository's, deployed rather than pulled down" },
  { press: inside(remembered("src/"), button("Edit src/")), say: "Edit changes it where it stands" },
  { look: inside(remembered("src/"), role("textbox", "Address")), say: "J and K move between places, E edits and Delete forgets" },
  { hover: inside(remembered("crates/fleet/"), button("Forget crates/fleet/")), say: "Forget drops it, and the next pull request there asks again" },
  { key: "Escape", on: places, say: "Esc closes the list" },
];

const prReviewWalkTool = walk("pr-review", [
  { press: place("Reviews"), say: "Reviews" },
  ...fromThePullRequest,
  ...fromMemory,
  ...asked,
  ...theRememberedPlaces,
]);

export { prReviewWalkTool as "pr-review-walk-tool" };
