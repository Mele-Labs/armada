import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";
import type { WorktreeSlot } from "@armada/protocol";

import { PoolSlots, type PoolSlotRow } from "./PoolSlots";

/** A repository's worktree pool, as Cleanup draws it: one row per slot. */
const meta: Meta<typeof PoolSlots> = {
  title: "Compositions/Pool slots",
  component: PoolSlots,
  args: { onOpenJob: fn() },
};
export default meta;

type Story = StoryObj<typeof PoolSlots>;

const AT = "/Users/user/armada/.armada/slots";

function slot(n: number, rest: Partial<WorktreeSlot> & Pick<WorktreeSlot, "held">): WorktreeSlot {
  return { manifest_id: "armada", slot: n, path: `${AT}/slot-${n}`, base: "main", warm: false, ...rest };
}

const ROWS: PoolSlotRow[] = [
  {
    slot: slot(1, {
      held: { state: "job", job_id: "01JOB", job_title: "Fix the reader" },
      branch: "armada/12-fix-the-reader",
      warm: true,
      behind: 0,
    }),
    heldFor: "2 hours",
  },
  {
    slot: slot(2, {
      held: { state: "session", holder: "zsh (pid 4120)" },
      branch: "fleet/slot-pool-in-cleanup",
      warm: true,
      behind: 7,
    }),
    heldFor: "40 minutes",
  },
  { slot: slot(3, { held: { state: "free" }, warm: false, behind: 3 }) },
  {
    slot: slot(4, {
      held: { state: "stranded", why: "2 uncommitted, first src/lib.rs" },
      branch: "fleet/old-try",
      behind: 12,
    }),
    heldFor: "3 days",
  },
  { slot: slot(5, { held: { state: "busy" } }) },
  { slot: slot(6, { held: { state: "unmade" } }) },
  { slot: slot(7, { held: { state: "not_a_checkout" } }) },
];

/**
 * Every state a slot can be in, under a header naming each column. **A Job
 * holder is a link that opens its Job**; a session holder is named and not
 * pressable; a slot not made has no warmth mark. Leased, free and stranded rows
 * take their hue; a slot not made stays neutral.
 */
export const EveryState: Story = {
  name: "Every state",
  args: { rows: ROWS },
  play: async ({ args, canvas, userEvent }) => {
    for (const heading of ["Slot", "State", "Build", "Branch", "Held by", "Held for", "Behind main"]) {
      await expect(canvas.getByRole("columnheader", { name: heading })).toBeInTheDocument();
    }
    const rows = canvas.getAllByRole("row").slice(1);
    await expect(rows).toHaveLength(ROWS.length);
    await expect(within(rows[3]!).getByRole("cell", { name: "12" })).toBeInTheDocument();

    await expect(canvas.queryByRole("button", { name: /zsh/ })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Fix the reader" }));
    await expect(args.onOpenJob).toHaveBeenCalledWith("01JOB");

    const unmade = rows[5]!;
    await expect(within(unmade).getAllByRole("img", { name: "Not made yet" })).toHaveLength(1);
    await expect(within(unmade).queryByRole("img", { name: /Warm|Cold/ })).toBeNull();

    // A hue, not a value: the leased and free rows and marks differ from the neutral one.
    const look = (row: HTMLElement, mark: string | RegExp) => ({
      row: getComputedStyle(row).backgroundColor,
      mark: getComputedStyle(within(row).getByRole("img", { name: mark })).color,
    });
    const neutral = look(unmade, "Not made yet");
    for (const [row, mark] of [[rows[0]!, "Held"], [rows[2]!, "Free"], [rows[3]!, /Stranded/]] as const) {
      const toned = look(row, mark);
      await expect(toned.row).not.toBe(neutral.row);
      await expect(toned.mark).not.toBe(neutral.mark);
    }
  },
};
