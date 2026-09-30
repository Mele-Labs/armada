import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { PlanBoard, type PlanBoardGroup } from "./PlanBoard";

/**
 * One story per state the plan can be read in: before anything runs, mid-run
 * with a group that failed its boundary, and a finished task a later one
 * edited. The words are the caller's, as they are in the app.
 */
const meta: Meta<typeof PlanBoard> = {
  title: "Compositions/Plan board",
  component: PlanBoard,
};
export default meta;

type Story = StoryObj<typeof PlanBoard>;

const APPROACH =
  "One read of everything running, then the stat's words, then the panel, then what holds " +
  "the next Drone back.";

const RUST = ["build", "test", "acceptance", "format"];
const BRIDGE = [
  "test",
  "typecheck",
  "bridge_build",
  "storybook",
  "desktop_test",
  "screens_test",
  "components_test",
];

/** Every Check at a boundary nothing has reached yet. */
const notRun = (names: readonly string[]) =>
  names.map((name) => ({ name, reads: "not run" as const }));

function planned(): PlanBoardGroup[] {
  return [
    {
      id: "g1",
      ordinal: 1,
      state: "pending",
      says: "not started",
      scope: { root: "crates/**" },
      tasks: [
        {
          id: "T1",
          title: "Serve one read of everything running",
          mark: "open",
          tier: "difficult",
          model: "opus",
        },
        {
          id: "T2",
          title: "Send an event when what is running changes",
          mark: "open",
          tier: "medium",
          model: "sonnet",
        },
      ],
      shapeSays: "2 tasks, one after another",
      concurrent: false,
      boundary: {
        checks: notRun(RUST),
        tests: [{ id: "c-api", spec: "crates/api/src/tests/running.rs", reads: "owed" }],
      },
    },
    {
      id: "g3",
      ordinal: 2,
      state: "pending",
      says: "not started",
      scope: { root: "packages/screens/src/**" },
      shapeSays: "2 tasks, at the same time",
      concurrent: true,
      tasks: [
        {
          id: "T5",
          title: "Draw what is running, in four lists",
          mark: "open",
          tier: "difficult",
          model: "opus",
          besideSays: "beside T6",
        },
        {
          id: "T6",
          title: "Open a Drone's Job from its row",
          mark: "open",
          tier: "medium",
          model: "sonnet",
          besideSays: "beside T5",
        },
      ],
      boundary: {
        checks: notRun(BRIDGE),
        tests: [
          { id: "c-board", spec: "packages/screens/src/Board.test.tsx", reads: "not covered" },
        ],
      },
    },
  ];
}

/**
 * The plan before anything runs.
 *
 * **A break test on absence.** A group says where it writes and not every file
 * it writes (owner, 28 Sep 2026), and a card that went back to listing them
 * would still pass every other assertion on this board.
 */
export const Planned: Story = {
  args: { approach: APPROACH, groups: planned(), onOpenTask: fn() },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("packages/screens/src/**")).toBeVisible();
    await expect(canvas.queryByText("packages/screens/src/Running.tsx")).toBeNull();
    await expect(canvas.queryByText("packages/screens/src/running-rows.tsx")).toBeNull();
    // Three figures per row, lined up down the card by the list's own tracks.
    const rows = canvasElement.querySelectorAll(".armada-plan-board__task-model");
    await expect(rows).toHaveLength(4);
  },
};

/**
 * A boundary that broke, with the group carrying it.
 *
 * **A `play`, because this is the reading that came off the second board.** The
 * commit a group left, what its boundary came to, how many times it has been
 * run and the failed Check's own output handed to the next Drone were drawn
 * only under the Workflow canvas until the owner took the groups off it (28 Sep
 * 2026). And the reason a task stopped says whose words it is.
 */
export const GroupFailed: Story = {
  args: {
    approach: APPROACH,
    groups: [
      {
        ...planned()[1]!,
        state: "retrying",
        says: "failed at its checks",
        tasks: [
          { ...planned()[1]!.tasks[0]!, mark: "done", turnsSays: "27 turns", costSays: "~$1.90" },
          {
            ...planned()[1]!.tasks[1]!,
            mark: "failed",
            turnsSays: "15 turns",
            costSays: "~$0.72",
            failedReason: "The row's press opened the Board rather than the Job",
          },
        ],
        boundary: {
          checks: BRIDGE.map((name) =>
            name === "screens_test"
              ? {
                  name,
                  reads: "failed" as const,
                  expected: "Every test in the screens package passes",
                  result: "1 of 1384 failed: the Drones row opened the Board",
                }
              : { name, reads: "passed" as const },
          ),
          verdictSays: "screens_test failed",
          verdictNamed: "failed",
          retrySays: "attempt 2",
        },
      },
    ],
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const row = canvas.getByRole("listitem", { name: /^T6 / });
    await expect(row).toHaveTextContent("Why it stopped");
    await expect(row).toHaveTextContent("The row's press opened the Board rather than the Job");

    const card = canvas.getByRole("list", { name: "Group 2 tasks" }).closest("li")!;
    // The failed boundary is open, so its own row says what failed and the
    // head does not say it again.
    await expect(card.querySelector('li[data-reads="failed"]')).toHaveTextContent("screens_test");
    await expect(card).not.toHaveTextContent("screens_test failed");
    await expect(card).toHaveTextContent("attempt 2");
    await expect(card).toHaveTextContent("Result1 of 1384 failed");
    // Retired on 29 Sep: the heading and the pair run together unlabelled.
    await expect(card).not.toHaveTextContent("What the gate wrote down");
    // Cut on 28 Sep, and neither comes back with the reading.
    await expect(card).not.toHaveTextContent("No task of group");
    await expect(card).not.toHaveTextContent("does not serve the cases");
    // The shape, drawn rather than said, and nothing about the Job's Drone cap.
    await expect(within(card).getByRole("img", { name: "2 tasks, at the same time" })).toBeVisible();
    await expect(card).not.toHaveTextContent("at the same time");
    await expect(card).not.toHaveTextContent("Drones at once");
  },
};

export const DoneTouchedLater: Story = {
  args: {
    approach: APPROACH,
    groups: [
      {
        ...planned()[1]!,
        state: "passed",
        says: "passed",
        tasks: [
          { ...planned()[1]!.tasks[0]!, mark: "done", turnsSays: "27 turns", costSays: "~$1.90" },
          {
            ...planned()[1]!.tasks[1]!,
            mark: "done",
            turnsSays: "15 turns",
            costSays: "~$0.72",
            touchedSays: "touched later · T7",
          },
        ],
        boundary: {
          checks: BRIDGE.map((name) => ({ name, reads: "passed" as const })),
          verdictSays: "all passed",
          verdictNamed: "passed",
          commit: "b81c3e4",
        },
      },
    ],
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    // The commit is at the boundary, which is where the group's end produced it.
    const card = canvas.getByRole("list", { name: "Group 2 tasks" }).closest("li")!;
    await expect(card).toHaveTextContent("b81c3e4");
    await expect(canvas.getByRole("listitem", { name: /^T6 / })).toHaveTextContent(
      "touched later · T7",
    );
  },
};

/**
 * Two groups claiming one file, said on the group that has the clash — the
 * owner's call of 28 Sep 2026, against a band above the whole plan.
 */
export const OrderContradictsScope: Story = {
  args: {
    approach: APPROACH,
    groups: planned().map((group, at) =>
      at === 1
        ? {
            ...group,
            overlaps: [
              {
                says: "Group 4 writes these files too",
                paths: ["packages/screens/src/running-rows.tsx"],
              },
            ],
          }
        : group,
    ),
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const card = canvas.getByRole("list", { name: "Group 2 tasks" }).closest("li")!;
    await expect(card).toHaveTextContent("Group 4 writes these files too");
    const first = canvas.getByRole("list", { name: "Group 1 tasks" }).closest("li")!;
    await expect(first).not.toHaveTextContent("writes these files too");
  },
};

/**
 * The row is one control and the task's id is part of its name, so a person
 * reaching it by keyboard hears which task they are opening. Verified against
 * a board rendered with `onOpenTask` absent, where nothing is pressable.
 */
export const OpeningATask: Story = {
  args: { ...Planned.args, openTaskId: "T5" } as Story["args"],
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: /T1 Serve one read/ }));
    await expect(args.onOpenTask).toHaveBeenCalledWith("T1");
    // `/T5/` alone now matches T6's row too, which says `beside T5`.
    await expect(canvas.getByRole("button", { name: /T5 Draw what is running/ })).toHaveAttribute(
      "aria-current",
      "true",
    );
    await expect(canvas.getByRole("button", { name: /T1 Serve one read/ })).not.toHaveAttribute(
      "aria-current",
    );
  },
};

/**
 * The plan at its gate, where a person edits it directly (owner, 30 Sep
 * 2026): Remove on each group asks its reason in place, and a group or a task
 * moves by dragging or by `⌥↑` / `⌥↓` on it. Propose a change is the one ask
 * to the Drone.
 */
export const WaitingAtItsGate: Story = {
  args: {
    approach: APPROACH,
    askable: true,
    groups: planned(),
    onOpenTask: fn(),
    remove: { label: "Remove", onRemove: fn(async () => null) },
    propose: { label: "Propose a change", send: "Send to the Drone", onPropose: fn() },
    move: { onMove: fn() },
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const first = canvas.getByRole("group", { name: "Ask about group 1" });
    await expect(within(first).queryByRole("button", { name: "Move up" })).toBeNull();
    await userEvent.click(within(first).getByRole("button", { name: "Remove" }));
    await expect(canvas.getByRole("region", { name: "Remove group 1" })).toBeVisible();
    canvas.getByRole("heading", { name: "Group 1" }).focus();
    await userEvent.keyboard("{Alt>}{ArrowDown}{/Alt}");
    await expect(args.move?.onMove).toHaveBeenCalledWith({ group: "g1", to: 1 });
    canvas.getByRole("button", { name: /T2 Send an event/ }).focus();
    await userEvent.keyboard("{Alt>}{ArrowDown}{/Alt}");
    // T2 is group one's last task, so ⌥↓ crosses it into the next group's top.
    await expect(args.move?.onMove).toHaveBeenCalledWith({ group: "g3", task: "T2", to: 0 });
  },
};

/**
 * A proposal is out, so Propose a change is off — a second while the first is
 * unanswered would be two asks about one plan with no order between them.
 */
export const AnAskIsOut: Story = {
  args: { ...WaitingAtItsGate.args, askPending: true } as Story["args"],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const second = canvas.getByRole("group", { name: "Ask about group 2" });
    await expect(within(second).getByRole("button", { name: "Propose a change" })).toBeDisabled();
  },
};
