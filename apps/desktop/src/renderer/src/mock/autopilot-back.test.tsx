// Coming back to Autopilot and what it answered meanwhile, held as the owner approved it in the walks `autopilot-back` and `autopilot-why`.
// A walk dispatches its keys in one task, so React draws nothing between two listeners and it cannot show a key that opens a HUD
// acting inside it too: those tests send the real keys, one press at a time.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { withState } from "../state-feed";
import { WALK_UI } from "../walk-find";
import { minutesPass } from "../autopilot-back";
import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const hardware = () => page.getByRole("region", { name: "Autopilot switch" });
const lever = () => hardware().getByRole("switch", { name: "Autopilot" });
const lamp = () => hardware().getByRole("button", { name: "What Autopilot answered" });
const asking = () => page.getByRole("dialog", { name: "Autopilot is running" });
const hud = () => page.getByRole("dialog", { name: "What Autopilot answered" });
const answer = (job: string) => hud().getByRole("group", { name: job, exact: true });
const isOn = () => lever().element().getAttribute("aria-checked") === "true";
const stage = () => document.querySelector(".deck-stage > .deck-pane")?.getAttribute("aria-label") ?? "";
/** The answers the list holds, by Job. */
const listed = () => [...document.querySelectorAll('[data-hud="autopilot-answers"] .console-ap__row')].map((one) => one.getAttribute("aria-label"));

/** Two frames: what a press set off a frame later has happened, so a test that says nothing followed has waited for it. */
const frames = () => new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => done())));

/** Back at the window: Autopilot running with four answers waiting, and ten minutes passed with nobody there. */
async function away() {
  const app = mount("autopilot-back");
  await onScreen();
  await expect.poll(isOn).toBe(true);
  await expect.poll(() => lamp().element().getAttribute("data-lit")).not.toBeNull();
  await frames();
  timePasses();
  return app;
}

/** A press that is not on anything: the first input after ten minutes. */
const anyKey = () => userEvent.keyboard("{Shift}");

async function answers() {
  const app = mount("autopilot-back");
  await onScreen();
  await expect.poll(() => lamp().element().getAttribute("data-lit")).not.toBeNull();
  await lamp().click();
  await expect.element(hud()).toBeVisible();
  await expect.poll(listed).toHaveLength(4);
  return app;
}

const focusRow = async (job: string) => {
  (answer(job).element() as HTMLElement).focus();
  await frames();
};

test("after ten minutes the first press asks and does not open what was under it; Keep it on opens the answers and it asks again only ten minutes later", async () => {
  await away();
  // Forced, because the press that asks is eaten before the page sees it, which Playwright's own check for where a press lands takes for a press that missed.
  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Friday lunch", exact: true }).click({ force: true });
  await expect.element(asking()).toBeVisible();
  await frames();
  expect(stage()).toBe("Flight map");

  await asking().getByRole("button", { name: "Keep it on" }).click();
  await expect.element(asking()).not.toBeInTheDocument();
  // Either answer opens the answers where any wait.
  await expect.element(hud()).toBeVisible();

  await anyKey();
  await frames();
  await expect.element(asking()).not.toBeInTheDocument();

  timePasses();
  await anyKey();
  await expect.element(asking()).toBeVisible();
  // Esc is Keep it on: it runs on and nothing is stopped.
  await userEvent.keyboard("{Escape}");
  await expect.element(asking()).not.toBeInTheDocument();
  expect(isOn()).toBe(true);
});

test("T turns it off from the question, and opens the answers", async () => {
  const app = await away();
  const stop = vi.spyOn(app.api, "stopAutopilot");
  await anyKey();
  await expect.element(asking()).toBeVisible();
  await userEvent.keyboard("t");
  await expect.poll(() => stop.mock.calls.length).toBe(1);
  await expect.element(asking()).not.toBeInTheDocument();
  await expect.poll(isOn).toBe(false);
  await expect.element(hud()).toBeVisible();
});

test("a press inside the switch is the owner using Autopilot, and does not ask", async () => {
  await away();
  await lamp().click();
  await expect.element(hud()).toBeVisible();
  await expect.element(asking()).not.toBeInTheDocument();
});

test("a walk's own bar is not input: it does not ask, and it does not count as being back", async () => {
  await away();
  const bar = document.createElement("div");
  bar.setAttribute(WALK_UI, "");
  bar.innerHTML = "<button>Reload</button>";
  document.body.append(bar);
  try {
    await page.getByRole("button", { name: "Reload", exact: true }).click();
    await frames();
    await expect.element(asking()).not.toBeInTheDocument();
    (document.activeElement as HTMLElement | null)?.blur();
    // Had the bar's press counted, the first press at the window would find him back.
    await anyKey();
    await expect.element(asking()).toBeVisible();
  } finally {
    bar.remove();
  }
});

test("back with nothing answered, Keep it on opens no list", async () => {
  mount("autopilot-why");
  await onScreen();
  await expect.element(lever()).toBeInTheDocument();
  await hardware().getByRole("button", { name: "Lift guard" }).click();
  await page.getByRole("region", { name: "Autopilot setup" }).getByRole("button", { name: "Start Autopilot" }).click();
  await expect.poll(isOn).toBe(true);
  await frames();
  // His keys were in the setup, which is his using Autopilot and so never the first press back.
  (document.activeElement as HTMLElement | null)?.blur();
  minutesPass(10 * 60_000);
  await anyKey();
  await expect.element(asking()).toBeVisible();
  await asking().getByRole("button", { name: "Keep it on" }).click();
  await expect.element(asking()).not.toBeInTheDocument();
  await frames();
  expect(document.querySelector('[data-hud="autopilot-answers"]')).toBeNull();
});

test("a row shows the Job, what was asked, what was chosen, why and when; Enter agrees and the row leaves", async () => {
  const app = await answers();
  const agree = vi.spyOn(withState(app.api), "reviewAutopilotDecision");
  const row = answer("Friday lunch");
  await expect.element(row).toHaveTextContent("Which size should the order be?");
  await expect.element(row).toHaveTextContent("Large");
  await expect.element(row).toHaveTextContent("The Job recommended Large, and the request names no size.");
  expect(row.element().querySelector(".console-ap__age")?.textContent?.trim()).not.toBe("");

  await focusRow("Friday lunch");
  await userEvent.keyboard("{Enter}");
  await expect.poll(listed).toEqual(["Reconnect fix", "Release notes", "Pull request 1894"]);
  expect(agree).toHaveBeenCalledTimes(1);
});

test("t opens a box on the row, and sending corrects the Job and the row leaves", async () => {
  const app = await answers();
  const send = vi.spyOn(withState(app.api), "overrideAutopilotDecision");
  await focusRow("Reconnect fix");
  await userEvent.keyboard("t");
  const box = hud().getByRole("textbox", { name: "Change" });
  await expect.element(box).toBeVisible();
  await box.fill("feat");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => send.mock.calls.length).toBe(1);
  expect(send.mock.calls[0]![1]).toBe("feat");
  await expect.poll(listed).toEqual(["Friday lunch", "Release notes", "Pull request 1894"]);
});

test("o opens the Job, and Back comes to the same row", async () => {
  await answers();
  await focusRow("Release notes");
  await userEvent.keyboard("o");
  await expect.element(page.getByRole("region", { name: "Job: Release notes" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(answer("Release notes")).toBeVisible();
  await expect.poll(() => (document.activeElement as HTMLElement | null)?.getAttribute("aria-label")).toBe("Release notes");
});

test("a row that cannot be corrected offers only Agree", async () => {
  await answers();
  const landed = answer("Pull request 1894");
  await expect.element(landed.getByRole("button", { name: "Agree" })).toBeVisible();
  await expect.element(landed.getByRole("button", { name: "Change" })).not.toBeInTheDocument();
  await expect.element(landed.getByRole("button", { name: "Open the Job" })).not.toBeInTheDocument();
  // A row with a Job offers all three.
  const lunch = answer("Friday lunch");
  await expect.element(lunch.getByRole("button", { name: "Change" })).toBeVisible();
  await expect.element(lunch.getByRole("button", { name: "Open the Job" })).toBeVisible();
});

test("the lamp lights while answers wait, carries no count, and answers are not calls", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(lever()).toBeInTheDocument();
  // Nothing waiting: unlit, and it does nothing.
  expect(lamp().element().getAttribute("data-lit")).toBeNull();
  expect(lamp().element().getAttribute("aria-disabled")).toBe("true");
  await lamp().element().click();
  expect(document.querySelector('[data-hud="autopilot-answers"]')).toBeNull();
});

test("answers waiting light the lamp with no number, pulse no row, and are not recalled by w", async () => {
  mount("autopilot-back");
  await onScreen();
  await expect.poll(() => lamp().element().getAttribute("data-lit")).not.toBeNull();
  expect(lamp().element().textContent).not.toMatch(/\d/);
  expect(lamp().element().getAttribute("aria-label")).toBe("What Autopilot answered");
  // The three Jobs they were given in are idle ones, and none calls.
  expect(document.querySelectorAll(".deck-row[data-calling]")).toHaveLength(0);
  await lamp().click();
  await expect.element(hud()).toBeVisible();
  expect(hud().element().querySelector(".console-hud__name, h2")?.textContent ?? "").not.toMatch(/\d/);
  // `w` brings a waiting call back, and there is none to bring.
  (document.activeElement as HTMLElement | null)?.blur();
  await userEvent.keyboard("{Escape}");
  await userEvent.keyboard("w");
  await frames();
  expect(document.querySelector('.armada-needs-panel[data-open="true"]')).toBeNull();
});

test("the lamp's own press opens the list and agrees with nothing in it", async () => {
  const app = await answers();
  const agree = vi.spyOn(withState(app.api), "reviewAutopilotDecision");
  await userEvent.keyboard("{Escape}");
  await expect.element(hud()).not.toBeInTheDocument();
  (lamp().element() as HTMLElement).focus();
  // One Enter is a keydown on the lamp, which opens the list, and then the list is up under the same key.
  await userEvent.keyboard("{Enter}");
  await expect.element(hud()).toBeVisible();
  await frames();
  expect(listed()).toHaveLength(4);
  expect(agree).not.toHaveBeenCalled();
});

test("the key that answers the question opens the list and does not also act in it", async () => {
  const app = await away();
  const agree = vi.spyOn(withState(app.api), "reviewAutopilotDecision");
  const stop = vi.spyOn(app.api, "stopAutopilot");
  await anyKey();
  await expect.element(asking()).toBeVisible();
  // Keep it on is the focused button, so Enter answers it.
  await userEvent.keyboard("{Enter}");
  await expect.element(hud()).toBeVisible();
  await frames();
  expect(listed()).toHaveLength(4);
  expect(agree).not.toHaveBeenCalled();
  expect(stop).not.toHaveBeenCalled();

  // And `t`, which turns it off, is not also `t` on the first row.
  timePasses();
  await userEvent.keyboard("{Escape}");
  await anyKey();
  await expect.element(asking()).toBeVisible();
  await userEvent.keyboard("t");
  await expect.element(hud()).toBeVisible();
  await frames();
  expect(document.querySelector('[data-hud="autopilot-answers"] textarea')).toBeNull();
  expect(listed()).toHaveLength(4);
});
