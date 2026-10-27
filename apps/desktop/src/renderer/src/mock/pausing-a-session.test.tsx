// A Job he parked stays quiet and asks before a note reaches it: its row dims, pulses nothing and draws the pause mark alone, and a note
// typed to it opens Resume and send or Cancel. Through `App` over `pausing-a-session`, where "Rebuild the stage grid" waits on him.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const row = () => page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Rebuild the stage grid" });
const box = () => page.getByRole("region", { name: /^Job: Rebuild the stage grid/ }).getByRole("textbox", { name: "Message" });

async function pause() {
  mount("pausing-a-session");
  await onScreen();
  await expect.element(row()).toBeVisible();
  expect(row().element().matches("[data-needs]")).toBe(true);
  (row().element() as HTMLElement).focus();
  await userEvent.keyboard("{ContextMenu}");
  await userEvent.click(page.getByRole("menuitem", { name: "Pause" }));
  await expect.poll(() => row().element().matches("[data-paused]")).toBe(true);
}

test("a paused row stops pulsing and draws the pause mark and no state glyph", async () => {
  await pause();
  const el = row().element();
  expect(el.matches("[data-needs], [data-calling]")).toBe(false);
  expect(el.querySelector(".deck-mark")).toBeNull();
  expect(el.querySelector('[role="img"][aria-label="Paused"]')).not.toBeNull();
  // Nor does the Job's own drone ask: it is not waiting on him.
  expect(document.body.textContent).not.toContain("Waiting on you");
});

test("a note to a paused Job asks first: Cancel sends nothing and keeps the note, Resume and send resumes and then sends", async () => {
  await pause();
  await row().click();
  await userEvent.click(box());
  await userEvent.keyboard("Use sixteen columns{Enter}");
  const confirm = page.getByRole("dialog", { name: "Job is paused" });
  await expect.element(confirm).toBeVisible();
  await expect.element(confirm.getByRole("button", { name: /^Resume and send/ })).toHaveFocus();

  await userEvent.keyboard("{Escape}");
  await expect.element(confirm).not.toBeInTheDocument();
  expect(row().element().matches("[data-paused]")).toBe(true);
  await expect.poll(() => (box().element() as HTMLElement).textContent).toBe("Use sixteen columns");
  expect(page.getByText("Use sixteen columns", { exact: true }).elements().filter((one) => !one.closest('[role="textbox"]'))).toHaveLength(0);

  await userEvent.click(box());
  await userEvent.keyboard("{Enter}");
  await expect.element(confirm).toBeVisible();
  await expect.element(confirm.getByRole("button", { name: /^Resume and send/ })).toHaveFocus();
  await userEvent.keyboard("{Enter}");
  await expect.element(confirm).not.toBeInTheDocument();
  await expect.poll(() => row().element().matches("[data-paused]")).toBe(false);
  await expect.element(page.getByText("Use sixteen columns", { exact: true })).toBeVisible();
});
