import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { RetroSheet, type RetroSheetItem } from "./RetroSheet";

/**
 * One Job's retro on its sheet, in Job 3's shape: the gate measuring from a
 * stale local main, a Drone waiting on a command it was not allowed, and an
 * item written before headlines. Each item reads as words, a headline, what
 * happened and a fix, and answers with Agree and Disagree.
 *
 * The sheet lays out inside the nearest positioned ancestor, so every story
 * draws one.
 */
const meta: Meta<typeof RetroSheet> = {
  title: "Compositions/Retro sheet",
  component: RetroSheet,
  args: { open: true, job: "Job 3", items: [], notes: [], onClose: fn() },
  decorators: [
    (Story) => (
      <div style={{ position: "relative", height: "var(--palette-max-height)", background: "var(--bg-base)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof RetroSheet>;

const answers = (agreeTip: string) => ({
  agreeTip,
  disagreeTip: "Discards it",
  onAgree: fn(),
  onDisagree: fn(),
});

const ITEMS: RetroSheetItem[] = [
  {
    id: "stale-main",
    who: "fleet",
    landsIn: "armada",
    statement: "The gate compared the step against a stale local main.",
    title: "The gate blamed the Drone for Fleet's own mistake",
    what: "It compared the step against a local main two commits behind origin, so two commits that edited armada.yml counted as the Drone's work.",
    fix: "Compare against origin/main, where the branch is cut from.",
    answers: answers("Proposes a Job on Armada's repository"),
    cites: [
      {
        id: "check:1",
        name: "out_of_bounds",
        mono: true,
        detail: "armada.yml changed; the line came in with 4e1c2a9 on main",
        when: "Oct 2, 9:12 PM",
      },
    ],
  },
  {
    id: "grep",
    who: "drone",
    landsIn: "kit",
    statement: "A Drone waited on grep.",
    title: "A Drone had to wait for grep to be allowed",
    what: "It asked to run grep on a check log, and the step waited until you allowed it.",
    fix: "Add grep on .armada/checks to the allowlist.",
    answers: answers("Saves it under Accepted"),
    cites: [{ id: "asked:1", name: "Allow grep on .armada/checks?", detail: "Allow", when: "Oct 2, 9:10 PM" }],
  },
  {
    id: "dock",
    who: "owner",
    statement: "The Judge's question waited in the dock while the plan was read twice.",
    answers: answers("Agrees with it"),
  },
];

/**
 * **Written, with the owner's notes under it.** Evidence is behind a control
 * on each item that cites rows, and an item with a statement alone draws it as
 * the body.
 */
export const Written: Story = {
  name: "A written retro",
  args: {
    items: ITEMS,
    notes: [{ id: "20261002-213000-k3f9", text: "Why did the restart show twice?", when: "Oct 2, 9:30 PM" }],
  },
  play: async ({ canvas }) => {
    const sheet = canvas.getByRole("dialog", { name: "Retro" });
    await expect(sheet).toBeVisible();
    await expect(canvas.getAllByRole("button", { name: "Agree" })).toHaveLength(3);
    // Collapsed until asked for.
    await expect(canvas.queryByText("out_of_bounds")).toBeNull();
    await userEvent.click(canvas.getAllByRole("button", { name: "Evidence" })[0]!);
    await expect(canvas.getByText("out_of_bounds")).toBeVisible();
    await expect(canvas.getByRole("region", { name: "Notes" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Lands in Armada" })).toBeVisible();
  },
};

/** The Job has not ended, or its retro is not written yet. */
export const Pending: Story = {
  name: "A retro not written yet",
  args: { status: "Not written yet" },
};

/** Written, and nothing got in the way. An empty slot stays empty. */
export const NothingInTheWay: Story = {
  name: "A retro with nothing in the way",
  args: { items: [], notes: [] },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("list")).toBeNull();
    await expect(canvas.queryByRole("region", { name: "Notes" })).toBeNull();
  },
};

/** The read itself failed: Fleet's words, in place of everything else. */
export const ReadFailed: Story = {
  name: "A retro that could not be read",
  args: { failure: "Fleet is not running", items: ITEMS },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The retro could not be read")).toBeVisible();
    await expect(canvas.queryByText("out_of_bounds")).toBeNull();
  },
};

/** At `--window-floor`: flush to both edges, icon close. */
export const AtTheFloor: Story = {
  name: "A retro at the floor",
  args: { items: ITEMS, floor: true },
};
