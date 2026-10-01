import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { GroupBoundary, type GroupBoundaryCheck } from "./GroupBoundary";

const meta: Meta<typeof GroupBoundary> = {
  title: "Compositions/Group boundary",
  component: GroupBoundary,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-step-panel-min)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof GroupBoundary>;

const NAMES = ["typecheck", "format", "screens_test", "components_test", "bridge_build", "storybook", "acceptance"];

const all = (reads: GroupBoundaryCheck["reads"]): GroupBoundaryCheck[] =>
  NAMES.map((name) => ({ name, reads }));

/**
 * Nothing has reached it: seven segments, no verdict and no commit.
 *
 * **A break test on absence.** The Check names are behind the strip's press,
 * and a disclosure that forgot to close would draw them — which reads as the
 * wrapping list the owner called a big miss, with no test failing.
 */
export const NotRun: Story = {
  args: {
    checks: all("not run"),
  },
  play: async ({ canvas, canvasElement }) => {
    const strip = canvas.getAllByRole("button", { expanded: false })[0]!;
    await expect(canvas.queryByText("screens_test")).toBeNull();
    await expect(canvasElement.querySelectorAll(".armada-step-bar__segment")).toHaveLength(7);
    await userEvent.click(strip);
    await expect(canvas.getByText("screens_test")).toBeVisible();
    // Nothing ran, so there is no Record row to open and no Check is a button.
    await expect(canvas.queryByRole("button", { name: "screens_test, not run" })).toBeNull();
  },
};

/** One in flight. The bar is the only thing on this surface that says so. */
export const OneInFlight: Story = {
  args: {
    checks: NAMES.map((name, at) => ({ name, reads: at < 3 ? "passed" : at === 3 ? "running" : "not run" })),
    verdictSays: "running now",
  },
};

/** All passed, with the commit the group left and the cases that ran beside them. */
export const AllPassed: Story = {
  args: {
    checks: all("passed"),
    verdictSays: "all passed",
    verdictNamed: "passed",
    commit: "7a2f0c5",
    tests: [
      { id: "c-panel", spec: "packages/screens/src/Running.test.tsx", reads: "owed" },
      { id: "c-board", spec: "packages/screens/src/Board.test.tsx", reads: "not covered" },
    ],
  },
  play: async ({ canvas }) => {
    // A boundary that passed stays shut, which is what makes a failed one
    // opening itself a reading rather than the default.
    const checks = canvas.getByRole("region", { name: "Checks at this boundary" });
    await expect(within(checks).getAllByRole("button")[0]).toHaveAttribute("aria-expanded", "false");
    await expect(checks).toHaveTextContent("all passed");
    await expect(checks).toHaveTextContent("7a2f0c5");
  },
};

/**
 * One failed, and the group stops behind it.
 *
 * **A `play`, because the claim is about what is drawn beside the red.** A
 * boundary that read failed with nothing said is the state this component was
 * built to end — the six that passed are on screen, which attempt this is is on
 * screen, and the failed Check carries what it was held to and what it got,
 * each under its own label. Open, the head carries no verdict, count or bar. Drawn at `--w-step-panel-min`, which is narrower
 * than the head's one line: it wraps, and cuts nothing.
 */
export const OneFailed: Story = {
  args: {
    checks: NAMES.map((name) =>
      name === "screens_test"
        ? {
            name,
            reads: "failed",
            expected: "Every test in the screens package passes",
            result: "1 of 1384 failed: the Drones row opened the Board",
            onOpen: fn(),
          }
        : { name, reads: "passed", onOpen: fn() },
    ),
    verdictSays: "screens_test failed",
    verdictNamed: "failed",
    retrySays: "attempt 2",
  },
  play: async ({ canvas, args }) => {
    const checks = canvas.getByRole("region", { name: "Checks at this boundary" });
    // **The break test.** What failed is why anybody is on the card, so the
    // strip opens itself — shut, every assertion below would read nothing and
    // the screen would be hiding the one thing it exists to show.
    await expect(within(checks).getAllByRole("button")[0]).toHaveAttribute("aria-expanded", "true");
    // The attempt is whole: the head wraps rather than ending a chip in `…`,
    // which is how `second run` read as `second r…` (the owner, 29 Sep).
    const attempt = within(checks).getByText("attempt 2");
    await expect(attempt.scrollWidth).toBeLessThanOrEqual(attempt.clientWidth);
    await expect(checks.querySelectorAll('[data-reads="passed"]')).toHaveLength(6);
    // Open, the head sums nothing up: the rows under it say what failed and
    // how many there are (the owner, 29 Sep 2026).
    await expect(checks).not.toHaveTextContent("screens_test failed");
    await expect(checks.querySelector(".armada-boundary__count")).toBeNull();
    await expect(checks.querySelector(".armada-step-bar")).toBeNull();
    // Each value under its label, and the heading that ran them together gone.
    await expect(checks).toHaveTextContent("ExpectedEvery test in the screens package passes");
    await expect(checks).toHaveTextContent("Result1 of 1384 failed");
    await expect(checks).not.toHaveTextContent("What the gate wrote down");
    await expect(checks).not.toHaveTextContent("The run's whole output is kept");
    // The guide's `?` is beside the label and never leads the Checks.
    await expect(checks.querySelector(".armada-boundary__checks > li")).toHaveTextContent("screens_test");
    // A Check is a button to its own Record row.
    const failed = within(checks).getByRole("button", { name: "screens_test, failed" });
    await userEvent.click(failed);
    const screens = args.checks!.find((check) => check.name === "screens_test")!;
    await expect(screens.onOpen).toHaveBeenCalledOnce();
    // The two sentences the owner cut on 28 Sep. The ordering rule is guide
    // 4's, and the reason there is no case named Fleet and not this Job.
    await expect(checks).not.toHaveTextContent("No task of group");
    await expect(canvas.queryByRole("region", { name: "Tests at this boundary" })).toBeNull();
  },
};
