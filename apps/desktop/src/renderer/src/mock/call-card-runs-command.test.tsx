// A command a Session waits on is run from its call card, where the Session works, by the runner the
// Session's ledger runs it by. Through `App` on `call-card-runs-command`: a clean exit settles the item
// and the card goes; a failure keeps the card and offers Run again, and the agent is told nothing.

import { beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetDismissals } from "../cockpit/dismissed";
import { s235CallCardRunsCommand } from "./scenarios/call-card-runs-command";
import type { SessionsStore } from "./sessions/script";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();
beforeEach(forgetDismissals);

const PASSES = "kill $(lsof -ti tcp:5788)";
const FAILS = "pnpm vitest run packages/components";
const RESTART = "scripts/preview --restart";
const SLOW = 12000;

const key = (name: RegExp) => page.getByRole("complementary").getByRole("button", { name });
const card = () => page.getByRole("region", { name: /^Job: Debug Bridge on Merge line/ });
const block = (command: string) => card().getByRole("group", { name: command });

/** Puts the cards in front of the one holding the command off, until it is the one in front. */
async function reach(command: string): Promise<void> {
  for (let tries = 0; tries < 4 && document.querySelector(".armada-callcard")?.textContent?.includes(command) !== true; tries += 1) {
    const front = document.querySelector(".armada-callcard")?.textContent;
    await userEvent.keyboard("l");
    await expect.poll(() => document.querySelector(".armada-callcard")?.textContent !== front, { timeout: 3000 }).toBe(true);
  }
  expect(document.querySelector(".armada-callcard")?.textContent).toContain(command);
}

/** The scenario with its Sessions store held, so a test sees what reached the runner and what settled an item. */
function open() {
  const held: { store?: SessionsStore } = {};
  const scenario = {
    ...s235CallCardRunsCommand,
    draft: {
      ...s235CallCardRunsCommand.draft,
      sessions: (board: Parameters<NonNullable<NonNullable<typeof s235CallCardRunsCommand.draft>["sessions"]>>[0]) => {
        const store = s235CallCardRunsCommand.draft!.sessions!(board);
        held.store = store;
        return store;
      },
    },
  };
  return { held, scenario };
}

async function show(command: string) {
  localStorage.removeItem("armada.bridge.dashboard-tab");
  const { held, scenario } = open();
  const app = mount(scenario);
  await onScreen();
  const run = vi.spyOn(held.store!, "run");
  const settle = vi.spyOn(held.store!, "answerWaiting");
  const told = vi.spyOn(app.api, "answerWaiting");
  await expect.element(card()).toBeVisible();
  await reach(command);
  return { run, settle, told };
}

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("Run on a command's card runs it in its Session, and a clean exit settles the item with Ran: exit 0 and the card goes", async () => {
  const { run, settle } = await show(PASSES);
  await expect.element(card().getByRole("code")).toHaveTextContent(PASSES);
  await key(/Run/).click();
  await expect.poll(() => run.mock.calls.length).toBe(1);
  expect(run.mock.calls[0]?.[0]).toBe("s21");
  expect(run.mock.calls[0]?.[1]).toBe(PASSES);
  // The output streams on the card and the exit shows on a mark before the card goes.
  await expect.element(card().getByRole("log", { name: "Output" })).toBeVisible();
  await expect.element(card().getByRole("img", { name: "Passed" })).toBeVisible();
  expect(settle).not.toHaveBeenCalled();
  await expect.poll(() => settle.mock.calls.length, { timeout: SLOW }).toBe(1);
  expect(settle).toHaveBeenLastCalledWith("s21", "c-stop", { text: "Ran: exit 0" });
  // The next command is in front.
  await expect.element(card().getByRole("code")).toHaveTextContent(FAILS);
  expect(run).toHaveBeenCalledTimes(1);
}, 30000);

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a failing command keeps its card, the key reads Run again, and the agent is told nothing", async () => {
  const { run, settle, told } = await show(FAILS);
  await key(/Run/).click();
  await expect.poll(() => run.mock.calls.length).toBe(1);
  expect(run.mock.calls[0]?.[1]).toBe(FAILS);
  await expect.element(card().getByRole("img", { name: "Failed" })).toBeVisible();
  await expect.element(key(/Run again/)).toBeVisible();
  // It stays: past the beat a clean exit holds the card for, nothing has settled and nothing is sent.
  await new Promise((done) => setTimeout(done, 2000));
  await expect.element(card().getByRole("img", { name: "Failed" })).toBeVisible();
  expect(settle).not.toHaveBeenCalled();
  expect(told).not.toHaveBeenCalled();
  // Run again runs it again.
  await key(/Run again/).click();
  await expect.poll(() => run.mock.calls.length).toBe(2);
  expect(run.mock.calls[1]?.[1]).toBe(FAILS);
}, 40000);

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a restart's card is one key that switches the build, with nothing to copy, run by hand or say", async () => {
  const { run } = await show(RESTART);
  await expect.element(key(/Refresh preview/)).toBeVisible();
  expect(page.getByRole("complementary").getByRole("button", { name: /Copy/ }).elements()).toHaveLength(0);
  expect(block(RESTART).getByRole("button", { name: "Run" }).elements()).toHaveLength(0);
  expect(card().getByRole("textbox").elements()).toHaveLength(0);
  await key(/Refresh preview/).click();
  await expect.poll(() => run.mock.calls.length).toBe(1);
  expect(run.mock.calls[0]?.[1]).toBe(RESTART);
}, 30000);

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("the play control on the command and the key on the rail drive one run", async () => {
  const { run } = await show(PASSES);
  await block(PASSES).getByRole("button", { name: "Run" }).click();
  await expect.poll(() => run.mock.calls.length).toBe(1);
  // While it goes the block offers Stop in Run's place, and the key's press starts nothing more.
  await expect.element(block(PASSES).getByRole("button", { name: "Stop" })).toBeVisible();
  await key(/Run/).click({ force: true }).catch(() => undefined);
  expect(run).toHaveBeenCalledTimes(1);
  await expect.element(card().getByRole("img", { name: "Passed" })).toBeVisible();
  expect(run).toHaveBeenCalledTimes(1);
}, 30000);
