// A Session's waiting line on the Cockpit's call card, through `App` on the `session-errands` mock
// Fleet. What the card offers follows the item's act: a slash command is sent to the Session, a shell
// command is copied, a pull request opens, a plain line is a text box, and only a question with
// options carries the standing answers. Send takes what was typed over a picked answer.

import { beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetDismissals } from "../cockpit/dismissed";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();
beforeEach(forgetDismissals);

const radio = (name: RegExp) => page.getByRole("radio", { name });
const send = () => page.getByRole("button", { name: /^Send/ });
const words = () => page.getByRole("textbox", { name: "Type something" });
const STANDING = [/Make the best decision/, /Just get it done/];

/** Puts the cards in front of the one named off, until it is the one in front. */
async function reach(label: string): Promise<void> {
  for (let tries = 0; tries < 8 && document.querySelector(".armada-callcard")?.textContent?.includes(label) !== true; tries += 1) {
    const front = document.querySelector(".armada-callcard")?.textContent;
    await userEvent.keyboard("l");
    await expect.poll(() => document.querySelector(".armada-callcard")?.textContent !== front, { timeout: 3000 }).toBe(true);
  }
  expect(document.querySelector(".armada-callcard")?.textContent).toContain(label);
}

async function open() {
  localStorage.removeItem("armada.bridge.dashboard-tab");
  const app = mount("session-errands");
  await onScreen();
  await expect.element(page.getByRole("region", { name: /^Session: Land the speed fixes/ })).toBeVisible();
  return app;
}

const noStanding = () => STANDING.forEach((name) => expect(radio(name).query()).toBeNull());

test("a run act with a slash command is Run <command>, and Send tells the Session the command as its reply", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  await expect.element(radio(/Run \/land 2109 2118 2119/)).toBeVisible();
  noStanding();
  await expect.element(words()).toBeVisible();
  await send().click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s20", item_id: "e-slash", text: "/land 2109 2118 2119" });
});

test("a run act with a shell command is Copy <command>, and pressing it copies and leaves the card", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  const write = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);
  await reach("Restart onto the preview");
  await expect.element(radio(/Copy scripts\/preview --restart --adopt/)).toBeVisible();
  noStanding();
  await send().click();
  await expect.poll(() => write.mock.calls.length).toBe(1);
  expect(write).toHaveBeenLastCalledWith("scripts/preview --restart --adopt");
  expect(answer).not.toHaveBeenCalled();
  await expect.element(radio(/Copy scripts\/preview --restart --adopt/)).toBeVisible();
});

test("an approve_pr act is Open #N, and pressing it opens the pull request's address", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  const link = vi.spyOn(app.api, "openLink").mockResolvedValue({ ok: true });
  await reach("Approve #2118");
  await expect.element(radio(/Open #2118/)).toBeVisible();
  noStanding();
  await send().click();
  await expect.poll(() => link.mock.calls.length).toBe(1);
  expect(link).toHaveBeenLastCalledWith("https://git.example/armada/pull/2118");
  expect(answer).not.toHaveBeenCalled();
});

test("a plain line has no answers, typing enables Send, and the Send button sends the words", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  await reach("Which branch should the migration rebase onto");
  expect(page.getByRole("radio").query()).toBeNull();
  // Nothing typed: the button opens the Session, as it does for any call with no answer.
  await expect.element(page.getByRole("button", { name: /^Open Session/ }).first()).toBeVisible();
  await userEvent.type(words(), "Main");
  await expect.element(send()).toBeEnabled();
  await send().click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s20", item_id: "e-plain", text: "Main" });
});

test("a question with options keeps the standing answers, and with an answer picked and words typed Send sends the words", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  await reach("Which clock should the store tests use?");
  for (const name of STANDING) await expect.element(radio(name)).toBeVisible();
  await userEvent.keyboard("2");
  await expect.element(radio(/The real one, stopped/)).toHaveAttribute("aria-checked", "true");
  await userEvent.type(words(), "Neither, use a counter");
  await send().click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s20", item_id: "e-ask", text: "Neither, use a counter" });
});

test("a picked answer with an empty box still sends the pick", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  await reach("Which clock should the store tests use?");
  await userEvent.keyboard("2");
  await send().click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s20", item_id: "e-ask", choice: 1 });
});
