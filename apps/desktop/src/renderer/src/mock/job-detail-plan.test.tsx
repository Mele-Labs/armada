// Job detail's Plan region, its settings panel, a command it waits on, and the
// dock answering that command — through `App`. Moved here from the `Screens/Job
// detail` stories' plan, settings and dock groups — #1224.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Outcome } from "@armada/protocol";
import { running, runningWaitingOnACommand } from "@armada/screens/src/fixtures/build/index";
import { JOB_ID, watchedRead } from "@armada/screens/src/fixtures/build/base";
import { WAITING_CALL } from "@armada/screens/src/fixtures/build/running";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { BridgeApi } from "../../../shared/api";
import { commandOutstanding, runningWithSettings } from "./job-detail-fixtures";
import type { FleetHandle, Scenario } from "./scenario";
import { onJob } from "./scenario";
import { mount, openHelm, unmountAfterEach } from "./testing";

unmountAfterEach();

/** What Fleet answers when it cannot be reached. */
const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** App with this Job open, over a scenario whose Fleet answers these calls its own way. */
async function opened(fixture: JobFixture, behaves?: Scenario["behaves"], whereOpen = false): Promise<BridgeApi> {
  const app = mount({ ...onJob(fixture, { whereOpen }), behaves });
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  return app.api;
}




// **The Plan region came off Overview on 29 Sep 2026**, and with it eleven
// claims made here: a plan partway done with its count and its rows, a
// dropped task reading its reason, the Working area folding what is done,
// the files each task changed, `Add task` and the four `Drop…` claims, and
// the sentence naming which step records a plan.
//
// **Overview draws a Plan card**, whose own claims are `overview-boards`' —
// what a group holds, how it runs, and where it stands. The rows and the
// acts belong to the Plan destination, which is being rebuilt; `PlanWell`
// has no renderer until it lands, so `Drop…` and `Add task` are unreachable
// and the claims about them are made nowhere rather than made wrongly here.

// The one claim that is Overview's own: a Job whose workflow records no plan
// draws a Plan card that says so, rather than a card of nothing.
test("a Job with no plan draws a Plan card that says why", async () => {
  await opened(running());
  const card = page.getByRole("region", { name: "Plan" });
  await expect.element(card).toBeVisible();
  await expect.element(card.getByRole("note")).toBeVisible();
});

test("Job settings: the sixth destination, and a choice sends this Job's id and the wire's word", async () => {
  const api = await opened(runningWithSettings());
  const setWhenBlocked = vi.spyOn(api, "setWhenBlocked");
  // The strip is the way in since 28 Sep 2026 — the header button is gone.
  expect(page.getByRole("button", { name: /^Job settings/ }).query()).toBeNull();
  await page.getByRole("tab", { name: /^Settings/ }).click();
  (page.getByRole("radio", { name: "Ask me first" }).element() as HTMLElement).click();
  await expect.poll(() => setWhenBlocked.mock.calls.length).toBe(1);
  expect(setWhenBlocked).toHaveBeenCalledWith(JOB_ID, "ask_me");
  await expect.element(page.getByText("gh issue view")).toBeVisible();
  expect(page.getByRole("button", { name: "Remove gh issue view" }).query()).toBeNull();
});

/** Choose an answer to the waiting command. The radios are inputs under the controls they style. */
function choose(name: string): void {
  const radio = page.getByRole("radio", { name }).element() as HTMLElement;
  radio.focus();
  radio.click();
}

test("waiting on a command: Allow for this job sends the call and the answer's wire name", async () => {
  const api = await opened(runningWaitingOnACommand());
  const answerCommand = vi.spyOn(api, "answerCommand");
  await expect.element(page.getByRole("radio", { name: "Allow for this job" })).toBeInTheDocument();
  choose("Allow for this job");
  await page.getByRole("button", { name: "Send this answer" }).click();
  await expect.poll(() => answerCommand.mock.calls.length).toBe(1);
  expect(answerCommand).toHaveBeenCalledWith(JOB_ID, WAITING_CALL, "allow_for_job", undefined, undefined);
});

test("always allowing picks the narrowest rule, and sends it", async () => {
  const api = await opened(runningWaitingOnACommand());
  const answerCommand = vi.spyOn(api, "answerCommand");
  await expect.element(page.getByRole("radio", { name: "Always allow in this repository" })).toBeInTheDocument();
  choose("Always allow in this repository");
  await expect.element(page.getByRole("radio", { name: "pnpm add" })).toBeChecked();
  await page.getByRole("button", { name: "Send this answer" }).click();
  await expect.poll(() => answerCommand.mock.calls.length).toBe(1);
  expect(answerCommand).toHaveBeenCalledWith(JOB_ID, WAITING_CALL, "always_allow", undefined, "pnpm add");
});

test("rejecting a command sends the reason typed", async () => {
  const api = await opened(runningWaitingOnACommand());
  const answerCommand = vi.spyOn(api, "answerCommand");
  await expect.element(page.getByRole("radio", { name: "Reject" })).toBeInTheDocument();
  choose("Reject");
  await userEvent.type(page.getByLabelText("Note (optional)"), "we are not taking that dependency");
  await page.getByRole("button", { name: "Send this answer" }).click();
  await expect.poll(() => answerCommand.mock.calls.length).toBe(1);
  expect(answerCommand).toHaveBeenCalledWith(JOB_ID, WAITING_CALL, "reject", "we are not taking that dependency", undefined);
});

test("reading a command before answering it names the model, and decides nothing", async () => {
  const explainCommand = vi.fn(async () => ({
    ok: true as const,
    explained: {
      explanation:
        "It adds reselect 5.1.1 to this repository as a development dependency and writes the lockfile. It reaches the network.",
      model: "haiku",
    },
  }));
  await opened(runningWaitingOnACommand(), () => ({ explainCommand }));
  await page.getByRole("button", { name: "Help me understand this command" }).click();
  await expect.poll(() => explainCommand.mock.calls.length).toBe(1);
  expect(explainCommand).toHaveBeenCalledWith(JOB_ID, WAITING_CALL);
  const reading = page.getByRole("status", { name: "What this command does" });
  await expect.element(reading.getByText(/adds reselect 5\.1\.1/)).toBeVisible();
  await expect.element(reading.getByText("haiku")).toBeVisible();
  await expect.element(page.getByRole("radio", { name: "Reject" })).toBeEnabled();
});

/**
 * A Job waiting on a command, listed in Helm's dock too. `answered` is what
 * Fleet does with an answer: clear the question from both, or refuse it.
 */
async function waitingInTheDock(answered: "clears" | "refused"): Promise<void> {
  const base = runningWaitingOnACommand();
  const scenario = onJob(base);
  const behaves = (fleet: FleetHandle): Partial<BridgeApi> => ({
    answerCommand: async () => {
      if (answered === "refused") return NOT_CONNECTED;
      const whole = base.watched.state === "read" ? base.watched.detail : undefined;
      fleet.publish({
        questions: [],
        ...(whole === undefined ? {} : { watched: watchedRead({ ...whole, command_waiting: undefined }) }),
      });
      return { ok: true };
    },
  });
  mount({ ...scenario, state: { ...scenario.state, questions: [commandOutstanding(base)] }, behaves });
  await expect.element(page.getByRole("region", { name: "A question from the drone" })).toBeVisible();
}

test("answered in the dock, Job detail agrees: the band and the card both go", async () => {
  await waitingInTheDock("clears");
  await openHelm();
  await page.getByRole("article").getByRole("button", { name: "Reject" }).click();
  await expect.poll(() => page.getByRole("article").query()).toBeNull();
  await expect.poll(() => page.getByRole("region", { name: "A question from the drone" }).query()).toBeNull();
});

// **Answered with the dock shut, which is how Bridge opens since #1583** — and
// it has to be shut here for a second reason worth stating: the dock draws
// over the content, so a control under it is not pressable until it is put
// away. One press does that, which is the trade the overlay was taken for.
test("answered on Job detail, the dock agrees", async () => {
  await waitingInTheDock("clears");
  const band = page.getByRole("region", { name: "A question from the drone" });
  const reject = band.getByRole("radio", { name: "Reject" }).element() as HTMLElement;
  reject.focus();
  reject.click();
  await band.getByRole("button", { name: "Send this answer" }).click();
  await expect.poll(() => page.getByRole("region", { name: "A question from the drone" }).query()).toBeNull();
  // Then Helm, which never held the question either.
  await openHelm();
  await expect.poll(() => page.getByRole("article").query()).toBeNull();
});

test("a refused answer stands on both, with the refusal said", async () => {
  await waitingInTheDock("refused");
  await openHelm();
  await page.getByRole("article").getByRole("button", { name: "Reject" }).click();
  await expect.element(page.getByRole("alert").filter({ hasText: "Fleet is not connected. Nothing was sent." })).toBeVisible();
  await expect.element(page.getByRole("region", { name: "A question from the drone" })).toBeVisible();
  await expect.element(page.getByRole("article")).toBeVisible();
});
