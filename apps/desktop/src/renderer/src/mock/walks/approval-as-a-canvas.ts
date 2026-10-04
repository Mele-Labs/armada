// The dispatch gate drawn as the run it will be (prototype, 3 Oct 2026): each
// node opens a card under it with what it tunes for this Job, another workflow
// rebuilds the steps, and how it lands reshapes the end of the run.

import { button, card, inside, region, role, text, walk } from "../walk";

const CANVAS = region("What you are approving");

export const approvalAsACanvas = walk("proto/feature-at-approval", [
  { look: CANVAS, say: "Brief to Land, left to right" },
  { press: card("Start feature"), say: "The workflow is a node of its own" },
  { type: "bug", into: role("combobox", "Workflow", { exact: true }), say: "Another workflow" },
  { look: card("Reproduction"), say: "and the steps are bug's" },
  { type: "feature", into: role("combobox", "Workflow", { exact: true }), say: "Back to feature" },
  { press: card("Base branch"), say: "Where the work starts" },
  { press: role("combobox", "Base branch"), say: "The repository's branches" },
  { type: "release/2026-10", into: role("combobox", "Base branch"), say: "It starts from the release branch" },
  { press: card("Plan the change"), say: "The Drone that plans" },
  { type: "opus", into: role("combobox", "Model on Plan the change"), say: "Plan runs on opus" },
  { look: inside(card("Plan the change"), text("opus")), say: "and the node says so" },
  { press: card("Checks"), say: "Write tests' gate" },
  { type: "3", into: role("spinbutton", "Judges on Write tests"), say: "Three Judges" },
  { press: role("checkbox", "You on Write tests"), say: "and it stops for you" },
  { press: card("Pull request"), say: "How the work leaves" },
  { type: "draft", into: inside(role("dialog", "Pull request"), role("combobox", "Delivery")), say: "A draft pull request" },
  { press: role("switch", "Auto-merge"), say: "that merges once approved" },
  { press: card("Land"), say: "Where it lands, and that it merges on its own" },
  { type: "local", into: inside(role("dialog", "Land"), role("combobox", "Delivery")), say: "Local only" },
  { look: inside(card("Land"), text("Local merge")), say: "No pull request: Land is a local merge" },
  { type: "ready", into: inside(role("dialog", "Land"), role("combobox", "Delivery")), say: "Back to a pull request" },
  { look: card("Pull request"), say: "The pull request is back before the review" },
  { press: card("Brief"), say: "What was asked" },
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
