// The Projects tab on the mock Fleet, through `App`: what a press writes through Fleet, what Fleet publishes as
// `projects.changed` reaching the cards, the rows Dispatch starts a Job with, the palette filing a task from another page,
// and the done ask answered from the Job's Needs you. The look is the walk's (`projects`); this holds what it decided.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toPlace, unmountAfterEach } from "./testing";

unmountAfterEach();

const SCENARIO = "projects";

const tab = () => page.getByRole("navigation", { name: "Places" }).getByRole("tab", { name: "Projects" });
const list = () => page.getByRole("region", { name: "Projects", exact: true });
const pocket = () => page.getByRole("region", { name: "Project Pocket launch" });
const field = () => page.getByRole("textbox", { name: "Add a task" });

/** Projects, on All, with Pocket launch open. */
async function open() {
  const app = mount(SCENARIO);
  await onScreen();
  await userEvent.click(tab());
  await userEvent.click(list().getByRole("button", { name: "Pocket launch" }));
  await expect.element(pocket()).toBeVisible();
  return app;
}

test("a task typed in the field is written through Fleet and is the top card once Fleet answers", async () => {
  const app = await open();
  const write = vi.spyOn(app.api, "addProjectTask");

  await userEvent.click(field());
  await userEvent.keyboard("Draft the store text{Enter}");

  await expect.poll(() => write.mock.calls.length).toBe(1);
  expect(write.mock.calls[0]!.slice(0, 2)).toEqual(["armada", "pocket-launch"]);
  expect(write.mock.calls[0]![2]).toMatchObject({ title: "Draft the store text" });
  await expect.element(pocket().getByRole("group", { name: "Draft the store text" })).toBeVisible();
});

test("a task added elsewhere shows on the card list when Fleet publishes projects.changed, with no press here", async () => {
  const app = await open();
  expect(pocket().getByRole("group", { name: "Written by another Bridge" }).query()).toBeNull();
  const reread = vi.spyOn(app.api, "listProjects");

  // The same call another Bridge, or a Job through the MCP, would make: Fleet answers it and publishes the projects whole.
  await app.api.addProjectTask("armada", "pocket-launch", { title: "Written by another Bridge" });

  await expect.element(pocket().getByRole("group", { name: "Written by another Bridge" })).toBeVisible();
  // It arrived on the event: the tab did not read Fleet again to find it.
  expect(reread).not.toHaveBeenCalled();
});

test("a tick on a card is written through Fleet", async () => {
  const app = await open();
  const tick = vi.spyOn(app.api, "tickProjectTask");

  await userEvent.click(pocket().getByRole("checkbox", { name: "Done?: Write the install guide" }));

  await expect.poll(() => tick.mock.calls.length).toBe(1);
  expect(tick.mock.calls[0]!.slice(0, 3)).toEqual(["armada", "pocket-launch", "task-guide"]);
});

test("Dispatch starts the Job with the rows that were selected, and Fleet then shows it holding the task", async () => {
  await open();
  const card = () => pocket().getByRole("group", { name: "Test on a small phone" });
  expect(card().getByRole("button", { name: /^Open / }).query()).toBeNull();

  await userEvent.click(page.getByRole("checkbox", { name: "Select Test on a small phone" }));
  await userEvent.click(pocket().getByRole("button", { name: "Dispatch", exact: true }));
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await expect.poll(() => (request.element() as HTMLTextAreaElement).value).toContain("Test on a small phone");
  await userEvent.click(page.getByRole("region", { name: "Dispatch", exact: true }).getByRole("button", { name: "Start", exact: true }));

  // The rows rode `project_rows`: Fleet holds the task for the Job it started, so the card carries the Job's chip.
  await userEvent.click(tab());
  await expect.element(card().getByRole("button", { name: /^Open / })).toBeVisible();
});

test("a task filed from the palette on another page is written through Fleet and the page stays", async () => {
  const app = mount(SCENARIO);
  await onScreen();
  await toPlace("Logbook");
  const write = vi.spyOn(app.api, "addProjectTask");

  await userEvent.keyboard("{Meta>}k{/Meta}");
  await userEvent.keyboard("Check the pairing copy");
  await userEvent.keyboard("{Tab}{Tab}");
  await expect.element(page.getByRole("option", { name: /^Add a task / })).toBeVisible();
  await userEvent.keyboard("{Enter}");
  await userEvent.click(page.getByRole("option", { name: "Pocket launch" }));

  await expect.poll(() => write.mock.calls.length).toBe(1);
  expect(write.mock.calls[0]!.slice(0, 2)).toEqual(["armada", "pocket-launch"]);
  expect(write.mock.calls[0]![2]).toMatchObject({ title: "Check the pairing copy" });
  await expect.element(page.getByRole("tab", { name: "Logbook" })).toHaveAttribute("aria-selected", "true");
});

test("the done ask on the Job is answered in Needs you and the task card follows to done", async () => {
  await open();
  await expect.element(page.getByRole("checkbox", { name: "Done?: Sign the build" })).toBeVisible();

  await userEvent.click(pocket().getByRole("button", { name: "Open Sign the release build" }));
  const needs = page.getByRole("region", { name: "Needs you" });
  await userEvent.click(needs.getByRole("button", { name: /^Needs you/ }));
  await expect.element(needs.getByRole("paragraph").getByText("Mark Sign the build done?")).toBeVisible();
  await userEvent.click(needs.getByRole("radio", { name: "Mark done", exact: true }));
  await userEvent.click(needs.getByRole("button", { name: "Answer" }));

  await userEvent.click(tab());
  await expect.element(page.getByRole("checkbox", { name: "Done: Sign the build" })).toBeVisible();
});
