// A Job whose thread has not been read cannot say whether anything was sent in it, and the first-Job card is claimed by the first Job that opens
// on a repository. Opening one unread must not claim it: no card and no glyph while the bars stand, and none when the rows arrive with the
// person's own message in them, since a repository already in use has no card to show. The walk `transcript-loading` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const COLD = "Fix the flaky store clock test";
const CACHED = "Fix the retry backoff reset";
const KEPT = "armada.bridge.first-jobs";
/** The scenario holds each first read for four seconds; the rows land after, and a poll allows for it. */
const READ = { timeout: 12_000 };

const jobs = () => page.getByRole("region", { name: "Jobs", exact: true });
const thread = () => page.getByRole("region", { name: "Thread", exact: true });
const card = () => thread().getByText("Armada configuration", { exact: true });
const glyph = () => page.getByRole("button", { name: /^Armada configuration kept/ });
/** Which Job holds the card of each repository, as the browser keeps it. */
const claims = (): Record<string, string | null> => JSON.parse(localStorage.getItem(KEPT) ?? "{}");

async function open(title: string) {
  mount("transcript-loading");
  await onScreen();
  await jobs().getByRole("button", { name: title, exact: true }).click();
  await expect.element(page.getByRole("region", { name: `Job: ${title}` })).toBeVisible();
}

test("an unread Job claims no card while it is read, and none when the person's message is in the rows", async () => {
  await open(COLD);

  // The bars stand: the thread is busy, and neither the card nor the glyph is drawn nor claimed.
  await expect.poll(() => document.querySelector('[aria-busy="true"].armada-helm-thread__rows') !== null).toBe(true);
  expect(page.getByText("passes 200 runs in a row").query()).toBeNull();
  expect(card().query()).toBeNull();
  expect(glyph().query()).toBeNull();
  expect(claims()).toEqual({});

  // The rows land, the first of them the person's own: this repository is in use, so no Job on it carries a card.
  await expect.element(thread().getByText("passes 200 runs in a row"), READ).toBeVisible();
  expect(card().query()).toBeNull();
  expect(claims()).toEqual({ [repository().root]: null });
});

test("a Job read before and read again claims no card either", async () => {
  await open(CACHED);

  await expect.element(thread().getByText("The counter lives on the connection")).toBeVisible();
  expect(card().query()).toBeNull();
  expect(claims()).toEqual({ [repository().root]: null });
});
