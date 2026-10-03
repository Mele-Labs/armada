import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { RetroSheet, type RetroSheetItem } from "./RetroSheet";

/**
 * One Job's retro on its sheet, in Job 3's shape (2 Oct 2026): an
 * out_of_bounds failure on armada.yml an upstream commit brought in, a Judge
 * `not_met`, the owner agreeing and then restarting, Helm's acts through
 * `curl`, a command he was asked to allow, and a Drone that never saw its
 * test pass.
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

const ITEMS: RetroSheetItem[] = [
  {
    who: "fleet",
    landsIn: "armada",
    statement: "The gate failed out_of_bounds on armada.yml, a line that came in with an upstream commit.",
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
    who: "drone",
    statement: "The plan was refused on addresses_the_request for a documentation task nobody asked for.",
    cites: [
      {
        id: "not_met:1",
        name: "addresses_the_request",
        mono: true,
        detail: "T4 adds documentation updates to design-system.md",
      },
    ],
  },
  {
    who: "owner",
    statement: "You agreed with the refusal, then restarted the step.",
    cites: [
      { id: "act:1", name: "answer_judge_question", detail: "owner via bridge", when: "Oct 1, 8:31 PM" },
      { id: "restart:1", name: "restart_step", detail: "owner via bridge", when: "Oct 1, 8:33 PM" },
    ],
  },
  {
    who: "owner",
    statement: "Helm restarted the step and re-ran the Checks with curl.",
    cites: [
      { id: "act:2", name: "restart_step", detail: "helm via http", when: "Oct 2, 9:20 PM" },
      { id: "act:3", name: "rerun_checks", detail: "helm via http", when: "Oct 2, 9:22 PM" },
    ],
  },
  {
    who: "drone",
    statement: "The Drone handed in without ever seeing screens_test pass.",
    cites: [{ id: "said:1", name: "implement", mono: true, detail: "I never saw screens_test pass locally." }],
  },
];

/**
 * **Written, with the owner's notes under it.** Each item leads with whose
 * way it got in, as a mark its tooltip names, and nothing on the sheet acts:
 * the one control is the close.
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
    // Nothing acts on a retro: the close is the only button.
    const buttons = canvas.getAllByRole("button").filter((one) => sheet.contains(one));
    await expect(buttons.map((one) => one.textContent)).toEqual([expect.stringContaining("Close")]);
    await expect(canvas.getByText("out_of_bounds")).toBeVisible();
    await expect(canvas.getByRole("region", { name: "Notes" })).toBeVisible();
    // Where the fix lands, beside whose way, on the item that says.
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
