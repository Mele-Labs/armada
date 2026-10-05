// The dispatch gate drawn as the run it will be (prototype, 3 Oct 2026), in
// three lanes — setup, the work, delivery — each gate beside its step: each node opens a card beside it with what it tunes for this Job,
// another workflow rebuilds the steps, Done when is one list on two nodes, and
// how it lands reshapes the end of the run.

import { button, card, inside, region, role, text, walk } from "../walk";

const CANVAS = region("What you are approving");

export const approvalAsACanvas = walk("proto/feature-at-approval", [
  { look: CANVAS, say: "Setup, the work and delivery, side by side" },
  { look: card("Checks"), say: "Each step's Checks hang beside it, on its row" },
  { look: role("combobox", "Workflow", { exact: true }), say: "The Work lane's head is the workflow: pick another here" },
  { type: "bug", into: role("combobox", "Workflow", { exact: true }), say: "Another workflow" },
  { look: card("Reproduction"), say: "and the Work lane is bug's steps" },
  { type: "feature", into: role("combobox", "Workflow", { exact: true }), say: "Back to feature" },
  { press: card("Base branch"), say: "Where the work starts" },
  { press: role("combobox", "Base branch"), say: "The repository's branches" },
  { type: "release/2026-10", into: role("combobox", "Base branch"), say: "It starts from the release branch" },
  { press: card("Plan the change"), say: "The Drone that plans" },
  { type: "opus", into: role("combobox", "Model on Plan the change"), say: "Plan runs on opus" },
  { look: inside(card("Plan the change"), text("opus")), say: "and the node says so" },
  { look: role("group", /^Groups, /), say: "Groups, until Plan has made them" },
  { press: card("Checks"), say: "Write tests' gate" },
  { type: "3", into: role("spinbutton", "Judges on Write tests"), say: "Three Judges" },
  { press: role("checkbox", "You on Write tests"), say: "and it stops for you" },
  { press: card("Done when"), say: "What counts as the work being done" },
  {
    type: "The guide catalogue still opens on guide 1, with guide 8 gone",
    into: inside(role("dialog", "Done when"), role("textbox", "Criterion 3", { exact: true })),
    say: "A criterion reworded before it leaves",
  },
  { press: card("Pull request"), say: "How the work leaves" },
  { type: "draft", into: inside(role("dialog", "Pull request"), role("combobox", "Delivery")), say: "A draft pull request" },
  { press: role("switch", "Auto-merge"), say: "that merges once approved" },
  { press: card("Land"), say: "Where it lands, and that it merges on its own" },
  { type: "local", into: inside(role("dialog", "Land"), role("combobox", "Delivery")), say: "Local only" },
  { look: inside(card("Land"), text("local only")), say: "No pull request, no merge, no push: the work stays on its branch" },
  { type: "ready", into: inside(role("dialog", "Land"), role("combobox", "Delivery")), say: "Back to a pull request" },
  { look: card("Pull request"), say: "The pull request is back before the review" },
  { press: card("Brief"), say: "What was asked" },
  {
    look: inside(role("dialog", "Brief"), role("textbox", "Criterion 3", { exact: true })),
    say: "The same list: the reworded line is here too",
  },
  {
    type: "Name the guide the rule refused in the error",
    into: role("textbox", "Note to the proposer"),
    say: "A note for the proposer",
  },
  { press: button("Send to proposer"), say: "Sent" },
  { look: text("Name the guide the rule refused in the error"), say: "and kept on the brief" },
  { press: button("Approve dispatch"), say: "Approve sends what moved" },
  { look: role("heading", "Add guide validation to the catalogue"), say: "Dispatched" },
]);
