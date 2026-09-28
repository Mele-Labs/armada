// The title row's picker: what it calls a repository, and the control beside
// it that opens that repository's Manifest — the owner, 28 Sep 2026.
//
// *"Is it possible to use a real fake name of a repo/manifest here instead of a
// ULID?"* and *"I should be able to select a manifest from the dropdown and get
// a button or link or icon or something to open that manifest"*.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The control beside the pick. Its name is read off the surface roster, never retyped here. */
const openManifest = () => page.getByRole("button", { name: "Open the Manifest", exact: true });

/** A ULID, which is what `ManifestId` is — so it is what a label must never be. */
const ULID = /^01[0-9A-HJKMNP-TV-Z]{24}$/;

test("the picker names a repository, and never its Manifest's minted id", async () => {
  mount("studios");
  await onScreen();

  const trigger = page.getByRole("button", { name: /armada/ });
  await expect.element(trigger).toBeVisible();
  expect(trigger.element().textContent ?? "").not.toMatch(ULID);
});

test("the control beside the pick opens that repository's Manifest", async () => {
  mount("manifest");
  await onScreen();

  await openManifest().click();
  // The surface's own file tab, named by the path it reads — what the rail row
  // has always landed on.
  await expect.element(page.getByRole("tab", { name: /armada\.yml/ })).toBeVisible();
});

test("on All repositories the control still opens the surface, which asks which one", async () => {
  // Two repositories served and neither set up, so there is no Manifest to
  // open: the surface asks, `AskRepository`'s own job, rather than this control
  // growing a second answer to the same question.
  mount("nothing-set-up");
  await onScreen();

  await openManifest().click();
  await expect.element(page.getByText(/[Pp]ick a repository/)).toBeVisible();
});

test("nothing served is nothing to open, so the control refuses the press", async () => {
  mount("first-launch");
  await onScreen();
  await expect.element(openManifest()).toBeDisabled();
});
