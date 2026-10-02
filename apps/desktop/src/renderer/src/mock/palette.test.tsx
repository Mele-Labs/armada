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

/** A Job open, and the palette searched for `query`. */
async function searched(query: string) {
  const fixture = running();
  mount(onJob(fixture));
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  const list = await palette();
  await userEvent.fill(list.getByRole("combobox"), query);
  return list;
}

// **A greyed row sends a person somewhere that exists.** The reason named the
// story's Produced chapter, and the Overview reframe of 29 Sep 2026 retired the
// chapters. `f` is what reaches the patch now, and only on an open Job's
// Overview, where `tab-overview.tsx` binds `useDetailKeys`.
test("searching diff, the greyed row says where the act is", async () => {
  const list = await searched("diff");
  const option = list.getByRole("option", { name: /^Open the diff/ });
  await expect.element(option).toHaveAttribute("aria-disabled", "true");
  await expect.element(option).toHaveTextContent("a job's Overview, with f");
});

// **The owner removed Open the log on 2 Oct 2026**: nothing on a Job is a
// single log. Its aliases went with it, so nothing in the palette answers the
// word, and the palette's own miss is what is waited for.
test("searching log offers no Open the log", async () => {
  const list = await searched("log");
  await expect.element(list.getByText("Nothing matches “log”.", { exact: false })).toBeVisible();
  expect(list.getByRole("option", { name: /^Open the log/ }).query()).toBeNull();
});

// **The owner removed Expand and collapse on 2 Oct 2026**, with the keys that
// moved through the activity log and opened its rows: the log went, and they
// reached nothing. Searched by its verb rather than its id, because the
// palette matches the label and `disclose` never matched anything — and by two
// words, because Toggle sidebar answers "expand the column".
test("searching expand and offers no Expand and collapse", async () => {
  const list = await searched("expand and");
  await expect.element(list.getByText("Nothing matches “expand and”.", { exact: false })).toBeVisible();
  expect(list.getByRole("option", { name: /^Expand and collapse/ }).query()).toBeNull();
});
