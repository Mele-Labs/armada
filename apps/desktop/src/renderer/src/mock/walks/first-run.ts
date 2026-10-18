// From opening Armada to a first Session on a folder, and Armada remembering how the folder runs so the next agent does not guess. The Session tries
// npm test, which fails, then pnpm test. Over the `first-run` scenario, in three takes by `?take=`: the Session asks at the end of its turn (`ask`,
// the default), the next Session's start offers it (`start`), or the repository holds it, marked in the picker (`repo`). Dispatch then offers the folder.

import { button, inside, region, role, text, walk, type Step } from "../walk";
import { firstRunTake } from "../first-run";

const stage = region("First Session");
const request = inside(stage, role("textbox", "Request"));
const thread = region("Thread");
const dispatch = role("region", "Dispatch", { exact: true });
const runs = inside(dispatch, role("group", "How my-app runs"));
const picker = role("combobox", "Repository");

/** The shared opening: a folder and a request, Start, and the commands the Session guesses. */
const opening: Step[] = [
  { key: "Meta+o", on: request, say: "⌘O picks ~/code/my-app, a folder with no Manifest" },
  { type: "Add a dark mode toggle to settings", into: request, say: "The request" },
  { press: inside(stage, button("Start", { exact: true })), say: "Start opens the Session" },
  { later: thread, say: "The Session guesses the test command" },
  { look: inside(thread, text(/^npm test$/)), say: "npm test exited 1, then pnpm test exited 0" },
];

const last: Step[] = [
  { key: "n", on: region("Work", { exact: true }), say: "N opens Dispatch" },
  { look: inside(dispatch, role("radio", "Repository my-app")), say: "Dispatch offers my-app, with how it runs" },
];

const takes: Record<ReturnType<typeof firstRunTake>, Step[]> = {
  ask: [
    { later: thread, say: "The turn ends with a question in Needs you" },
    { press: inside(region("Needs you"), button(/^Open npm test failed/)), say: "The row opens the question" },
    { press: role("radio", /^Remember/), say: "Remember is picked" },
    { press: inside(region("Needs you"), button("Answer", { exact: true })), say: "Answer sends it" },
    { look: inside(thread, region("How my-app runs changed")), say: "The next line states what was kept" },
    ...last,
  ],
  start: [
    { later: thread, say: "The turn ends with nothing extra" },
    { key: "n", on: region("Work", { exact: true }), say: "N opens Dispatch for the next Session" },
    { look: runs, say: "Under the request, how my-app runs, from the last Session" },
    { press: inside(dispatch, button("Remember", { exact: true })), say: "Remember keeps them" },
    { look: inside(runs, role("img", "Tests: Remembered")), say: "Each row is marked remembered" },
    { look: inside(dispatch, role("radio", "Repository my-app")), say: "Dispatch offers my-app, with how it runs" },
  ],
  repo: [
    { later: thread, say: "The turn ends with nothing extra" },
    { press: picker, say: "The picker opens" },
    { hover: role("button", "How my-app runs"), say: "my-app carries a mark" },
    { press: role("button", "How my-app runs"), say: "The mark opens how my-app runs" },
    { look: role("group", "How my-app runs"), say: "Tests and Dev server were seen. Lint was not" },
    { press: inside(role("group", "How my-app runs"), button(/^Remember/)), say: "Remember keeps what was seen" },
    { key: "Escape", on: picker, say: "Esc closes the picker" },
    ...last,
  ],
};

const firstRun = walk("first-run", [...opening, ...takes[firstRunTake()]]);

export { firstRun as "first-run" };
