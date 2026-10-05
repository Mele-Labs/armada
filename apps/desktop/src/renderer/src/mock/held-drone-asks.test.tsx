// A Drone held on a command it was not given asks on the Drone itself — its
// card and row in the task's panel, the step's panel, and the Drones tab —
// with Overview's box and its `answer_command`, not only under Needs you.

import { describe, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { ARC_JOB_ID } from "@armada/screens/src/fixtures/build/arc";
import { HELD_CALL } from "@armada/screens/src/fixtures/build/arc-executing";

import { entered, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const T5 = "Draw what is running, in four lists";

/** T5's panel, opened from its row on the Plan list. */
async function taskPanel() {
  await page.getByRole("tab", { name: /^Plan/ }).last().click();
  await page.getByRole("tab", { name: "List" }).last().click();
  await page.getByRole("listitem", { name: `T5 ${T5}` }).getByRole("button").click();
  const panel = page.getByRole("dialog", { name: T5 });
  await entered(panel);
  return panel;
}

/** The radios are inputs under the controls they style. */
function choose(name: string): void {
  const radio = page.getByRole("radio", { name }).element() as HTMLElement;
  radio.focus();
  radio.click();
}

describe("a Drone held on a command", () => {
  test("the task's Drone card asks, names the command, and its mark says it needs you", async () => {
    mount("arc/executing-held");
    const panel = await taskPanel();
    const card = panel.getByRole("group", { name: "Drone on T5" });
    await expect.element(card).toHaveTextContent("pnpm add -D reselect@5.1.1");
    await expect.element(card.getByRole("radio", { name: "Allow for this job" })).toBeInTheDocument();
    await expect.element(card).toHaveTextContent("Needs you");
    // Its row in the task's Drones carries the same mark.
    await expect
      .element(panel.getByRole("list", { name: "Drones on this task" }))
      .toHaveTextContent("Needs you");
  });

  test("answering from the card sends the call Overview sends, and the ask clears", async () => {
    const app = mount("arc/executing-held");
    const answerCommand = vi.spyOn(app.api, "answerCommand");
    const panel = await taskPanel();
    choose("Allow for this job");
    await panel.getByRole("button", { name: "Send this answer" }).click();
    await expect.poll(() => answerCommand.mock.calls.length).toBe(1);
    expect(answerCommand).toHaveBeenCalledWith(ARC_JOB_ID, HELD_CALL, "allow_for_job", undefined, undefined);
    await expect.poll(() => page.getByRole("radio", { name: "Allow for this job" }).query()).toBeNull();
    await expect.element(panel.getByRole("group", { name: "Drone on T5" })).toHaveTextContent("Running");
  });

  test("the step's panel asks on the held Drone's row", async () => {
    mount("arc/executing-held");
    await page.getByRole("tab", { name: /^Workflow/ }).last().click();
    await page.getByRole("button", { name: /^Implement, /i }).last().click();
    const drones = page.getByRole("region", { name: "Drones" });
    await expect.element(drones).toHaveTextContent("Needs you");
    await expect.element(drones).toHaveTextContent("pnpm add -D reselect@5.1.1");
  });

  test("the Drones tab marks the held row, and its sheet asks", async () => {
    mount("arc/executing-held");
    await page.getByRole("tab", { name: /^Drones/ }).last().click();
    const table = page.getByRole("region", { name: "Drones on this Job" });
    await expect.element(table).toHaveTextContent("Needs you");
    await table.getByText("Drone on T5").first().click();
    await expect.element(page.getByText("pnpm add -D reselect@5.1.1")).toBeVisible();
  });

  test("a Drone that is not held shows no ask and is running", async () => {
    mount("arc/executing-sequential");
    const panel = await taskPanel();
    expect(panel.getByRole("radio", { name: "Allow for this job" }).query()).toBeNull();
    await expect.element(panel.getByRole("group", { name: "Drone on T5" })).not.toHaveTextContent("Needs you");
  });
});
