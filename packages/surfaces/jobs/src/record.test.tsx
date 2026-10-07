// What a person reads on a Job's Record, through `App` and nothing else.
//
// **Split out of `arc.test.tsx`** on 28 September 2026, when the owner's six
// notes on the ledger doubled this block and that file reached its own ceiling.
// The claims are unchanged: each names what is on screen, never a component.

import { expect, test, describe } from "vitest";
import { page } from "vitest/browser";

import { entered, mount, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Jobs only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "jobs"] } as const;

describe("the Record", () => {
  test(
    "arc/planned: the Record's who column says Judge on the row where the plan was judged, " +
      "and Fleet on the row where the plan was recorded",
    async () => {
      await record("arc/planned");
      const rows = ledger();

      expect(whoSaid(rows, /Pressing the stat lists the Drone's Job and step/)).toBe("Judge");
      expect(whoSaid(rows, /^Plan recorded · /)).toBe("Fleet");
    },
  );

  test(
    "arc/executing-sequential: a Check's run is its own row, with Check in the who column — " +
      "not Fleet, and not the Drone that produced the work",
    async () => {
      await record("arc/executing-sequential");
      const rows = ledger();

      expect(whoSaid(rows, /^typecheck$/)).toBe("Check");
      expect(whoSaid(rows, /^screens_test$/)).toBe("Check");
      // The same Job's Drone and Fleet are both on screen, so Check is a
      // distinction the table is drawing rather than the only word it has.
      expect(rows.map((row) => row["Who"])).toContain("Drone");
      expect(rows.map((row) => row["Who"])).toContain("Fleet");
    },
  );

  // **The group is named where it holds more than one task.** Today's wire has
  // no groups, so `taskGroupsOf` derives one per task — and `Implement · group
  // 1 · T1` would then put the same fact on the row twice. The coordinate
  // still carries the group; `draft/ledger.test.ts` is where that is pinned.
  test(
    "arc/executing-sequential: each row says where in the Job it happened, down to the task " +
      "where it has one",
    async () => {
      await record("arc/executing-sequential");
      const rows = ledger();

      expect(rows.every((row) => row["Where"] !== "")).toBe(true);
      expect(rows.map((row) => row["Where"])).toContain("Implement · T1");
      expect(rows.map((row) => row["Where"])).toContain("Plan the change");
    },
  );

  // **Reworded from "the approval row".** The instant a Job was approved is a
  // status move, and status moves live in `GET /jobs/:job_id/events`, which no
  // arc fixture answers — so the row carrying this claim is the Job's creation.
  // The claim itself is unchanged: a fact about the Job names no step.
  test(
    "arc/approved-frozen: the row for the Job's own machine moving names no step at all, " +
      "because that is a fact about the Job",
    async () => {
      await record("arc/approved-frozen");
      const rows = ledger();

      expect(rows.map((row) => row["Where"])).toContain("The Job itself");
      expect(whoSaid(rows, /^Job created$/)).toBe("You");
    },
  );
});

// The owner's six notes on the ledger, 28 September 2026. Each test quotes the
// words that earned it.
describe("the Record, as the owner asked for it", () => {
  /** *`"Who"`* — on a column header reading `Who ran it`. */
  test("arc/executing-concurrent: the column of producers is headed Who, and nothing longer", async () => {
    await record("arc/executing-concurrent");

    expect(headings()).toContain("Who");
    expect(headings()).not.toContain("Who ran it");
  });

  /**
   * *"There should be a 'Job' filter."* — left on the line saying two rows were
   * under All alone. The filter is what empties that line.
   */
  test(
    "arc/executing-concurrent: the Job filter narrows the table to the Job's own machine " +
      "moving, and leaves no row under All alone",
    async () => {
      await record("arc/executing-concurrent");
      const all = ledger().length;

      await filterTo("Job");
      await expect.poll(() => ledger().length).toBeLessThan(all);
      const under = ledger();

      expect(under.every((row) => row["Where"] === "The Job itself")).toBe(true);
      expect(under.map((row) => row["What"])).toContain("Job created");
      expect(under.map((row) => row["What"])).toContain("Job started");
      // Every row All holds now answers to a filter, so the line that said
      // otherwise is gone rather than reworded.
      expect(page.getByRole("note").elements()).toHaveLength(0);
    },
  );

  /**
   * *"Instead of a kind column ... on the All filter we just have an icon on the
   * left side of the row indicating the kind with a hover tooltip showing the
   * kind name."* The tooltip's own text is what `Kind` reads here.
   */
  test(
    "arc/executing-concurrent: a row's kind is a mark under All and nothing at all under " +
      "one family",
    async () => {
      await record("arc/executing-concurrent");

      expect(headings()).toContain("Kind");
      // The mark's tooltip is the whole of what the column says, so the kind
      // reads here exactly as the column it replaced spelled it.
      expect(ledger().map((row) => row["Kind"])).toContain("task_files");

      await filterTo("Checks");
      await expect.poll(() => headings()).not.toContain("Kind");
    },
  );

  /** *"Maybe we should show the file name and on hover I can see the full path."* */
  test(
    "arc/executing-concurrent: a file cell reads as the filename, with the whole path on the " +
      "pointer",
    async () => {
      await record("arc/executing-concurrent");
      await filterTo("Files");
      await expect.poll(() => chips().length).toBeGreaterThan(0);

      for (const chip of chips()) {
        const whole = chip.getAttribute("title") ?? "";
        expect(whole).toContain("/");
        expect(chip.textContent?.trim()).toBe(whole.slice(whole.lastIndexOf("/") + 1));
      }
    },
  );

  /**
   * *"I see this row is red, but is it going to negatively impact the job?"* No.
   * `outside_plan` is *a mark, not a judgement* (`packages/protocol/src/events.ts`)
   * and the Judge weighs it, so the row carries no failing hue and never did
   * deserve one.
   */
  test(
    "arc/group-failed: a file nobody declared reads as out of scope, and its row takes " +
      "no failing hue",
    async () => {
      await record("arc/group-failed");
      const drifted = [...document.querySelectorAll("tbody tr")].filter((row) =>
        /out of scope/.test(row.textContent ?? ""),
      );

      expect(drifted.length).toBeGreaterThan(0);
      for (const row of drifted) expect(row.getAttribute("data-tone")).toBeNull();
      // `docs/concepts/plan.md` forbids the file scope being called a plan, and
      // the owner asked what the phrase meant on this exact cell.
      const table = document.querySelector("table")?.textContent ?? "";
      expect(table).not.toContain("outside the plan");
      expect(table).not.toContain("inside the plan");
    },
  );

  // The owner, 29 Sep 2026: *not just the step from completing but a task in
  // the plan, right?*
  test(
    "arc/group-failed: screens_test's row names group three as held back, and pressing T6 " +
      "there opens T6's own row",
    async () => {
      await record("arc/group-failed");
      await page.getByRole("button", { name: /^screens_test$/ }).click();
      await entered(page.getByRole("dialog", { name: "screens_test" }));

      await expect.element(page.getByText("Blocked group 3 from passing.")).toBeVisible();
      await expect.element(page.getByRole("button", { name: /T5 · Draw what is running/ })).toBeVisible();
      await page.getByRole("button", { name: /T6 · Open a Drone's Job from its row/ }).click();
      await expect.element(page.getByRole("heading", { name: /^T6 marked failed/ })).toBeVisible();
    },
  );

  // The owner, 2 Oct 2026: *click on the specific check to open a panel and
  // see the live log of the check. This should work anywhere that little
  // component is displayed.* It opened the Check's own Record row before.
  test(
    "arc/group-failed: pressing screens_test on T5's group boundary opens that Check's log",
    async () => {
      await record("arc/group-failed");
      await page.getByRole("button", { name: /^T5 marked done/ }).click();
      await entered(page.getByRole("dialog", { name: /^T5 marked done/ }));
      await page.getByRole("button", { name: "screens_test, failed" }).click();

      const log = page.getByRole("dialog", { name: "Check log" });
      await expect.element(log).toBeVisible();
      await expect.element(log.getByText("AssertionError: expected 'board' to be 'job'")).toBeVisible();
    },
  );
});

// The ways out of a Record row and into one, each a press a person makes on
// another destination's words. Owner's notes of 29 Sep 2026.
describe("the Record, and the destinations beside it", () => {
  test(
    "arc/group-failed: pressing the step a Check row's eyebrow names opens Workflow with that " +
      "step's panel open",
    async () => {
      await record("arc/group-failed");
      await page.getByRole("button", { name: /^screens_test$/ }).click();
      await entered(page.getByRole("dialog", { name: "screens_test" }));
      await page.getByRole("button", { name: "Implement", exact: true }).click();

      await expect.element(page.getByRole("tabpanel", { name: "Workflow" })).toBeVisible();
      await expect.element(page.getByRole("dialog", { name: "Implement" })).toBeVisible();
    },
  );

  test(
    "arc/group-failed: pressing a failed Check on the Plan's group boundary opens its log " +
      "where the plan is, rather than leaving for the Record",
    async () => {
      mount("arc/group-failed", SLICES);
      await page.getByRole("tab", { name: /^Plan/ }).click();
      await page.getByRole("tab", { name: "List" }).click();
      await page.getByRole("button", { name: "screens_test, failed" }).first().click();

      const log = page.getByRole("dialog", { name: "Check log" });
      await expect.element(log.getByText("AssertionError: expected 'board' to be 'job'")).toBeVisible();
      await expect.element(page.getByRole("tabpanel", { name: "Plan" })).toBeVisible();
    },
  );

  test(
    "arc/group-failed: the step menu narrows the table to one step's rows, and Any step " +
      "brings the Job's own back",
    async () => {
      await record("arc/group-failed");
      const all = ledger().length;
      const head = page.getByRole("region", { name: "What the Record holds" });

      await head.getByRole("button", { name: "Any step" }).click();
      await page.getByRole("menuitem", { name: "Implement" }).click();
      await expect.poll(() => ledger().length).toBeLessThan(all);
      expect(ledger().every((row) => row["Where"]?.startsWith("Implement"))).toBe(true);

      await head.getByRole("button", { name: "Implement", exact: true }).click();
      await page.getByRole("menuitem", { name: "Any step" }).click();
      await expect.poll(() => ledger().length).toBe(all);
    },
  );
});

/**
 * Choose a filter the way a person does: the menu in the panel's head, whose
 * trigger reads the chosen filter. Since 29 Sep 2026 the filters are a counted
 * menu there and not a strip of tabs, which wrapped at laptop width — and the
 * trigger carries no number, since the rows it would count are drawn under it.
 */
async function filterTo(name: string): Promise<void> {
  const head = page.getByRole("region", { name: "What the Record holds" });
  await head.getByRole("button", { name: /^All$/ }).click();
  await page.getByRole("menuitem", { name: new RegExp(`^${name} \\d+$`) }).click();
  // The trigger reads the filter chosen, and not what it holds.
  await expect
    .element(head.getByRole("button", { name: new RegExp(`^${name}$`) }))
    .toHaveAttribute("aria-haspopup", "menu");
}

/** App on an arc moment, with the Record open. */
async function record(name: string): Promise<void> {
  mount(name, SLICES);
  await page.getByRole("tab", { name: /^Record/ }).click();
  await expect.poll(() => ledger().length).toBeGreaterThan(0);
}

/** The table's column headings, in order. */
function headings(): string[] {
  const table = document.querySelector("table");
  if (table === null) return [];
  return [...table.querySelectorAll("thead th")].map((one) => one.textContent?.trim() ?? "");
}

/** Every path chip in a What cell. */
function chips(): Element[] {
  return [...document.querySelectorAll("td.armada-ledger__what .armada-path")];
}

/**
 * The Record's rows, each keyed by the column its cells sit under.
 *
 * Read through the table's own header rather than by position, so a column
 * added or reordered moves the keys with it instead of silently shifting every
 * assertion one cell along.
 */
function ledger(): Record<string, string>[] {
  const table = document.querySelector("table");
  if (table === null) return [];
  const columns = headings();
  return [...table.querySelectorAll("tbody tr")].map((row) =>
    Object.fromEntries(
      [...row.querySelectorAll("td")].map((cell, at) => [
        columns[at] ?? String(at),
        cell.textContent?.trim() ?? "",
      ]),
    ),
  );
}

/** Who ran the one row whose What matches. */
function whoSaid(rows: Record<string, string>[], what: RegExp): string | undefined {
  return rows.find((row) => what.test(row["What"] ?? ""))?.["Who"];
}
