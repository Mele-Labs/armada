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

import { mount, listed, rows, unmountAfterEach } from "./testing";

unmountAfterEach();

/** Every row on screen drawing the proposing badge, by the registry's own verb. */
function proposingRows(): HTMLElement[] {
  return rows().filter((row) => row.textContent?.includes("proposing") === true);
}

/** One row's own cell under a named column, by the label the card stacks over it. */
function cell(row: HTMLElement, label: string): HTMLElement | undefined {
  return [...row.querySelectorAll<HTMLElement>(".armada-job-row__field")].find(
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

  test("the three columns it has nothing for are empty, not filled in", async () => {
    mount("arc/proposing-dispatched");
    await listed();
    const row = proposingRows()[0]!;
    for (const label of ["Workflow", "Progress", "Run time"]) {
      const under = cell(row, label);
      expect(under, `the row lost its ${label} cell, which shifts every column behind it`).toBeDefined();
      const value = under?.querySelector(".armada-job-row__field-value")?.textContent ?? "";
      expect(value.trim(), `${label} invented a value for a fact this Job has none of`).toBe("");
    }
    // And the one it does have a fact for still says it.
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

  test("the absences read as not yet rather than as a Job that arrived broken", async () => {
    mount("arc/proposing-reading");
    await expect.element(page.getByText("A model is reading the request")).toBeVisible();
    await expect
      .element(page.getByText("The proposer has not chosen a workflow yet."))
      .toBeVisible();
    await expect.element(page.getByText(/The proposer has not written one yet/)).toBeVisible();
    await expect.element(page.getByText("No plan has been recorded.")).toBeVisible();
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
