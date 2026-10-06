import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, fn, within } from "storybook/test";

import { PoolSlots, type TileRow } from "./PoolSlots";
import { openTile, slot, tile } from "./PoolSlots.fixtures";

/** A repository's worktree pool, as Cleanup draws it: a bay per slot, styled by availability. */
const meta: Meta<typeof PoolSlots> = {
  title: "Compositions/Pool slots",
  component: PoolSlots,
  args: { onOpenJob: fn() },
};
export default meta;

type Story = StoryObj<typeof PoolSlots>;

const ROWS: TileRow[] = [
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
    await expect(held.getByLabelText("Held for 2 hours")).toBeInTheDocument();
    await expect(held.getByLabelText("0 commits behind main")).toBeInTheDocument();
    await userEvent.click(held.getByRole("button", { name: "Fix the reader" }));
    await expect(args.onOpenJob).toHaveBeenCalledWith("01JOB");

    await expect(within(bay(2)).getByText("zsh (pid 4120)")).toBeInTheDocument();
    // Its name is the tile's one button, and a session has no Job to open.
    await expect(within(bay(2)).getAllByRole("button")).toHaveLength(1);

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

/**
 * A bare figure says what it measures, in its tooltip and its name: the age a
 * holder has had the slot, and how far the checkout is behind its base, with
 * the unit singular for one.
 */
export const Figures: Story = {
  name: "Figures",
  args: {
    rows: [
      {
        slot: slot(1, {
          held: { state: "session", holder: "zsh (pid 4120)" },
          branch: "fleet/a",
          base: "develop",
          behind: 1,
        }),
        heldFor: "1 day",
      },
      { slot: slot(2, { held: { state: "session", holder: "zsh (pid 4121)" }, behind: 3 }), heldFor: "3 days" },
    ],
  },
  play: async ({ canvas }) => {
    const bay = (n: number) => tile(canvas, `slot-${n}`);
    await expect(bay(1).getByLabelText("Held for 1 day")).toHaveTextContent("1 day");
    await expect(bay(1).getByLabelText("1 commit behind develop")).toHaveTextContent("1 behind");
    await expect(bay(2).getByLabelText("3 commits behind main")).toHaveTextContent("3 behind");
    await expect(bay(2).getByLabelText("Held for 3 days")).toBeInTheDocument();
  },
};

const CLOSED: TileRow[] = [
  {
    slot: slot(1, {
      held: { state: "job", job_id: "01JOB", job_title: "Fix the reader" },
      branch: "armada/12-fix-the-reader",
      warm: true,
      behind: 0,
      closed: true,
    }),
    heldFor: "2 hours",
  },
  { slot: slot(2, { held: { state: "free" }, warm: true, closed: true }) },
  { slot: slot(3, { held: { state: "unmade" }, closed: true }) },
  { slot: slot(4, { held: { state: "free" }, warm: true }) },
];

/**
 * Closed bays. **A free or unmade bay closed is shuttered** and says Closed
 * where its state was; **a held bay closed keeps its holder** and takes the
 * closed mark in its band, since its lease runs to the end.
 */
export const Closed: Story = {
  name: "Closed",
  args: { rows: CLOSED },
  play: async ({ canvas }) => {
    const bay = (n: number) => tile(canvas, `slot-${n}`);

    await expect(bay(1).getByRole("img", { name: "Held" })).toBeInTheDocument();
    await expect(bay(1).getByRole("img", { name: "Closed" })).toBeInTheDocument();
    await expect(bay(1).getByRole("button", { name: "Fix the reader" })).toBeInTheDocument();

    await expect(bay(2).getByRole("img", { name: "Closed" })).toBeInTheDocument();
    await expect(bay(2).queryByRole("img", { name: "Free" })).toBeNull();
    await expect(bay(3).getByRole("img", { name: "Closed" })).toBeInTheDocument();
    await expect(bay(4).queryByRole("img", { name: "Closed" })).toBeNull();

    // Shuttered is its own shape: solid where a free bay is dashed.
    const style = (n: number) => getComputedStyle(canvas.getByRole("listitem", { name: `slot-${n}` }));
    await expect(style(2).borderTopStyle).toBe("solid");
    await expect(style(4).borderTopStyle).toBe("dashed");
  },
};

/**
 * **A tile carries no acts: a press opens its panel**, and the acts are there.
 * Close or Reopen on every bay, Remove only on a free or unmade one, and a ghost
 * tile after the last bay adds one. Each glyph is named by its tooltip.
 */
export const Acts: Story = {
  name: "Acts",
  args: { rows: [...ROWS, { slot: slot(8, { held: { state: "unmade" }, closed: true }) }], onAct: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.queryByRole("button", { name: /^(Close|Reopen|Remove) slot$/ })).toBeNull();

    const act = async (n: number, name: string) => {
      const open = await openTile(canvas, userEvent, `slot-${n}`);
      await userEvent.click(open.getByRole("button", { name }));
      await userEvent.keyboard("{Escape}");
    };
    await act(3, "Close slot");
    await expect(args.onAct).toHaveBeenLastCalledWith("close", 3);
    await act(3, "Remove slot");
    await expect(args.onAct).toHaveBeenLastCalledWith("remove", 3);
    await act(6, "Remove slot");
    await expect(args.onAct).toHaveBeenLastCalledWith("remove", 6);
    await act(8, "Reopen slot");
    await expect(args.onAct).toHaveBeenLastCalledWith("open", 8);

    // Held, a session's, stranded, busy and not a checkout: nothing to remove.
    for (const n of [1, 2, 4, 5, 7]) {
      const open = await openTile(canvas, userEvent, `slot-${n}`);
      await expect(open.queryByRole("button", { name: "Remove slot" })).toBeNull();
      await expect(open.getByRole("button", { name: "Close slot" })).toBeInTheDocument();
      await userEvent.keyboard("{Escape}");
    }

    await userEvent.click(canvas.getByRole("button", { name: "Add a slot" }));
    await expect(args.onAct).toHaveBeenLastCalledWith("add");

    // Every act has an edge at rest, so none reads as a bare glyph.
    const open = await openTile(canvas, userEvent, "slot-3");
    for (const one of open.getAllByRole("button", { name: /^(Close|Remove) slot$/ })) {
      await expect(getComputedStyle(one).borderTopStyle).toBe("solid");
    }
  },
};

/**
 * What Fleet said when it would not, **in the panel of the tile it was about**:
 * a slot written to between the read and the press. An add the pool refused is
 * said on the add tile, which has no panel. A tile waiting on Fleet sends
 * nothing twice.
 */
export const Refused: Story = {
  name: "Refused",
  args: {
    rows: [
      { slot: slot(1, { held: { state: "free" } }), refused: "Slot not removed: 1 uncommitted, first notes.md" },
      { slot: slot(2, { held: { state: "free" } }), acting: true },
    ],
    onAct: fn(),
    addRefused: "Slot not added: .armada/slots/pool: permission denied",
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(tile(canvas, "slot-1").queryByRole("alert")).toBeNull();
    const first = await openTile(canvas, userEvent, "slot-1");
    await expect(first.getByRole("alert")).toHaveTextContent("Slot not removed: 1 uncommitted, first notes.md");
    await userEvent.keyboard("{Escape}");

    const second = await openTile(canvas, userEvent, "slot-2");
    await expect(second.queryByRole("alert")).toBeNull();
    // Past the pointer, which a waiting tile's acts refuse: the press itself sends nothing.
    fireEvent.click(second.getByRole("button", { name: "Remove slot" }));
    await expect(args.onAct).not.toHaveBeenCalled();
    await userEvent.keyboard("{Escape}");

    await expect(canvas.getByRole("alert")).toHaveTextContent("Slot not added: .armada/slots/pool: permission denied");
  },
};
