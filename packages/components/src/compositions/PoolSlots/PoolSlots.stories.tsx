import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";
import type { WorktreeSlot } from "@armada/protocol";

import { PoolSlots, type PoolSlotRow } from "./PoolSlots";

/** A repository's worktree pool, as Cleanup draws it: a bay per slot, styled by availability. */
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
 * Every state a slot can be in, a bay each. **A held bay is a filled card and
 * its Job a link that opens it**; a free bay is an open dashed outline holding
 * only its name, its state and its warmth; a stranded bay is hatched and says
 * why; a slot not made is a ghost with no warmth. A bare figure is named.
 */
export const EveryState: Story = {
  name: "Every state",
  args: { rows: ROWS },
  play: async ({ args, canvas, userEvent }) => {
    const bays = within(canvas.getByRole("list", { name: "Worktree slots" })).getAllByRole("listitem");
    await expect(bays).toHaveLength(ROWS.length);
    const bay = (n: number) => canvas.getByRole("listitem", { name: `slot-${n}` });

    const held = within(bay(1));
    await expect(held.getByRole("img", { name: "Held" })).toBeInTheDocument();
    await expect(held.getByLabelText("Held for: 2 hours")).toBeInTheDocument();
    await expect(held.getByLabelText("Commits behind main: 0")).toBeInTheDocument();
    await userEvent.click(held.getByRole("button", { name: "Fix the reader" }));
    await expect(args.onOpenJob).toHaveBeenCalledWith("01JOB");

    await expect(within(bay(2)).getByText("zsh (pid 4120)")).toBeInTheDocument();
    await expect(within(bay(2)).queryByRole("button")).toBeNull();

    const free = within(bay(3));
    await expect(free.getByRole("img", { name: "Free" })).toBeInTheDocument();
    await expect(free.getByRole("img", { name: "Cold" })).toBeInTheDocument();
    await expect(free.queryByLabelText(/Held for|Commits behind/)).toBeNull();

    await expect(within(bay(4)).getByText("2 uncommitted, first src/lib.rs")).toBeInTheDocument();
    await expect(within(bay(6)).getByRole("img", { name: "Not made yet" })).toBeInTheDocument();
    await expect(within(bay(6)).queryByRole("img", { name: /Warm|Cold/ })).toBeNull();

    // Availability is the bay's own shape: filled and solid, open and dashed, hatched.
    const style = (n: number) => getComputedStyle(bay(n));
    await expect(style(1).borderTopStyle).toBe("solid");
    await expect(style(3).borderTopStyle).toBe("dashed");
    await expect(style(6).borderTopStyle).toBe("dashed");
    await expect(style(4).backgroundImage).toContain("repeating-linear-gradient");
    await expect(style(1).backgroundColor).not.toBe(style(3).backgroundColor);
  },
};
