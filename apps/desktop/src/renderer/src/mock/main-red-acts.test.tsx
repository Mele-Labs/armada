// The band's two ways to a Job press `fix_main`: Dispatch carries the brief as it stands in the
// dialog, Send back carries the Job chosen, and the Jobs offered are the Board's rows at their
// review or over, the pull request's own Job first.

import type { FixMain, SendSessionMessage } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { DEBOUNCE } from "./main-red-hub";
import { mainRed, sessionRed } from "./main-red-hub";
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
