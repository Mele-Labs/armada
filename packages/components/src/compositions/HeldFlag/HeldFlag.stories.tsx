import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { GAMING_PATTERN_HEADLINE } from "../../generated/vocabulary";
import { HeldFlag, type HeldFinding } from "./HeldFlag";

const meta: Meta<typeof HeldFlag> = {
  title: "Compositions/Held flag",
  component: HeldFlag,
};
export default meta;

type Story = StoryObj<typeof HeldFlag>;

/** The registry's own headline, as the step panel hands it down. */
function meaning(pattern: string): Pick<HeldFinding, "label" | "headline"> {
  const said = GAMING_PATTERN_HEADLINE[pattern] ?? pattern;
  return { label: said, headline: said };
}

const TEST_FILE = "packages/settings/test/useColumnSelectors.test.ts";

const ASKED =
  "Does this change alter an existing assertion so that it asserts less than it did, and is " +
  "that assertion made nowhere else in this change?";

const BRIEF = ".armada/briefs/77-split-the-settings-reducer/implement.1.gaming.assertion_weakened.txt";

/** The words `flag-held.tsx` says for each of the two acts Send it back can be. */
const RESTARTS =
  "Restarts the step with a fresh drone. Its brief carries the flag, and your note if you write one.";
const REDIRECTS =
  "Sends the flag back to the drone still on this step, with your note if you write one, and it " +
  "works the step again in the same session.";

function answers(sendBack = RESTARTS) {
  return {
    carryOn: {
      consequence: "Overrules the flag. It is not the last step, so the job carries on at the next one.",
      onCarryOn: fn(),
    },
    sendBack: { consequence: sendBack, onSendBack: fn() },
    onOpenBrief: fn(),
  };
}

const ADDED_LINE: HeldFinding = {
  ...meaning("assertion_weakened"),
  hunk: {
    path: TEST_FILE,
    lines: [
      { kind: "hunk", text: '@@ -38,7 +38,7 @@ describe("useColumnSelectors", () => {' },
      { kind: "context", text: '   it("keeps hidden columns out of the visible set", () => {' },
      { kind: "context", text: "     const visible = selectVisible(state);" },
      { kind: "removed", text: '-    expect(visible).toEqual(["name", "status", "owner"]);' },
      { kind: "added", text: "+    expect(visible.length).toBeGreaterThan(0);" },
      { kind: "context", text: "   });" },
    ],
  },
  asked: ASKED,
  brief: BRIEF,
};

/**
 * **A flag on an added line**, where the Drone has gone and Send it back
 * restarts the step. Fleet placed the flag at line 41 of the post-image, and
 * the hunk holding that line is what the card draws.
 *
 * **Nothing to type.** Carry on with no note sends a blank reason, which
 * Fleet takes on a gaming flag.
 */
export const OnAnAddedLine: Story = {
  args: { ...answers(), findings: [ADDED_LINE] },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Carry on" }));
    await expect(args.carryOn.onCarryOn).toHaveBeenCalledWith("");
  },
};

/**
 * **A flag on a removed line.** A line the change took out has no post-image
 * number, so Fleet sent the file and no line — and the hunk is found by the
 * words the check quoted, never by a line guessed near it.
 *
 * **The reason is what the person typed**, and nothing else.
 */
export const OnARemovedLine: Story = {
  args: {
    ...answers(),
    findings: [
      {
        ...meaning("assertion_weakened"),
        hunk: {
          path: TEST_FILE,
          lines: [
            { kind: "hunk", text: "@@ -52,9 +52,7 @@" },
            { kind: "context", text: '   it("drops a column that was hidden", () => {' },
            { kind: "context", text: '     const next = reducer(state, hide("owner"));' },
            { kind: "removed", text: '-    expect(next.hidden).toContain("owner");' },
            { kind: "removed", text: '-    expect(selectVisible(next)).not.toContain("owner");' },
            { kind: "context", text: "     expect(next.version).toBe(state.version + 1);" },
            { kind: "context", text: "   });" },
          ],
        },
        asked: ASKED,
        brief: BRIEF,
      },
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.type(canvas.getByRole("textbox", { name: "Note (optional)" }), "See reducer.test.ts");
    await userEvent.click(canvas.getByRole("button", { name: "Carry on" }));
    await expect(args.carryOn.onCarryOn).toHaveBeenCalledWith("See reducer.test.ts");
  },
};

/**
 * **Send it back where the Drone still holds its session**, which is nearly
 * every flag: a gaming flag escalates the Job and leaves the Drone there. The
 * answer is a redirect, and the sentence under it says so.
 */
export const SendItBackToTheDroneStillThere: Story = {
  args: { ...answers(REDIRECTS), findings: [ADDED_LINE] },
  // **One note, sent with whichever answer is pressed** (owner, 2 Oct 2026).
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.type(canvas.getByRole("textbox", { name: "Note (optional)" }), "Put it back");
    await userEvent.click(canvas.getByRole("button", { name: "Send it back" }));
    await expect(args.sendBack.onSendBack).toHaveBeenCalledWith("Put it back");
    await expect(args.carryOn.onCarryOn).not.toHaveBeenCalled();
  },
};

/**
 * **Under Overview's lead**, which already says what the flag caught: the
 * finding names itself to a reader who cannot see it and draws no headline.
 */
export const UnderTheLead: Story = {
  args: { ...answers(REDIRECTS), findings: [{ ...ADDED_LINE, headline: undefined }] },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("article", { name: /An assertion was removed/ })).toBeVisible();
    await expect(canvas.queryByText(/An assertion was removed/)).toBeNull();
  },
};

/**
 * **A pattern the diff decides, which was asked nothing.** No question and no
 * brief, so the card draws neither.
 */
export const DecidedByTheDiff: Story = {
  args: {
    ...answers(),
    findings: [
      {
        ...meaning("check_config_edited"),
        hunk: {
          path: "packages/settings/package.json",
          lines: [
            { kind: "hunk", text: "@@ -6,7 +6,7 @@" },
            { kind: "context", text: '   "scripts": {' },
            { kind: "removed", text: '-    "test": "vitest run",' },
            { kind: "added", text: '+    "test": "vitest run --passWithNoTests src/selectors",' },
            { kind: "context", text: "   }," },
          ],
        },
      },
    ],
  },
};

/**
 * **Where no hunk could be located**, the citation and the file are drawn
 * instead of a nearby hunk. A deleted test file is quoted whole, and a guessed
 * window would send a person to the wrong lines believing them.
 */
export const NoHunkLocated: Story = {
  args: {
    ...answers(),
    findings: [
      {
        ...meaning("test_deleted"),
        cited: "`packages/settings/test/useColumnSelectors.test.ts` was deleted in the step it gates.",
        at: { file: TEST_FILE },
      },
    ],
  },
};

/**
 * **Two flags on one step.** Two findings, each with what it means and its own
 * lines, and one answer — the step is held once, and a person decides once.
 *
 * Send it back with no note sends none, rather than an empty string.
 */
export const TwoFlagsOnOneStep: Story = {
  args: {
    ...answers(),
    findings: [
      ADDED_LINE,
      {
        ...meaning("test_skipped"),
        hunk: {
          path: TEST_FILE,
          lines: [
            { kind: "hunk", text: "@@ -70,6 +70,6 @@" },
            { kind: "removed", text: '-  it("reorders columns by drag", () => {' },
            { kind: "added", text: '+  it.skip("reorders columns by drag", () => {' },
            { kind: "context", text: '     const next = reducer(state, move("owner", 0));' },
          ],
        },
      },
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Send it back" }));
    await expect(args.sendBack.onSendBack).toHaveBeenCalledWith(undefined);
  },
};

/**
 * **Carry on just pressed, and Fleet has not answered.** Only that control
 * marks itself busy; Send it back is off like every other control, and
 * nothing says "already sending" until the five-second mark. #1117.
 */
export const WaitingOnFleet: Story = {
  args: {
    ...answers(),
    findings: [ADDED_LINE],
    disabled: true,
    pending: "carryOn",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Carrying on…" })).toHaveAttribute(
      "aria-busy",
      "true",
    );
    await expect(canvas.getByRole("button", { name: "Send it back" })).toBeDisabled();
    await expect(canvas.queryByRole("status")).toBeNull();
  },
};

/**
 * **Send it back where Fleet offers neither a redirect nor a restart.** The
 * answer is drawn, off, with the reason under it, rather than left out.
 */
export const SendItBackWithheld: Story = {
  args: {
    ...answers(),
    findings: [ADDED_LINE],
    sendBack: {
      consequence: RESTARTS,
      onSendBack: fn(),
      withheld: "Fleet offers neither a redirect nor a restart on this step.",
    },
  },
};
