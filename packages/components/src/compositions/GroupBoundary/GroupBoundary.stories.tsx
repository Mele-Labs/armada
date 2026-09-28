import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";

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
    clause: "will run at this boundary",
    checks: all("not run"),
  },
  play: async ({ canvas, canvasElement }) => {
    const strip = canvas.getAllByRole("button", { expanded: false })[0]!;
    await expect(canvas.queryByText("screens_test")).toBeNull();
    await expect(canvasElement.querySelectorAll(".armada-step-bar__segment")).toHaveLength(7);
    await userEvent.click(strip);
    await expect(canvas.getByText("screens_test")).toBeVisible();
  },
};

/** One in flight. The bar is the only thing on this surface that says so. */
export const OneInFlight: Story = {
  args: {
    clause: "running at this boundary",
    checks: NAMES.map((name, at) => ({ name, reads: at < 3 ? "passed" : at === 3 ? "running" : "not run" })),
    verdictSays: "running now",
  },
};

/** All passed, with the commit the group left and the cases that ran beside them. */
export const AllPassed: Story = {
  args: {
    clause: "ran at this boundary",
    checks: all("passed"),
    verdictSays: "all 7 passed",
    verdictNamed: "passed",
    commit: "7a2f0c5",
    testsClause: "ran at this boundary",
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
    await expect(checks).toHaveTextContent("all 7 passed");
    await expect(checks).toHaveTextContent("7a2f0c5");
  },
};

/**
 * One failed, and the group stops behind it.
 *
 * **A `play`, because the claim is about what is drawn beside the red.** A
 * boundary that read failed with nothing said is the state this component was
 * built to end — the six that passed are on screen, the retry count is on
 * screen, and what the next Drone is told is the Check's own output rather than
 * a summary of it.
 */
export const OneFailed: Story = {
  args: {
    clause: "ran at this boundary",
    checks: NAMES.map((name) => ({ name, reads: name === "screens_test" ? "failed" : "passed" })),
    verdictSays: "screens_test failed",
    verdictNamed: "failed",
    retrySays: "second run",
    toldNext: "every test in the screens package passes\n1 of 1384 failed: the Drones row opened the Board",
    toldNextSays: "The run's whole output is kept. A Drone picking this group up can ask for the Check again and is handed the last of what it printed.",
  },
  play: async ({ canvas }) => {
    const checks = canvas.getByRole("region", { name: "Checks at this boundary" });
    // **The break test.** What failed is why anybody is on the card, so the
    // strip opens itself — shut, every assertion below would read nothing and
    // the screen would be hiding the one thing it exists to show.
    await expect(within(checks).getAllByRole("button")[0]).toHaveAttribute("aria-expanded", "true");
    await expect(checks).toHaveTextContent("screens_test");
    await expect(checks).toHaveTextContent("second run");
    await expect(checks).toHaveTextContent("1 of 1384 failed");
    await expect(checks.querySelectorAll('[data-reads="passed"]')).toHaveLength(6);
    // Not the output, and the region says so rather than claiming to be it.
    await expect(checks).toHaveTextContent("What the gate wrote down");
    await expect(checks).toHaveTextContent("The run's whole output is kept");
    // The two sentences the owner cut on 28 Sep. The ordering rule is guide
    // 4's, and the reason there is no case named Fleet and not this Job.
    await expect(checks).not.toHaveTextContent("No task of group");
    await expect(canvas.queryByRole("region", { name: "Tests at this boundary" })).toBeNull();
  },
};
