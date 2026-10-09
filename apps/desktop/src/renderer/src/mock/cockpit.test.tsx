// The Dashboard's one panel, through `App` on the `dashboard-cockpit` mock Fleet: its filters and the
// keys that step them, the call that comes forward over it and the deck behind, the standing
// answers, and the grid and the map offering the same acts.

import { beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetDismissals } from "../cockpit/dismissed";
import { s205DashboardNeedsYou } from "./scenarios/dashboard-needs-you";
import type { Scenario } from "./moment";
import { motion, mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();
beforeEach(forgetDismissals);

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

test("the merge line is a footer of dots, Fleet's landings nearest main, then the queue, then the open pull requests", async () => {
  await motion();
  await dashboard();
  const dots = [...document.querySelectorAll<HTMLElement>("footer.armada-view__horizon .armada-view__dot")];
  const names = dots.map((one) => one.getAttribute("aria-label")!);
  // Nearest main first in the document, which the belt draws from the right.
  expect(names.slice(-5).map((one) => one.split(" ")[0])).toEqual(["#1890", "#1891", "#1893", "#1894", "#1895"]);
  expect(names[0]).toBe("docs/wire-lock-signed · Preparing to land");
  expect(names.slice(0, -5).every((one) => !one.startsWith("#"))).toBe(true);
  expect(dots.slice(-5).map((one) => one.hasAttribute("data-queued"))).toEqual([true, true, false, false, false]);
  expect(names.at(-5)).toContain("Job: Debounce the Job Board's resize handler");
  // No text on the band, and nothing of it in the top bar.
  expect(dots.every((one) => one.textContent === "" || one.querySelector(".armada-tooltip__bubble") !== null)).toBe(true);
  expect(document.querySelector(".armada-view__band .armada-view__dot")).toBeNull();
  expect(document.querySelector(".armada-view__chip")).toBeNull();
  const foot = document.querySelector<HTMLElement>("footer.armada-view__horizon")!;
  expect(foot.compareDocumentPosition(document.querySelector(".armada-view__stage")!) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
  // The card is on the dot: number and title, branch, state, place and owner.
  const card = dots.at(-5)!.querySelector(".armada-horizon-card")!;
  expect(card.textContent).toContain("#1890");
  expect(card.textContent).toContain("Gate policy on every run");
  expect(card.textContent).toContain("fleet/gate-policy-every-run");
  expect(card.textContent).toContain("Place 1");
  expect(card.textContent).toContain("Debounce the Job Board's resize handler");
  // The loop is on the dot's own face and never on the trigger the card opens inside, or the card would pulse with it.
  const running = dots.find((one) => one.getAttribute("data-state") === "running")!;
  expect(getComputedStyle(running).animationName).toBe("none");
  expect(getComputedStyle(running, "::before").animationName).toBe("armada-dot-breathe");
  expect(getComputedStyle(running.querySelector(".armada-horizon-card")!).opacity).toBe("1");
  // A pull request's checks failing turns its dot red.
  const state = () => document.querySelector('.armada-view__dot[aria-label^="#1894"]')?.getAttribute("data-state");
  expect(state()).toBe("running");
  timePasses();
  timePasses();
  timePasses();
  timePasses();
  await expect.poll(state).toBe("failing");
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
  const app = mount("dashboard-needs-you");
  const claim = vi.spyOn(app.api, "claimPullRequest");
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

  // Attach lists the live Sessions (a Job is not offered until the route takes one), a filter narrows
  // them, and Enter claims the pull request for it through Fleet: the card is owned.
  await userEvent.keyboard("2");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("listbox", { name: "Sessions" })).toBeVisible();
  expect(page.getByRole("listbox", { name: "Sessions" }).getByRole("img", { name: "Job" }).query()).toBeNull();
  await userEvent.type(page.getByRole("textbox", { name: "Attach to" }), "Migration notes");
  await userEvent.keyboard("{Enter}");
  expect(claim).toHaveBeenCalledWith({ number: 1823, session_id: "s10" });
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

test("a Session's answer goes to Fleet as an index, its words as text, and a mode alone as the mode", async () => {
  localStorage.removeItem(FILTER);
  const app = mount("dashboard-needs-you");
  const answer = vi.spyOn(app.api, "answerWaiting");
  await onScreen();
  await userEvent.keyboard("{Escape}");

  // The second of its options, by number.
  await reach(/^Session question/);
  await userEvent.keyboard("2");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("region", { name: /^Session question/ })).not.toBeInTheDocument();
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s14", item_id: "ask:q14:0", choice: 1 });

  // A walk needs no choice.
  await reach(/^Session walk/);
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => answer.mock.calls.length).toBe(2);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s15", item_id: "walk:https://git.example/pairing" });

  // The permission handed to the agent: g sends the mode and nothing else.
  await reach(/^Session: Flaky/);
  await userEvent.keyboard("g");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => answer.mock.calls.length).toBe(3);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s9", item_id: "perm:s9", mode: "quick" });
});

test("an item nothing holds clears the card quietly, and any other refusal stays on it with Fleet's words", async () => {
  localStorage.removeItem(FILTER);
  const app = mount("dashboard-needs-you");
  const refuse = (code: string, message: string) => ({ ok: false as const, outcome: { ok: false as const, why: "refused" as const, error: { code, message, run_id: "", fields: {}, chain: [] as string[] } } });
  const answer = vi.spyOn(app.api, "answerWaiting");
  await onScreen();
  await userEvent.keyboard("{Escape}");

  answer.mockResolvedValueOnce(refuse("fleet.session_waiting_empty", "an answer needs a choice, words or a mode"));
  await reach(/^Session question/);
  await userEvent.keyboard("1");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByText("an answer needs a choice, words or a mode").first()).toBeVisible();
  await expect.element(page.getByRole("region", { name: /^Session question/ })).toBeVisible();

  answer.mockResolvedValueOnce(refuse("fleet.session_waiting_unheld", "nothing is waiting under that item."));
  await userEvent.keyboard("1");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("region", { name: /^Session question/ })).not.toBeInTheDocument();
  expect(page.getByText("nothing is waiting under that item.").query()).toBeNull();
});

test("a pull request Fleet will not hand over stays unowned, and the toast names who holds it", async () => {
  localStorage.removeItem(FILTER);
  const app = mount("dashboard-needs-you");
  vi.spyOn(app.api, "claimPullRequest").mockResolvedValueOnce({
    ok: false,
    outcome: { ok: false, why: "refused", error: { code: "fleet.pull_request_not_claimable", message: "pull request #1823 is held by session Release script", run_id: "", fields: {}, chain: [] as string[] } },
  });
  await onScreen();
  await userEvent.keyboard("{Escape}");
  await reach(/^Pull request: #1823/);
  await userEvent.keyboard("2");
  await userEvent.keyboard("{Enter}");
  await userEvent.type(page.getByRole("textbox", { name: "Attach to" }), "Migration notes");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByText("pull request #1823 is held by session Release script").first()).toBeVisible();
  expect(page.getByRole("group", { name: "Owner" }).query()).toBeNull();
});

test("a dismissed call stays gone across a remount, and the same pull request failing anew shows again", async () => {
  localStorage.removeItem(FILTER);
  // The scenario with a hand on its Fleet, so the pull request's checks can pass and then fail again.
  let hand: Parameters<NonNullable<Scenario["behaves"]>>[0] | undefined;
  const scenario: Scenario = { ...s205DashboardNeedsYou, behaves: (fleet) => ((hand = fleet), {}) };
  const checks = (ci: string) => {
    const lines = (hand!.state().mergeLines?.lines ?? []).map((line) => ({
      ...line,
      hub: { ...line.hub!, pull_requests: line.hub!.pull_requests!.map((pull) => (pull.number === 1823 ? { ...pull, ci } : pull)) },
    }));
    hand!.publish({ mergeLines: { lines } });
  };

  const first = mount(scenario);
  await onScreen();
  await userEvent.keyboard("{Escape}");
  await reach(/^Pull request: #1823/);
  await expect.element(page.getByRole("button", { name: /^Dismiss/ })).toHaveTextContent("d");
  await userEvent.keyboard("d");
  await expect.element(page.getByRole("region", { name: /^Pull request: #1823/ })).not.toBeInTheDocument();
  expect(JSON.parse(localStorage.getItem("armada.bridge.dismissed-calls") ?? "[]")).toEqual([expect.stringContaining("pull:1823")]);
  first.unmount();
  document.querySelectorAll("#root").forEach((one) => one.remove());

  // The window opens again: the other pull requests are called, this one is not.
  mount(scenario);
  await onScreen();
  await userEvent.keyboard("{Escape}");
  await reach(/^Pull request: #1822/);
  await userEvent.keyboard("l");
  await expect.poll(() => page.getByRole("region", { name: /^Pull request: #1823/ }).query()).toBeNull();
  await expect.poll(() => page.getByRole("button", { name: /^Pull request: #1823/ }).query()).toBeNull();

  // Its checks pass, which lets go of the dismissal, and when they fail again it is called again.
  checks("passed");
  await expect.poll(() => localStorage.getItem("armada.bridge.dismissed-calls")).toBe("[]");
  checks("failed");
  // Wait for it to be called, in front or behind, before cycling the deck to it.
  await expect
    .poll(() => page.getByRole("region", { name: /^Pull request: #1823/ }).query() !== null || page.getByRole("button", { name: /^Pull request: #1823/ }).query() !== null, { timeout: 5000 })
    .toBe(true);
  await reach(/^Pull request: #1823/);
});

test("d on a Session's call dismisses that item on Fleet and the card goes", async () => {
  localStorage.removeItem(FILTER);
  const app = mount("dashboard-needs-you");
  const dismiss = vi.spyOn(app.api, "dismissWaiting");
  await onScreen();
  await userEvent.keyboard("{Escape}");
  await reach(/^Session walk/);
  await userEvent.keyboard("d");
  await expect.element(page.getByRole("region", { name: /^Session walk/ })).not.toBeInTheDocument();
  expect(dismiss).toHaveBeenCalledWith({ session_id: "s15", item_id: "walk:https://git.example/pairing" });
});
