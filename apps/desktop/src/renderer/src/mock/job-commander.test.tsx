// A Job's Commander through `App`, over the wire's own shapes: the thread its socket carries, the
// card on `JobDetail.commander_card`, and `PlanTask.by`. The mock's agent is a script; everything it
// publishes is what a Fleet publishes, so these are the claims of #2169 on the screens side:
// the owner writes to it and reads the answer, sees which tasks it touched, and answers its card
// from Now and from the Cockpit.

import { describe, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const CARD = "01M3WJ6FGZ0CARD0000000T7C1";
const commander = () => page.getByRole("region", { name: "Commander", exact: true });
const now = () => page.getByRole("region", { name: "Now" });

// One name, so the whole file runs as one Check: `armada check app_smoke "job commander"`.
describe("job commander", () => {
  test("a Job's Commander is drawn over its canvas once its thread is open", async () => {
    mount("job-commander-woken");
    await onScreen();
    await expect.element(commander()).toBeVisible();
  });

  test("what woke it is a row of its thread, and what it did is a row and a mark", async () => {
    mount("job-commander-woken");
    await onScreen();
    timePasses();
    await expect.element(commander().getByText("Drone on T7 ended with nothing handed in")).toBeVisible();
    timePasses();
    await expect.element(commander().getByText("Turn cap 600", { exact: true })).toBeVisible();
    await expect.element(commander().getByText("T7 restarted", { exact: true })).toBeVisible();
    // A split is the two acts it is, each naming its task.
    await expect.element(commander().getByText("T7 edited", { exact: true })).toBeVisible();
    await expect.element(commander().getByText("Added Draw a Drone's permission ask on the call card", { exact: true })).toBeVisible();
    // Its hand on the tasks it touched: T7 on the Drone working it, and the new task on the canvas.
    await expect.element(now().getByRole("img", { name: "Commander" })).toBeVisible();
  });

  test("the owner's words go to this Job's Commander, and its reply comes back in the same thread", async () => {
    const app = mount("job-commander-asked");
    const asked = vi.spyOn(app.api, "askCommander");
    await onScreen();
    await page.getByRole("textbox", { name: "Tell the Commander" }).fill("why did T7's Drone stop?");
    await userEvent.click(commander().getByRole("button", { name: "Send", exact: true }).element());
    expect(asked).toHaveBeenCalledWith(expect.any(String), "why did T7's Drone stop?");
    await expect.element(commander().getByText("why did T7's Drone stop?")).toBeVisible();
    await expect.element(commander().getByText(/turn 288 of 300/)).toBeVisible();
  });

  test("a card is in the Job's Now, and the pick names the card and the option, with no prose", async () => {
    const app = mount("job-commander-asks");
    const answered = vi.spyOn(app.api, "answerCommanderCard");
    await onScreen();
    const card = now().getByRole("region", { name: /^Commander asks/ });
    await expect.element(card).toBeVisible();
    await card.getByRole("radio", { name: "Skip desktop_test on Group 3" }).click();
    await card.getByRole("button", { name: "Answer" }).click();
    expect(answered).toHaveBeenCalledWith(expect.any(String), CARD, "1");
    // Fleet wakes the Commander with it, in a row of its own, and the card is gone.
    await expect.element(commander().getByText("The owner answered your card")).toBeVisible();
    await expect.element(now().getByRole("region", { name: /^Commander asks/ })).not.toBeInTheDocument();
  });

  test("a Job with a card out is waiting on him, and the Job is in the Cockpit's Your move", async () => {
    mount("job-commander-asks");
    await onScreen();
    await expect.element(commander()).toHaveAttribute("data-state", "waiting");
    await page.getByRole("button", { name: "Cockpit", exact: true }).click();
    await expect.element(page.getByRole("region", { name: /^Commander asks/ })).toBeVisible();
  });

  test("a refused pick leaves the card to be answered again", async () => {
    const app = mount("job-commander-asks");
    vi.spyOn(app.api, "answerCommanderCard").mockResolvedValue({ ok: false, why: "not_connected" });
    await onScreen();
    const card = now().getByRole("region", { name: /^Commander asks/ });
    await card.getByRole("radio", { name: "Edit the Check in the Manifest" }).click();
    await card.getByRole("button", { name: "Answer" }).click();
    await expect.element(card.getByRole("button", { name: "Answer" })).toBeEnabled();
  });
});
