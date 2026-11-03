// A terminal the Job's agent opened while the person was away has no tab: the + lists it under Terminal, and picking it opens it in a tab.
// Through `App` on `session-shell-away`. The whole story is the walk `job-open-terminal`.

import { beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { forgetPanes } from "../job-panes/store";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";
import { s323SessionShellAway } from "./scenarios/session-shell-away";
import { shellCalls } from "./scenarios/session-shell";
import { timePasses } from "./time-passes";

unmountAfterEach();
beforeEach(() => forgetPanes());

const threadPane = () =>
  page.getByRole("region", {
    name: "Pane: Thread",
    exact: true,
  });

test("the + lists a terminal the agent opened unseen, and opens it in a tab", async () => {
  mount(s323SessionShellAway);
  await onScreen();
  await toSession(/Deploy the store/);
  await expect.element(threadPane()).toBeVisible();
  timePasses();
  // The agent's terminal is open in Fleet, and the Job drew no tab for it.
  expect(
    page
      .getByRole("tab", {
        name: "Terminal 1",
      })
      .elements(),
  ).toHaveLength(0);

  await threadPane()
    .getByRole("button", {
      name: "Add tab",
    })
    .click();
  await expect
    .element(
      page.getByRole("menuitem", {
        name: "Open Terminal 1",
        exact: true,
      }),
    )
    .toBeVisible();
  await page
    .getByRole("menuitem", {
      name: "Open Terminal 1",
      exact: true,
    })
    .click();

  await expect
    .element(
      page.getByRole("tab", {
        name: "Terminal 1",
        exact: true,
      }),
    )
    .toBeVisible();
  // It is the shell the agent has, not a second one.
  expect(shellCalls()!.started).toHaveLength(1);
});
