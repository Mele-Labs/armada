// What `helm-talking` exists to make reachable, through `App`.
//
// The dock's own controls had no scenario behind them: every moment published
// `helm: { state: "none" }`, so the thread, the composer in its pointed state
// and the record's split button could only be seen by a test building the
// state inline — which is how #1488 came to stand up a scenario, screenshot
// through it and delete it again. This asserts the two things of it that are
// the dock's own, the composer and the split button. The thread is a Session's, held in
// `apps/desktop/src/renderer/src/mock/helm-session.test.tsx`.
//
// `helm-dock.test.tsx` beside this owns the *unpointed* dock and *Start fresh*.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { entered, mount, openHelm, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Helm only: what the dock reads is its own slice's.
const SLICES = { slices: ["core", "helm"] } as const;

test("pointed at a repository, the composer takes a message", async () => {
  mount("helm-talking", SLICES);
  await openHelm();

  const field = page.getByRole("textbox", { name: "Ask Helm" });
  await expect.element(field).toBeEnabled();
  // Send is refused on a blank field and offered on a typed one — the whole of
  // what the composer decides, and unreachable while Helm is pointed at nothing.
  const send = page.getByRole("button", { name: /^Send/ });
  await expect.element(send).toBeDisabled();
  await field.fill("has this test ever passed on main");
  await expect.element(send).toBeEnabled();

  // No switch: pointed at the one repository set up, there is nothing to switch to.
  expect(page.getByRole("button", { name: /^Point Helm at a/ }).query()).toBeNull();
});

test("the record's split button offers both acts, and Details opens this session and no other", async () => {
  mount("helm-talking", SLICES);
  await openHelm();

  // The face is the act the pair exists for, and the caret carries the reading.
  await expect.element(page.getByRole("button", { name: "Copy debug info" })).toBeEnabled();
  await page.getByRole("button", { name: "More ways to report this answer" }).click();
  await page.getByRole("menuitem", { name: "Details" }).click();

  const sheet = page.getByRole("dialog", { name: "Job record" });
  await entered(sheet);
  const shown = page.getByText(/armada helm session/);
  await expect.element(shown).toBeVisible();

  // The record is the conversation on screen, from Fleet's side — one session,
  // not a second fixture that happens to be drawable.
  const text = shown.element().textContent ?? "";
  expect(text).toContain("why did 77 stop");
  expect(text).toContain("called list_sessions");
});
