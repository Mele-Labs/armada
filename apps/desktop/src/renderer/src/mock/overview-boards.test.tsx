// What each Overview arrangement is, as its artboard draws it — through `App`
// and nothing else.
//
// **A claim here is about shape, not about copy.** The owner put the built
// train beside its board on 28 September 2026 and the two were different
// screens: the thing waiting on him was not on the screen at all, one column
// where the board draws two, a member drawn as five label-and-value lines
// where the board draws one row, a task count in words where the board draws
// a bar. Every one of those is below, measured off the DOM rather than read
// off a string — a sentence can be right while the screen is wrong.

import { expect, test, describe } from "vitest";
import { page } from "vitest/browser";

import { conceptSaid } from "@armada/components";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * Mount a moment and wait for the region the claim is about to be drawn.
 * **A box read before the paint is a box of nothing**, and every claim here is
 * about where something sits.
 */
async function drawn(scenario: string, selector: string): Promise<void> {
  mount(scenario);
  await onScreen();
  await expect.poll(() => document.querySelectorAll(selector).length).toBeGreaterThan(0);
}

/**
 * What an element draws, without the tooltip bubble inside it. `Tooltip
 * asChild` puts the bubble in the child, so `textContent` is the value and
 * its own explanation run together.
 */
function saidBy(selector: string): string {
  const found = document.querySelector(selector);
  if (found === null) throw new Error(`nothing matched ${selector}`);
  return found.childNodes[0]?.textContent ?? "";
}

/** How many of the Record's rows name this in their What cell. */
function rowsSaying(what: string): number {
  return [...document.querySelectorAll("table tbody tr td.armada-ledger__what")].filter((cell) =>
    (cell.textContent ?? "").includes(what),
  ).length;
}

/** One element's box, or a failure naming what was not on screen. */
function boxOf(selector: string): DOMRect {
  const found = document.querySelector(selector);
  if (found === null) throw new Error(`nothing matched ${selector}`);
  return found.getBoundingClientRect();
}

describe("the train — Overview for a Job whose members are Jobs", () => {
  test("the thing waiting on you leads the screen, before the order it is about", async () => {
    await drawn("members/stacked", ".armada-members");

    const lead = page.getByText("Member 2 is asking you something.");
    await expect.element(lead).toBeVisible();
    await expect.element(page.getByRole("button", { name: "Answer it" })).toBeVisible();

    // Above the order, not under it: the whole complaint was that a person
    // had to look for what was waiting on them.
    expect(boxOf(".armada-lead").bottom).toBeLessThanOrEqual(
      boxOf(".armada-members").top,
    );
  });

  test("the order and the decision are two columns, side by side", async () => {
    await drawn("members/stacked", ".armada-decision");

    const order = boxOf(".armada-members");
    const decision = boxOf(".armada-decision");
    // Side by side is two facts: the decision starts after the order ends,
    // and the two share the same band of the screen.
    expect(decision.left).toBeGreaterThanOrEqual(order.right);
    expect(decision.top).toBeLessThan(order.bottom);
  });

  test("a member is one row, not a list of labelled fields", async () => {
    await drawn("members/stacked", ".armada-members__member");

    const rows = document.querySelectorAll(".armada-members__member");
    expect(rows).toHaveLength(3);

    // No definition list anywhere in the order: five `label: value` lines per
    // member is what made two members fill the window.
    expect(document.querySelectorAll(".armada-members dl")).toHaveLength(0);
    for (const name of ["Pull request", "Branch", "Targets", "Writes", "Tasks"]) {
      expect(page.getByText(name, { exact: true }).query()).toBeNull();
    }

    // Two lines of content, and the row is shorter than the three-line block
    // a member's title, its chips and its bar would take if they stacked.
    const first = rows[0]!.getBoundingClientRect();
    expect(first.height).toBeLessThan(80);
  });

  test("progress is a bar per member and on the foot, never a count in words", async () => {
    await drawn("members/stacked", ".armada-members__member");

    // One bar per member, and one for the order itself.
    expect(document.querySelectorAll(".armada-members .armada-step-bar").length).toBe(4);
    // The first member's plan is four tasks, all done — four segments, all past.
    const first = document.querySelectorAll(".armada-members__member .armada-step-bar")[0]!;
    const segments = first.querySelectorAll(".armada-step-bar__segment");
    expect(segments).toHaveLength(4);
    expect([...segments].every((one) => one.getAttribute("data-state") === "past")).toBe(true);

    // `4 of 4` is the reading the bar replaced.
    expect(page.getByText("4 of 4", { exact: true }).query()).toBeNull();
  });

  test("the line between two members says how they are related, in both directions", async () => {
    await drawn("members/stacked", ".armada-members__join");

    await expect
      .element(page.getByText("Member 2 waits on what member 1's merge publishes."))
      .toBeVisible();
    await expect
      .element(page.getByText("Member 3 branches off member 2, so it keeps working."))
      .toBeVisible();
    // The edge belongs to neither row, so it is drawn between them.
    const joins = document.querySelectorAll(".armada-members__join");
    expect(joins).toHaveLength(2);
  });

  test("the decision is answered where it is read, and says what the answer moves", async () => {
    const { api } = mount("members/stacked");
    await onScreen();
    await expect.poll(() => document.querySelectorAll(".armada-decision").length).toBe(1);

    const decision = page.getByRole("region", { name: "Member 2 · asked of you" });
    await expect.element(decision).toBeVisible();
    await expect.element(decision).toHaveTextContent("Refused");
    await expect.element(decision).toHaveTextContent("02-selectors-cover-the-empty-store");
    await expect
      .element(decision)
      .toHaveTextContent("1 pull request behind this one is waiting on the answer");

    const answered: unknown[] = [];
    api.answerJudge = (...args: unknown[]) => {
      answered.push(args);
      return Promise.resolve({ ok: true }) as never;
    };
    await decision.getByRole("button", { name: "Disagree, just this step" }).click();
    // Against the member's own Job id, which is the same verdict as answering
    // it there — never the parent's.
    expect(answered).toHaveLength(1);
    expect((answered[0] as string[])[0]).toBe("01M2D7A1XK001MEMBER00000B");
  });
});

describe("the proposal — Overview at and just past the approval gate", () => {
  test("the request and the settings are two columns, side by side", async () => {
    await drawn("arc/proposing-review", ".armada-proposal__settings");

    const request = boxOf(".armada-proposal__request-column");
    const settings = boxOf(".armada-proposal__settings");
    expect(settings.left).toBeGreaterThanOrEqual(request.right);
    expect(settings.top).toBeLessThan(request.bottom);
  });

  // **Read on Settings, because that is where a frozen setting is read.**
  // Overview drew the frozen proposal until 29 Sep 2026; it leads with the one
  // thing now, and what froze at the gate is Settings'. Every value on an open
  // proposal is a control and a control brings its own box — the last value
  // that was not one was `Workflow — 4 steps`, and it became the picker when
  // the gates folded under it (`2b4j`, 28 Sep).
  test("a setting's value sits in a box of its own, and its label does not", async () => {
    mount("arc/approved-frozen");
    await onScreen();
    await page.getByRole("tab", { name: /^Settings/ }).click();
    await expect
      .poll(() => document.querySelectorAll(".armada-proposal__field-value").length)
      .toBeGreaterThan(0);

    const value = document.querySelector(
      ".armada-proposal__field-value:not([data-bare])",
    );
    expect(value, "no boxed setting value on screen").not.toBeNull();
    const box = getComputedStyle(value as Element);
    // A border and a ground of its own: the grid without them was one wall of
    // same-weight text, which is what the owner refused.
    expect(parseFloat(box.borderTopWidth)).toBeGreaterThan(0);
    expect(box.backgroundColor).not.toBe("rgba(0, 0, 0, 0)");

    const label = document.querySelector(".armada-proposal__field-label");
    expect(getComputedStyle(label as Element).backgroundColor).toBe("rgba(0, 0, 0, 0)");
  });

  test("a setting's label says what its word means, and the sentence is the one table's", async () => {
    await drawn("arc/proposing-review", ".armada-proposal__field-label");

    // The bubble is named by `aria-describedby` rather than by a role, which
    // is `Tooltip`'s own decision — so the claim is that the label describes
    // itself with the one sentence `concepts.tsx` holds for the word.
    const labels = [...document.querySelectorAll(".armada-proposal__field-label")];
    const label = labels.find((one) => one.childNodes[0]?.textContent === "Drones at once");
    expect(label, "no label reads `Drones at once`").toBeDefined();
    const bubble = document.getElementById(label?.getAttribute("aria-describedby") ?? "");
    expect(bubble?.textContent).toBe(conceptSaid("Drones at once"));
    expect(bubble?.textContent).toContain("side by side");
  });

  test("the wire's own advance_gate is off the screen", async () => {
    await drawn("arc/proposing-review", ".armada-proposal__gate");

    for (const wire of ["auto_if_judge_passes", "manifest_rule:review_gate", "human_always"]) {
      expect(page.getByText(wire, { exact: true }).query()).toBeNull();
    }
    // What a person reads instead is what Fleet does with it.
    await expect.element(page.getByText("It advances unless the Judge refuses it.")).toBeVisible();
  });

  test("an answer nothing observes is offered without a paragraph apologising for it", async () => {
    await drawn("arc/proposing-review", ".armada-proposal__settings");

    expect(page.getByText(/Nothing on a Job's record answers/).query()).toBeNull();
  });
});

describe("the brief — the Proposer's words, as the markdown they were written in", () => {
  // **The card the owner pinned the ask on**, 1 Oct 2026: *"Can we please
  // support markdown when displaying text from agents?"*
  test("a backticked name in the brief is drawn as code, not as backticks", async () => {
    await drawn("arc/executing-concurrent", ".armada-overview-board__brief");

    const brief = page.getByRole("region", { name: "Brief" });
    const name = brief.getByText("get_capacity");
    await expect.element(name).toBeVisible();
    expect(name.element().tagName).toBe("CODE");
    expect(brief.element().textContent).not.toContain("`");
  });
});

describe("the strip, and the header above it", () => {
  test("Settings is the sixth destination and the header button is gone", async () => {
    await drawn("arc/proposing-review", ".armada-destinations");

    for (const name of ["Overview", "Workflow", "Plan", "Record", "Pulse", "Settings"]) {
      await expect.element(page.getByRole("tab", { name: new RegExp(`^${name}`) })).toBeVisible();
    }
    expect(page.getByRole("button", { name: /^Job settings/ }).query()).toBeNull();
  });

  test("the destinations are underline tabs, not a boxed control", async () => {
    await drawn("arc/proposing-review", ".armada-destinations");

    const strip = document.querySelector(".armada-destinations") as HTMLElement;
    expect(strip, "the strip is not the destinations strip").not.toBeNull();
    // No track round them: two boxed controls stacked have no hierarchy, and
    // a panel's own filters are the boxed one.
    const track = getComputedStyle(strip);
    expect(track.backgroundColor).toBe("rgba(0, 0, 0, 0)");
    expect(parseFloat(track.borderTopWidth)).toBe(0);

    // The active one is marked by a rule under it rather than by a fill.
    const active = strip.querySelector('[aria-selected="true"]') as HTMLElement;
    expect(getComputedStyle(active).backgroundColor).toBe("rgba(0, 0, 0, 0)");
    expect(parseFloat(getComputedStyle(active).borderBottomWidth)).toBeGreaterThan(0);
  });

  test("the header draws the Job's number, and the whole handle is what copies", async () => {
    await drawn("arc/proposing-review", ".armada-job-head__id");

    // What is drawn is the number. The handle is a branch name and a worktree
    // path, and drawn whole it read as an id that had come out wrong.
    expect(saidBy(".armada-job-head__id [data-copies]")).toBe("3");
    // The whole of it is still here: it is the name, the hover and the copy.
    const id = document.querySelector(".armada-job-head__id [data-copies]");
    expect(id?.getAttribute("aria-label")).toBe("3-show-what-s-running-in-the-drones-stat");
  });
});

// **Where the lead's one act lands.** Until 29 Sep 2026 every act on this lead
// selected a step — the state `InsideAJob` read and the reframe deleted — so
// all four of them were a press that changed nothing on screen. The owner
// pressed `Read what it produced` and said so. A claim here is that the press
// arrives somewhere: the destination, and the row it was sent to open.
describe("the lead's act — what the press reaches", () => {
  test(
    "arc/group-failed: Read what it produced opens the Record on the failed Check's own row",
    async () => {
      mount("arc/group-failed");
      await onScreen();
      await page.getByRole("button", { name: "Read what it produced" }).click();

      await expect.element(page.getByRole("tabpanel", { name: "Record" })).toBeVisible();
      // The row is open, not merely the destination: its heading and the lines
      // the Check printed are what a person came for.
      await expect.element(page.getByRole("heading", { name: "screens_test" })).toBeVisible();
      await expect
        .element(page.getByText("AssertionError: expected 'board' to be 'job'"))
        .toBeVisible();
    },
  );

  // The same act on a Job with no plan under it. **The Check's step is carried
  // and not inferred** — a group boundary's Checks run on the step that works
  // the groups, and this one ran on `regression_verify`, which no plan names.
  test(
    "job/retryingCheckFailure: Read what it produced opens the row of the Check that failed " +
      "on the step it ran on",
    async () => {
      mount("job/retryingCheckFailure");
      await onScreen();
      await page.getByRole("button", { name: "Read what it produced" }).click();

      await expect.element(page.getByRole("tabpanel", { name: "Record" })).toBeVisible();
      await expect.element(page.getByRole("heading", { name: "cargo_nextest" })).toBeVisible();
    },
  );

  test("job/escalatedGateFailure: Read what stopped it opens the Record, newest first", async () => {
    mount("job/escalatedGateFailure");
    await onScreen();
    await page.getByRole("button", { name: "Read what stopped it" }).click();

    await expect.element(page.getByRole("tabpanel", { name: "Record" })).toBeVisible();
    // Unfiltered: no one row is why a Job stopped, so the whole record is the
    // reading and the Check that failed is in it.
    await expect.poll(() => rowsSaying("cargo_nextest")).toBeGreaterThan(0);
  });

  // **The review gate keeps its own arrangement.** What `Review it` would point
  // at is already under the lead, so the lead offers no button over it — the
  // suppression that has held since the gate was given a home.
  test("job/reviewAtDelivery: the decision is under the lead, and no act is offered over it", async () => {
    mount("job/reviewAtDelivery");
    await onScreen();
    await expect
      .element(page.getByRole("region", { name: "Comments on the pull request" }))
      .toBeVisible();

    expect(page.getByRole("button", { name: "Review it" }).query()).toBeNull();
    await expect.element(page.getByRole("tabpanel", { name: "Overview" })).toBeVisible();
  });

  // **Nothing to press where nothing is built.** Fleet serves neither status,
  // so the sentence is all there is — `#1675` found four buttons reaching
  // nothing and the owner kept one route of the four.
  test("job/awaitingRepair and job/awaitingAttestation offer a sentence and no act", async () => {
    await drawn("job/awaitingRepair", ".armada-lead");
    await expect.element(page.getByRole("heading", { name: "Out of retries" })).toBeVisible();
    expect(document.querySelectorAll(".armada-lead button")).toHaveLength(0);

    await drawn("job/awaitingAttestation", ".armada-lead");
    await expect
      .element(page.getByRole("heading", { name: "A criterion needs your attestation" }))
      .toBeVisible();
    expect(document.querySelectorAll(".armada-lead button")).toHaveLength(0);
  });
});

// **One panel, not two.** The owner, 30 Sep 2026, on a lead that said *A Drone
// wants to run a command it was not given / pnpm add -D reselect@5.1.1* over a
// box that said *The drone is waiting on you / The drone wants to run pnpm add
// -D reselect@5.1.1*: *"This panel duplicates what is shown in the panel
// below. Why can't we just have one panel?"*
//
// Measured by containment and by box, never off a sentence: the copy can be
// right while the controls are still a sibling of the panel that names them.
describe("the lead and what it is about are one panel", () => {
  test("job/runningWaitingOnACommand: the answers are inside the lead, not beside it", async () => {
    await drawn("job/runningWaitingOnACommand", ".armada-question");

    const lead = document.querySelector(".armada-lead");
    const answers = document.querySelector(".armada-question");
    expect(lead, "no lead on screen").not.toBeNull();
    expect(lead?.contains(answers as Node)).toBe(true);
    // One panel: nothing answering sits outside the one that says why.
    expect(document.querySelectorAll(".armada-overview-board > .armada-question")).toHaveLength(0);
    // And the radios themselves, not merely the section around them.
    const radio = page.getByRole("radio", { name: "Allow for this job" });
    await expect.element(radio).toBeVisible();
    expect(lead?.contains(await radio.element())).toBe(true);
    // Inside the panel's border, which is what a reader sees as one thing.
    const box = boxOf(".armada-lead");
    const send = boxOf(".armada-question button[type='button']:last-of-type");
    expect(send.bottom).toBeLessThanOrEqual(box.bottom);
  });

  test("job/runningWaitingOnACommand: the box no longer restates what the lead said", async () => {
    await drawn("job/runningWaitingOnACommand", ".armada-question");

    expect(page.getByText("The drone is waiting on you").query()).toBeNull();
    expect(page.getByText(/^The drone wants to run/).query()).toBeNull();
    // The command is said once, on the lead's own second line.
    expect(saidBy(".armada-lead__because")).toContain("pnpm add -D reselect@5.1.1");
  });

  test("markdown/agent-text: the Drone's question draws as markdown, and Armada's line stays plain", async () => {
    await drawn("markdown/agent-text", ".armada-lead__asked");

    await expect
      .element(page.getByRole("heading", { name: "A Drone asked you something" }))
      .toBeVisible();
    const said = document.querySelector(".armada-lead__said") as HTMLElement;
    // Weight, a list and a name in code, drawn rather than spelled.
    expect(said.querySelector("strong")?.textContent).toBe("heading");
    expect([...said.querySelectorAll("li")].map((item) => item.textContent)).toEqual([
      "RunningList hides it today",
      "the plan's brief says draw it",
    ]);
    expect(said.querySelector("code")?.textContent).toBe("RunningList");
    expect(said.textContent).not.toMatch(/\*\*|`/);
  });

  test("job/runningWaitingOnACommand: the elapsed is the lead's, at its top right", async () => {
    await drawn("job/runningWaitingOnACommand", ".armada-lead__elapsed");

    expect(document.querySelectorAll(".armada-question__waiting")).toHaveLength(0);
    const elapsed = boxOf(".armada-lead__elapsed");
    const headline = boxOf(".armada-lead__headline");
    expect(elapsed.left).toBeGreaterThanOrEqual(headline.right);
    expect(elapsed.top).toBeLessThan(headline.bottom);
  });

  test("job/reviewAtDelivery: the review gate is inside the lead, and its record is open", async () => {
    await drawn("job/reviewAtDelivery", ".armada-verdict");

    const lead = document.querySelector(".armada-lead");
    expect(lead?.contains(document.querySelector(".armada-verdict") as Node)).toBe(true);
    // **The face, not the entry.** `Approve the work` was a button of its own
    // until the review gate took two split buttons on 30 Sep 2026; it is
    // behind Merge's caret now and is not in the document until the caret is
    // opened. What this claim is about is where the decision sits, and the
    // face is the part of it a person sees without pressing anything.
    const merge = await page.getByRole("button", { name: "Merge and take the work" }).element();
    expect(lead?.contains(merge)).toBe(true);
    // Open since the owner took the fold away, 2 Oct 2026 (#1680).
    const proves = await page.getByRole("region", { name: "What proves it" }).element();
    expect(lead?.contains(proves)).toBe(true);
  });
});

// **The second line is a line or it is not there**, which is the part only the
// screen can say: `job/reviewAtDelivery` drew the headline over nothing at all
// until the lead counted the whole Job, and an element with an empty string in
// it would have passed every sentence claim in `lead.test.ts`.
describe("what the gates found, on the lead's own second line", () => {
  test("job/reviewAtDelivery: the review gate says what the whole Job's evidence came to", async () => {
    await drawn("job/reviewAtDelivery", ".armada-lead__because");

    expect(saidBy(".armada-lead__because")).toBe(
      "All 3 Checks passed and the Judge met both criteria",
    );
    // Under the headline it belongs to, and inside the one panel.
    const because = boxOf(".armada-lead__because");
    expect(because.top).toBeGreaterThanOrEqual(boxOf(".armada-lead__headline").top);
    expect(because.bottom).toBeLessThanOrEqual(boxOf(".armada-lead").bottom);
  });

  test("job/reviewAfterAnOverrule: a refusal somebody overruled is counted, not laundered", async () => {
    await drawn("job/reviewAfterAnOverrule", ".armada-lead__because");

    expect(saidBy(".armada-lead__because")).toBe(
      "All 3 Checks passed and the Judge met 1 of 2 criteria",
    );
  });

  test("job/awaitingApproval: a lead with no fact under it draws no second line", async () => {
    await drawn("job/awaitingApproval", ".armada-lead");

    expect(document.querySelectorAll(".armada-lead__because")).toHaveLength(0);
  });
});

describe("the Land board — Overview for a Job that finished", () => {
  test("how it was answered leads, and what it produced reads beside what it cost", async () => {
    await drawn("arc/landed", ".armada-land__cost");

    await expect.element(page.getByText("Landed", { exact: true })).toBeVisible();
    expect(boxOf(".armada-land__lead").bottom).toBeLessThanOrEqual(
      boxOf(".armada-land__produced").top,
    );
    const produced = boxOf(".armada-land__produced");
    const cost = boxOf(".armada-land__cost");
    expect(cost.left).toBeGreaterThanOrEqual(produced.right);
    expect(cost.top).toBeLessThan(produced.bottom);
  });
});
