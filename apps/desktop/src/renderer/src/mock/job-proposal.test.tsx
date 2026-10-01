// What a person can change on a Job at its approval gate, through `App` and
// nothing else — and what is left of the screen once they have approved it.
//
// **The owner's second pass over what #1633 built is what these claim.** He
// asked why the request could not be edited, how a criterion is added before
// dispatch, what `answered by the check` meant and what `from armada/1162`
// was. Each is a sentence below about what is on screen, so a rename of the
// composition leaves them alone and a change to the reading fails them.
//
// **Beside `arc.test.tsx` rather than inside it.** That file is the arc's own
// roster, one claim per moment, and it already runs eighty-four of them; this
// is one moment read closely, and keeping it apart is what makes it a file
// somebody can run on its own while working on the screen.

import { afterEach, expect, test, describe } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

// The project's own size, back after every test: the viewport outlives an
// unmount, so the width claims at the foot would otherwise set it for
// everything that ran after them.
const RESTING = { width: 1440, height: 900 };
afterEach(async () => {
  await page.viewport(RESTING.width, RESTING.height);
});

/** The proposal, still open. Every control on it is a person's. */
const open = () => mount("arc/proposing-review");

/** The same Job one press later, where nothing is a control any more. */
const frozen = () => mount("arc/approved-frozen");

const heldTo = () => page.getByRole("region", { name: "Done when" });
const asked = () => page.getByRole("textbox", { name: "What was asked" });

/** One criterion's field, by the place it sits in the brief. */
const criterion = (at: number) => page.getByRole("textbox", { name: `Criterion ${at}` });

/**
 * A length token, read off the document rather than retyped. `job-detail-
 * width.test.tsx`'s own `floor`, which is where the argument for reading
 * rather than restating is written down.
 */
function length(token: "--w-proposal-settings" | "--w-step-panel-min"): number {
  const value = parseFloat(getComputedStyle(document.documentElement).getPropertyValue(token));
  if (!Number.isFinite(value)) throw new Error(`${token} is not declared`);
  return value;
}

const boxOf = (selector: string): DOMRect => {
  const element = document.querySelector<HTMLElement>(selector);
  if (element === null) throw new Error(`${selector} is not drawn`);
  return element.getBoundingClientRect();
};

/** A `DOMRect` is a float and a token is an integer. `job-detail-width`'s. */
const ROUNDING = 0.5;

describe("what a person may still change", () => {
  test(
    "the request is a field you type in, with nothing to press first and nothing to press " +
      "after",
    async () => {
      open();

      // The words themselves, editable where they stand. The owner refused a
      // toggle on 28 Sep: before approval it is a field, after it is a
      // reading, and there is no third state in between.
      await expect.element(asked()).toBeVisible();
      await expect
        .element(asked())
        .toHaveValue(
          "The stat reads its two numbers off `get_capacity`, and nothing on it leads to the one " +
            "Drone that is running or to anything else the machine has out.",
        );

      await asked().fill("Make the Drones stat say what is running, and keep it live.");
      await expect
        .element(asked())
        .toHaveValue("Make the Drones stat say what is running, and keep it live.");

      // Nothing commits it and nothing takes it back, because there is no
      // moment where it was not already the proposal.
      expect(page.getByRole("button", { name: "Edit", exact: true }).all()).toHaveLength(0);
      expect(page.getByRole("button", { name: "Done", exact: true }).all()).toHaveLength(0);
    },
  );

  test("a criterion is reworded where it is read, and the rest are left alone", async () => {
    open();

    await expect
      .element(criterion(1))
      .toHaveValue("The rail's Drones stat reads one running beside the machine's most");
    await criterion(1).fill("The rail's Drones stat reads one running beside the machine's four");

    await expect
      .element(criterion(1))
      .toHaveValue("The rail's Drones stat reads one running beside the machine's four");
    await expect
      .element(criterion(2))
      .toHaveValue("Pressing the stat lists the Drone's Job and step, and any Check or Judge call out");
  });

  test(
    "a criterion added at the gate is appended empty, says you wrote it, and says the Judge " +
      "is what will decide it",
    async () => {
      open();

      await page.getByRole("button", { name: "Add a criterion" }).click();

      // Appended, never inserted: a citation names a criterion's place in the
      // brief, so a line above the others renumbers every citation written.
      await expect.element(criterion(3)).toBeVisible();
      await expect.element(criterion(3)).toHaveValue("");
      await expect
        .element(criterion(1))
        .toHaveValue("The rail's Drones stat reads one running beside the machine's most");

      // A Check is the workflow's and is frozen at creation, so a line typed
      // here has no Check to run against it and says the Judge instead.
      await expect.element(heldTo()).toHaveTextContent("You wrote this");
      await expect.element(heldTo()).toHaveTextContent("The Judge will decide it");

      await criterion(3).fill("The stat stays live while a Drone starts and stops");
      await expect
        .element(criterion(3))
        .toHaveValue("The stat stays live while a Drone starts and stops");
    },
  );

  test("a criterion removed takes its own line and leaves the one beside it", async () => {
    open();

    await page.getByRole("button", { name: "Remove criterion 1" }).click();

    // What was second is first, and there is no third.
    await expect
      .element(criterion(1))
      .toHaveValue("Pressing the stat lists the Drone's Job and step, and any Check or Judge call out");
    expect(page.getByRole("textbox", { name: "Criterion 2" }).all()).toHaveLength(0);
    // The words that went are gone from the region, not merely from the field
    // — a fill would leave the old text nowhere and still pass the line above.
    await expect
      .element(heldTo())
      .not.toHaveTextContent("The rail's Drones stat reads one running");
  });

  test(
    "picking another workflow replaces every gate row, because a gate belongs to a step",
    async () => {
      open();

      const picker = page.getByRole("combobox", { name: "Workflow" });
      await expect.element(picker).toHaveValue("feature");
      await expect.element(page.getByRole("listitem", { name: "Plan the change" })).toBeVisible();

      await userEvent.selectOptions(picker, "bug");

      await expect.element(picker).toHaveValue("bug");
      // The feature workflow's own steps are gone rather than carried across
      // by position, which would put a tick meant for `handoff` on whatever
      // the bug workflow runs fourth.
      expect(page.getByRole("listitem", { name: "Plan the change" }).all()).toHaveLength(0);
      // `bug` is the reference sample and it runs seven, so the rebuild is a
      // rebuild rather than a relabelling of the four the Job had.
      await expect
        .element(page.getByRole("region", { name: "Workflow" }))
        .toHaveTextContent("bug — 7 steps");
      await expect.element(page.getByRole("listitem", { name: "repro" })).toBeVisible();
      await expect.element(page.getByRole("listitem", { name: "close" })).toBeVisible();

      // Two of its seven defer to the repository, which is the fourth state
      // the three tick boxes cannot express.
      expect(page.getByText("The repository decides", { exact: false }).all()).toHaveLength(2);

      // And one is gated on a Check with no Judge and nobody, which is the
      // other combination `feature` never shows.
      await expect
        .element(page.getByRole("listitem", { name: "close" }))
        .toHaveTextContent("Its Checks are the whole gate");

      // **What the new steps declare is read off the new workflow.** It was
      // read off the Job's frozen steps, which hold none of these, so every
      // row of a picked workflow said *this step declares no Check* under a
      // ticked box. Six of the seven declare one.
      await expect
        .element(page.getByRole("listitem", { name: "repro" }))
        .not.toHaveTextContent("declares no Check");
      expect(page.getByText("declares no Check", { exact: false }).all()).toHaveLength(0);
    },
  );
});

describe("the step the repository decides", () => {
  /** The row the owner could not read (`rhxt`, 29 Sep). */
  const deferred = () => page.getByRole("listitem", { name: "Review the change" });

  // **Only the whole screen carries this.** The word is the repository's, not
  // the step's: it comes off the proposal the window holds and reaches the row
  // through `gateRowsOf`, so a story handed a row cannot show that the two
  // ends are joined.
  test(
    "the row says what its repository's policy resolves to today, and that this Job moves " +
      "with it",
    async () => {
      open();

      await expect.element(deferred()).toHaveTextContent("The repository decides — review_gate");
      // `human_always`, as this repository's `armada.yml` declares it, in the
      // verb `enum-verbs.toml` generates for that value.
      await expect.element(deferred()).toHaveTextContent("Today that policy says a person answers");
      await expect.element(deferred()).toHaveTextContent("for this Job as well");
      // And the wire's own spelling stays off the screen.
      await expect.element(deferred()).not.toHaveTextContent("human_always");
    },
  );

  test("what the button would change is on the row before it is pressed", async () => {
    open();

    await expect
      .element(deferred())
      .toHaveTextContent("Deciding it for this Job hands this step to the three boxes instead.");
    expect(deferred().getByRole("checkbox").all()).toHaveLength(0);

    // Pressed, the three boxes are the answer, and the row stops offering the
    // choice it has already taken.
    await deferred().getByRole("button", { name: "Decide it for this Job" }).click();

    expect(deferred().getByRole("checkbox").all()).toHaveLength(3);
    await expect
      .element(deferred())
      .not.toHaveTextContent("hands this step to the three boxes");
    await expect
      .element(deferred())
      .toHaveTextContent("This Job decides this step for itself, in place of the repository's");
  });
});

describe("what each criterion says about itself", () => {
  test(
    "a criterion names the issue its words came from as an issue, and says what will decide " +
      "it in the future tense it is in",
    async () => {
      open();

      // `from armada/1162` read as a repository, a path or a branch as
      // readily as an issue, which is what the owner asked (`u7y9`).
      await expect.element(heldTo()).toHaveTextContent("From issue armada/1162");
      // `answered by the check` read as a verdict already in, on a Job that
      // has not started (`f9yw`).
      await expect.element(heldTo()).toHaveTextContent("A Check will decide it");
      await expect.element(heldTo()).toHaveTextContent("The Judge will decide it");
      await expect.element(heldTo()).not.toHaveTextContent("answered by the");
    },
  );

  test("no criterion is labelled by its number", async () => {
    open();

    // The positive first: an assertion about absence passes against a window
    // that has not drawn yet.
    await expect.element(criterion(1)).toBeVisible();
    await expect.element(heldTo()).not.toHaveTextContent("Criterion 1");
    await expect.element(heldTo()).not.toHaveTextContent("Criterion 2");
  });

  test("the reference is text until an address for it is known", async () => {
    open();

    await expect.element(heldTo()).toHaveTextContent("From issue armada/1162");
    // Nothing on the wire carries the issue's forge address, and main opens
    // no address it is not already holding — so the reference reads as the
    // reference it is rather than as a control that presses into nothing.
    expect(heldTo().getByRole("button", { name: "armada/1162" }).all()).toHaveLength(0);
  });
});

describe("approved", () => {
  test(
    "nothing on the screen takes an answer once it is approved, and the request is a reading " +
      "again",
    async () => {
      frozen();
      // **Settings holds what froze, since 29 Sep 2026** — Overview shows what
      // needs you, and a reading that never changes is a setting.
      await page.getByRole("tab", { name: /^Settings/ }).click();

      const froze = page.getByRole("region", { name: "Frozen at approval" });
      await expect.element(froze).toHaveTextContent(/Nothing here changes while the Job runs\./);
      expect(froze.getByRole("textbox").all()).toHaveLength(0);
      expect(froze.getByRole("combobox").all()).toHaveLength(0);
      expect(froze.getByRole("checkbox").all()).toHaveLength(0);
      expect(froze.getByRole("spinbutton").all()).toHaveLength(0);
      expect(page.getByRole("button", { name: "Add a criterion" }).all()).toHaveLength(0);
      expect(page.getByRole("button", { name: /^Remove criterion/ }).all()).toHaveLength(0);

      // And the words the Job is held to are still read somewhere: Plan's
      // lead. **Not `Done when`** — that region belongs to the proposal, and
      // the proposal is Settings' once a Job is approved.
      await page.getByRole("tab", { name: /^Plan/ }).click();
      await expect
        .element(page.getByRole("tabpanel", { name: "Plan" }))
        .toHaveTextContent("The rail's Drones stat reads one running beside the machine's most");
    },
  );

  test("the workflow reads as its name and its steps, with no picker to change it", async () => {
    frozen();
    await page.getByRole("tab", { name: /^Settings/ }).click();

    const workflow = page.getByRole("region", { name: "Workflow" });
    await expect.element(workflow).toHaveTextContent("feature — 4 steps");
    expect(workflow.getByRole("combobox").all()).toHaveLength(0);
  });
});

describe("the settings column's width", () => {
  /**
   * **A measurement, so it is here and not in a story.** What the column
   * rests at is decided by the window, and a story drawing the composition in
   * a sized `div` cannot see it.
   */
  async function at(width: number): Promise<void> {
    await page.viewport(width, 900);
    open();
    await expect.element(page.getByRole("region", { name: "Workflow" })).toBeVisible();
  }

  // 1512 is the owner's laptop, and the width he called crammed. The column
  // was on `--w-run-column`'s 380px, borrowed; it has a token of its own now.
  test("at 1512 the settings column rests at its own token, not the run column's", async () => {
    await at(1512);

    const rest = length("--w-proposal-settings");
    expect(rest).toBeGreaterThan(380);
    expect(boxOf(".armada-proposal__settings").width).toBeCloseTo(rest, 0);
  });

  // **Widening the settings column must not widen it further.** The rail
  // grows with the window and a `fr` on both tracks would spend every pixel
  // of a maximised screen on a panel of labels — `bridge.md`'s own finding
  // about the run column, one screen along. The request is the track that
  // takes the slack, because it is the thing that holds prose.
  test("the settings column is the same width at 1100 and at 1512, and the request is not", async () => {
    await at(1100);
    const narrow = {
      settings: boxOf(".armada-proposal__settings").width,
      request: boxOf(".armada-proposal__request-column").width,
    };

    await at(1512);
    const wide = {
      settings: boxOf(".armada-proposal__settings").width,
      request: boxOf(".armada-proposal__request-column").width,
    };

    // Every pixel the pair gained went to the request, which is the exact
    // form of the claim rather than a margin somebody picked.
    const gained = wide.request + wide.settings - (narrow.request + narrow.settings);
    expect(gained).toBeGreaterThan(0);
    expect(wide.request - narrow.request).toBeCloseTo(gained, 0);
    expect(wide.settings).toBeCloseTo(narrow.settings, 0);
    // And neither is crushed at the narrow end, which is the defect this
    // pair of floors exists for.
    expect(narrow.settings).toBeGreaterThanOrEqual(length("--w-step-panel-min") - ROUNDING);
  });

  // The reading the owner was looking at when he called the panel crammed:
  // the Job's own title, cut off at `Show what is running in the Drones s`.
  // An input scrolls what it cannot show, so a value wider than its box is
  // exactly `scrollWidth` over `clientWidth` — and the claim is every field
  // on the panel rather than that one, because which is longest changes with
  // the Job.
  test("at 1512 no field in the settings column is scrolling its own value", async () => {
    await at(1512);

    const fields = [
      ...document.querySelectorAll<HTMLInputElement>(".armada-proposal__settings input"),
    ];
    expect(fields.length).toBeGreaterThan(0);
    const cut = fields
      .filter((field) => field.scrollWidth > field.clientWidth + ROUNDING)
      .map((field) => `${field.getAttribute("aria-label") ?? "?"}: ${field.value}`);
    expect(cut).toEqual([]);
  });
});
