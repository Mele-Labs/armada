// The Dashboard's one panel, through `App` on the `dashboard-cockpit` mock Fleet: its filters and the
// keys that step them, the call that comes forward over it and the deck behind, the standing
// answers, and the grid and the map offering the same acts.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const FILTER = "armada.bridge.dashboard-tab";
const VIEW = "armada.bridge.cockpit-view";

/** The Dashboard, opened as a viewer who remembers nothing. */
async function dashboard(): Promise<void> {
  localStorage.removeItem(FILTER);
  localStorage.removeItem(VIEW);
  mount("dashboard-cockpit");
  await onScreen();
  await expect.element(page.getByRole("tab", { name: "Your move" })).toHaveAttribute("aria-selected", "true");
  // Nothing needs the owner, so the dispatch bar has the cursor; Escape hands the keys to the panel.
  await expect.element(page.getByRole("textbox", { name: "Request" })).toHaveFocus();
  await userEvent.keyboard("{Escape}");
}

const call = (kind: RegExp) => page.getByRole("region", { name: kind });
const behind = (kind: RegExp) => page.getByRole("button", { name: kind });
const selected = (name: string) => expect.element(page.getByRole("tab", { name })).toHaveAttribute("aria-selected", "true");
const radio = (name: RegExp) => page.getByRole("radio", { name });

test("Your move with nothing waiting is empty and says nothing, and the cursor is in the dispatch bar", async () => {
  localStorage.removeItem(FILTER);
  mount("dashboard-cockpit");
  await expect.element(page.getByRole("textbox", { name: "Request" })).toHaveFocus();
  await expect.element(page.getByRole("listbox", { name: "Tiles" })).toBeVisible();
  expect(document.querySelectorAll('[role="listbox"][aria-label="Tiles"] [role="option"]').length).toBe(0);
});

test("[ and ] step the filters round, Option and a digit jumps, and the filter is kept", async () => {
  await dashboard();
  await userEvent.keyboard("]");
  await selected("Active");
  await userEvent.keyboard("]");
  await selected("Done");
  await userEvent.keyboard("]");
  await selected("Your move");
  // `[[` is the bracket itself, in user-event's own syntax.
  await userEvent.keyboard("[[");
  await selected("Done");
  await userEvent.keyboard("{Alt>}2{/Alt}");
  await selected("Active");
  expect(localStorage.getItem(FILTER)).toBe("running");
  for (const name of ["Your move", "Active", "Done"]) {
    expect(page.getByRole("tab", { name }).element().textContent).not.toMatch(/\d/);
  }
});

test("the merge line draws every open pull request, the queue nearest main, and a press opens one", async () => {
  await dashboard();
  const blocks = [...document.querySelectorAll<HTMLElement>(".armada-view__pull")];
  // Nearest main first in the document, which the belt draws from the right: queue by place, then the rest as listed.
  expect(blocks.map((one) => one.querySelector("span")?.textContent)).toEqual(["#1890", "#1891", "#1893", "#1894", "#1895"]);
  expect(blocks.map((one) => one.hasAttribute("data-queued"))).toEqual([true, true, false, false, false]);
  expect(blocks[0]!.getAttribute("aria-label")).toContain("Job: Debounce the Job Board's resize handler");
  // A pull request's checks failing turns its block red.
  expect(blocks[3]!.getAttribute("data-state")).toBe("running");
  timePasses();
  timePasses();
  timePasses();
  timePasses();
  await expect.poll(() => document.querySelector('.armada-view__pull[aria-label^="#1894"]')?.getAttribute("data-state")).toBe("failing");
});

test("n brings the cursor back to the dispatch bar from the panel", async () => {
  await dashboard();
  await expect.element(page.getByRole("textbox", { name: "Request" })).not.toHaveFocus();
  await userEvent.keyboard("n");
  await expect.element(page.getByRole("textbox", { name: "Request" })).toHaveFocus();
});

test("a call comes forward over the panel whatever the filter, the next steps up, and one put off goes to the back until w", async () => {
  await dashboard();
  timePasses();
  await expect.element(call(/^Plan question/)).toBeVisible();
  // Over the panel on every filter: it is what needs the owner.
  await page.getByRole("tab", { name: "Done" }).click();
  await expect.element(call(/^Plan question/)).toBeVisible();

  timePasses();
  await expect.element(behind(/^Check failed/)).toBeVisible();

  // Put the first off: the second comes forward and the first stands behind it, dashed.
  await userEvent.keyboard("l");
  await expect.element(call(/^Check failed/)).toBeVisible();
  await expect.element(behind(/^Plan question/)).toBeVisible();
  expect(behind(/^Plan question/).element().hasAttribute("data-deferred")).toBe(true);

  // Answer the second; the one put off stays at the edge with nothing in front, and w brings it back.
  await userEvent.keyboard("1");
  await userEvent.keyboard("{Enter}");
  await expect.element(call(/^Check failed/)).not.toBeInTheDocument();
  await expect.element(behind(/^Plan question/)).toBeVisible();
  await userEvent.keyboard("w");
  await expect.element(call(/^Plan question/)).toBeVisible();
});

test("the standing answers b and g are on an agent's question, and not on a call Fleet raises about a state", async () => {
  await dashboard();
  timePasses();
  timePasses();
  timePasses();
  // A Plan question: an agent asked it, so both stand under the numbered answers.
  await expect.element(call(/^Plan question/)).toBeVisible();
  await expect.element(radio(/Make the best decision/)).toBeVisible();
  await expect.element(radio(/Just get it done/)).toBeVisible();
  await userEvent.keyboard("b");
  await expect.element(radio(/Make the best decision/)).toHaveAttribute("aria-checked", "true");
  await userEvent.keyboard("{Enter}");

  // A failed Check: Fleet raised it, so it keeps its own act and nothing more.
  await expect.element(call(/^Check failed/)).toBeVisible();
  await expect.element(radio(/Retry/)).toBeVisible();
  expect(radio(/Make the best decision/).query()).toBeNull();
  expect(radio(/Just get it done/).query()).toBeNull();

  // A Session's ask: both stand, and say what they hand the agent.
  await userEvent.keyboard("l");
  await expect.element(call(/^Session/)).toBeVisible();
  await expect.element(radio(/Make the best decision/)).toBeVisible();
  await userEvent.keyboard("g");
  await expect.element(radio(/Just get it done/)).toHaveAttribute("aria-checked", "true");
});

test("the grid and the map offer the same acts: a press picks, x asks to kill, Enter opens, and the choice is kept", async () => {
  await dashboard();
  await page.getByRole("tab", { name: "Active" }).click();
  const star = () => page.getByRole("option", { name: /^Debounce the Job Board/ });

  // The grid.
  await expect.element(page.getByRole("listbox", { name: "Tiles" })).toBeVisible();
  await star().click();
  await expect.element(star()).toHaveAttribute("aria-selected", "true");
  await userEvent.keyboard("x");
  await expect.element(page.getByRole("dialog")).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();

  // The map: the same star, the same acts.
  await userEvent.keyboard("m");
  await expect.element(page.getByRole("listbox", { name: "Map" })).toBeVisible();
  expect(localStorage.getItem(VIEW)).toBe("map");
  await star().click();
  await expect.element(star()).toHaveAttribute("aria-selected", "true");
  await userEvent.keyboard("x");
  await expect.element(page.getByRole("dialog")).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => page.getByRole("tab", { name: "Active" }).query()).toBeNull();
});

test("the call deck comes forward over the map too", async () => {
  await dashboard();
  await page.getByRole("tab", { name: "Active" }).click();
  await userEvent.keyboard("m");
  await expect.element(page.getByRole("listbox", { name: "Map" })).toBeVisible();
  timePasses();
  await expect.element(call(/^Plan question/)).toBeVisible();
});

/** Put calls off until the one named is in front: the stack comes round to it. */
async function reach(kind: RegExp): Promise<void> {
  for (let tries = 0; tries < 20 && page.getByRole("region", { name: kind }).query() === null; tries += 1) {
    const front = document.querySelector(".armada-callcard")?.getAttribute("aria-label");
    await userEvent.keyboard("l");
    await expect.poll(() => document.querySelector(".armada-callcard")?.getAttribute("aria-label") !== front, { timeout: 3000 }).toBe(true);
  }
  await expect.element(page.getByRole("region", { name: kind })).toBeVisible();
}

test("a pull request that a Session or a Job already owns sends him to the owner, and one nobody owns offers a Drone", async () => {
  localStorage.removeItem(FILTER);
  mount("dashboard-needs-you");
  await onScreen();
  await userEvent.keyboard("{Escape}");

  // Opened by a Job: that Job is the one to open.
  await reach(/^Pull request: #1819/);
  await expect.element(radio(/Open the Job/)).toBeVisible();
  expect(radio(/Send a Drone/).query()).toBeNull();

  // Held by a Session: named, with the Session to open, and no Drone.
  await reach(/^Pull request: #1822/);
  await expect.element(page.getByRole("group", { name: "Owner" })).toHaveTextContent("Session · Armada Pocket");
  await expect.element(radio(/Open the Session/)).toBeVisible();
  await expect.element(radio(/Poke/)).toBeVisible();
  expect(radio(/Send a Drone/).query()).toBeNull();

  // A person's, with nobody on it: a Drone stays, and so does a way to attach it to who is on it.
  await reach(/^Pull request: #1823/);
  await expect.element(radio(/Send a Drone/)).toBeVisible();
  await expect.element(radio(/Attach/)).toBeVisible();
  expect(page.getByRole("group", { name: "Owner" }).query()).toBeNull();

  // Attach lists the live Jobs and Sessions, a filter narrows them, and Enter attaches it: the card is owned.
  await userEvent.keyboard("2");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("listbox", { name: "Jobs and Sessions" })).toBeVisible();
  await userEvent.type(page.getByRole("textbox", { name: "Attach to" }), "Migration notes");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("group", { name: "Owner" })).toHaveTextContent(/Session · Migration notes/);
  await expect.element(radio(/Open the Session/)).toBeVisible();
  expect(radio(/Send a Drone/).query()).toBeNull();
});

test("main's red goes to the Job that broke it: 1 opens it, 2 pokes it", async () => {
  localStorage.removeItem(FILTER);
  mount("dashboard-needs-you");
  await onScreen();
  await userEvent.keyboard("{Escape}");
  await reach(/^Main/);
  await expect.element(radio(/Open the Job/)).toBeVisible();
  expect(radio(/Hand to a Job/).query()).toBeNull();
  await expect.element(radio(/Poke/)).toBeVisible();
  await userEvent.keyboard("1");
  await userEvent.keyboard("{Enter}");
  // The Job opens: the Dashboard's filters are no longer drawn.
  await expect.poll(() => page.getByRole("tab", { name: "Your move" }).query()).toBeNull();
});

test("a Session's own question shows its options and their lines, never Allow or Deny, and takes a reply in words", async () => {
  localStorage.removeItem(FILTER);
  mount("dashboard-needs-you");
  await onScreen();
  await userEvent.keyboard("{Escape}");
  await reach(/^Session question/);
  // The question whole, down to its last sentence.
  await expect.element(page.getByText(/Nothing you see changes either way. What should happen to it\?/)).toBeVisible();
  // Its own options, the recommended one marked, each with the agent's line.
  await expect.element(radio(/File an issue/)).toBeVisible();
  await expect.element(radio(/File an issue/)).toHaveTextContent("Recommended");
  await expect.element(radio(/Fix it now/)).toHaveTextContent("Change the restart script");
  await expect.element(radio(/Drop it/)).toBeVisible();
  for (const name of [/Allow/, /Deny/, /Refuse/]) expect(radio(name).query()).toBeNull();
  // Type something is a reply on the card, and it lands in the Session: the call is answered.
  await userEvent.keyboard("t");
  await expect.element(page.getByRole("textbox", { name: "Type something" })).toHaveFocus();
  await userEvent.type(page.getByRole("textbox", { name: "Type something" }), "File it, and fix it on Friday");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("region", { name: /^Session question/ })).not.toBeInTheDocument();
});

test("a permission is the only Session call that offers Allow once and Refuse, and a walk's act is Approve", async () => {
  localStorage.removeItem(FILTER);
  mount("dashboard-needs-you");
  await onScreen();
  await userEvent.keyboard("{Escape}");
  await reach(/^Session walk/);
  await expect.element(radio(/Approve/)).toBeVisible();
  await reach(/^Session: Flaky/);
  await expect.element(radio(/Allow once/)).toBeVisible();
  await expect.element(radio(/Refuse/)).toBeVisible();
});
