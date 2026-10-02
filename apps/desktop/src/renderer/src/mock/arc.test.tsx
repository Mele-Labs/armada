// The milestone's claim: what a person sees at each moment of the arc, through
// `App` and nothing else.
//
// **Written before the screens, and every one of them now owed nothing.**
// Each claim below was an `it.todo` first — one moment of #1533's roster, in
// the words of what somebody looking at the window should be able to read —
// and the issue that built each board turned its own into a passing test:
// #1535 the Plan, #1536 Implement, #1537 the Record, #1538 Pulse, #1539 the
// canvas, #1540 Dispatch, #1541 classifying, #1542 Land, #1543 landing in
// order, #1544 the wave. The wave was the last of them.
//
// **A claim names what a person sees, never a component.** A claim written
// against `RunningPanel` would have to be rewritten by whoever renames it; one
// written against what is on screen survives the rename and fails the day the
// sentence stops being true.

import { expect, test, describe, vi } from "vitest";
import { page } from "vitest/browser";

import { issueLink } from "@armada/protocol";

import {
  GUIDE_ALWAYS_LOOKS,
  GUIDE_LOOK,
  GUIDE_MEMBER_LINK,
  GUIDE_PLAN_ASKS,
  GUIDE_TIERS,
} from "@armada/components";
import type { Guide } from "@armada/components";

import { SCENARIOS, scenarioNamed } from "./scenario";
import { entered, mount, listed, rows, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * A moment, with one of the Job's destinations open. **The tab is
 * pressed the way a person presses it** — the strip is the whole of navigation
 * inside a Job, so a test that reached a destination any other way would pass
 * over a strip that had stopped working.
 */
async function at(moment: string, tab: string) {
  mount(moment);
  await expect.element(page.getByRole("tab", { name: new RegExp(`^${tab}`) })).toBeVisible();
  await page.getByRole("tab", { name: new RegExp(`^${tab}`) }).click();
  await expect.element(page.getByRole("tabpanel", { name: tab })).toBeVisible();
}

/**
 * One group's card, by the name the card carries.
 *
 * **By its name and never by its text.** A card now says `Group 4 writes these
 * files too` where it overlaps group 4, so a text match for group 4 reaches
 * whichever card mentions it first.
 */
const groupCard = (ordinal: number) =>
  page.getByRole("listitem", { name: `Group ${ordinal}`, exact: true });

/** One task's row, by the name the board gives it. */
const taskRow = (id: string) => page.getByRole("listitem", { name: new RegExp(`^${id} `) });

/**
 * Pull one of a group's boundary strips open. **A press, because what is
 * behind one is a list** — the strip carries the kind, how many and what it
 * came to, and the names wrapping under a bar was the miss the owner named on
 * 28 Sep 2026.
 */
async function openStrip(ordinal: number, kind: "Checks" | "Tests") {
  await groupCard(ordinal).getByRole("button", { name: new RegExp(`^${kind}`) }).click();
}

/** One guide's `?`, by the name `GuideMark` gives it. */
const markFor = (guide: Guide) =>
  page.getByRole("button", { name: `Open guide ${guide.number}, ${guide.title}` });

/** The `?` over the group cards, which is all the board says about an ask. */
const planAsksMark = () => markFor(GUIDE_PLAN_ASKS);

/** Every scenario the arc put in the picker, by name. */
const ARC = SCENARIOS.filter((one) => one.name.startsWith("arc/")).map((one) => one.name);

/**
 * Every arc moment the approval press is behind, up to the merge.
 *
 * **`arc/landed` is not here**, and that is the one exception rather than an
 * omission: a Job that is over is read for what it came to, so Land takes this
 * destination. Every other moment past the press is a Job running on values
 * nobody may move.
 */
const PAST_THE_PRESS = [
  "arc/approved-frozen",
  "arc/planned",
  "arc/plan-revision-refused",
  "arc/executing-sequential",
  "arc/executing-concurrent",
  "arc/group-failed",
  "arc/done-touched",
];

// What can be claimed today: each moment loads, and the window draws rather
// than blanking. The boards are what the todos below wait on.
describe("every arc moment loads", () => {
  test.for(ARC)("%s draws a window", async (name) => {
    const { scenario } = mount(name);
    expect(scenario.draft).toBeDefined();
    await expect.element(page.getByRole("navigation").first()).toBeVisible();
  });

  test("the picker's moments are in the order the work happens", () => {
    expect(ARC[0]).toBe("arc/dispatch-typing");
    expect(ARC[ARC.length - 1]).toBe("arc/landed");
  });

  test.for(["members/stacked", "members/merged", "epic/wave", "kinds"])(
    "%s draws a window",
    async (name) => {
      expect(scenarioNamed(name), `no scenario named ${name}`).toBeDefined();
      mount(name);
      await expect.element(page.getByRole("navigation").first()).toBeVisible();
    },
  );

  test("the kinds Board opens every Job it holds", async () => {
    const { scenario } = mount("kinds");
    await listed();
    const done = page.getByRole("button", { name: /^Done/ });
    if (done.query()?.getAttribute("aria-expanded") === "false") await done.click();
    await expect
      .poll(() => rows().map((row) => row.dataset.jobId).sort())
      .toEqual(scenario.state.jobs.map((job) => job.id).sort());
  });
});

// What the first destination says, with nothing pressed. **The claims below
// press a tab; this one refuses to**, because the defect it stands against was
// invisible to every one of them: Overview is where a person lands, and a
// moment that fell through to an older arrangement drew it without a single
// claim noticing (the owner did, 25 Sep 2026).
//
// **It said the frozen board until 29 Sep 2026**, and that was the next
// defect: he opened a Job whose group had failed and the screen drew the
// configuration he approved. Overview shows what needs him now, led by one
// sentence, and the frozen values are Settings'.
describe("what Overview opens on", () => {
  test.for(PAST_THE_PRESS)(
    "%s: Overview leads with one sentence saying where the Job is, and draws none of what " +
      "froze at approval",
    async (name) => {
      expect(ARC, `${name} is not one of the arc's scenarios`).toContain(name);
      mount(name);

      // Named, not the first tabpanel on screen: the arrangement this replaced
      // carried no tabpanel at all, so a positional locator would have read a
      // sheet's panel and passed against the defect.
      const overview = page.getByRole("tabpanel", { name: "Overview" });
      await expect.element(overview).toBeVisible();
      // The lead, whatever it says: every one of these moments has a sentence
      // and it is the first thing in the panel.
      await expect.element(overview.getByRole("heading", { level: 2 }).first()).toBeVisible();
      // The two sentences only the frozen board says, and it is not here.
      await expect.element(overview).not.toHaveTextContent(/Frozen\. This is what every Drone is given\./);
      expect(overview.getByRole("region", { name: "Done when" }).all()).toHaveLength(0);
    },
  );
});

// What Settings holds once the press is behind. **Only what no other
// destination draws** — the owner's call of 29 Sep 2026, so the request stays
// the brief's and the criteria stay Plan's.
describe("what froze at approval", () => {
  test.for(PAST_THE_PRESS)(
    "%s: Settings says when it froze, and holds the gates, how it lands and the tier models",
    async (name) => {
      mount(name);
      await page.getByRole("tab", { name: /^Settings/ }).click();

      const settings = page.getByRole("tabpanel", { name: "Settings" });
      const froze = settings.getByRole("region", { name: "Frozen at approval" });
      await expect.element(froze).toHaveTextContent(/Nothing here changes while the Job runs\./);
      await expect.element(froze).toHaveTextContent("feature");
      await expect.element(froze).toHaveTextContent("One branch for the whole Job");
      await expect.element(froze).toHaveTextContent("Difficult");
      // Frozen is values, never fields a person could move.
      expect(froze.getByRole("checkbox").all()).toHaveLength(0);
      expect(froze.getByRole("combobox").all()).toHaveLength(0);
    },
  );
});

// The moment before this one — a request dispatched and not yet read — is a Job
// on the Board rather than a form waiting on it, so what a person sees there is
// `proposing.test.tsx`'s to claim.
describe("classifying", () => {
  test(
    "arc/proposing-review: the workflow the proposer chose is named with its four steps, and " +
      "every one of them has a gate row a person can still change",
    async () => {
      mount("arc/proposing-review");

      // The picker and the gate rows are one region since 28 Sep 2026: you
      // choose the workflow, and its steps are what is under it (`2b4j`).
      const workflow = page.getByRole("region", { name: "Workflow" });
      await expect.element(workflow).toBeVisible();
      await expect
        .element(workflow.getByRole("combobox", { name: "Workflow" }))
        .toHaveValue("feature");
      await expect.element(workflow).toHaveTextContent("feature — 4 steps");

      // A Judge on the plan step and no Check, which is what `feature.json`
      // declares — and the box is a person's to move.
      const judge = page.getByRole("checkbox", { name: "Judge on Plan the change" });
      await expect.element(judge).toBeChecked();
      const checks = page.getByRole("checkbox", { name: "Checks on Plan the change" });
      await expect.element(checks).not.toBeChecked();
      await checks.click();
      await expect.element(checks).toBeChecked();
      // A tick moves the gate and never what the step declares, so asking for
      // a Check on a step that declares none says so rather than looking done.
      await expect
        .element(page.getByRole("listitem", { name: "Plan the change" }))
        .toHaveTextContent("declares no Check");
    },
  );

  test(
    "arc/proposing-review: the handoff step reads that the repository decides, which is not " +
      "one of the three tick boxes beside it",
    async () => {
      mount("arc/proposing-review");

      const handoff = page.getByRole("listitem", { name: "Review the change" });
      await expect.element(handoff).toHaveTextContent("The repository decides — review_gate");
      // `advance_gate` is off the screen since 28 Sep 2026: it is Fleet's own
      // spelling, in a place a person is deciding.
      await expect.element(handoff).not.toHaveTextContent("manifest_rule:review_gate");
      // The three boxes are drawn on every step that answers for itself and on
      // no step that defers, so the fourth state is a state and not a tick.
      expect(handoff.getByRole("checkbox").all()).toHaveLength(0);

      await page.getByRole("button", { name: "Decide it for this Job" }).click();
      await expect
        .element(page.getByRole("checkbox", { name: "You on Review the change" }))
        .toBeVisible();
    },
  );

  test(
    "arc/proposing-review: the gates region says what each step's gate is and nothing about " +
      "what no tick turns off — that is behind its mark, whatever is ticked",
    async () => {
      mount("arc/proposing-review");

      // The mark first, and not only because it is the claim: an assertion
      // that something is absent passes against a window that has not drawn
      // yet, so the negative below has to stand behind a positive.
      await expect.element(markFor(GUIDE_ALWAYS_LOOKS)).toBeVisible();
      // The line that used to stand over the boxes is off the screen: it was
      // true of a Job nobody had approved, so it is guide 9 (#1602).
      await expect
        .element(page.getByText(/Fleet checks that the work stayed inside/))
        .not.toBeInTheDocument();

      // Every box off on a step, and the mark is still there — which is the
      // claim the sentence used to carry: it is not a summary of the ticks.
      const judge = page.getByRole("checkbox", { name: "Judge on Plan the change" });
      await judge.click();
      await expect.element(judge).not.toBeChecked();
      await expect
        .element(page.getByRole("listitem", { name: "Plan the change" }))
        .toHaveTextContent("Nothing stops it.");
      await expect.element(markFor(GUIDE_ALWAYS_LOOKS)).toBeVisible();
    },
  );

  test(
    "arc/proposing-review: each of the three tiers names the model it resolves to, and what a " +
      "tier is stays behind the mark on the label",
    async () => {
      mount("arc/proposing-review");

      await expect.element(page.getByRole("combobox", { name: "Difficult" })).toHaveValue("opus");
      await expect.element(page.getByRole("combobox", { name: "Medium" })).toHaveValue("sonnet");
      await expect.element(page.getByRole("combobox", { name: "Easy" })).toHaveValue("haiku");
      await expect
        .element(page.getByText(/The planner marks each task/))
        .not.toBeInTheDocument();
      await expect.element(markFor(GUIDE_TIERS)).toBeVisible();

      // This Job's share of the machine, beside what the machine allows.
      await expect.element(page.getByRole("spinbutton", { name: "Drones at once" })).toHaveValue(2);
      // The machine's cap says what it costs this Job, rather than only
      // stating itself (`pojb`).
      await expect
        .element(page.getByText(/This machine runs 4 Drones at once across every Job/))
        .toBeVisible();
      await expect
        .element(page.getByText(/gives this one fewer than you ask for here/))
        .toBeVisible();
    },
  );

  test(
    "arc/approved-frozen: Settings says when it was approved, and draws the tiers as values",
    async () => {
      mount("arc/approved-frozen");
      await page.getByRole("tab", { name: /^Settings/ }).click();

      // The instant sits in the card's own head since 29 Sep 2026, named by
      // its tooltip rather than by a clause. It is written out in the reader's
      // own locale, so what is asserted is that it is there and dated — never
      // its spelling.
      await expect
        .poll(() => document.querySelector(".armada-settings-tab__frozen-at")?.textContent)
        .toMatch(/\d{4}/);
      // Frozen is drawn as values rather than as fields nobody may move.
      const froze = page.getByRole("region", { name: "Frozen at approval" });
      expect(froze.getByRole("checkbox").all()).toHaveLength(0);
      expect(froze.getByRole("combobox", { name: "Difficult" }).all()).toHaveLength(0);
      await expect.element(froze).toHaveTextContent("Difficult");
    },
  );

  test(
    "arc/approved-frozen: the first criterion says the issue it came from has been edited " +
      "since, and the Job still shows the words it froze",
    async () => {
      window.localStorage.removeItem("armada.bridge.plan-lead-open");
      mount("arc/approved-frozen");
      // **Plan leads with what the Job is held to**, and has since the owner
      // asked why it did not. Overview stopped drawing it a second time on
      // 29 Sep 2026, so this is the one place it reads.
      await page.getByRole("tab", { name: /^Plan/ }).click();

      // **Plan's own lead card, not `Done when`.** That region is the
      // proposal's, and the proposal is Settings' once a Job is approved.
      const held = page.getByRole("tabpanel", { name: "Plan" });
      await expect.element(held).toHaveTextContent("What this Job is held to");
      // **Shut to one line by default** (29 Sep 2026): the band and the first
      // criterion. Where it came from is behind the press.
      await expect
        .element(held)
        .toHaveTextContent("The rail's Drones stat reads one running beside the machine's most");
      await expect.element(held).not.toHaveTextContent("From issue armada/1162");
      await held.getByRole("button", { name: /What this Job is held to/, expanded: false }).click();
      // The word `issue` is on the line: a bare `owner/number` is a
      // repository, a path and a branch as readily as an issue (`u7y9`).
      await expect.element(held).toHaveTextContent("From issue armada/1162");
      await expect.element(held).toHaveTextContent(/The issue has been edited since/);
      // The press is remembered per viewer; the next claim starts shut.
      window.localStorage.removeItem("armada.bridge.plan-lead-open");
    },
  );

  test(
    "arc/approved-frozen: the handoff gate reads that this Job overrode the repository's rule " +
      "for itself",
    async () => {
      mount("arc/approved-frozen");
      await page.getByRole("tab", { name: /^Settings/ }).click();

      const handoff = page.getByRole("listitem", { name: "Review the change" });
      await expect
        .element(handoff)
        .toHaveTextContent("This Job decides this step for itself, in place of the repository's review_gate.");
      // `advance_gate` is off the screen; what a person reads is who looks.
      await expect.element(handoff).not.toHaveTextContent("human_always");
      await expect.element(handoff).toHaveTextContent("You");
    },
  );
});

/**
 * The Plan destination, as its list. **The list, not the graph**: Plan opens on
 * the graph since 25 Sep 2026, and every claim below is about what a group's
 * card says — which is the list's job. Pressed rather than assumed, because the
 * arrangement is remembered per viewer and another file's press would otherwise
 * decide this one.
 */
async function planList(moment: string) {
  await at(moment, "Plan");
  await page.getByRole("tab", { name: "List" }).click();
  await expect.element(page.getByRole("list", { name: "Groups, in the order they run" })).toBeVisible();
}

describe("the plan", () => {
  test("arc/planned: the Plan tab draws four groups in the order they run, with eight tasks under them and no task in two groups", async () => {
    await planList("arc/planned");
    const groups = page.getByRole("list", { name: "Groups, in the order they run" });
    await expect.element(groups).toBeVisible();
    const headings = [...groups.element().querySelectorAll("h3")].map((one) => one.textContent);
    expect(headings).toEqual(["Group 1", "Group 2", "Group 3", "Group 4"]);
    const ids = [...groups.element().querySelectorAll("[aria-label$='tasks'] > li")].map(
      (one) => one.getAttribute("aria-label")?.split(" ")[0],
    );
    expect(ids).toEqual(["T1", "T2", "T3", "T4", "T5", "T6", "T7", "T8"]);
  });

  test("arc/planned: each task names the files it will touch, the tier the planner gave it, and the model that tier resolved to", async () => {
    await planList("arc/planned");
    await expect.element(taskRow("T1")).toHaveTextContent("opus");
    await expect.element(taskRow("T3")).toHaveTextContent("haiku");
    // **Where the group writes, not every file it writes** — the owner, 28 Sep
    // 2026. The root is the boundary the gate measures a diff against; the
    // paths themselves are in the task's own inspector.
    await expect.element(groupCard(1)).toHaveTextContent("crates/**");
    await expect.element(groupCard(1)).not.toHaveTextContent("crates/api/src/running.rs");
  });

  test("arc/planned: each group says which Checks run at its end — four where it writes Rust, seven where it writes Bridge", async () => {
    await planList("arc/planned");
    // No clause about where they run (owner, 30 Sep 2026: *just fluff*).
    await expect.element(groupCard(1).getByRole("region", { name: "Checks at this boundary" })).toBeVisible();
    await expect.element(groupCard(1)).not.toHaveTextContent("will run at this boundary");
    // The count is on the strip and the names are behind its press.
    await openStrip(1, "Checks");
    await expect.element(groupCard(1)).toHaveTextContent("acceptance");
    await openStrip(2, "Checks");
    await expect.element(groupCard(2)).toHaveTextContent("components_test");
  });

  test("arc/planned: the case that covers a file two groups touch is drawn at the last of them, and the case with no spec reads as not covered rather than as passing", async () => {
    await planList("arc/planned");
    await openStrip(4, "Tests");
    await openStrip(2, "Tests");
    await expect.element(groupCard(4)).toHaveTextContent("overview.test.ts");
    await expect.element(groupCard(2)).not.toHaveTextContent("overview.test.ts");
    await expect.element(groupCard(2)).toHaveTextContent("Board.test.tsx");
    await expect.element(groupCard(2)).toHaveTextContent("not covered");
  });

  test("arc/planned: a task's inspector says what its Drone will be told, what it may run beside and the tests it owes", async () => {
    await planList("arc/planned");
    await taskRow("T5").getByRole("button").first().click();
    const sheet = page.getByRole("dialog").first();
    // The panel's words, rewritten on 28 Sep 2026 — *everything on this task
    // panel sounds like an AI bot phrased it*.
    await expect.element(sheet).toHaveTextContent("Brief");
    await expect.element(sheet).toHaveTextContent("Keep the four lists in this order");
    await expect.element(sheet).toHaveTextContent("difficult · opus");
    await expect.element(sheet).toHaveTextContent("Runs beside");
    await expect.element(sheet).toHaveTextContent("T6");
    await expect.element(sheet).toHaveTextContent("Tests for this task");
    await expect.element(sheet).toHaveTextContent("Running.test.tsx");
    await expect.element(sheet).toHaveTextContent("Done when");
    await expect.element(sheet).not.toHaveTextContent("What the planner holds it to");
  });

  /**
   * **A break test on where.** The same sentence sat in a band above the plan
   * naming two groups, and the owner asked for it inside the group instead
   * (28 Sep 2026) — *in Group 4 it should have a warning callout*. Every word
   * was on the page before; only the card it sits on changed.
   */
  test("arc/planned: a file two groups claim is a warning inside each of them, and nowhere else", async () => {
    await planList("arc/planned");
    expect(
      await page.getByRole("region", { name: "Two groups claim the same file" }).elements(),
    ).toHaveLength(0);
    await expect.element(groupCard(4)).toHaveTextContent("Group 3 writes these files too");
    await expect.element(groupCard(4)).toHaveTextContent("running-rows.tsx");
    await expect.element(groupCard(3)).toHaveTextContent("Group 4 writes these files too");
    await expect.element(groupCard(1)).not.toHaveTextContent("writes these files too");
  });

  test("arc/group-failed: the Plan tab draws group three's failure with the one Check that failed named, and the six that passed beside it", async () => {
    await planList("arc/group-failed");
    await expect.element(groupCard(3)).toHaveTextContent("retrying");
    await expect.element(groupCard(3)).toHaveTextContent("attempt 2");
    await expect.element(groupCard(3)).toHaveTextContent("screens_test");
    await expect.element(groupCard(3)).toHaveTextContent("typecheck");
    await expect.element(taskRow("T6")).toHaveTextContent("opened the Board rather than the Job");
  });

  test("arc/done-touched: T6 still reads done on the Plan tab and carries a flag naming T7", async () => {
    await planList("arc/done-touched");
    await expect.element(taskRow("T6")).toHaveTextContent("touched later · T7");
    await expect.element(taskRow("T6").getByText("Done")).toBeInTheDocument();
    await expect.element(groupCard(3)).toHaveTextContent("passed");
    await expect.element(taskRow("T7")).toHaveTextContent("6 turns");
  });

  test("arc/done-touched: a finished task shows what its own agent cost and a working one does not", async () => {
    await planList("arc/done-touched");
    await expect.element(taskRow("T1")).toHaveTextContent("34 turns");
    await expect.element(taskRow("T1")).toHaveTextContent("~$2.40");
    await expect.element(taskRow("T7")).not.toHaveTextContent("$");
  });

  test("arc/plan-revision-refused: the Judge's refusal names the one task that was revised, and the other seven are untouched beside it", async () => {
    await planList("arc/plan-revision-refused");
    const asked = page.getByRole("region", {
      name: "What you asked the plan's Drone to change",
    });
    await expect.element(asked).toHaveTextContent("out of T5");
    await expect.element(asked).toHaveTextContent("Refused");
    await expect
      .element(asked)
      .toHaveTextContent("the plan names every file the panel's rows are drawn from");
    await expect.element(asked).toHaveTextContent("no other task claims it");
    await expect
      .element(asked)
      .toHaveTextContent("T5 is the one task the ask touched. The other 7 stand");
    // The plan itself is the one the Drone recorded: eight tasks, four groups.
    const ids = [
      ...page
        .getByRole("list", { name: "Groups, in the order they run" })
        .element()
        .querySelectorAll("[aria-label$='tasks'] > li"),
    ].map((one) => one.getAttribute("aria-label")?.split(" ")[0]);
    expect(ids).toEqual(["T1", "T2", "T3", "T4", "T5", "T6", "T7", "T8"]);
  });

  test("arc/plan-revision-refused: the case that fell out of the revised scope reads dropped, with the revision that dropped it", async () => {
    await planList("arc/plan-revision-refused");
    await openStrip(4, "Tests");
    await expect.element(groupCard(4)).toHaveTextContent("Running.test.tsx");
    await expect.element(groupCard(4)).toHaveTextContent("dropped by a scope revision");
    await expect
      .element(page.getByRole("region", { name: "What you asked the plan's Drone to change" }))
      .toHaveTextContent("It drops packages/screens/src/Running.test.tsx.");
  });

  test("arc/plan-revision-refused: a group offers Remove and Propose a change while the plan waits", async () => {
    await planList("arc/plan-revision-refused");
    const asks = page.getByRole("group", { name: "Ask about group 1" });
    await expect.element(asks.getByRole("button", { name: "Remove" })).toBeEnabled();
    await expect.element(asks.getByRole("button", { name: "Propose a change" })).toBeEnabled();
    // The board says nothing about what an ask is. The `?` on the groups' own
    // head is where that went — #1602. The head is the noun and nothing more:
    // *The groups, in the order they run* would be the removed sentence in a
    // label's clothes.
    const head = page.getByRole("heading", { name: "Groups", exact: true }).last();
    await expect.element(head).toHaveTextContent(/^Groups$/);
    await expect.element(planAsksMark()).toBeVisible();
    await expect
      .element(page.getByRole("tabpanel", { name: "Plan" }))
      .not.toHaveTextContent("The plan is the Drone's record");
  });

  test("arc/plan-revision-refused: a task's inspector proposes a change, and the control is off until something is typed", async () => {
    await planList("arc/plan-revision-refused");
    await taskRow("T6").getByRole("button").first().click();
    const sheet = page.getByRole("dialog").first();
    await sheet.getByRole("button", { name: "Propose a change" }).click();
    const send = sheet.getByRole("button", { name: "Send to the Drone" });
    await expect.element(send).toBeDisabled();
    await sheet.getByRole("textbox").first().fill("Split the rows out of this one");
    await expect.element(send).toBeEnabled();
  });

  test("arc/planned: a plan already past its gate offers no changes at all, and draws no mark about asking", async () => {
    await planList("arc/planned");
    await expect.element(page.getByRole("tabpanel", { name: "Plan" })).toBeVisible();
    expect(page.getByRole("group", { name: "Ask about group 1" }).elements()).toHaveLength(0);
    expect(planAsksMark().elements()).toHaveLength(0);
  });
});

// The nine claims about a plan that is running are in
// `arc-implement.test.tsx`, live. The owner decided on 28 Sep 2026 that the
// board's reading moves under the Plan tab's List view, and this file is at its
// ceiling.

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

/** App on an arc moment, with the Record open. */
async function record(name: string): Promise<void> {
  mount(name);
  await page.getByRole("tab", { name: /^Record/ }).click();
  await expect.poll(() => ledger().length).toBeGreaterThan(0);
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
  const columns = [...table.querySelectorAll("thead th")].map((one) => one.textContent?.trim() ?? "");
  return [...table.querySelectorAll("tbody tr")].map((row) =>
    Object.fromEntries(
      [...row.querySelectorAll("td")].map((cell, at) => [
        columns[at] ?? String(at),
        cell.textContent?.trim() ?? "",
      ]),
    ),
  );
}

/** Who ran the one row whose What matches. The column is `Who`: the owner cut
    the longer heading, and a merge left these three claims reading the old one. */
function whoSaid(rows: Record<string, string>[], what: RegExp): string | undefined {
  return rows.find((row) => what.test(row["What"] ?? ""))?.["Who"];
}

describe("Pulse", () => {
  test("arc/executing-sequential: Pulse names the one process the running Drone holds, the worktree it belongs to, and the instant every figure was read at", async () => {
    mount("arc/executing-sequential");
    await onPulse();

    const processes = page.getByRole("region", { name: "Processes" });
    // Exact: the row's kill names the pid too, in its hidden description.
    await expect.element(processes.getByText("52118", { exact: true })).toBeVisible();
    // The branch is on the process row and on the worktree row, which is the
    // whole of "the worktree it belongs to": one occurrence is a table that
    // lists what is running and does not say where.
    expect(processes.getByText(ARC_BRANCH).all()).toHaveLength(1);
    await expect
      .element(page.getByRole("region", { name: "Worktrees" }).getByText(ARC_BRANCH))
      .toBeVisible();
    await expect.element(page.getByText(/^Updated .* ago$/)).toBeVisible();
  });

  test("arc/executing-concurrent: with no agent running, Pulse says so rather than drawing an empty table", async () => {
    mount("arc/executing-concurrent");
    await onPulse();

    await expect
      .element(page.getByText(/Fleet holds no process for this job/))
      .toBeVisible();
    expect(page.getByRole("columnheader", { name: /Process/ }).query()).toBeNull();
  });

  // The parent of three landings holds a plan and no checkout of its own, so
  // every one of Pulse's lists is empty here — which is the state each of them
  // has its own sentence for. It is also the one moment that proves none of
  // them draws a bare empty table. Nothing on screen names a shape (#1530).
  test("members/stacked: the parent says what it holds rather than drawing three empty lists", async () => {
    mount("members/stacked");
    await onPulse();

    await expect.element(page.getByText(/the process table would not read/)).toBeVisible();
    await expect.element(page.getByText("No worktree on disk.")).toBeVisible();
    await expect.element(page.getByText(/Nothing has been written to this job's logs/)).toBeVisible();
    expect(document.body.textContent).not.toMatch(/convoy|train/i);
  });

  test(
    "arc/executing-sequential: Pulse says when it was read, and nothing about how often or " +
      "what a look is — that is the mark beside the act",
    async () => {
      mount("arc/executing-sequential");
      await onPulse();

      // The two standing sentences that used to ride here. Both were true of a
      // Job that never ran, and both are guide 14 now (#1602).
      await expect
        .element(page.getByText(/A process can exit between the reading and this screen/))
        .not.toBeInTheDocument();
      await expect.element(page.getByText(/Looking costs no model call/)).not.toBeInTheDocument();
      // What stays is the reading's age. How often it is taken again went too
      // (owner, 29 Sep): the head says `Updated 4s ago` and nothing beside it.
      await expect.element(page.getByText(/^Updated .* ago$/)).toBeVisible();
      await expect.element(page.getByText(/every 10s while open/)).not.toBeInTheDocument();
      await expect.element(markFor(GUIDE_LOOK)).toBeVisible();
      await expect.element(page.getByRole("button", { name: "Refresh" })).toBeVisible();
    },
  );
});

/** The branch the arc's one Job works on — `arc-base.ts`'s own spelling. */
const ARC_BRANCH = "armada/3-show-what-s-running-in-the-drones-stat";

/** Open Pulse on the Job the moment opens, the way a person reaches it. */
async function onPulse(): Promise<void> {
  await page.getByRole("tab", { name: "Pulse" }).click();
  await expect.element(page.getByRole("tabpanel", { name: "Pulse" })).toBeVisible();
}

describe("landing", () => {
  test(
    "arc/landed: Land shows the test runs, with the one case that has no spec named as not covered",
    async () => {
      mount("arc/landed");
      await expect.element(page.getByText("Test runs", { exact: true })).toBeVisible();
      for (const spec of [
        "crates/api/src/tests/running.rs",
        "packages/screens/src/overview.test.ts",
        "packages/screens/src/Running.test.tsx",
        "packages/screens/src/Board.test.tsx",
      ]) {
        await expect.element(page.getByText(spec, { exact: true }).first()).toBeVisible();
      }
      // The case with no spec says so in words, and nothing beside it reads as a pass.
      await expect.element(page.getByText("not covered").first()).toBeVisible();
      await expect.element(page.getByText("no spec covers Board.tsx")).toBeVisible();
    },
  );

  test(
    "arc/landed: the run a person made themselves is drawn in the same table as Fleet's, and " +
      "says which of them ran it",
    async () => {
      mount("arc/landed");
      await expect.element(page.getByText("Test runs", { exact: true })).toBeVisible();
      await expect.element(page.getByText("Fleet").first()).toBeVisible();
      await expect.element(page.getByText("you", { exact: true }).first()).toBeVisible();
    },
  );

  test("arc/landed: the pull request is on screen as an address a press opens", async () => {
    const { api } = mount("arc/landed");
    const opened: string[] = [];
    const was = api.openPullRequest;
    api.openPullRequest = async (jobId: string) => {
      opened.push(jobId);
      return was(jobId);
    };
    // The header carries the same address on its `#1604` link, so the board's
    // row is named by the text it draws rather than by the title both share.
    const address = page.getByText("https://git.example/armada/pull/1604", { exact: true });
    await expect.element(address).toBeVisible();
    // `exact`, because a role name given as a string matches a substring: the
    // `?` beside the verb is named `Open guide 1, …` and sits earlier in the
    // document, so `.first()` was pressing it — #1602.
    await page.getByRole("button", { name: "Open", exact: true }).first().click();
    await expect.poll(() => opened).toHaveLength(1);
  });

  test("arc/landed: what it cost counts the agents and the Checks the retry ran again", async () => {
    mount("arc/landed");
    await expect.element(page.getByText("Drones", { exact: true }).first()).toBeVisible();
    // Eight tasks and a retried group of two: ten agents, and its seven Checks twice.
    await expect.element(page.getByText("10", { exact: true }).first()).toBeVisible();
    await expect.element(page.getByText("32", { exact: true }).first()).toBeVisible();
    await expect.element(page.getByText(/group three ran again/)).toBeVisible();
  });

  test("arc/landed: the board says what it left behind", async () => {
    mount("arc/landed");
    await expect.element(page.getByText("Left behind", { exact: true })).toBeVisible();
    await expect
      .element(page.getByText(".armada/worktrees/3-show-what-s-running-in-the-drones-stat"))
      .toBeVisible();
    // What reclaiming a checkout does is true of a Job that never ran, so the
    // section names what is there and says nothing about it — guide 13 (#1602).
    await expect
      .element(page.getByText(/Reclaiming the worktree takes the checkout back/))
      .not.toBeInTheDocument();
  });
  test("members/stacked: three pull requests are drawn in the order they land — one merged, one waiting on you, one stacked on it — and nothing on screen names a shape", async () => {
    mount("members/stacked");
    const order = page.getByRole("list", { name: "Pull requests, in the order they land" });
    await expect.element(order).toBeVisible();

    const cards = order.getByRole("listitem").elements();
    expect(cards.map((card) => card.getAttribute("aria-label"))).toEqual([
      "1. Give the store one shape",
      "2. Read the store through selectors",
      "3. Drop the store singleton",
    ]);

    // The first is in, the second is a person's to answer, and the third is
    // branched off the second and still working.
    await expect.element(member(1).getByText("merged")).toBeVisible();
    await expect.element(member(2).getByText("awaiting review")).toBeVisible();
    await expect.element(member(3).getByText(/branches off member 2/)).toBeVisible();
    // Stacked means its pull request targets the branch before it, and not main.
    await expect
      .element(member(3).getByText(/armada\/23-read-the-store-through-selectors/))
      .toBeVisible();
  });

  test(
    "members/stacked: a card says which link it carries and never what the three links mean — " +
      "that is the one mark over the band",
    async () => {
      mount("members/stacked");
      await expect
        .element(page.getByRole("list", { name: "Pull requests, in the order they land" }))
        .toBeVisible();

      // What `stacked` does while the branch under it is open was the second
      // half of the card's own sentence. It is guide 2 now (#1602).
      await expect
        .element(page.getByText(/keeps working, and rebases when that branch lands/))
        .not.toBeInTheDocument();
      await expect.element(markFor(GUIDE_MEMBER_LINK)).toBeVisible();
      // One mark for the band, not one per member: three cards carry links.
      expect(markFor(GUIDE_MEMBER_LINK).elements()).toHaveLength(1);
    },
  );

  test("members/merged: the third member reads as merged into the one before it and parked as a draft, so nothing is asked of a reviewer yet", async () => {
    mount("members/merged");
    await expect
      .element(page.getByRole("list", { name: "Pull requests, in the order they land" }))
      .toBeVisible();

    await expect.element(member(3).getByText(/waits for member 2 to merge/)).toBeVisible();
    // Parked rather than stacked: it targets where the Job lands, which the
    // join says once, and it has opened nothing for anybody to review.
    await expect.element(member(3).getByText(/targets main/)).toBeVisible();
    await expect.element(member(3).getByText("No pull request yet")).toBeVisible();
  });

  test("members/stacked: the parent says it is done when every member has landed, which is not the same as its own pull request merging", async () => {
    mount("members/stacked");

    // The second member reached `awaiting_review` and the third is running;
    // one pull request is in, which is what the count says and what a count
    // of finished Jobs would not.
    await expect
      .element(page.getByText("Every member has landed. 1 of 3 pull requests merged.").first())
      .toBeVisible();
  });
});

/** One member's card, by where it sits in the order. */
const member = (ordinal: number) =>
  page
    .getByRole("list", { name: "Pull requests, in the order they land" })
    .getByRole("listitem")
    .nth(ordinal - 1);

describe("the wave", () => {
  /**
   * The wave's own region. **Scoped, because the dock draws the same questions
   * from every repository** — a card in both places is the dock's own design,
   * and an unscoped locator reads the dock's copy first.
   */
  const wave = () => page.getByRole("region", { name: "The wave" }).first();

  /** One Job of the wave on the canvas, by the name its card carries. */
  const waveCard = (title: string) =>
    wave().getByRole("button", { name: new RegExp(`^${title}, `) });

  /** A wave Job's panel, once it has travelled in. */
  async function panelOf(title: string) {
    const panel = page.getByRole("dialog", { name: title });
    await entered(panel);
    return panel;
  }

  test("epic/wave: each Job of the live wave is drawn with what it has reached", async () => {
    mount("epic/wave");
    // Each card names the Job and the verb the registry gives its status, so
    // "what it has reached" is read off the card and not counted from a list.
    for (const [title, said] of [
      ["Refuse an unknown code at the seam", "done"],
      ["Name the fault in the toast", "done"],
      ["Carry the code into the journal", "awaiting review"],
      ["Say which half refused", "needs you"],
      ["Drop the second error shape", "running"],
    ] as const) {
      await expect.element(waveCard(title)).toHaveAccessibleName(`${title}, ${said}`);
    }
  });

  test("epic/wave: the graph draws a Job that waits on another behind it", async () => {
    mount("epic/wave");
    // The wave's own direction, as the edge reads it aloud. Dropping the second
    // error shape can only happen once every surface carries the first, so it
    // waits on them — the board had this pair the other way round.
    await expect
      .element(
        page.getByLabelText("Drop the second error shape waits on Say which half refused"),
      )
      .toBeInTheDocument();
  });

  test("epic/wave: the same five Jobs are a list, with what each waits on", async () => {
    mount("epic/wave");
    await wave().getByRole("tab", { name: "List" }).click();
    const list = page.getByRole("listbox", { name: "Wave 2, as a list" });
    await expect.element(list).toBeVisible();
    await expect.poll(() => list.element().querySelectorAll('[role="option"]').length).toBe(5);
    // The order the graph draws is the order the list says in words.
    await expect
      .element(list.getByText("Carry the code into the journal, Say which half refused"))
      .toBeVisible();
  });

  test(
    "epic/wave: Needs you is a line per Job, and a line opens that Job's panel with its " +
      "answer at the top",
    async () => {
      mount("epic/wave");
      const needs = page.getByRole("listbox", { name: "Needs you" });
      await expect.poll(() => needs.element().querySelectorAll('[role="option"]').length).toBe(3);

      // The Judge's refusal, answered in the panel rather than by opening the Job.
      await needs.getByRole("option", { name: /Say which half refused/ }).click();
      const blocked = await panelOf("Say which half refused");
      await expect
        .element(blocked.getByText("Does the message say which half refused — Bridge or Fleet?"))
        .toBeVisible();
      await expect
        .element(blocked.getByRole("button", { name: "Disagree, just this step" }))
        .toBeVisible();
      await blocked.getByRole("button", { name: /^Close/ }).click();

      // The permission ask: a command the Manifest has not cleared, with the
      // three answers Fleet offered.
      await needs.getByRole("option", { name: /Drop the second error shape/ }).click();
      const asking = await panelOf("Drop the second error shape");
      const ask = asking.getByRole("article", {
        name: /^The drone wants to run a command it was not given/,
      });
      await expect.element(ask).toBeVisible();
      for (const answer of ["Allow for this job", "Always allow in this repository", "Reject"]) {
        await expect.element(ask.getByRole("button", { name: answer })).toBeVisible();
      }
    },
  );

  test("epic/wave: the strip names each wave, and the loop how many it may run", async () => {
    mount("epic/wave");
    await expect.element(wave().getByRole("tab", { name: "Wave 1 · the seam" })).toBeVisible();
    await expect
      .element(wave().getByRole("tab", { name: "Wave 2 · every surface" }))
      .toHaveAttribute("aria-selected", "true");
    await expect.element(wave().getByText("Up to 5 waves")).toBeVisible();
  });

  // The owner, 30 Sep 2026: "I can't seem to scroll when I have a wave
  // selected." The wave sat outside Overview's scroller and squeezed the board
  // under it to nothing, so the wheel moved nothing.
  test("epic/wave: Overview scrolls from the wave down through the board under it", async () => {
    mount("epic/wave");
    await expect.element(wave().getByText("Up to 5 waves")).toBeVisible();
    const board = page.getByRole("tabpanel", { name: "Overview" }).element() as HTMLElement;
    let scroller: HTMLElement | null = board;
    while (scroller !== null && getComputedStyle(scroller).overflowY !== "auto") {
      scroller = scroller.parentElement;
    }
    expect(scroller).not.toBeNull();
    const box = scroller!;
    expect(box.contains(wave().element())).toBe(true);
    expect(box.clientHeight).toBeGreaterThan(200);
    box.scrollTop = box.scrollHeight;
    await expect.poll(() => box.scrollTop).toBeGreaterThan(0);
  });

  test("epic/wave: pressing a past wave draws its Jobs on the graph", async () => {
    mount("epic/wave");
    await wave().getByRole("tab", { name: "Wave 1 · the seam" }).click();
    await expect.element(waveCard("Handle every refusal at the seam")).toBeVisible();
    expect(waveCard("Say which half refused").query()).toBeNull();
  });

  test("epic/wave: a Job's panel opens that Job", async () => {
    mount("epic/wave");
    await waveCard("Carry the code into the journal").click();
    const panel = await panelOf("Carry the code into the journal");
    await panel.getByRole("button", { name: "Open job" }).click();
    await expect
      .element(page.getByRole("button", { name: "34-carry-the-code-into-the-log" }).first())
      .toBeVisible();
  });

  test("epic/wave: Plan names the model that judged the split, and each criterion", async () => {
    await at("epic/wave", "Plan");
    const judged = page.getByRole("note", { name: "How the Judge read the split" });
    await expect.element(judged.getByText("draws_the_split")).toBeVisible();
    await expect.element(judged.getByText("each_piece_carries_its_own_brief")).toBeVisible();
    await expect.element(judged.getByText("haiku")).toBeVisible();
  });

  test("epic/wave: a Job's panel on Plan offers dropping it from the wave", async () => {
    await at("epic/wave", "Plan");
    await waveCard("Drop the second error shape").click();
    const panel = await panelOf("Drop the second error shape");
    await expect.element(panel.getByRole("button", { name: /drop from the wave$/i })).toBeVisible();
  });

  // The owner's, 30 Sep 2026: the split being approved is drawn, as real Jobs
  // at awaiting approval, and one Approve the plan releases every one of them.
  // Fleet has no route for that yet, so it says so, naming #1694 and the Jobs.
  test("epic/plan-review: the gate draws the proposed wave, and Approve the plan asks Fleet to release it all, which is not built", async () => {
    const written: string[] = [];
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: (text: string) => (written.push(text), Promise.resolve()) },
    });
    const app = mount("epic/plan-review");
    const approveWave = vi.spyOn(app.api, "approveWave");
    await page.getByRole("tab", { name: /^Plan/ }).click();
    await expect
      .element(wave().getByRole("tab", { name: "Wave 2 · every surface" }))
      .toHaveAttribute("aria-selected", "true");
    const proposed = [
      "Refuse an unknown code at the seam",
      "Name the fault in the toast",
      "Carry the code into the journal",
      "Say which half refused",
      "Drop the second error shape",
    ];
    for (const title of proposed) {
      await expect.element(waveCard(title)).toHaveAccessibleName(`${title}, needs approval`);
    }
    // The gate's one approve answers them, so none is a line of its own.
    expect(page.getByRole("listbox", { name: "Needs you" }).query()).toBeNull();

    await page.getByRole("button", { name: "Approve the plan" }).click();
    await expect.poll(() => approveWave.mock.calls.length).toBe(1);
    const [, sent] = approveWave.mock.calls[0]!;
    expect(sent.jobs).toHaveLength(proposed.length);
    await expect.element(page.getByText("Not implemented", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Copy debug info" }).click();
    await expect.poll(() => written).toHaveLength(1);
    expect(written[0]).toContain("bridge.not_implemented");
    expect(written[0]).toContain(issueLink(1694));
    expect(written[0]).toContain("POST /jobs/{job_id}/approve_wave");
    for (const id of sent.jobs) expect(written[0]).toContain(id);
  });

  // The owner's, 30 Sep 2026: a proposed Job is edited in its own panel,
  // directly through Fleet, and never by opening it — the Job's own screen
  // would offer approving it alone. Fleet has no route yet, so Save says so,
  // naming #1699 and carrying the edit.
  test("epic/plan-review: a proposed Job's panel edits it directly, which Fleet has not built", async () => {
    const written: string[] = [];
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: (text: string) => (written.push(text), Promise.resolve()) },
    });
    const app = mount("epic/plan-review");
    const editJob = vi.spyOn(app.api, "editJob");
    await page.getByRole("tab", { name: /^Plan/ }).click();
    await waveCard("Say which half refused").click();
    const panel = await panelOf("Say which half refused");
    await panel.getByRole("button", { name: "Edit this Job" }).click();
    const form = panel.getByRole("region", { name: "Edit this Job" });
    const save = form.getByRole("button", { name: "Save", exact: true });
    await expect.element(save).toBeDisabled();
    await form.getByLabelText("Title").fill("Say which half refused, in the toast");
    await save.click();

    await expect.poll(() => editJob.mock.calls.length).toBe(1);
    const [, sent] = editJob.mock.calls[0]!;
    expect(sent).toEqual({ title: "Say which half refused, in the toast" });
    await expect.element(page.getByText("Not implemented", { exact: true })).toBeVisible();
    // Nothing was done, so what was typed stays.
    await expect.element(form.getByLabelText("Title")).toHaveValue("Say which half refused, in the toast");
    // The failure pops up over the panel, so it is copied with the panel open.
    await page.getByRole("button", { name: "Copy debug info" }).click();
    await expect.poll(() => written).toHaveLength(1);
    await expect.element(panel).toBeVisible();
    expect(written[0]).toContain("bridge.not_implemented");
    expect(written[0]).toContain(issueLink(1699));
    expect(written[0]).toContain("POST /jobs/{job_id}/edit");
    expect(written[0]).toContain("Say which half refused, in the toast");
  });

  test("epic/wave: Waits for opens the Job waited on, with a way back", async () => {
    await at("epic/wave", "Plan");
    await waveCard("Say which half refused").click();
    const panel = await panelOf("Say which half refused");
    await panel.getByRole("button", { name: /^Name the fault in the toast, done/ }).click();
    const next = await panelOf("Name the fault in the toast");
    await next.getByRole("button", { name: "Back to Say which half refused" }).click();
    await panelOf("Say which half refused");
  });
});

describe("one Job per workflow kind", () => {
  // The Workflow tab draws `JobDetail.steps`, which is the workflow the Job
  // froze. So what these claims read is the workflow file, through the screen.
  async function stepsOf(name: string) {
    const scenario = scenarioNamed(name)!;
    mount(scenario);
    await page.getByRole("tab", { name: /^Workflow/ }).click();
    const job = scenario.state.jobs[0]!;
    return scenario.state.holds.workflows.find((one) => one.id === job.workflow_id)!.steps;
  }

  /** The card for one step of the run, by the name and state it carries. */
  const stepCard = (label: string) => page.getByRole("button", { name: new RegExp(`^${label}, `) });

  test.for([
    ["kind/feature", 4],
    ["kind/bug", 3],
    ["kind/revert", 2],
    ["kind/prototype", 3],
    ["kind/refactor", 3],
    ["kind/design-plan", 2],
    ["kind/code-review", 3],
    ["kind/epic", 3],
  ] as const)("%s draws the steps its own workflow file declares", async ([name, count]) => {
    const steps = await stepsOf(name);
    expect(steps, `${name} declares ${String(count)} steps`).toHaveLength(count);
    for (const step of steps) await expect.element(stepCard(step.label).first()).toBeVisible();
    // And no step the boards invented: `design-plan` is draft and present with
    // no `decide`, and only a feature Job has a step called Write tests.
    if (!steps.some((step) => step.label === "Write tests")) {
      expect(stepCard("Write tests").query(), `${name} draws a step it never declared`).toBeNull();
    }
    expect(stepCard("Decide").query(), `${name} draws a step it never declared`).toBeNull();
  });

  test("no Job on the Board runs a workflow called verify-and-ship", async () => {
    const { scenario } = mount("kinds");
    await listed();
    expect(scenario.state.holds.workflows.map((one) => one.id)).not.toContain("verify-and-ship");
    expect(document.body.textContent).not.toContain("verify-and-ship");
  });

  test("kind/code-review: the run ends at a step that delivers a review, and offers no diff to land", async () => {
    const steps = await stepsOf("kind/code-review");
    const last = steps[steps.length - 1]!;
    expect(last.label).toBe("Deliver the review");
    await expect.element(stepCard(last.label).first()).toBeVisible();
    // Nothing on this run lands anything: a code review delivers a review.
    expect(page.getByRole("button", { name: /^Merge(?! line)/ }).query()).toBeNull();
    expect(page.getByRole("button", { name: /^Land/ }).query()).toBeNull();
  });
});
