// The Dashboard's one panel, through `App` on the `dashboard-cockpit` mock Fleet: its filters and the
// keys that step them, the call that arrives in its tray and the card it opens as, the standing
// answers, the grid and the timeline, and the Bays filter's metro lines.

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
  await expect.element(page.getByRole("tab", { name: "Active" })).toHaveAttribute("aria-selected", "true");
}

const call = (kind: RegExp) => page.getByRole("region", { name: kind });
const behind = (kind: RegExp) => page.getByRole("button", { name: kind });
const selected = (name: string) => expect.element(page.getByRole("tab", { name })).toHaveAttribute("aria-selected", "true");
const radio = (name: RegExp) => page.getByRole("radio", { name });
/** An act runs on one press, so it is a button on the rail, and an option is a radio. */
const act = (name: RegExp) => page.getByRole("button", { name });
/** The calls waiting along the foot of the panel, each a button naming its kind and title. */
const tray = () => document.querySelector(".armada-tray");

test("Active with nothing waiting draws the work at hand with nothing in front of it, and the dispatch bar above it is the one line", async () => {
  localStorage.removeItem(FILTER);
  mount("dashboard-cockpit");
  await expect.element(page.getByRole("listbox", { name: "Tiles" })).toBeVisible();
  expect(document.querySelectorAll('[role="listbox"][aria-label="Tiles"] [role="option"]').length).toBeGreaterThan(0);
  expect(document.querySelector(".armada-callcard")).toBeNull();
  expect(tray()).toBeNull();
  // The bar is a line at rest: its frame is not grown, so nothing of the panel is drawn under it.
  await expect.element(page.getByRole("textbox", { name: "Request" })).toBeVisible();
  expect(page.getByRole("button", { name: "Dispatch", exact: true }).query()).toBeNull();
});

test("[ and ] step the filters round, Option and a digit jumps, and the filter is kept", async () => {
  await dashboard();
  await userEvent.keyboard("]");
  await selected("Done");
  await userEvent.keyboard("]");
  await selected("Bays");
  await userEvent.keyboard("]");
  await selected("Active");
  // `[[` is the bracket itself, in user-event's own syntax.
  await userEvent.keyboard("[[");
  await selected("Bays");
  await userEvent.keyboard("{Alt>}2{/Alt}");
  await selected("Done");
  await userEvent.keyboard("{Alt>}1{/Alt}");
  await selected("Active");
  expect(localStorage.getItem(FILTER)).toBe("running");
  expect(page.getByRole("tab", { name: "Your move" }).query()).toBeNull();
  for (const name of ["Active", "Done", "Bays"]) {
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

test("n opens the title bar's one bar from the panel, ready for the work", async () => {
  await dashboard();
  await userEvent.keyboard("n");
  await expect.element(page.getByRole("combobox")).toHaveFocus();
  await expect.element(page.getByRole("combobox")).toHaveAttribute("placeholder", "Describe the work, or paste a link to a ticket.");
});

test("a call arrives in the tray on every filter and covers nothing, w opens it, and the next arrives after", async () => {
  await dashboard();
  expect(behind(/^Plan question/).query()).toBeNull();
  timePasses();
  // It is in the tray with its key printed on it, and the work is still there to press.
  await expect.element(behind(/^Plan question/)).toBeVisible();
  await expect.element(behind(/^Plan question/)).toHaveTextContent("w");
  expect(call(/^Plan question/).query()).toBeNull();
  await expect.element(page.getByRole("listbox", { name: "Tiles" })).toBeVisible();
  // Another filter keeps it in the tray, and no tab carries a dot for it.
  await page.getByRole("tab", { name: "Done" }).click();
  await expect.element(behind(/^Plan question/)).toBeVisible();
  expect(document.querySelector('[role="tab"] [role="img"]')).toBeNull();
  await page.getByRole("tab", { name: "Active" }).click();

  timePasses();
  await expect.element(behind(/^Check failed/)).toBeVisible();
  expect(call(/^Check failed/).query()).toBeNull();

  // w opens the nearest; put it off and the card goes back to the tray, dashed, and nothing opens by itself.
  await userEvent.keyboard("w");
  await expect.element(call(/^Plan question/)).toBeVisible();
  await userEvent.keyboard("l");
  await expect.poll(() => call(/^Plan question/).query()).toBeNull();
  expect(call(/^Check failed/).query()).toBeNull();
  await expect.element(behind(/^Plan question/)).toBeVisible();
  expect(behind(/^Plan question/).element().hasAttribute("data-deferred")).toBe(true);
  expect(behind(/^Check failed/).element().hasAttribute("data-deferred")).toBe(false);

  // The next opens on w. Answer it, which is an act and runs on its number; the one put off stays in the tray, and w opens it.
  await userEvent.keyboard("w");
  await expect.element(call(/^Check failed/)).toBeVisible();
  await userEvent.keyboard("1");
  await expect.poll(() => call(/^Check failed/).query()).toBeNull();
  expect(call(/^Plan question/).query()).toBeNull();
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
  await userEvent.keyboard("w");
  await expect.element(call(/^Plan question/)).toBeVisible();
  await expect.element(radio(/Make the best decision/)).toBeVisible();
  await expect.element(radio(/Just get it done/)).toBeVisible();
  await userEvent.keyboard("b");
  await expect.element(radio(/Make the best decision/)).toHaveAttribute("aria-checked", "true");
  await userEvent.keyboard("{Enter}");

  // A failed Check: Fleet raised it, so it keeps its own act and nothing more.
  await expect.poll(() => call(/^Plan question/).query()).toBeNull();
  await userEvent.keyboard("w");
  await expect.element(call(/^Check failed/)).toBeVisible();
  await expect.element(act(/Retry/)).toBeVisible();
  expect(radio(/Make the best decision/).query()).toBeNull();
  expect(radio(/Just get it done/).query()).toBeNull();

  // A Session's ask: both stand, and say what they hand the agent.
  await userEvent.keyboard("l");
  await expect.poll(() => call(/^Check failed/).query()).toBeNull();
  await userEvent.keyboard("w");
  await expect.element(call(/^Session/)).toBeVisible();
  await expect.element(radio(/Make the best decision/)).toBeVisible();
  await userEvent.keyboard("g");
  await expect.element(radio(/Just get it done/)).toHaveAttribute("aria-checked", "true");
});

test("the grid offers its acts, and m turns it into the timeline, a choice that is kept", async () => {
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

  // The timeline takes its place, and the choice is kept.
  await userEvent.keyboard("m");
  await expect.element(page.getByRole("region", { name: "Timeline" })).toBeVisible();
  expect(page.getByRole("listbox", { name: "Tiles" }).query()).toBeNull();
  expect(localStorage.getItem(VIEW)).toBe("map");
  await userEvent.keyboard("m");
  await expect.element(page.getByRole("listbox", { name: "Tiles" })).toBeVisible();
  expect(localStorage.getItem(VIEW)).toBe("grid");
});

test("Bays is a filter of its own: each piece of work in a bay, and each waiting for one, a line through the same stations to main", async () => {
  await dashboard();
  await page.getByRole("tab", { name: "Bays" }).click();
  await selected("Bays");
  const line = (name: RegExp) => page.getByRole("button", { name });
  const stop = (name: RegExp) => page.getByRole("button", { name });
  await expect.element(line(/^Bay 01: Debounce the Job Board/)).toBeVisible();
  // A bay a Session holds is drawn under the Session's own name, not the process the pool reads.
  await expect.element(line(/^Bay 03: Flaky store test$/)).toBeVisible();
  // A pull request no bay holds is on the lines too, from Pull request on: the merge line is all here.
  await expect.element(page.getByRole("list", { name: /pull requests outside a bay$/ }).getByRole("button", { name: "Pull request: chore/bump-the-lockfile" })).toBeVisible();
  // Waiting work is a line too, at the first station.
  await expect.element(page.getByRole("list", { name: "Waiting for a bay" }).getByRole("button", { name: /^Waiting: Store a pause marker/ })).toBeVisible();
  // The stop where work is now says its place in the merge queue inside it.
  await expect.element(stop(/^Debounce the Job Board's resize handler: Merge queue/)).toHaveTextContent("1");
  await expect.element(stop(/^Pin the clock in the store tests: Merge queue/)).toHaveTextContent("2");
  // Free and closed bays draw no line, and are counted in the strip.
  await expect.element(page.getByRole("img", { name: /held, 3 free, 3 closed$/ })).toBeVisible();
  expect(document.querySelector("footer.armada-view__horizon")).toBeNull();

  // Hovering a stop opens its card; a press keeps it, and its acts go where they say.
  await stop(/^Pin the clock in the store tests: In a bay/).hover();
  await expect.element(page.getByRole("dialog", { name: "In bay 06" })).toBeVisible();
  await expect.element(page.getByRole("dialog", { name: "In bay 06" }).getByText("fix/pin-store-clock")).toBeVisible();
  await stop(/^Pin the clock in the store tests: Merge queue/).click();
  const card = page.getByRole("dialog", { name: "2nd in the merge queue" });
  await expect.element(card).toHaveAttribute("data-pinned");
  await expect.element(card.getByRole("button", { name: "Open #1891" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => document.querySelector(".armada-metro__card")).toBeNull();

  // A hovered card stays while the pointer crosses to it, so its acts can be pressed without pinning it.
  await stop(/^Debounce the Job Board's resize handler: Merge queue/).hover();
  const hovered = page.getByRole("dialog", { name: "1st in the merge queue" });
  await hovered.hover();
  await expect.element(hovered.getByRole("button", { name: "Open #1890" })).toBeVisible();
  await expect.poll(() => hovered.element().hasAttribute("data-pinned")).toBe(false);

  // Keys step down the lines and act as on the grid's tiles; a line's title opens its work.
  await userEvent.keyboard("j");
  await userEvent.keyboard("x");
  await expect.element(page.getByRole("dialog")).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await line(/^Bay 06: Pin the clock in the store tests$/).click();
  await expect.poll(() => page.getByRole("tab", { name: "Bays" }).query()).toBeNull();
});

test("a bay a hosted Session holds is released from its line: the idle one frees, the one mid-turn is refused and opens", async () => {
  await dashboard();
  await page.getByRole("tab", { name: "Bays" }).click();
  await selected("Bays");
  const confirm = (slot: string) => page.getByRole("group", { name: `Release ${slot}`, exact: true });

  await page.getByRole("button", { name: "Release bay 14" }).click();
  await expect.element(confirm("slot-14").getByText("Held by Theme token audit, on branch fix/theme-token-audit")).toBeVisible();
  await confirm("slot-14").getByRole("button", { name: "Release", exact: true }).click();
  await expect.poll(() => page.getByRole("button", { name: "Release bay 14" }).query()).toBeNull();
  expect(page.getByRole("button", { name: /^Bay 14:/ }).query()).toBeNull();

  await page.getByRole("button", { name: "Release bay 03" }).click();
  await confirm("slot-3").getByRole("button", { name: "Release", exact: true }).click();
  await expect.element(page.getByRole("alert")).toHaveTextContent("Slot not released: Flaky store test is working");
  await expect.element(page.getByRole("button", { name: "Open Session" })).toBeVisible();
});

test("a call arrives in the tray over the timeline too, covers nothing, and opens over it on w", async () => {
  await dashboard();
  await userEvent.keyboard("m");
  await expect.element(page.getByRole("region", { name: "Timeline" })).toBeVisible();
  timePasses();
  await expect.element(behind(/^Plan question/)).toBeVisible();
  expect(call(/^Plan question/).query()).toBeNull();
  await userEvent.keyboard("w");
  await expect.element(call(/^Plan question/)).toBeVisible();
});

/** Open the call named: w opens the nearest in the tray, and l puts one off so the next is nearest, until it comes round. */
async function reach(kind: RegExp): Promise<void> {
  for (let tries = 0; tries < 40 && page.getByRole("region", { name: kind }).query() === null; tries += 1) {
    const front = document.querySelector(".armada-callcard")?.getAttribute("aria-label");
    await userEvent.keyboard(front === undefined ? "w" : "l");
    await expect.poll(() => document.querySelector(".armada-callcard")?.getAttribute("aria-label") !== front, { timeout: 3000 }).toBe(true);
  }
  await expect.element(page.getByRole("region", { name: kind })).toBeVisible();
}

test("a pull request that a Session or a Job already owns sends him to the owner, and one nobody owns offers a Drone", async () => {
  localStorage.removeItem(FILTER);
  const app = mount("dashboard-needs-you");
  const claim = vi.spyOn(app.api, "claimPullRequest");
  await onScreen();

  // Opened by a Job: that Job is the one to open.
  await reach(/^Pull request: #1819/);
  await expect.element(act(/Open the Job/)).toBeVisible();
  expect(act(/Send a Drone/).query()).toBeNull();

  // Held by a Session: named, with the Session to open, and no Drone.
  await reach(/^Pull request: #1822/);
  await expect.element(page.getByRole("group", { name: "Owner" })).toHaveTextContent("Session · Armada Pocket");
  await expect.element(act(/Open the Session/)).toBeVisible();
  await expect.element(act(/Poke/)).toBeVisible();
  expect(act(/Send a Drone/).query()).toBeNull();

  // A person's, with nobody on it: a Drone stays, and so does a way to attach it to who is on it.
  await reach(/^Pull request: #1823/);
  await expect.element(act(/Send a Drone/)).toBeVisible();
  await expect.element(act(/Attach/)).toBeVisible();
  expect(page.getByRole("group", { name: "Owner" }).query()).toBeNull();

  // Attach lists the live Jobs and Sessions, each with its glyph, a filter narrows them, and Enter
  // claims the pull request for the one picked through Fleet: the card is owned. Attach is an act: its number opens the list.
  await userEvent.keyboard("2");
  const listed = page.getByRole("listbox", { name: "Jobs and Sessions" });
  await expect.element(listed).toBeVisible();
  await expect.element(listed.getByRole("img", { name: "Job" }).first()).toBeVisible();
  await expect.element(listed.getByRole("img", { name: "Session" }).first()).toBeVisible();
  await userEvent.type(page.getByRole("textbox", { name: "Attach to" }), "Docs for the pairing");
  await userEvent.keyboard("{Enter}");
  expect(claim).toHaveBeenCalledWith({ number: 1823, session_id: "s15" });
  await expect.element(page.getByRole("group", { name: "Owner" })).toHaveTextContent(/Session · Docs for the pairing screen/);
  await expect.element(act(/Open the Session/)).toBeVisible();
  expect(act(/Send a Drone/).query()).toBeNull();
});

test("Attach to a Job claims the pull request with its job_id, and the card is the Job's", async () => {
  localStorage.removeItem(FILTER);
  const app = mount("dashboard-needs-you");
  const claim = vi.spyOn(app.api, "claimPullRequest");
  await onScreen();
  await reach(/^Pull request: #1823/);
  await userEvent.keyboard("2");
  await userEvent.type(page.getByRole("textbox", { name: "Attach to" }), "Split the writer");
  await expect.element(page.getByRole("listbox", { name: "Jobs and Sessions" }).getByRole("option")).toHaveLength(1);
  await userEvent.keyboard("{Enter}");
  expect(claim).toHaveBeenCalledWith({ number: 1823, job_id: "01M2C1TJ8G0073EVERYSTATE00" });
  await expect.element(page.getByRole("group", { name: "Owner" })).toHaveTextContent(/Job · Split the writer from the clock/);
  await expect.element(act(/Open the Job/)).toBeVisible();
  expect(act(/Send a Drone/).query()).toBeNull();
});

test("main's red goes to the Job that broke it: 1 opens it, 2 pokes it", async () => {
  localStorage.removeItem(FILTER);
  mount("dashboard-needs-you");
  await onScreen();
  await reach(/^Main/);
  await expect.element(act(/Open the Job/)).toBeVisible();
  expect(act(/Hand to a Job/).query()).toBeNull();
  await expect.element(act(/Poke/)).toBeVisible();
  await userEvent.keyboard("1");
  // The Job opens: the Dashboard's filters are no longer drawn.
  await expect.poll(() => page.getByRole("tab", { name: "Active" }).query()).toBeNull();
});

test("a poke on main's red says main is red and what failed, never that a pull request's checks did", async () => {
  localStorage.removeItem(FILTER);
  const app = mount("dashboard-needs-you");
  const redirect = vi.spyOn(app.api, "redirectDrone");
  await onScreen();
  await reach(/^Main/);
  await userEvent.keyboard("2");
  await expect.poll(() => redirect.mock.calls.length).toBe(1);
  const said = String(redirect.mock.calls[0]![1]);
  expect(said).toMatch(/^main is red/);
  expect(said).toContain("fixes main");
  expect(said).not.toContain("checks failed");
});

test("a Session's own question shows its options and their lines, never Allow or Deny, and takes a reply in words", async () => {
  localStorage.removeItem(FILTER);
  mount("dashboard-needs-you");
  await onScreen();
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
  // The call ends in the Session's own box, which t reaches.
  await userEvent.keyboard("t");
  await expect.element(page.getByRole("textbox", { name: "Message" })).toHaveFocus();
  await userEvent.type(page.getByRole("textbox", { name: "Message" }), "File it, and fix it on Friday");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("region", { name: /^Session question/ })).not.toBeInTheDocument();
});

test("a permission is the only Session call that offers Allow once and Refuse, and a walk's act is Approve", async () => {
  localStorage.removeItem(FILTER);
  mount("dashboard-needs-you");
  await onScreen();
  await reach(/^Session walk/);
  await expect.element(act(/Approve/)).toBeVisible();
  await reach(/^Session: Flaky/);
  await expect.element(radio(/Allow once/)).toBeVisible();
  await expect.element(radio(/Refuse/)).toBeVisible();
});

test("a walk waiting on approval offers Open walk on v, which opens its page and leaves the card, and a Session question does not", async () => {
  localStorage.removeItem(FILTER);
  mount("dashboard-needs-you");
  await onScreen();
  const window = page.getByRole("dialog", { name: "Bridge's window on Docs for the pairing screen" });

  await reach(/^Session question/);
  // The acts are on the rail beside the card, so they are found in the window and not in the card.
  expect(page.getByRole("button", { name: /Open walk/ }).query()).toBeNull();
  await userEvent.keyboard("v");
  expect(window.query()).toBeNull();

  await reach(/^Session walk/);
  const open = page.getByRole("button", { name: /Open walk/ });
  await expect.element(open).toBeVisible();
  await expect.element(open).toHaveTextContent("v");
  await expect.element(page.getByRole("button", { name: /Open Session/ })).toBeVisible();
  await userEvent.keyboard("v");
  await expect.element(window).toHaveTextContent(/git\.example/);
  await userEvent.click(page.getByRole("button", { name: "Close window" }));
  await expect.element(window).not.toBeInTheDocument();
  await userEvent.click(open);
  await expect.element(window).toBeVisible();
  // Opening it answers nothing: Approve still does.
  await expect.element(call(/^Session walk/)).toBeVisible();
});

test("a Session's answer goes to Fleet as an index, its words as text, and a mode alone as the mode", async () => {
  localStorage.removeItem(FILTER);
  const app = mount("dashboard-needs-you");
  const answer = vi.spyOn(app.api, "answerWaiting");
  await onScreen();

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
  await reach(/^Pull request: #1823/);
  await userEvent.keyboard("2");
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
  await reach(/^Pull request: #1823/);
  await expect.element(page.getByRole("button", { name: /Dismiss/ })).toHaveTextContent("d");
  // Dismiss is for good, so it is under a cover: the first press lifts it and the second does it.
  await userEvent.keyboard("d");
  await expect.element(page.getByRole("region", { name: /^Pull request: #1823/ })).toBeVisible();
  await userEvent.keyboard("d");
  await expect.element(page.getByRole("region", { name: /^Pull request: #1823/ })).not.toBeInTheDocument();
  expect(JSON.parse(localStorage.getItem("armada.bridge.dismissed-calls") ?? "[]")).toEqual([expect.stringContaining("pull:1823")]);
  first.unmount();
  document.querySelectorAll("#root").forEach((one) => one.remove());

  // The window opens again: the other pull requests are called, this one is not.
  mount(scenario);
  await onScreen();
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
  await reach(/^Session walk/);
  await userEvent.keyboard("d");
  await userEvent.keyboard("d");
  await expect.element(page.getByRole("region", { name: /^Session walk/ })).not.toBeInTheDocument();
  expect(dismiss).toHaveBeenCalledWith({ session_id: "s15", item_id: "walk:https://git.example/pairing" });
});
