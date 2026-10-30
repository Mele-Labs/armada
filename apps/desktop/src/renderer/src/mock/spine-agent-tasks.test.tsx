// The agent's task list on the Job's spine, and no "Working on" band over the box: a Job whose turn is running with a list drawn draws the list in the story and nothing
// over the message box. Through `App` on `spine-agent-tasks`. The walk `spine-agent-tasks` plays it.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { s340SpineAgentTasks } from "./scenarios/spine-agent-tasks";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("a Job whose turn is running with a task list draws the list in its story and no Working on band over the box", async () => {
  mount(s340SpineAgentTasks);
  await onScreen();
  // Its turn is running, and the agent has a list under way. The spine is the stage's own column, outside the Job's thread.
  await toSession(/Name the store migrations/);
  await expect.element(page.getByRole("list", { name: "Tasks" })).toBeVisible();
  await expect.element(page.getByRole("list", { name: "Tasks" }).getByText("Rename the files to their order")).toBeVisible();
  expect(page.getByRole("region", { name: "Working on" }).elements()).toHaveLength(0);
  expect(document.querySelectorAll(".armada-session-working")).toHaveLength(0);
});
