// The band's ways to a Session: a new Job is drafted holding the brief and opened, Send back
// messages the Session chosen, and Take over asks once before it drafts the Job.

import type { SendSessionMessage } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mainRed, redHeldElsewhere, sessionRed } from "./main-red-hub";
import { scenarioNamed, type Scenario } from "./scenario";
import { mount, onScreen, toPageByName, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The scenario standing at one of its moments. */
function standingAt(moment: number): Scenario {
  const { later } = mainRed(repository().root);
  const base = scenarioNamed("main-red-hub")!;
  return {
    ...base,
    state: { ...base.state, ...later[moment - 1]! },
    later: [],
  };
}

const band = () => page.getByRole("region", { name: "Merge line" }).getByRole("status", { name: "Main is red" });

/** The Job drafted for the red is open with the brief as its first message, and the sheet never came up. */
async function expectRedDrafted() {
  const job = page.getByRole("region", { name: /^Job: / });
  await expect.element(job).toBeVisible();
  await expect.element(job).toHaveTextContent("components_test fails on main.");
  await expect.element(job).toHaveTextContent("Test: Theme tokens > the dark ground keeps its contrast");
  await expect.element(job).toHaveTextContent("Merged in #1815 (nick/theme-tokens).");
  expect(page.getByRole("region", { name: "Dispatch", exact: true }).query()).toBeNull();
}

test("Dispatch a new Job drafts a Job holding the brief and opens it, with no Dispatch panel", async () => {
  mount(standingAt(5));
  await onScreen();
  await toPageByName("Merge line");

  await band().getByRole("button", { name: "Dispatch a new Job" }).click();
  await expectRedDrafted();
});

test("Send back to a Session posts its message to /sessions/message", async () => {
  const messages: SendSessionMessage[] = [];
  const { later } = sessionRed(repository().root);
  const base = scenarioNamed("main-red-session")!;
  mount({
    ...base,
    state: { ...base.state, ...later[0]! },
    later: [],
    behaves: () => ({
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
});

/** The red claimed on another machine, standing at one of its moments: 1 is a stale claim. */
function claimedAt(moment: number): Scenario {
  const { later } = redHeldElsewhere(repository().root);
  const base = scenarioNamed("red-held-elsewhere")!;
  return {
    ...base,
    state: { ...base.state, ...later[moment - 1]! },
    later: [],
  };
}

test("Take over asks once, then drafts a Job holding the brief and opens it, with no Dispatch panel", async () => {
  mount(claimedAt(1));
  await onScreen();
  await page.getByRole("button", { name: "Merge line", exact: true }).click();

  await band().getByRole("button", { name: "Take over" }).click();
  const ask = page.getByRole("dialog", { name: "Take over from nick-mbp" });
  await expect.element(ask.getByRole("group", { name: "Claimed, stale" })).toHaveTextContent("3h ago");
  await ask.getByRole("button", { name: "Take over", exact: true }).click();

  await expectRedDrafted();
});
