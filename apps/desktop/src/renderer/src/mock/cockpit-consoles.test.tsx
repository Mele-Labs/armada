// The call card with its acts on the Helm rail beside it, through `App` on the mock Fleet: the deck on
// Your move alone with a dot and no count, an act that runs on one press while an option is picked and
// then sent, Dismiss under its cover, a Job's box hidden until `t`, the Sleep card's acts, and a walk's
// call held back while its window is open.

import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetDismissals } from "../cockpit/dismissed";
import { s205DashboardNeedsYou } from "./scenarios/dashboard-needs-you";
import { s212CockpitConsoles } from "./scenarios/cockpit-consoles";
import type { Scenario } from "./moment";
import { mockSleep } from "./sleep";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();
beforeEach(() => {
  forgetDismissals();
  localStorage.removeItem("armada.bridge.dashboard-tab");
});
afterEach(() => vi.restoreAllMocks());

const call = (kind: RegExp) => page.getByRole("region", { name: kind });
// The acts are keycaps on the rail beside the card; the Session's own box under it has a Send of its own.
const key = (name: RegExp) => page.getByRole("complementary").getByRole("button", { name });
const tab = (name: string) => page.getByRole("tab", { name });

/** Brings the call named to the front: its strip on the rail's other side is pressed. */
async function reach(kind: RegExp): Promise<void> {
  const strip = page.getByRole("complementary").getByRole("button", { name: kind });
  await expect.poll(() => call(kind).query() !== null || strip.query() !== null).toBe(true);
  if (call(kind).query() === null) await strip.click();
  await expect.element(call(kind)).toBeVisible();
}

async function consoles() {
  const app = mount("cockpit-consoles");
  await onScreen();
  // Leaves the dispatch bar the window starts in, so the keys reach the card.
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("region", { name: /^Check failed|^Drone stuck|^Session/ }).first()).toBeVisible();
  return app;
}

test("the deck is drawn on Your move alone, and the tab carries a dot with no figure", async () => {
  await consoles();
  const dot = tab("Your move").getByRole("img", { name: "Needs you" });
  await expect.element(dot).toBeVisible();
  expect(tab("Your move").element().textContent).not.toMatch(/\d/);
  await tab("Active").click();
  expect(document.querySelector(".armada-callcard")).toBeNull();
  await expect.element(dot).toBeVisible();
  await tab("Your move").click();
  await expect.element(page.getByRole("region").first()).toBeVisible();
  expect(document.querySelector(".armada-callcard")).not.toBeNull();
});

test("an act runs on one press, by its key or its number, and leaves the card", async () => {
  const app = await consoles();
  const restart = vi.spyOn(app.api, "restartStep");
  await reach(/^Drone stuck/);
  // Retry step is the only act, so it is number 1 and there is no Send to press after it.
  await expect.element(key(/Retry step/)).toBeVisible();
  await userEvent.keyboard("1");
  await expect.poll(() => restart.mock.calls.length).toBe(1);
  expect(restart).toHaveBeenLastCalledWith(expect.any(String), undefined);
  await expect.element(call(/^Drone stuck/)).not.toBeInTheDocument();

  await reach(/^Check failed/);
  const press = vi.spyOn(app.api, "restartStep");
  await key(/Retry$/).click();
  await expect.element(key(/Retry$/)).toHaveAttribute("aria-busy", "true");
  expect(press).toHaveBeenCalledTimes(1);
});

test("an option is picked and then sent, so a pick alone sends nothing", async () => {
  const app = mount("session-errands");
  await onScreen();
  const answer = vi.spyOn(app.api, "answerWaiting");
  for (let tries = 0; tries < 8 && !document.querySelector(".armada-callcard")?.textContent?.includes("Which clock"); tries += 1) await userEvent.keyboard("l");
  await userEvent.keyboard("2");
  await expect.element(page.getByRole("radio", { name: /The real one, stopped/ })).toHaveAttribute("aria-checked", "true");
  expect(answer).not.toHaveBeenCalled();
  await key(/Send/).click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "s20", item_id: "e-ask", choice: 1 });
});

test("Dismiss is under a cover: the first press lifts it and the second dismisses", async () => {
  await consoles();
  const kept = () => JSON.parse(localStorage.getItem("armada.bridge.dismissed-calls") ?? "[]") as string[];
  // A call Fleet raised about a state: an agent's own item is removed with a reason instead, which has no cover.
  await reach(/^Drone stuck/);
  const cap = () => key(/Dismiss/).element().closest(".armada-keycap")!;
  expect(cap().getAttribute("data-cover")).toBe("down");
  await key(/Dismiss/).click();
  expect(cap().getAttribute("data-cover")).toBe("lifted");
  await expect.element(call(/^Drone stuck/)).toBeVisible();
  expect(kept()).toEqual([]);
  await key(/Dismiss/).click();
  await expect.element(call(/^Drone stuck/)).not.toBeInTheDocument();
  expect(kept()).toHaveLength(1);
});

test("a Job's box is hidden until t, and its words go to the Drone, or start the step again where it has none", async () => {
  const app = await consoles();
  const redirect = vi.spyOn(app.api, "redirectDrone");
  const restart = vi.spyOn(app.api, "restartStep");
  const box = () => page.getByRole("textbox", { name: "Message" });

  await reach(/^Drone stuck/);
  expect(box().query()).toBeNull();
  await userEvent.keyboard("t");
  await expect.element(box()).toHaveFocus();
  await userEvent.type(box(), "Stop the loop and run the test once\n");
  await expect.poll(() => redirect.mock.calls.length).toBe(1);
  expect(redirect).toHaveBeenLastCalledWith(expect.any(String), "Stop the loop and run the test once", undefined);
  expect(restart).not.toHaveBeenCalled();

  await reach(/^Check failed/);
  expect(box().query()).toBeNull();
  await userEvent.keyboard("t");
  await userEvent.type(box(), "Start from the failing case\n");
  await expect.poll(() => restart.mock.calls.length).toBe(1);
  expect(restart).toHaveBeenLastCalledWith(expect.any(String), "Start from the failing case");
});

test("a call Fleet serves draws only what its issue carries: Retry step and the box for a stuck Drone, nothing for an overlap", async () => {
  const [stuckJob, overlapJob] = Object.keys(s212CockpitConsoles.draft!.calls!).slice(1, 3) as [string, string];
  const calls = {
    [stuckJob]: {
      served: true as const,
      issues: [
        {
          key: "stuck:d1",
          of: "drone" as const,
          name: "Implement Drone",
          text: "No output for 12m",
          said: "Drone stuck",
          target: "d1",
          acts: [
            { glyph: "redirect" as const, said: "Redirect Drone" },
            { glyph: "retry_step" as const, said: "Retry step" },
          ],
        },
      ],
    },
    [overlapJob]: {
      served: true as const,
      issues: [{ key: "overlap:j9", of: "drone" as const, name: "Pin the clock", text: "crates/store/src/clock.rs", said: "Write scope overlaps" }],
    },
  };
  const scenario: Scenario = { ...s212CockpitConsoles, draft: { ...s212CockpitConsoles.draft!, calls } };
  const app = mount(scenario);
  await onScreen();
  await userEvent.keyboard("{Escape}");
  const redirect = vi.spyOn(app.api, "redirectDrone");
  const box = () => page.getByRole("textbox", { name: "Message" });

  await reach(/^Write scope overlaps/);
  expect(key(/Retry step/).query()).toBeNull();
  await userEvent.keyboard("t");
  expect(box().query()).toBeNull();

  await reach(/^Drone stuck/);
  await expect.element(key(/Retry step/)).toBeVisible();
  await userEvent.keyboard("t");
  await userEvent.type(box(), "Run the one test\n");
  await expect.poll(() => redirect.mock.calls.length).toBe(1);
  expect(redirect).toHaveBeenLastCalledWith(stuckJob, "Run the one test", "d1");
});

// ---- Sleep's decisions as calls ------------------------------------------------------------------

async function asleep() {
  const review = vi.spyOn(mockSleep, "review");
  const override = vi.spyOn(mockSleep, "override");
  mount("sleep-cockpit");
  await onScreen();
  // The first key in the window turns Sleep off, and the night's decisions come as calls.
  await userEvent.keyboard("l");
  await reach(/^Decided while you slept: Order the lunch/);
  return { review, override };
}

test("Agree keeps a Sleep decision as it was: it is reviewed and the call leaves", async () => {
  const { review, override } = await asleep();
  await key(/Agree/).click();
  await expect.poll(() => review.mock.calls.length).toBe(1);
  expect(review).toHaveBeenLastCalledWith("d1");
  expect(override).not.toHaveBeenCalled();
  await expect.element(call(/^Decided while you slept: Order the lunch/)).not.toBeInTheDocument();
});

test("Change on a decision made in a Session goes through that Session's own box, and is corrected and reviewed", async () => {
  const { review, override } = await asleep();
  await key(/Agree/).click();
  await reach(/^Decided while you slept: Name the branch/);
  await key(/Change/).click();
  const box = page.getByRole("textbox", { name: "Message" });
  await expect.element(box).toHaveFocus();
  await userEvent.type(box, "feat\n");
  await expect.poll(() => override.mock.calls.length).toBe(1);
  expect(override).toHaveBeenLastCalledWith("d2", "feat");
  expect(review).toHaveBeenLastCalledWith("d2");
});

test("Change on a decision made in a Job reveals a plain box, and Esc puts it away", async () => {
  const { override } = await asleep();
  await key(/Agree/).click();
  await reach(/^Decided while you slept: Name the branch/);
  await key(/Agree/).click();
  await reach(/^Decided while you slept: Shorten the reconnect wait/);
  expect(page.getByRole("textbox", { name: "Change" }).query()).toBeNull();
  await key(/Change/).click();
  await expect.element(page.getByRole("textbox", { name: "Change" })).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("textbox", { name: "Change" })).not.toBeInTheDocument();
  expect(override).not.toHaveBeenCalled();
});

test("Open the Session goes to where it was decided, and Later reviews nothing", async () => {
  const { review } = await asleep();
  await key(/Later/).click();
  await expect.element(call(/^Decided while you slept: Order the lunch/)).not.toBeInTheDocument();
  expect(review).not.toHaveBeenCalled();
  await reach(/^Decided while you slept: Order the lunch/);
  await key(/Open the Session/).click();
  await expect.element(page.getByRole("region", { name: "Thread" })).toHaveTextContent("Which size?");
  expect(review).not.toHaveBeenCalled();
});

test("Dismiss on a Sleep decision takes two presses, and reviews it for good", async () => {
  const { review } = await asleep();
  await key(/Dismiss/).click();
  expect(review).not.toHaveBeenCalled();
  await key(/Dismiss/).click();
  await expect.poll(() => review.mock.calls.length).toBe(1);
  expect(review).toHaveBeenLastCalledWith("d1");
});

// ---- A walk's call waits for its window ------------------------------------------------------------

test("a walk's call is held back while its window is open, and comes when it closes", async () => {
  const address = "https://git.example/pairing";
  let open: string[] = [address];
  const hear = new Set<(addresses: readonly string[]) => void>();
  const scenario: Scenario = {
    ...s205DashboardNeedsYou,
    behaves: () => ({
      getSessionWindows: async () => open,
      onSessionWindows: (on: (addresses: readonly string[]) => void) => (hear.add(on), () => void hear.delete(on)),
    }),
  };
  mount(scenario);
  await onScreen();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => document.querySelector(".armada-callcard") !== null).toBe(true);
  const walk = () => document.querySelector('[aria-label^="Session walk"]');
  // Every other call is dealt through, and the walk's is not among them.
  for (let tries = 0; tries < 12; tries += 1) await userEvent.keyboard("l");
  expect(walk()).toBeNull();

  open = [];
  hear.forEach((on) => on([]));
  await expect.poll(() => page.getByRole("button", { name: /^Session walk/ }).query() !== null || walk() !== null).toBe(true);
});
