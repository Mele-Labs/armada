// The band's two ways to a Job press `fix_main`: Dispatch carries the brief as it stands in the
// dialog, Send back carries the Job chosen, and the Jobs offered are the Board's rows at their
// review or over, the pull request's own Job first.

import type { FixMain, Outcome, SendSessionMessage } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { DEBOUNCE } from "./main-red-hub";
import { mainRed, redHeldElsewhere, sessionRed } from "./main-red-hub";
import { scenarioNamed, type Scenario } from "./scenario";
import { mount, onScreen, toPageByName, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The scenario standing at one of its moments, with `fixMain` answered into `sent`. */
function standingAt(moment: number, sent: FixMain[]): Scenario {
  const { later } = mainRed(repository().root);
  const base = scenarioNamed("main-red-hub")!;
  return {
    ...base,
    state: { ...base.state, ...later[moment - 1]! },
    later: [],
    behaves: () => ({
      fixMain: async (fix) => {
        sent.push(fix);
        return { ok: true };
      },
    }),
  };
}

const band = () => page.getByRole("region", { name: "Merge line" }).getByRole("status", { name: "Main is red" });

test("Dispatch a new Job sends the brief as edited, and no Job", async () => {
  const sent: FixMain[] = [];
  mount(standingAt(6, sent));
  await onScreen();
  // The merge line has its own page; the Dashboard reads main's red as one of its calls.
  await toPageByName("Merge line");

  await band().getByRole("button", { name: "Dispatch a new Job" }).click();
  const dialog = page.getByRole("dialog", { name: "Dispatch a Job to fix main" });
  await dialog.getByRole("textbox", { name: "Brief" }).fill("components_test fails on main.\nTest: Theme tokens");
  await dialog.getByRole("button", { name: "Dispatch", exact: true }).click();

  expect(sent).toEqual([{ root: repository().root, brief: "components_test fails on main.\nTest: Theme tokens" }]);
});

test("Dispatch a new Session opens the Dispatch panel on a Session, holding the brief", async () => {
  const sent: FixMain[] = [];
  mount(standingAt(6, sent));
  await onScreen();
  await toPageByName("Merge line");

  await band().getByRole("button", { name: "Dispatch a new Session" }).click();
  const panel = page.getByRole("region", { name: "Dispatch", exact: true });
  await expect.element(panel.getByRole("tab", { name: "Session" })).toHaveAttribute("aria-selected", "true");
  await expect.element(panel.getByRole("textbox", { name: "Request" })).toHaveValue("components_test fails on main.\nTest: Theme tokens > the dark ground keeps its contrast\nMerged in #1815 (nick/theme-tokens).");
  expect(sent).toEqual([]);
});

test("Send back lists the Board's Jobs at their review or over, and sends the one chosen", async () => {
  const sent: FixMain[] = [];
  mount(standingAt(3, sent));
  await onScreen();
  // The merge line has its own page; the Dashboard reads main's red as one of its calls.
  await toPageByName("Merge line");

  await band().getByRole("button", { name: "Send back", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Send the work back" });
  await expect.element(dialog.getByRole("radio", { name: /Debounce the Job Board/ })).toBeVisible();
  // A Job still running is not one the work can go back to.
  expect(dialog.getByRole("radio", { name: /Store a pause marker/ }).query()).toBeNull();
  await dialog.getByRole("radio", { name: /Debounce the Job Board/ }).click({ force: true });
  await dialog.getByRole("button", { name: "Send back", exact: true }).click();

  expect(sent).toEqual([{ root: repository().root, job: DEBOUNCE.job.id }]);
});

test("Send back to a Session posts its message to /sessions/message, not fix_main", async () => {
  const fixed: FixMain[] = [];
  const messages: SendSessionMessage[] = [];
  const { later } = sessionRed(repository().root);
  const base = scenarioNamed("main-red-session")!;
  mount({
    ...base,
    state: { ...base.state, ...later[0]! },
    later: [],
    behaves: () => ({
      fixMain: async (fix) => {
        fixed.push(fix);
        return { ok: true };
      },
      sendSessionMessage: async (send) => {
        messages.push(send);
        return { ok: true, value: {} as never };
      },
    }),
  });
  await onScreen();
  await toPageByName("Merge line");

  await band().getByRole("button", { name: "Send back", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Send the work back" });
  await dialog.getByRole("textbox", { name: "Message" }).fill("screens_test fails on main after #1830.");
  await dialog.getByRole("button", { name: "Send back", exact: true }).click();

  expect(messages).toEqual([{ session_id: "session-c279e883", text: "screens_test fails on main after #1830." }]);
  expect(fixed).toEqual([]);
});

/** The red claimed on another machine, standing at one of its moments: 1 is a stale claim, 4 is red with nobody holding it. */
function claimedAt(moment: number, answer: (fix: FixMain) => Outcome, sent: FixMain[]): Scenario {
  const { later } = redHeldElsewhere(repository().root);
  const base = scenarioNamed("red-held-elsewhere")!;
  return {
    ...base,
    state: { ...base.state, ...later[moment - 1]! },
    later: [],
    behaves: () => ({
      fixMain: async (fix) => {
        sent.push(fix);
        return answer(fix);
      },
    }),
  };
}

const refusal = (code: string, message: string): Outcome => ({ ok: false, why: "refused", error: { code, message, run_id: "", fields: {}, chain: [] } });

test("Take over asks once, then sends fix_main with take_over and no brief", async () => {
  const sent: FixMain[] = [];
  mount(claimedAt(1, () => ({ ok: true }), sent));
  await onScreen();
  await page.getByRole("button", { name: "Merge line", exact: true }).click();

  await band().getByRole("button", { name: "Take over" }).click();
  expect(sent).toEqual([]);
  const ask = page.getByRole("dialog", { name: "Take over from nick-mbp" });
  await expect.element(ask.getByRole("img", { name: /^Claimed 3h ago, stale$/ })).toBeVisible();
  await ask.getByRole("button", { name: "Take over", exact: true }).click();

  expect(sent).toEqual([{ root: repository().root, take_over: true }]);
});

test("a claim the store could not make comes back to the band, with Try again, and is no toast", async () => {
  const sent: FixMain[] = [];
  mount(claimedAt(4, () => refusal("fleet.main_claim_not_made", "the red was not taken. store down"), sent));
  await onScreen();
  await page.getByRole("button", { name: "Merge line", exact: true }).click();

  await band().getByRole("button", { name: "Dispatch a new Job" }).click();
  await page.getByRole("dialog", { name: "Dispatch a Job to fix main" }).getByRole("button", { name: "Dispatch", exact: true }).click();
  await expect.element(band().getByText("Not made, store unreachable")).toBeVisible();
  expect(page.getByText("the red was not taken. store down").query()).toBeNull();

  await band().getByRole("button", { name: "Try again" }).click();
  expect(sent).toHaveLength(2);
  expect(sent[1]).toEqual(sent[0]);
});

test("a red another machine holds, not yet read into the hub, comes back to the band the same way", async () => {
  const sent: FixMain[] = [];
  mount(claimedAt(4, () => refusal("fleet.main_taken_elsewhere", "Job c41e90a7 on work-mac-mini has held this red for 2m"), sent));
  await onScreen();
  await page.getByRole("button", { name: "Merge line", exact: true }).click();

  await band().getByRole("button", { name: "Dispatch a new Job" }).click();
  await page.getByRole("dialog", { name: "Dispatch a Job to fix main" }).getByRole("button", { name: "Dispatch", exact: true }).click();
  await expect.element(band().getByText("Held on another machine")).toBeVisible();
  expect(page.getByText("Job c41e90a7 on work-mac-mini has held this red for 2m").query()).toBeNull();
});

test("every other refusal of a fix is still a toast, and draws no row in the band", async () => {
  const sent: FixMain[] = [];
  mount(claimedAt(4, () => refusal("fleet.cannot_fix_main", "main is not red"), sent));
  await onScreen();
  await page.getByRole("button", { name: "Merge line", exact: true }).click();

  await band().getByRole("button", { name: "Dispatch a new Job" }).click();
  await page.getByRole("dialog", { name: "Dispatch a Job to fix main" }).getByRole("button", { name: "Dispatch", exact: true }).click();
  await expect.element(page.getByText("main is not red").first()).toBeVisible();
  expect(band().getByText("Try again").query()).toBeNull();
});
