// A Session's waiting line on the Cockpit's call card, through `App` on the `session-errands` mock
// Fleet. What the card offers follows the item's act: a slash command is sent to the Session, a shell
// command is copied, a pull request opens, a plain line is a text box, and only a question with
// options carries the standing answers. An act is a key on the rail that runs on one press; the words
// are typed in the Session's own box under the card, and a pick goes with them.

import { beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetDismissals } from "../cockpit/dismissed";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();
beforeEach(forgetDismissals);

const radio = (name: RegExp) => page.getByRole("radio", { name });
// The acts are keycaps on the rail beside the card; the Session's box is under it, with a Send of its own.
const key = (name: RegExp) => page.getByRole("complementary").getByRole("button", { name });
const card = () => page.getByRole("region", { name: /^Session/ });
const send = () => card().getByRole("button", { name: /Send/ });
const words = () => page.getByRole("textbox", { name: "Message" });
const COMMAND = "kill $(lsof -ti tcp:5788) && scripts/preview --restart --adopt";
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
  await expect.element(page.getByRole("region", { name: /^Job: Land the speed fixes/ })).toBeVisible();
  return app;
}

const noStanding = () => STANDING.forEach((name) => expect(radio(name).query()).toBeNull());

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a run act with a slash command is Run <command>, and Send tells the Session the command as its reply", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  await expect.element(key(/Run/)).toBeVisible();
  await expect.element(card().getByRole("code")).toHaveTextContent("/land 2109 2118 2119");
  expect(radio(/Run \/land/).query()).toBeNull();
  noStanding();
  await expect.element(words()).toBeVisible();
  await key(/Run/).click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s20", item_id: "e-slash", text: "/land 2109 2118 2119" });
});

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a run act with a shell command is a Copy key, the command is on the card in its own case, and pressing it copies and leaves the card", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  const write = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);
  await reach("Restart onto the preview");
  // The key says the verb alone; the whole command, in the case it was written, is a line on the card.
  await expect.element(key(/Copy/)).toBeVisible();
  expect(key(/Copy/).element().textContent).not.toContain("lsof");
  expect(getComputedStyle(key(/Copy/).element().querySelector(".armada-keycap__verb")!).textTransform).toBe("uppercase");
  // No key clips what it says, top or bottom.
  for (const one of document.querySelectorAll<HTMLElement>(".armada-console--right .armada-keycap > button")) expect(one.scrollHeight).toBeLessThanOrEqual(one.clientHeight + 1);
  const target = card().getByRole("code");
  await expect.element(target).toHaveTextContent(COMMAND);
  expect(getComputedStyle(target.element()).textTransform).toBe("none");
  // Cut or not, it is whole on hover.
  await target.hover();
  await expect.poll(() => [...document.querySelectorAll<HTMLElement>(".armada-tooltip__label")].some((one) => one.textContent === COMMAND && one.checkVisibility({ visibilityProperty: true, opacityProperty: true }))).toBe(true);
  noStanding();
  await key(/Copy/).click();
  await expect.poll(() => write.mock.calls.length).toBe(1);
  expect(write).toHaveBeenLastCalledWith(COMMAND);
  expect(answer).not.toHaveBeenCalled();
  await expect.element(key(/Copy/)).toBeVisible();
});

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("an approve_pr act is Open #N, and pressing it opens the pull request's address", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  const link = vi.spyOn(app.api, "openLink").mockResolvedValue({ ok: true });
  await reach("Approve #2118");
  await expect.element(key(/Open #2118/)).toBeVisible();
  noStanding();
  await key(/Open #2118/).click();
  await expect.poll(() => link.mock.calls.length).toBe(1);
  expect(link).toHaveBeenLastCalledWith("https://git.example/armada/pull/2118");
  expect(answer).not.toHaveBeenCalled();
});

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a plain line has no answers, typing enables the box's Send, and it sends the words", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  await reach("Which branch should the migration rebase onto");
  expect(page.getByRole("radio").query()).toBeNull();
  // Nothing typed: the key opens the Session, as it does for any call with no answer.
  await expect.element(key(/Open Job/).first()).toBeVisible();
  await userEvent.type(words(), "Main");
  await expect.element(send()).toBeEnabled();
  await send().click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s20", item_id: "e-plain", text: "Main" });
});

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a question with options keeps the standing answers, and with an answer picked and words typed Send sends the pick with the words", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  await reach("Which clock should the store tests use?");
  for (const name of STANDING) await expect.element(radio(name)).toBeVisible();
  await userEvent.keyboard("2");
  await expect.element(radio(/The real one, stopped/)).toHaveAttribute("aria-checked", "true");
  await userEvent.type(words(), "Neither, use a counter");
  await send().click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s20", item_id: "e-ask", choice: 1, text: "Neither, use a counter" });
});

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a picked answer with an empty box still sends the pick", async () => {
  const app = await open();
  const answer = vi.spyOn(app.api, "answerWaiting");
  await reach("Which clock should the store tests use?");
  await userEvent.keyboard("2");
  await key(/Send/).click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s20", item_id: "e-ask", choice: 1 });
});
