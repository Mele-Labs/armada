// The command palette, through `App`: what it leaves out with no Job focused,
// and what it still draws dimmed. `docs/contracts/design-system.md`, Command
// palette.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { running } from "@armada/screens/src/fixtures/build/index";
import { boardJobs, boardWorkflows } from "@armada/screens/src/fixtures/build/board";

import { onBoard, onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** ⌘K, and the palette's list once it is up. */
async function palette() {
  await userEvent.keyboard("{Meta>}k{/Meta}");
  const list = page.getByRole("dialog", { name: "Command palette" });
  await expect.element(list).toBeVisible();
  return list;
}

test("with no Job focused, an act on one Job is left out, and a row not built yet still draws", async () => {
  mount(onBoard(boardJobs(), { workflows: boardWorkflows() }));
  await expect.element(page.getByRole("heading", { name: "Running" }).first()).toBeVisible();
  const list = await palette();
  await expect.element(list.getByRole("option", { name: /^Pilot/ })).toBeInTheDocument();
  for (const verb of ["Open", "Review", "Attest", "Redirect", "Kill"]) {
    expect(list.getByRole("option", { name: new RegExp(`^${verb}\\b`) }).query()).toBeNull();
  }
  expect(list.getByText("no job focused", { exact: true }).query()).toBeNull();
});

test("with a Job open, its acts are back", async () => {
  const fixture = running();
  mount(onJob(fixture));
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  const list = await palette();
  await expect.element(list.getByRole("option", { name: /^Kill/ }).first()).toBeInTheDocument();
});

// **A greyed row sends a person somewhere that exists.** Both reasons named a
// chapter of the step's story, and the Overview reframe of 29 Sep 2026 retired
// the chapters. `L` and `f` are what reach the log and the patch now, and only
// on an open Job's Overview, where `tab-overview.tsx` binds `useDetailKeys`.
test.each([
  ["log", /^Open the log/, "a job's Overview, with L"],
  ["diff", /^Open the diff/, "a job's Overview, with f"],
])("searching %s, the greyed row says where the act is", async (query, row, reason) => {
  const fixture = running();
  mount(onJob(fixture));
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  const list = await palette();
  await userEvent.fill(list.getByRole("combobox"), query);
  const option = list.getByRole("option", { name: row });
  await expect.element(option).toHaveAttribute("aria-disabled", "true");
  await expect.element(option).toHaveTextContent(reason);
});
