// From opening Armada to talking to an agent: a folder and a request on the Cockpit's stage, a Session on that folder with no Manifest, and
// what Fleet offers once the turn ends, from what it saw. Over the `first-run` scenario, in two takes by `?take=`: the suggestions as the
// thread's last card (default), or as items in the Needs you band (`?take=band`). Dispatch then offers the folder.

import { button, deckRow, inside, region, role, text, walk, type Step } from "../walk";
import { firstRunTake } from "../first-run";

const stage = region("First Session");
const request = inside(stage, role("textbox", "Request"));
const thread = region("Thread");
const dispatch = role("region", "Dispatch", { exact: true });

const card = region("Next for my-app");
const needs = region("Needs you");
const suggestion = "Check every change with pnpm test";

/** The suggestions arrive, are read, and Set up is pressed: in the thread's card, or through the Needs you band. */
const offered: Step[] =
  firstRunTake() === "band"
    ? [
        { press: inside(needs, button("Needs you", { exact: true })), say: "The suggestions are in Needs you, from what Fleet saw" },
        { hover: inside(needs, button(`Open ${suggestion}`)), say: "The row's tooltip names the fact it stands on" },
        { press: inside(needs, button(`Open ${suggestion}`)), say: "The item opens with Set up and Not now" },
        { press: inside(needs, button("Set up", { exact: true })), say: "Set up starts a linked Session to write armada.yml from the facts" },
      ]
    : [
        { look: card, say: "Next for my-app is the thread's last card, under the Session's message" },
        { hover: inside(card, role("listitem", suggestion)), say: "The tooltip names the fact the suggestion stands on" },
        { press: inside(card, button(/^Set up/)), say: "Set up starts a linked Session to write armada.yml from the facts" },
      ];

const firstRun = walk("first-run", [
  { look: request, say: "Nothing is set up. The stage is a folder and a request, with focus in the request" },
  { key: "Meta+o", on: request, say: "⌘O opens the folder dialog" },
  { look: inside(stage, button("Folder ~/code/my-app")), say: "~/code/my-app is the folder. A folder that is not a repository is accepted, and Start makes it one" },
  { type: "Add a dark mode toggle to settings", into: request, say: "The request" },
  { press: inside(stage, button("Start", { exact: true })), say: "Start opens the Session with no Manifest" },
  { look: deckRow("Dark mode toggle"), say: "The Session is a row in Work" },
  { look: role("combobox", "Repository"), say: "my-app is the Cockpit's repository and is in the picker" },
  { look: inside(thread, text(/git init/)), say: "The first event is git init in the folder" },
  { look: region("Progress"), say: "The Field and Progress stand in the right column" },
  { later: thread, say: "Next the tests and the dev server run" },
  { look: inside(thread, text(/pnpm test passed/)), say: "pnpm test exited 0 and pnpm dev listens on :5173" },
  { later: thread, say: "Next the turn ends" },
  ...offered,
  { look: deckRow("Set up my-app"), say: "The linked Session stands under its parent in Work" },
  { look: inside(deckRow("Set up my-app"), role("img", "Linked Session")), say: "led by link" },
  { key: "n", on: region("Work", { exact: true }), say: "N opens Dispatch" },
  { look: inside(dispatch, role("radio", "Repository my-app")), say: "my-app is offered as a repository, with no Manifest" },
]);

export { firstRun as "first-run" };
