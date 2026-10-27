// A job's Bash write outside its slots, through `App`: its card draws a Calls like row over the call's
// kind, starting on the script, with Only in off. Allowed for the script, the next drive.mjs call runs
// with no card and, once time passes, a cleanup.mjs call is held on a new one naming cleanup.mjs.
// Allowed for every node call, that one runs too. The walk `callsLikeThis` tells the first.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const ANSWER = "Allow calls like this for this job";
const BROWSER = "Bash node drive.mjs --browser /Users/user/Library/Caches/ms-playwright/chromium/chrome";

test("Calls like this: allowed for the script, the next drive.mjs call runs and cleanup.mjs asks", async () => {
  mount("calls-like-this");
  await onScreen();
  await toSession(/Browser checks/);

  const card = page.getByRole("article", { name: "Waiting on you" });
  const like = card.getByRole("group", { name: "Calls like" });
  await expect.element(card.getByText("Permission to write", { exact: true })).toBeVisible();
  await expect.element(like.getByRole("button", { name: "drive.mjs" })).toHaveAttribute("aria-pressed", "true");
  await userEvent.click(like.getByRole("button", { name: "node" }));
  await expect.element(like.getByRole("button", { name: "node" })).toHaveAttribute("aria-pressed", "true");
  await userEvent.click(like.getByRole("button", { name: "drive.mjs" }));
  await expect.element(like.getByRole("button", { name: "drive.mjs" })).toHaveAttribute("aria-pressed", "true");
  await expect.element(card.getByRole("switch", { name: "Only in shot.png" })).not.toBeChecked();
  await userEvent.click(card.getByRole("button", { name: ANSWER }));

  const thread = page.getByRole("region", { name: "Thread" });
  const browser = thread.getByText(BROWSER, { exact: true });
  await expect.element(browser).toBeInTheDocument();
  await expect.element(card).not.toBeInTheDocument();
  await userEvent.click(thread.getByLabelText(/^Tool calls: /));
  await expect.element(browser).toBeVisible();

  timePasses();
  await expect.element(like.getByRole("button", { name: "cleanup.mjs" })).toHaveAttribute("aria-pressed", "true");
  await expect.element(card.getByText("node cleanup.mjs /Users/user/code/other-repo", { exact: true })).toBeVisible();
});

test("Calls like this: allowed for every node call, cleanup.mjs runs with no card", async () => {
  mount("calls-like-this");
  await onScreen();
  await toSession(/Browser checks/);

  const card = page.getByRole("article", { name: "Waiting on you" });
  await userEvent.click(card.getByRole("group", { name: "Calls like" }).getByRole("button", { name: "node" }));
  await userEvent.click(card.getByRole("button", { name: ANSWER }));

  const thread = page.getByRole("region", { name: "Thread" });
  const cleanup = thread.getByText("Bash node cleanup.mjs /Users/user/code/other-repo", { exact: true });
  await expect.element(cleanup).toBeInTheDocument();
  await userEvent.click(thread.getByLabelText(/^Tool calls: /));
  await expect.element(cleanup).toBeVisible();
  await expect.element(card).not.toBeInTheDocument();
});
