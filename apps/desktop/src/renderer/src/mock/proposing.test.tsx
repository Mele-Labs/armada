// A Job being proposed, through `App`: the row, the page, the lead and the act,
// and where Dispatch leaves you.
//
// **The claim this file exists for is an address.** The owner asked three times
// for a dispatched request to be "like a job … so that I could come back to each
// of them" — so what these press is that a request has a row from the moment it
// is sent, that the row opens, and that the press does not hold anybody
// anywhere. #1159.
//
// **A claim names what a person sees, never a component**, which is
// `arc.test.tsx`'s own rule.

import { expect, test, describe } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { JobDetail, JobSummary, ProposalSettled } from "@armada/protocol";
import { filled } from "@armada/jobs";

import { scenarioNamed } from "./scenario";
import type { FleetHandle } from "./scenario";
import { mount, listed, rows, unmountAfterEach } from "./testing";

unmountAfterEach();

/** Every row on screen drawing the proposing badge, by the registry's own verb. */
function proposingRows(): HTMLElement[] {
  return rows().filter((row) => row.textContent?.includes("proposing") === true);
}

/** Every cell of one row's field run, in the order the columns name them. */
function cells(row: HTMLElement): HTMLElement[] {
  return [...row.querySelectorAll<HTMLElement>(".armada-job-row__field")];
}

/** One row's own cell under a named column, by the label the card stacks over it. */
function cell(row: HTMLElement, label: string): HTMLElement | undefined {
  return cells(row).find(
    (field) => field.querySelector(".armada-job-row__field-label")?.textContent === label,
  );
}

describe("a dispatched request is a row", () => {
  test("two requests dispatched at once are two rows, each carrying what was typed", async () => {
    mount("arc/proposing-dispatched");
    await listed();
    await expect.poll(() => proposingRows().length).toBe(2);
    // The title is the request, because nobody wrote a title — the proposer
    // answers one, and it has not answered.
    const said = proposingRows().map((row) => row.textContent ?? "");
    expect(said.some((one) => one.includes("Make the stat say what is running"))).toBe(true);
    expect(said.some((one) => one.includes("Say which of the two was given back"))).toBe(true);
  });

  test("the row reads the registry's verb and carries its glyph", async () => {
    mount("arc/proposing-dispatched");
    await listed();
    const row = proposingRows()[0]!;
    // `enum-verbs.toml` spells the word and names the glyph; nothing here does.
    const badge = row.querySelector(".armada-badge");
    expect(badge?.textContent).toContain("proposing");
    expect(badge?.querySelector("svg"), "the badge drew its verb with no glyph").not.toBeNull();
  });

  test("the workflow still settling blinks, and the two it never settles are empty, heading and all", async () => {
    mount("arc/proposing-dispatched");
    await listed();
    const row = proposingRows()[0]!;
    const running = rows().find((one) => one.textContent?.includes("running") === true)!;

    // **Every cell is still there**, because the columns are the list's and a
    // dropped field shifts every one behind it.
    expect(cells(row), "the row lost a cell, which shifts every column behind it").toHaveLength(
      cells(running).length,
    );
    // **The workflow is the proposer's to settle, and it has not**: a blinking
    // caret under its heading, named on hover (the owner's `text-cursor`, 3 Oct
    // 2026). A state is never text, so the cell prints no word for it.
    const workflow = cell(row, "Workflow");
    expect(workflow, "the Workflow column lost its heading over the caret").toBeDefined();
    expect(
      workflow?.querySelector('[role="img"][aria-label="Workflow, still being settled"]'),
      "the Workflow column drew no settling caret",
    ).not.toBeNull();
    // **And the two it has no fact for say nothing at all** — no value, and
    // no heading standing over the blank. Read by position, since the label
    // that used to name each is exactly what is gone.
    for (const at of [1, 2]) {
      const under = cells(row)[at]!;
      const named = cells(running)[at]?.querySelector(".armada-job-row__field-label")?.textContent;
      expect(under.textContent?.trim(), `${named ?? at} said something about a fact this Job has none of`).toBe("");
      expect(
        under.querySelector(".armada-job-row__field-label"),
        `${named ?? at} left its heading over a blank`,
      ).toBeNull();
    }
    // And the one it does have a fact for still says it, under its own name.
    expect(cell(row, "Dispatched by")?.textContent).toContain("Dispatched by");
  });
});

describe("coming back to one", () => {
  test("the row opens the Job, and its page says a model is reading the request", async () => {
    mount("arc/proposing-dispatched");
    await listed();
    const row = proposingRows()[0]!;
    await userEvent.click(row);

    await expect
      .element(page.getByText("A model is reading the request"))
      .toBeVisible();
    // The page a Job opens on, so the destinations are there to be pressed.
    await expect.element(page.getByRole("tab", { name: /^Overview/ })).toBeVisible();
  });

  test("the wait names the reach and offers the one act", async () => {
    mount("arc/proposing-reading");
    await expect.element(page.getByText("A model is reading the request")).toBeVisible();
    // `starting` is the reach worth telling apart: the call never got to the
    // vendor, and waiting will not fix it.
    const wait = page.getByRole("status").filter({ hasText: "Starting the proposer" }).first();
    await expect.element(wait).toBeVisible();
    await expect.element(wait).toHaveTextContent("sonnet");
    // Elapsed against Fleet's own budget, which is what makes the figure mean
    // something. Read as a shape: the instant moves with the window's clock.
    await expect.element(wait).toHaveTextContent(/\d+[ms]/);
    await expect.element(wait).toHaveTextContent("left");
    await expect.element(page.getByRole("button", { name: "Stop the proposer" })).toBeVisible();
  });

  test("a call nearly out of budget asks whether to keep waiting", async () => {
    mount("arc/proposing-slow");
    const wait = page.getByRole("status").filter({ hasText: "The model is thinking" }).first();
    await expect.element(wait).toBeVisible();
    await expect.element(wait).toHaveTextContent(/tokens of thinking/);
    await expect.element(wait).toHaveTextContent(/taking longer than expected/);
    await expect.element(page.getByRole("button", { name: "Stop the proposer" })).toBeVisible();
  });

  test("pressing stop waits on the answer rather than reporting one", async () => {
    mount("arc/proposing-slow");
    const stop = page.getByRole("button", { name: "Stop the proposer" });
    await expect.element(stop).toBeVisible();
    await stop.click();
    // The press is what waits. **No red notice**: `fleet.proposer_stopped` is
    // declared apart so a person's own press is not drawn as Armada breaking.
    await expect.element(page.getByRole("button", { name: "Stopping…" })).toBeVisible();
  });

  test("the absences draw nothing, and never read as a Job that arrived broken", async () => {
    mount("arc/proposing-reading");
    await expect.element(page.getByText("A model is reading the request")).toBeVisible();
    expect(page.getByText("This Job's frozen workflow has no steps.").query()).toBeNull();
    expect(page.getByText(/The proposer has not/).query()).toBeNull();
    expect(page.getByText("No brief was written.").query()).toBeNull();
    expect(page.getByText("No plan has been recorded.").query()).toBeNull();
  });
});

describe("where the press leaves you", () => {
  test("Dispatch leaves the composer, opens nothing, and the request is on the Board", async () => {
    mount("every-state");
    await listed();
    const before = rows().length;

    await page.getByRole("button", { name: "Dispatch", exact: true }).first().click();
    const select = page.getByLabelText("Repository", { exact: true }).element() as HTMLSelectElement;
    const offered = [...select.options].find((one) => !one.disabled && one.value !== "");
    await userEvent.selectOptions(select, offered!.value);
    await expect.element(page.getByRole("heading", { name: "Dispatch a job" })).toBeVisible();

    const asked = "Say on the Cleared tab whether the branch was kept";
    await userEvent.fill(page.getByLabelText("Request", { exact: true }), asked);
    // The composer's own press, not the title row's that opened it.
    await page.getByRole("button", { name: "Dispatch", exact: true }).last().click();

    // Back where he was, with the row added: the composer is gone, no Job page
    // opened, and nothing is holding him anywhere.
    await expect.poll(() => page.getByRole("heading", { name: "Dispatch a job" }).query()).toBe(null);
    expect(page.getByRole("tab", { name: /^Overview/ }).query(), "the press opened a Job").toBe(null);
    await expect.poll(() => rows().length).toBe(before + 1);
    expect(proposingRows().map((row) => row.textContent ?? "").some((one) => one.includes(asked))).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// A proposal that fills in as it is written — the owner, 30 Sep 2026: *"Now that
// we have this I would really push for us to find a way to make the proposer not
// report the job whole. Is there anyway for it to fill in as it goes?"* — and
// four fields in his order, which is his reasoning rather than a layout.
//
// **The moments are the fixture, and the handle is the claim.** `arc/proposing-*
// -landed` are four snapshots the owner steps through; a claim about a field
// arriving needs the arrival, so these mount one and publish the next.

/** The second request's own row, which is the one the moments settle fields on. */
const SECOND_REQUEST_SAYS = "The Cleared tab keeps rows whose worktree is gone";
/** What the proposer answers as its title, in the moments. */
const PROPOSED_TITLE = "Say which of the two a clear gave back";

/** One moment, mounted, with the handle its scenario publishes through. */
function watching(moment: string): FleetHandle {
  const scenario = scenarioNamed(moment);
  if (scenario === undefined) throw new Error(`no scenario ${moment}`);
  let handle: FleetHandle | undefined;
  mount({
    ...scenario,
    behaves: (fleet) => {
      handle = fleet;
      return {};
    },
  });
  if (handle === undefined) throw new Error("the scenario never behaved");
  return handle;
}

/**
 * One more field settled, folded onto the Job the way Fleet will.
 *
 * **Through `filled`, which is the one fold** — a settled field is the Job
 * becoming more complete, so this publishes the row and the detail rather than a
 * channel beside them. `also` is anything the moment is also meant to say, which
 * is how a call that died is drawn.
 */
function settle(
  fleet: FleetHandle,
  jobId: string,
  settled: ProposalSettled,
  also: Partial<JobSummary> = {},
): void {
  const state = fleet.state();
  const watched = state.watched;
  const before = state.jobs.find((one) => one.id === jobId);
  if (before === undefined) throw new Error("no Job to settle onto");
  // **Open or not.** Nobody has to be reading a Job for its fields to settle —
  // the press left the composer — so the detail is folded where it is this Job's
  // and a stand-in stands in where the page is shut.
  const open = watched.state === "read" && watched.jobId === jobId ? watched : null;
  const standing: JobDetail =
    open?.detail ?? {
      job: before,
      created_at: before.created_at,
      steps: [],
      acceptance_criteria: [],
      dependencies: [],
    };
  const moved = filled({ ...before, ...also }, { ...standing, job: before }, settled);
  const out = state.proposing;
  fleet.publish({
    jobs: state.jobs.map((one) => (one.id === jobId ? moved.job : one)),
    ...(open === null ? {} : { watched: { ...open, detail: moved.detail } }),
    ...(out === null ? {} : { proposing: { ...out, settled } }),
  });
}

/**
 * Open the Job whose fields are settling, the way a person does — by the row.
 * **The moments open nothing**: the press left the composer, so a moment of a
 * proposal filling in is the Board with the row on it.
 */
async function opened(): Promise<void> {
  await listed();
  await userEvent.click(settling());
  await expect.element(page.getByText("A model is reading the request")).toBeVisible();
}

/** The id of the Job the moment's own call is about, off the state. */
function idOf(fleet: FleetHandle): string {
  const found = fleet
    .state()
    .jobs.find((one) => one.status === "proposing" && one.workflow_id !== "");
  if (found === undefined) throw new Error("no Job has a settled workflow");
  return found.id;
}

/** The row for the request whose fields are settling. */
function settling(): HTMLElement {
  const row = proposingRows().find((one) => one.textContent?.includes(SECOND_REQUEST_SAYS) === true);
  return row ?? proposingRows().find((one) => one.textContent?.includes(PROPOSED_TITLE) === true)!;
}

describe("a proposal fills in as it is written", () => {
  test("the workflow lands first, and its column gets back both the heading and the value", async () => {
    mount("arc/proposing-workflow-landed");
    await listed();
    const row = settling();

    // The one of the three columns that fills before the Job is approved: the
    // workflow is chosen during `proposing` and frozen on the way out of it.
    expect(cell(row, "Workflow")?.textContent, "the Workflow column stayed blank").toContain(
      "Workflow",
    );
    expect(cell(row, "Workflow")?.textContent).toContain("feature");
    // And the two that have nothing yet still have nothing: the step machine is
    // initialised at `proposing -> awaiting_approval`, and nothing has run.
    for (const at of [1, 2]) {
      expect(cells(row)[at]?.textContent?.trim()).toBe("");
    }
    // The title is still the request, because the title has not landed.
    expect(row.textContent).toContain(SECOND_REQUEST_SAYS);
  });

  test("the request standing in for the title reads as words, not as the markdown it was typed in", async () => {
    mount("arc/proposing-workflow-landed");
    await listed();
    const row = settling();
    // The request carries a bold word and a code span — `arc-proposing.ts`.
    expect(row.textContent).toContain("the worktree alone");
    expect(row.textContent).toContain("the branch as well");
    expect(row.textContent).not.toContain("**");
    expect(row.textContent).not.toContain("`");
  });

  test("the title lands second, and the row says it", async () => {
    mount("arc/proposing-title-landed");
    await listed();
    expect(settling().textContent).toContain(PROPOSED_TITLE);
  });

  test("done-when lands third, one line at a time, in the wait that says what the call has got to", async () => {
    mount("arc/proposing-done-when-landed");
    await opened();
    const wait = page.getByRole("status").filter({ hasText: "The answer is arriving" }).first();
    await expect.element(wait).toHaveTextContent("Done when");
    await expect
      .element(wait)
      .toHaveTextContent("The Cleared tab names the branch on every row whose worktree is gone");
    // The fourth has not landed and nothing stands in for it.
    await expect.element(wait).not.toHaveTextContent("Urgency");
  });

  test("the settings land last, and every field before them is still there", async () => {
    mount("arc/proposing-settings-landed");
    await opened();
    const wait = page.getByRole("status").filter({ hasText: "The answer is arriving" }).first();
    await expect.element(wait).toHaveTextContent("Urgency");
    await expect.element(wait).toHaveTextContent("normal");
    await expect.element(wait).toHaveTextContent("Workflow");
    await expect.element(wait).toHaveTextContent("Title");
    await expect.element(wait).toHaveTextContent("Done when");
  });

  test("the model lands with the rest of the settings, because they are one line", async () => {
    mount("arc/proposing-settings-landed");
    await opened();
    const wait = page.getByRole("status").filter({ hasText: "The answer is arriving" }).first();
    await expect.element(wait).toHaveTextContent("Model");
    await expect.element(wait).toHaveTextContent("opus");
  });

  test("a settings line that named no model says nothing about one, and the Job keeps configuration's", async () => {
    mount("arc/proposing-model-left-to-configuration");
    await listed();
    // **Absent stays absent.** Nothing stands in for a model the proposer
    // declined to name: the Job reaches configuration's choice, which is what
    // the row was already carrying.
    expect(settling().textContent).not.toContain("opus");
    await opened();
    const wait = page.getByRole("status").filter({ hasText: "The answer is arriving" }).first();
    await expect.element(wait).toHaveTextContent("Urgency");
    await expect.element(wait).not.toHaveTextContent("Model");
  });

  test("the title changes under a reader on the Job's page, and the words he typed stay on it", async () => {
    // **Open before the title lands**, which is the case worth holding down:
    // the row he is reading is the row that changes.
    const fleet = watching("arc/proposing-workflow-landed");
    await opened();
    await expect.element(page.getByText(new RegExp(SECOND_REQUEST_SAYS)).first()).toBeVisible();

    settle(fleet, idOf(fleet), { workflow_id: "feature", title: PROPOSED_TITLE });

    // The title is the proposer's now.
    await expect.element(page.getByText(PROPOSED_TITLE).first()).toBeVisible();
    // **And nothing he typed left the screen.** Until a title lands the row's
    // title *is* the request, so the moment it is replaced the request becomes
    // the brief — which is where Fleet puts it when the call answers anyway.
    await expect.element(page.getByText(new RegExp(SECOND_REQUEST_SAYS)).first()).toBeVisible();
    await expect
      .element(page.getByText(/The proposer has not written one yet/))
      .not.toBeInTheDocument();
  });

  test("a call that dies after the workflow leaves a workflow, the request as the title, and no approval", async () => {
    // **On the Board and not on the page**, because that is where he is: the
    // press left the composer and nobody is sitting on this.
    const fleet = watching("arc/proposing-workflow-landed");
    await listed();

    // The call faulted. `job-statuses.toml` routes that to `escalated`, and the
    // coming-back message carries no reading — so what had settled stays and
    // nothing arrives after it.
    settle(fleet, idOf(fleet), { workflow_id: "feature" }, { status: "escalated" });
    fleet.publish({ proposing: null });

    // `enum-verbs.toml` spells `escalated` as **needs you**, and the badge is
    // read through the registry here as everywhere else.
    await expect
      .poll(() => rows().some((one) => one.textContent?.includes("needs you") === true))
      .toBe(true);
    const row = rows().find((one) => one.textContent?.includes(SECOND_REQUEST_SAYS) === true)!;
    // **The row is still readable as words somebody wrote.** A title that never
    // landed is the request, never a blank — which is what keeps a half-read Job
    // from looking like a Job that arrived broken.
    expect(row.textContent).toContain(SECOND_REQUEST_SAYS);
    expect(row.textContent).not.toContain(PROPOSED_TITLE);
    // And nothing offers to approve a Job the proposer never finished reading.
    expect(page.getByRole("button", { name: "Approve dispatch" }).query()).toBe(null);
  });
});
