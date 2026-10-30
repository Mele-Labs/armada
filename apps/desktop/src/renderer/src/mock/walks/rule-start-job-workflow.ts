// A rule's Start a Job act names the workflow the Job it starts follows (armada-95v). Over the `workspaces-pieces` scenario.

import { inside, region, role, button, toSystemPage, walk } from "../walk";

const LANES = region("Workflow", { exact: true });
const PANEL = role("dialog");
const combo = (name: string) => inside(PANEL, role("combobox", name, { exact: true }));

const ruleStartJobWorkflow = walk("workspaces-pieces", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { press: inside(LANES, button("Start the ticket", { exact: true })), say: "A rule" },
  { type: "start_job", into: combo("Add an act"), say: "Add Start a Job" },
  { type: "feature", into: combo("Start a Job workflow"), say: "The workflow the new Job follows" },
  { type: "Look at the ticket", into: inside(PANEL, role("textbox", "Prompt", { exact: true })), say: "Its prompt" },
  { key: "Enter", on: inside(PANEL, role("textbox", "Prompt", { exact: true })), say: "Enter keeps the act" },
  { look: combo("Start a Job 2 workflow"), say: "Kept with the rule" },
], { width: 2000, height: 1200 });

export { ruleStartJobWorkflow as "rule-start-job-workflow" };
