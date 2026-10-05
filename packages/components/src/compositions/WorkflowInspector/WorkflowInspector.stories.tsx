import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { Button } from "../../primitives/Button/Button";
import { CriterionVerdicts } from "../CriterionVerdicts/CriterionVerdicts";
import { WorkflowInspector } from "./WorkflowInspector";

const meta: Meta<typeof WorkflowInspector> = {
  title: "Compositions/Workflow inspector",
  component: WorkflowInspector,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-step-panel-min)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WorkflowInspector>;

const checks = [
  { name: "test", outcome: "passed", named: "passed" as const },
  { name: "typecheck", outcome: "passed", named: "passed" as const },
  { name: "screens_test", outcome: "failed", named: "failed" as const },
  { name: "components_test" },
];

const tests = [
  { id: "c1", title: "A stat with one Drone out reads one running", outcome: "passed", named: "passed" as const },
  { id: "c2", title: "Pressing the stat lists the Drone's Job", outcome: "not covered" },
];

/**
 * A group mid-flight: two tasks, the Checks at its boundary, and the cases
 * beside them.
 *
 * **A `play`, because the way out is a claim.** The caller draws this panel on
 * a layer over the workflow canvas, so Close is what gives the canvas back —
 * Helm's dock is closed by the Close in its own head and this is the same
 * arrangement. A panel drawn on a layer with no way off it is the v1 complaint.
 */
export const AGroupRunning: Story = {
  args: {
    name: "Group 3",
    kind: "group",
    doing: "Two tasks running at once, then seven Checks at this group's end.",
    tasks: [
      { id: "T5", title: "Read what every Drone is holding", said: "working", facts: ["4 files", "12 turns"] },
      { id: "T6", title: "Say it in the stat's own words", said: "done", facts: ["2 files", "$0.31"] },
    ],
    checks,
    tests,
    redirect: {
      value: "",
      onChange: fn(),
      onSend: fn(),
      drones: [
        { id: "d-5", label: "Drone on T5" },
        { id: "d-6", label: "Drone on T6" },
      ],
      reaches: "d-5",
      onReaches: fn(),
    },
    stop: {
      children: "Hold to stop this group",
      askLabel: "Stop this group",
      description: "Stops the agents on this group once held until it fills. Letting go sooner stops nothing.",
      onCommit: fn(),
      onAsk: fn(),
    },
    onClose: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    // In the head, beside what the panel is reading, and not at the foot under
    // the reply box — the dock's own place for it.
    const head = canvas.getByRole("heading", { name: "Group 3" }).closest("header");
    await expect(head).not.toBeNull();
    const close = canvas.getByRole("button", { name: "Close" });
    await expect(head!.contains(close)).toBe(true);
    await userEvent.click(close);
    await expect(args.onClose).toHaveBeenCalled();
  },
};

/** A step nothing has entered: no tasks, no runs, and nothing to stop. */
export const AStepNotStarted: Story = {
  args: {
    name: "Write tests",
    kind: "step",
    doing: "Nothing has entered this step.",
    tasksAbsent: "No task is planned under this step.",
    checks: checks.map((check) => ({ name: check.name })),
  },
};

/**
 * A Check with a log is a press that opens it; one that has not run is a plain
 * row, because there is nothing to open.
 */
export const ChecksThatOpen: Story = {
  args: {
    name: "Write tests",
    kind: "step",
    checks: [
      { name: "test", outcome: "2m 10s", live: "running", onOpen: fn() },
      { name: "typecheck", outcome: "passed", named: "passed" as const, onOpen: fn() },
      { name: "desktop_test", live: "waiting" },
    ],
  },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    const running = canvas.getByRole("button", { name: "Open test" });
    running.click();
    await expect(args.checks?.[0]?.onOpen).toHaveBeenCalledTimes(1);
    await expect(canvas.getByRole("button", { name: "Open typecheck" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Open desktop_test" })).toBeNull();
  },
};

/**
 * A step with one Drone on it names it in a sentence rather than a picker, and
 * a task a later task edited keeps its flag.
 *
 * **A `play`, because the rule is about what is *not* drawn.** One Drone and a
 * picker would ask a person to choose between one thing; the two lists must
 * also stay two — a case folded in among the Checks reads as a Check that
 * passed, which is the one confusion `#1530` named.
 */
export const OneDroneAndTwoLists: Story = {
  args: {
    name: "Implement",
    kind: "step",
    doing: "One Drone, on task T7.",
    tasks: [
      {
        id: "T6",
        title: "Say it in the stat's own words",
        said: "done",
        facts: ["$0.31"],
        flag: "T7 edited a file this task had finished. It stays done.",
      },
      { id: "T7", title: "Hold the next Drone back", said: "working", facts: ["9 turns"] },
    ],
    checks,
    tests,
    redirect: { value: "", onChange: fn(), onSend: fn(), drones: [{ id: "d-7", label: "Drone on T7" }] },
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("combobox")).toBeNull();
    await expect(canvas.getByText("Reaches Drone on T7")).toBeVisible();
    const boundaryChecks = canvas.getByRole("region", { name: "Checks at this step" });
    const boundaryTests = canvas.getByRole("region", { name: "Tests at this step" });
    await expect(boundaryChecks).toHaveTextContent("screens_test");
    await expect(boundaryChecks).not.toHaveTextContent("not covered");
    await expect(boundaryTests).toHaveTextContent("not covered");
  },
};

/**
 * A step stopped on a Judge's refusal: the refused criterion's finding, and the
 * acts Fleet offers for it, in one region near the top.
 *
 * **A `play`, because the acts belong with the reason.** The panel of the
 * owner's stopped step said nothing about why, and offered nothing (Job 3,
 * 2 Oct 2026).
 */
export const AStepStoppedOnARefusal: Story = {
  args: {
    name: "Plan the change",
    kind: "step",
    state: { activity: "stopped", said: "stopped" },
    checks: [{ name: "plan_recorded", outcome: "passed", named: "passed" as const }],
    stopped: {
      why: (
        <CriterionVerdicts
          rows={[
            {
              ordinal: 1,
              criterionId: "addresses_the_request",
              text: "The plan addresses what was asked, and nothing beyond it",
              named: "not_met",
              verdict: "refused",
              expected: "Tasks to retire guides 8 and 20, and a rule that guides' pieces are drawn somewhere",
              produced: "T4 adds documentation updates to design-system.md",
              consequence: "Scope expands beyond what was requested",
            },
          ]}
        />
      ),
      acts: (
        <>
          <Button variant="secondary">Overrule the verdict</Button>
          <Button variant="secondary">Restart step</Button>
        </>
      ),
    },
  },
  play: async ({ canvas }) => {
    const why = canvas.getByRole("region", { name: "Why it stopped" });
    await expect(why).toHaveTextContent("The plan addresses what was asked");
    await expect(why).toHaveTextContent("T4 adds documentation updates");
    await expect(within(why).getByRole("button", { name: "Restart step" })).toBeVisible();
  },
};
