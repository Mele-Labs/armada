// Studios on All repositories, through `App`, against what is set up and what is not: no
// repository with a Manifest, which is the dead end the surface used to draw as a picker with
// nothing pickable in it, and the mixture, where the picker holds both halves at once.
//
// **Beside `studios.test.tsx` rather than in it**: that file is #1287's and #1341's definition of
// done, and this is one state neither of them names.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach, scenarioNamed, toSystemPage } from "@armada/desktop/mock";

const SLICES = { slices: ["core", "studios"] } as const;

unmountAfterEach();

/** Studios, from the rail, on a window that opens on Overview. */
async function openStudios(): Promise<void> {
  await toSystemPage("Studios");
}

/**
 * The ask's own menu button, which is the picker this state must not draw. **Named exactly**:
 * Helm's footer draws a second menu button — named *Point Helm at a repository*, or *at a
 * different repository* once Helm is pointed at one — and it is not this one.
 */
const picker = () => page.getByRole("button", { name: "Choose a repository", exact: true });

test("no repository set up: Studios says so and offers Setup, rather than a picker holding nothing", async () => {
  // `nothing-set-up` is the moment: repositories served, none of them with a Manifest, and the
  // rail on All repositories because nobody picked one.
  mount("nothing-set-up", SLICES);
  await openStudios();

  await expect
    .element(page.getByText("No repository is set up yet, so none of them keeps Studios."))
    .toBeVisible();
  await expect
    .element(page.getByText("A Studio is kept against a repository's Manifest. Set one up, and its Studios open here."))
    .toBeVisible();
  // Not an empty question: no select at all, and no ask that would have held one.
  expect(picker().query()).toBeNull();
  expect(page.getByText("Pick a repository to open its Studios").query()).toBeNull();

  // The way forward is Bridge's existing one: the Manifest surface, whose own ask picks a
  // repository, and picking one with no Manifest opens Setup on it.
  await page.getByRole("button", { name: "Set up a repository" }).click();
  await expect.element(page.getByText("Pick a repository to open its Manifest")).toBeVisible();
});

/** The titles of the panels the page draws, one for each repository that keeps a Studio. */
const panelTitles = () =>
  [...document.querySelectorAll(".armada-studios-page > .armada-card .armada-card-title")].map((one) => one.textContent ?? "");

test("default on All draws one panel for each repository that keeps Studios, and asks for no repository", async () => {
  const scenario = scenarioNamed("default")!;
  mount({ ...scenario, state: { ...scenario.state, repository: null } }, SLICES);
  await openStudios();

  await expect.poll(() => panelTitles().length).toBeGreaterThan(0);
  const titles = panelTitles();
  expect(new Set(titles).size).toBe(titles.length);
  expect(page.getByText("Pick a repository to open its Studios").query()).toBeNull();
  expect(picker().query()).toBeNull();
  expect(page.getByText("No repository is set up yet, so none of them keeps Studios.").query()).toBeNull();
});
