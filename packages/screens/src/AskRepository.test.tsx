// The two callers this ask serves, and the one thing that tells them apart —
// #959: the Manifest surface's own ask still picks a repository nobody has
// set up, to open Setup on it; New job's own use shows the same entry greyed
// out and cannot pick it, since a Job needs a Manifest and nothing here may
// route anywhere.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import type { RepositorySummary } from "@armada/protocol";
import { AskRepository } from "./AskRepository";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const SET_UP: RepositorySummary = {
  root: "/Users/user/armada",
  records_root: "/records/armada",
  manifest: {
    id: "armada",
    repository: "armada",
    path: "/Users/user/armada/armada.yml",
    records_root: "/records/armada",
    version: 1,
    checks: [],
  },
};
const NOT_SET_UP: RepositorySummary = { root: "/Users/user/scratch", records_root: "/records/scratch" };

test("the Manifest surface's own ask can pick a repository nobody has set up", async () => {
  const picked: string[] = [];
  mount(
    <AskRepository
      repositories={[SET_UP, NOT_SET_UP]}
      title="Pick a repository to open its Manifest"
      next="Picking it opens Setup."
      onPick={(root) => picked.push(root)}
    />,
  );

  await page.getByRole("button", { name: "Choose a repository" }).click();
  await expect.element(page.getByRole("menuitem", { name: "scratch" })).not.toHaveAttribute("aria-disabled");
  await page.getByRole("menuitem", { name: "scratch" }).click();

  expect(picked).toEqual([NOT_SET_UP.root]);
});

test("New job's own ask shows a repository nobody has set up, greyed out and unpickable", async () => {
  const picked: string[] = [];
  mount(
    <AskRepository
      repositories={[SET_UP, NOT_SET_UP]}
      title="Pick the repository this Job is for"
      next="The Board stays on All."
      onPick={(root) => picked.push(root)}
      onlySetUp
    />,
  );

  await page.getByRole("button", { name: "Choose a repository" }).click();
  await expect.element(page.getByRole("menuitem", { name: "scratch — Set it up first" })).toHaveAttribute("aria-disabled", "true");
  await page.getByRole("menuitem", { name: "armada" }).click();

  // The greyed row takes no press at all — only the one Manifest is offered.
  expect(picked).toEqual([SET_UP.root]);
});

test("the filter field is there at any size, and typing narrows the rows to what matches", async () => {
  mount(
    <AskRepository
      repositories={[SET_UP, NOT_SET_UP]}
      title="Pick a repository to open its Manifest"
      next="Picking it opens Setup."
      onPick={() => undefined}
    />,
  );

  await page.getByRole("button", { name: "Choose a repository" }).click();
  await expect.element(page.getByRole("menuitem", { name: "armada" })).toBeVisible();
  await page.getByRole("searchbox", { name: "Find a repository" }).fill("scr");

  await expect.element(page.getByRole("menuitem", { name: "scratch" })).toBeVisible();
  expect(page.getByRole("menuitem", { name: "armada" }).query()).toBeNull();
});
