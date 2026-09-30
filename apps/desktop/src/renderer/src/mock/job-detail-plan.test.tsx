// Job detail's Plan region, its settings panel, a command it waits on, and the
// dock answering that command — through `App`. Moved here from the `Screens/Job
// detail` stories' plan, settings and dock groups — #1224.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Outcome } from "@armada/protocol";
import { issueLink } from "@armada/protocol";
import { ARC_JOB_ID } from "@armada/screens/src/fixtures/build/arc";
import { running, runningWaitingOnACommand } from "@armada/screens/src/fixtures/build/index";
import { JOB_ID, watchedRead } from "@armada/screens/src/fixtures/build/base";
import { WAITING_CALL } from "@armada/screens/src/fixtures/build/running";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { PLAN_PARTWAY, PLAN_WITH_A_DROPPED_TASK, withPlan } from "@armada/screens/src/fixtures/plans";

import type { BridgeApi } from "../../../shared/api";
import { commandOutstanding, runningWithSettings } from "./job-detail-fixtures";
import type { FleetHandle, Scenario } from "./scenario";
import { onJob } from "./scenario";
import { entered, mount, openHelm, unmountAfterEach } from "./testing";

unmountAfterEach();

/** What Fleet answers when it cannot be reached. */
const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** App with this Job open, over a scenario whose Fleet answers these calls its own way. */
async function opened(fixture: JobFixture, behaves?: Scenario["behaves"], whereOpen = false): Promise<BridgeApi> {
  const app = mount({ ...onJob(fixture, { whereOpen }), behaves });
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  return app.api;
}




// **The Plan region came off Overview on 29 Sep 2026**, and with it the
// claims about its rows: a plan partway done with its count, the Working
// area folding what is done, the files each task changed, and the sentence
// naming which step records a plan. Overview's Plan card makes its own claims
// in `overview-boards`.
//
// **Drop and Add task came back on Plan** (owner, 30 Sep 2026): Drop this
// task in a task's panel, asking its reason in place, and Add task in each
// group's head on the list. The claims below are the ones the region made,
// moved to where the acts now are.

/** The Plan destination, on the list, where a group's head carries Add task. */
async function onThePlanList(): Promise<void> {
  await page.getByRole("tab", { name: /^Plan/ }).click();
  await page.getByRole("tab", { name: "List" }).click();
  await expect.element(page.getByRole("list", { name: "Groups, in the order they run" })).toBeVisible();
}

/** A task's panel, opened from its row on the list. */
async function panelOf(id: string, title: string) {
  await onThePlanList();
  await page.getByRole("listitem", { name: `${id} ${title}` }).getByRole("button").click();
  const panel = page.getByRole("dialog", { name: title });
  await entered(panel);
  return panel;
}

const OPEN_TASK = { id: "T3", title: "Add a unit test that does not construct the store" };
const DONE_TASK = { id: "T1", title: "Extract selectColumnOrder into its own module" };

/** A Fleet that cannot be reached for a plan edit. */
const planUnreachable: Scenario["behaves"] = () => ({
  addTask: async () => ({ ok: false, outcome: NOT_CONNECTED }),
  dropTask: async () => ({ ok: false, outcome: NOT_CONNECTED }),
});

test("a dropped task's panel reads its reason, and offers no second drop", async () => {
  await opened(withPlan(PLAN_WITH_A_DROPPED_TASK));
  const panel = await panelOf(OPEN_TASK.id, OPEN_TASK.title);
  await expect.element(panel.getByText("The existing integration test already exercises this path.")).toBeVisible();
  expect(panel.getByRole("button", { name: "Drop this task" }).query()).toBeNull();
});

test("Drop this task is on an open task's panel, and not on a done one's", async () => {
  await opened(withPlan(PLAN_PARTWAY));
  const open = await panelOf(OPEN_TASK.id, OPEN_TASK.title);
  await expect.element(open.getByRole("button", { name: "Drop this task" })).toBeInTheDocument();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();
  const done = await panelOf(DONE_TASK.id, DONE_TASK.title);
  expect(done.getByRole("button", { name: "Drop this task" }).query()).toBeNull();
});

test("a drop reason not typed yet is a hint, and Drop waits for one", async () => {
  await opened(withPlan(PLAN_PARTWAY));
  const panel = await panelOf(OPEN_TASK.id, OPEN_TASK.title);
  await panel.getByRole("button", { name: "Drop this task" }).click();
  await expect.element(panel.getByText("A reason is needed.")).toHaveAttribute("data-tone", "muted");
  await expect.element(panel.getByRole("button", { name: "Drop", exact: true })).toBeDisabled();
});

test("a drop tried with no reason turns the hint into an error", async () => {
  await opened(withPlan(PLAN_PARTWAY));
  const panel = await panelOf(OPEN_TASK.id, OPEN_TASK.title);
  await panel.getByRole("button", { name: "Drop this task" }).click();
  await panel.getByLabelText("Reason").click();
  await userEvent.keyboard("{Enter}");
  await expect.element(panel.getByText("A reason is needed.")).toHaveAttribute("data-tone", "error");
});

test("a drop sends this Job's id, the task and the reason typed", async () => {
  const api = await opened(withPlan(PLAN_PARTWAY));
  const dropTask = vi.spyOn(api, "dropTask");
  const panel = await panelOf(OPEN_TASK.id, OPEN_TASK.title);
  await panel.getByRole("button", { name: "Drop this task" }).click();
  await userEvent.type(panel.getByLabelText("Reason"), "Already covered elsewhere.");
  await panel.getByRole("button", { name: "Drop", exact: true }).click();
  await expect.poll(() => dropTask.mock.calls.length).toBe(1);
  expect(dropTask).toHaveBeenCalledWith(JOB_ID, { task: OPEN_TASK.id, reason: "Already covered elsewhere." });
});

test("a refused drop says nothing was sent, keeps the reason typed, and taps", async () => {
  const api = await opened(withPlan(PLAN_PARTWAY), planUnreachable);
  const tap = vi.spyOn(api, "tap");
  const panel = await panelOf(OPEN_TASK.id, OPEN_TASK.title);
  await panel.getByRole("button", { name: "Drop this task" }).click();
  await userEvent.type(panel.getByLabelText("Reason"), "Already covered elsewhere.");
  await panel.getByRole("button", { name: "Drop", exact: true }).click();
  await expect.element(panel.getByText("Fleet is not connected. Nothing was sent.")).toBeVisible();
  await expect.element(panel.getByLabelText("Reason")).toHaveValue("Already covered elsewhere.");
  // From the panel that holds its own answer, rather than from a toast. #1326.
  await expect.poll(() => tap.mock.calls.length).toBe(1);
  expect(tap).toHaveBeenCalledWith("level_change");
});

test("Add task is in each group's head, and opens with a title, an optional detail, and nothing to add until titled", async () => {
  await opened(withPlan(PLAN_PARTWAY));
  await onThePlanList();
  const groups = page.getByRole("list", { name: "Groups, in the order they run" });
  const heads = groups.getByRole("button", { name: "Add task" }).all();
  expect(heads.length).toBe(PLAN_PARTWAY.tasks.length);
  await heads[0]!.click();
  const dialog = page.getByRole("dialog");
  await expect.element(dialog.getByLabelText("Title")).toBeVisible();
  await expect.element(dialog.getByLabelText("Detail — optional")).toBeVisible();
  await expect.element(dialog.getByRole("button", { name: "Add task" })).toBeDisabled();
});

// The owner's, 30 Sep 2026: a group's Add task adds into that group, after its
// last task, rather than at the plan's end.
test("Add task in a group's head sends the task after that group's last task", async () => {
  const api = await opened(withPlan(PLAN_PARTWAY));
  const addTask = vi.spyOn(api, "addTask");
  await onThePlanList();
  const second = page.getByRole("listitem", { name: "Group 2" });
  const last = PLAN_PARTWAY.tasks[1]!;
  await expect.element(second.getByText(last.id, { exact: true })).toBeVisible();
  await second.getByRole("button", { name: "Add task" }).click();
  const dialog = page.getByRole("dialog");
  await entered(dialog);
  await userEvent.type(dialog.getByLabelText("Title"), "Add a regression test");
  await dialog.getByRole("button", { name: "Add task" }).click();
  expect(addTask).toHaveBeenCalledWith(JOB_ID, expect.objectContaining({ after: last.id }));
});

// The owner's, 30 Sep 2026: pressing a group on the graph opens its panel, and
// Add task in that panel's head adds into that group, as the list's does.
test("pressing a group on the graph opens its panel, and Add task there sends it after that group's last task", async () => {
  const api = await opened(withPlan(PLAN_PARTWAY));
  const addTask = vi.spyOn(api, "addTask");
  await page.getByRole("tab", { name: /^Plan/ }).click();
  await page.getByRole("tab", { name: "Graph" }).click();
  await page.getByRole("button", { name: /^Group 2,/ }).click();
  const panel = page.getByRole("dialog", { name: "Group 2" });
  await entered(panel);
  const last = PLAN_PARTWAY.tasks[1]!;
  await expect.element(panel.getByText(last.id, { exact: true })).toBeVisible();
  await panel.getByRole("button", { name: "Add task" }).click();
  const dialog = page.getByRole("dialog", { name: /Add a task/ });
  await entered(dialog);
  await userEvent.type(dialog.getByLabelText("Title"), "Add a regression test");
  await dialog.getByRole("button", { name: "Add task" }).click();
  expect(addTask).toHaveBeenCalledWith(JOB_ID, expect.objectContaining({ after: last.id }));
});

// A plan waiting on its first review (owner, 30 Sep 2026): graph and list
// offer the same acts, a task opened from a group's panel closes back to it,
// a group takes a proposed change, and a task can be edited.

/** `arc/plan-review` on the Plan graph, with group 3's panel open. */
async function groupThreeOnTheGraph() {
  const app = mount("arc/plan-review");
  await page.getByRole("tab", { name: /^Plan/ }).click();
  await page.getByRole("tab", { name: "Graph" }).click();
  await page.getByRole("button", { name: /^Group 3,/ }).click();
  const panel = page.getByRole("dialog", { name: "Group 3" });
  await entered(panel);
  return { api: app.api, panel };
}

test("a group's panel on the graph offers Remove and Propose a change, as its card on the list does, and no Move up", async () => {
  const { panel } = await groupThreeOnTheGraph();
  const asks = panel.getByRole("group", { name: "Ask about group 3" });
  for (const name of ["Remove", "Propose a change"]) {
    await expect.element(asks.getByRole("button", { name })).toBeVisible();
  }
  expect(asks.getByRole("button", { name: "Move up" }).query()).toBeNull();
  expect(asks.getByRole("button", { name: "Move down" }).query()).toBeNull();
});

// The owner's, 30 Sep 2026: *edits to the plan should just be made directly
// through fleet*. A move is a drop, or ⌥↑ / ⌥↓ on the focused row; Fleet has
// no route for it yet, so it says so, and the plan stays as it was.
test("⌥↓ on a task row sends the move to Fleet, which says it is not built, naming #1685 and the move, and the order stays", async () => {
  const written: string[] = [];
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: (text: string) => (written.push(text), Promise.resolve()) },
  });
  const app = mount("arc/plan-review");
  const movePlan = vi.spyOn(app.api, "movePlan");
  await onThePlanList();
  const row = page.getByRole("listitem", { name: /^T3 / }).getByRole("button");
  (row.element() as HTMLElement).focus();
  await userEvent.keyboard("{Alt>}{ArrowDown}{/Alt}");
  await expect.poll(() => movePlan.mock.calls.length).toBe(1);
  expect(movePlan).toHaveBeenCalledWith(ARC_JOB_ID, { group: "g2", task: "T3", to: 1 });
  await expect.element(page.getByText("Not implemented", { exact: true })).toBeVisible();
  const second = page.getByRole("list", { name: "Group 2 tasks" }).element();
  expect([...second.children].map((one) => one.getAttribute("aria-label")?.split(" ")[0])).toEqual(["T3", "T4"]);
  await page.getByRole("button", { name: "Copy debug info" }).click();
  await expect.poll(() => written).toHaveLength(1);
  const pasted = written[0]!;
  expect(pasted).toContain("bridge.not_implemented");
  expect(pasted).toContain(issueLink(1685));
  expect(pasted).toContain("POST /jobs/{job_id}/plan/move");
  expect(pasted).toContain("T3");
  expect(pasted).toContain("g2");
});

test("Remove on a group asks one reason in place and drops each of its tasks with it", async () => {
  const app = mount("arc/plan-review");
  const dropTask = vi.spyOn(app.api, "dropTask");
  await onThePlanList();
  const fourth = page.getByRole("listitem", { name: "Group 4" });
  await fourth.getByRole("button", { name: "Remove" }).click();
  const form = fourth.getByRole("region", { name: "Remove group 4" });
  await userEvent.type(form.getByLabelText("Reason"), "Out of scope for this Job.");
  await form.getByRole("button", { name: "Remove" }).click();
  await expect.poll(() => dropTask.mock.calls.length).toBe(2);
  expect(dropTask).toHaveBeenNthCalledWith(1, ARC_JOB_ID, { task: "T7", reason: "Out of scope for this Job." });
  expect(dropTask).toHaveBeenNthCalledWith(2, ARC_JOB_ID, { task: "T8", reason: "Out of scope for this Job." });
});

test("a task opened from a group's panel closes back to that group, and one opened from the plan closes to the plan", async () => {
  const { panel } = await groupThreeOnTheGraph();
  await panel.getByRole("listitem", { name: /^T5 / }).getByRole("button").click();
  const task = page.getByRole("dialog", { name: "Draw what is running, in four lists" });
  await entered(task);
  await expect.element(task.getByRole("button", { name: "Back to Group 3" })).toBeVisible();
  await task.getByRole("button", { name: /^Close/ }).click();
  await expect.element(page.getByRole("dialog", { name: "Group 3" })).toBeVisible();
  expect(page.getByRole("dialog", { name: "Draw what is running, in four lists" }).query()).toBeNull();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();
  // From the plan itself, Close is just close.
  await page.getByRole("tab", { name: "List" }).click();
  await page.getByRole("listitem", { name: /^T5 / }).getByRole("button").click();
  const again = page.getByRole("dialog", { name: "Draw what is running, in four lists" });
  await entered(again);
  expect(again.getByRole("button", { name: /^Back to/ }).query()).toBeNull();
  await again.getByRole("button", { name: /^Close/ }).click();
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();
});

test("Propose a change on a group sends the planning Drone an instruction naming the group and its tasks", async () => {
  const { api, panel } = await groupThreeOnTheGraph();
  const redirect = vi.spyOn(api, "redirectDrone");
  await panel.getByRole("button", { name: "Propose a change" }).click();
  const send = panel.getByRole("button", { name: "Send to the Drone" });
  await expect.element(send).toBeDisabled();
  await userEvent.type(panel.getByRole("textbox", { name: "Propose a change" }), "Run T6 after T5 rather than beside it");
  await send.click();
  await expect.poll(() => redirect.mock.calls.length).toBe(1);
  const [jobId, instruction] = redirect.mock.calls[0]!;
  expect(jobId).toBe(ARC_JOB_ID);
  expect(instruction).toContain("on group 3 (T5, T6): Run T6 after T5 rather than beside it");
  expect(instruction).toContain("refuse it and say what that reason is");
});

test("Edit this task opens filled from the task, and Save says the route is not built, naming #1657 and what changed", async () => {
  const written: string[] = [];
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: (text: string) => (written.push(text), Promise.resolve()) },
  });
  const { panel } = await groupThreeOnTheGraph();
  await panel.getByRole("listitem", { name: /^T6 / }).getByRole("button").click();
  const task = page.getByRole("dialog", { name: "Open a Drone's Job from its row" });
  await entered(task);
  await task.getByRole("button", { name: "Edit this task" }).click();
  await expect.element(task.getByLabelText("Title")).toHaveValue("Open a Drone's Job from its row");
  await expect.element(task.getByLabelText("Brief")).toHaveValue("The row opens the Job, not the Drone.");
  await expect.element(task.getByLabelText("Files")).toHaveValue("packages/screens/src/running-rows.tsx");
  await expect.element(task.getByLabelText("Done when")).toHaveValue("Pressing a Drone's row opens that Job");
  await expect.element(task.getByLabelText("Model")).toHaveValue("sonnet");
  const save = task.getByRole("button", { name: "Save" });
  await expect.element(save).toBeDisabled();
  await userEvent.selectOptions(task.getByLabelText("Model"), "haiku");
  await save.click();
  await expect.element(page.getByText("Not implemented", { exact: true })).toBeVisible();
  // Nothing was done, so what was typed stays.
  await expect.element(task.getByLabelText("Model")).toHaveValue("haiku");
  // The failure is drawn under the panel's dim, so the panels go first.
  await userEvent.keyboard("{Escape}");
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();
  await page.getByRole("button", { name: "Copy debug info" }).click();
  await expect.poll(() => written).toHaveLength(1);
  const pasted = written[0]!;
  expect(pasted).toContain("bridge.not_implemented");
  expect(pasted).toContain(issueLink(1657));
  expect(pasted).toContain("POST /jobs/{job_id}/tasks/{task_id}/edit");
  expect(pasted).toContain("T6");
  expect(pasted).toContain("haiku");
});

test("a refused Add task says nothing was sent, and keeps the title typed", async () => {
  await opened(withPlan(PLAN_PARTWAY), planUnreachable);
  await onThePlanList();
  await page.getByRole("button", { name: "Add task" }).first().click();
  const dialog = page.getByRole("dialog");
  await entered(dialog);
  await userEvent.type(dialog.getByLabelText("Title"), "Add a regression test");
  await dialog.getByRole("button", { name: "Add task" }).click();
  await expect.element(dialog.getByText("Fleet is not connected. Nothing was sent.")).toBeVisible();
  await expect.element(dialog.getByLabelText("Title")).toHaveValue("Add a regression test");
});

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
