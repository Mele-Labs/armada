// The Walk review tool: Armada plays the pull request's change where it runs, as a walk in a Bridge window or as a run of
// requests. Where it runs is found in his order, one source at a time: the pull request, what Armada remembers of the
// area, then him. Over the `pr-review` scenario, after `pr-review-tab`, which it extends nothing of.

import { button, dialog, inside, place, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const list = region("Pull requests");
const flight = region("Flight controls");
const reader = (number: number) => region(`Pull request #${number}`);
const tool = inside(flight, button("Walk"));
const results = region("Walk results");
const fact = (words: string) => inside(inside(results, role("list", "Where it runs")), role("listitem", words));
const setup = (words: RegExp) => inside(inside(results, role("list", "Setup")), role("listitem", words));
const request = (n: number) => inside(inside(results, region("API walk")), role("article", new RegExp(`^Request ${n}:`)));
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
  { look: region("Web walk"), say: "The walk is written from the description, How to Test and the themes, and played in the reader" },
  { look: inside(region("Web walk"), role("list", "Steps")), say: "Each step is what is said and what it acts on, found by role and name" },
  { look: inside(region("Web walk"), role("listitem", "Walk step 2")), say: "The same verbs and targets a walk window plays" },
  { key: "Enter", on: inside(region("Web walk"), role("article", "Walk window")), say: "↵ on the walk opens it in a Bridge walk window. Here it is drawn in place" },
  { look: inside(region("Web walk"), role("group", "Walk window frame")), say: "The first step, with Next" },
  { press: inside(region("Web walk"), button("Next")), say: "Next goes to the second step" },
  { look: inside(region("Web walk"), role("note", /Say: Restart Fleet/)), say: "Press Restart Fleet, as the description's own testing says" },
  { press: inside(region("Web walk"), button("Close the walk window")), say: "Closed" },
  { look: inside(tool, role("img", /^Kept/)), say: "The walk is kept for the commit it was written on, as a tool's result is" },
];

const fromMemory: Step[] = [
  { press: inside(list, button(/#1899/)), say: "A pull request that touches Fleet" },
  { press: tool, say: "Walk. The question is off, so it runs" },
  { look: inside(fact("From the pull request"), role("img", "Nothing found")), say: "The pull request says nothing about where it runs" },
  { look: fact("From what Armada remembers of this area"), say: "So Armada looks at what it remembers of this repository and the area of code the pull request touches" },
  { look: inside(fact("From what Armada remembers of this area"), text(/Pull it down, run armada fleet --dev/)), say: "crates/fleet: pull it down, run it, and the API is on the port Fleet takes in development" },
  { look: setup(/Check out marta\/plan-windows-cache into slot 3/), say: "A change pulled down is set up first, a line at a time" },
  { look: setup(/Run armada fleet --dev/), say: "The command it remembered" },
  { look: inside(setup(/Wait for http:\/\/localhost:7420/), role("img", "Done")), say: "It waits for the address to answer before it walks" },
  { look: region("API walk"), say: "This one is an API walk: requests, in order, against the API there" },
  { look: request(1), say: "Each request is a card with its method, path and body" },
  { key: "Enter", on: request(1), say: "↵ runs the request under focus" },
  { look: inside(request(1), role("group", "Response")), say: "The response: status and a trimmed body" },
  { look: inside(request(1), role("note", /Look for: read_at is there/)), say: "And what to look for in it" },
  { key: "j", on: request(1), say: "J moves to the next request. K goes back" },
  { key: "Enter", on: request(2), say: "↵ runs the second" },
  { look: inside(request(2), role("note", /Look for: The same read_at/)), say: "The same read_at and a fast answer: Fleet served what it held" },
  { look: inside(inside(request(2), role("group", "Response")), text("200")), say: "The status, as a badge" },
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
  { look: setup(/Check out devon\/bay-history-query into slot 2/), say: "Then the setup" },
  { look: region("Web walk"), say: "And the walk, a web walk of the bay's history" },
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
