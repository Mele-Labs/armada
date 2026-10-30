// The Workflows page over the lifecycles and rules Fleet keeps (30 Oct 2026): the lanes are read from the pieces a Fleet serves for store-web and its Workspace, a change is saved as the pieces
// it changes, and the person's own piece can be committed to the repository. A mock Fleet keeps the pieces for the window; the Workspace the long walk `workspaces-lanes` draws is the page's own mock.

import { button, inside, region, role, text, toSystemPage, walk } from "../walk";

const group = (name: string) => role("group", name, { exact: true });
const LANES = region("Workflow", { exact: true });
const PANEL = role("dialog");
const RUNS_IN = group("Runs in");
const tab = (name: string) => inside(RUNS_IN, role("tab", name, { exact: true }));
const kept = inside(PANEL, group("Where it is kept"));

const pieces = walk("workspaces-pieces", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { look: LANES, say: "The feature workflow over a lane each for Ticket, Pull request and Deploy, read from the lifecycles Fleet keeps" },
  { look: RUNS_IN, say: "Default, all of my-store, and a tab for each of its repositories" },
  { press: tab("store-web"), say: "store-web's tab: its own Ticket names In review as In QA" },
  { look: inside(LANES, button("Ticket: In QA", { exact: true })), say: "In QA is store-web's own name for the state, from the piece the repository committed" },
  { press: inside(LANES, button("Start the ticket", { exact: true })), say: "A rule of yours: when the pull request enters Draft, move the ticket to In progress" },
  { look: inside(PANEL, role("radio", "Acts")), say: "Autonomy is Acts" },
  { press: inside(PANEL, role("radio", "Asks")), say: "Choosing Asks saves at once, as a piece in your Kit" },
  { look: inside(PANEL, role("radio", "Asks")), say: "Read back from Fleet: it asks" },
  { press: inside(kept, button("Commit to repository", { exact: true })), say: "Commit to repository starts a Job of its own, with a pull request" },
  { look: inside(PANEL, text("Job started")), say: "The Job is started" },
  { press: inside(LANES, button("Close the ticket", { exact: true })), say: "A rule store-web committed is the repository's: nothing to commit" },
  { press: inside(LANES, button("Mark ready", { exact: true })), say: "A rule for my-store alone" },
]);

export { pieces as "workspaces-pieces" };
