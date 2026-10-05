import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, fn, within } from "storybook/test";
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
    await expect(held.getByLabelText("Held for 2 hours")).toBeInTheDocument();
    await expect(held.getByLabelText("0 commits behind main")).toBeInTheDocument();
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
    const bay = (n: number) => within(canvas.getByRole("listitem", { name: `slot-${n}` }));
    await expect(bay(1).getByLabelText("Held for 1 day")).toHaveTextContent("1 day");
    await expect(bay(1).getByLabelText("1 commit behind develop")).toHaveTextContent("1 behind");
    await expect(bay(2).getByLabelText("3 commits behind main")).toHaveTextContent("3 behind");
    await expect(bay(2).getByLabelText("Held for 3 days")).toBeInTheDocument();
  },
};

const CLOSED: PoolSlotRow[] = [
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
    const bay = (n: number) => within(canvas.getByRole("listitem", { name: `slot-${n}` }));

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
 * The acts. **Close or reopen on every bay, remove only on a free or unmade
 * one**, and a ghost tile after the last bay adds one. Each glyph is named.
 */
export const Acts: Story = {
  name: "Acts",
  args: { rows: [...ROWS, { slot: slot(8, { held: { state: "unmade" }, closed: true }) }], onAct: fn() },
  play: async ({ args, canvas, userEvent }) => {
    const bay = (n: number) => within(canvas.getByRole("listitem", { name: `slot-${n}` }));

    await userEvent.click(bay(3).getByRole("button", { name: "Close" }));
    await expect(args.onAct).toHaveBeenLastCalledWith("close", 3);
    await userEvent.click(bay(3).getByRole("button", { name: "Remove" }));
    await expect(args.onAct).toHaveBeenLastCalledWith("remove", 3);
    await userEvent.click(bay(6).getByRole("button", { name: "Remove" }));
    await expect(args.onAct).toHaveBeenLastCalledWith("remove", 6);
    await userEvent.click(bay(8).getByRole("button", { name: "Reopen" }));
    await expect(args.onAct).toHaveBeenLastCalledWith("open", 8);

    // Held, a session's, stranded, busy and not a checkout: nothing to remove.
    for (const n of [1, 2, 4, 5, 7]) {
      await expect(bay(n).queryByRole("button", { name: "Remove" })).toBeNull();
      await expect(bay(n).getByRole("button", { name: "Close" })).toBeInTheDocument();
    }

    await userEvent.click(canvas.getByRole("button", { name: "Add a slot" }));
    await expect(args.onAct).toHaveBeenLastCalledWith("add");
  },
};

/**
 * What Fleet said when it would not, **on the bay it was about**: a slot
 * written to between the read and the press, and an add the pool refused.
 * A bay waiting on Fleet sends nothing twice.
 */
export const Refused: Story = {
  name: "Refused",
  args: {
    rows: [
      { slot: slot(1, { held: { state: "free" } }), refused: "Not removed: 1 uncommitted, first notes.md" },
      { slot: slot(2, { held: { state: "free" } }), acting: true },
    ],
    onAct: fn(),
    addRefused: "Not added: .armada/slots/pool: permission denied",
  },
  play: async ({ args, canvas }) => {
    const bay = (n: number) => within(canvas.getByRole("listitem", { name: `slot-${n}` }));
    await expect(bay(1).getByRole("alert")).toHaveTextContent("Not removed: 1 uncommitted, first notes.md");
    await expect(bay(2).queryByRole("alert")).toBeNull();
    await expect(canvas.getAllByRole("alert")).toHaveLength(2);

    // Past the pointer, which a waiting bay's acts refuse: the press itself sends nothing.
    fireEvent.click(bay(2).getByRole("button", { name: "Remove" }));
    await expect(args.onAct).not.toHaveBeenCalled();
  },
};

const STRANDED: Pick<WorktreeSlot, "held" | "branch" | "behind" | "stranded"> = {
  held: { state: "stranded", why: "2 uncommitted, first src/lib.rs" },
  branch: "fleet/old-try",
  behind: 12,
  stranded: {
    uncommitted: ["src/lib.rs", "src/reader/retry.rs"],
    commits: [
      { sha: "9d41e07b2c", subject: "Retry a short read once" },
      { sha: "3b7a1c9e55", subject: "Split the reader from the parser" },
    ],
    unpushed: 1,
  },
};

const FOUND = {
  commit: "9d41e07b2c",
  uncommitted: true,
  read: ["src/reader/mod.rs", "src/reader/retry.rs"],
  searched: ["retry_short_read in src/"],
};

const bay = (canvas: { getByRole: (role: "listitem", options: { name: string }) => HTMLElement }, n: number) =>
  within(canvas.getByRole("listitem", { name: `slot-${n}` }));

/**
 * A stranded bay with no rescue offers Rescue, and only a stranded one does: a
 * held, a free and a ghost bay have nothing to rescue. The act sends the slot.
 */
export const RescueOffered: Story = {
  name: "Rescue offered",
  args: {
    rows: [
      { slot: slot(1, { held: { state: "free" } }) },
      { slot: slot(4, STRANDED), heldFor: "3 days" },
      { slot: slot(6, { held: { state: "unmade" } }) },
    ],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getAllByRole("button", { name: "Rescue" })).toHaveLength(1);
    await expect(bay(canvas, 4).queryByRole("region", { name: "Finding" })).toBeNull();
    await userEvent.click(bay(canvas, 4).getByRole("button", { name: "Rescue" }));
    await expect(args.onRescue).toHaveBeenCalledWith("start", 4);
    // The pool's own acts stay beside it.
    await expect(bay(canvas, 4).getByRole("button", { name: "Close" })).toBeInTheDocument();
  },
};

/**
 * A Scout reading: the live state is a pulsing mark and the files so far, with
 * Stop where Rescue was. **Scrap and Stash wait for the Finding**, so neither
 * is on the bay while it reads.
 */
export const Reading: Story = {
  name: "Reading",
  args: {
    rows: [{ slot: slot(4, { ...STRANDED, rescue: { ...FOUND, state: "reading", searched: [] } }) }],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const finding = within(bay(canvas, 4).getByRole("region", { name: "Finding" }));
    await expect(finding.getByRole("img", { name: "Reading" })).toBeInTheDocument();
    await expect(finding.getByRole("list", { name: "Read" })).toHaveTextContent("src/reader/retry.rs");
    await expect(finding.queryByRole("list", { name: "Searched" })).toBeNull();
    await expect(bay(canvas, 4).queryByRole("button", { name: /Rescue|Scrap|Stash/ })).toBeNull();
    await userEvent.click(bay(canvas, 4).getByRole("button", { name: "Stop" }));
    await expect(args.onRescue).toHaveBeenCalledWith("stop", 4);
  },
};

/**
 * The Finding, on the bay: the summary, what it read and searched, the slot's
 * commits, the commit it read and that uncommitted changes sat on it, and what
 * was cut. **Stash acts at once**; Scrap waits on its confirm.
 */
export const FindingAnswered: Story = {
  name: "Finding answered",
  args: {
    rows: [
      {
        slot: slot(4, {
          ...STRANDED,
          rescue: {
            ...FOUND,
            state: "answered",
            cut: 1200,
            summary: "A retry for a short read, finished in the commit and half-moved in the working files.",
          },
        }),
      },
    ],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const finding = within(bay(canvas, 4).getByRole("region", { name: "Finding" }));
    await expect(finding.getByText(/A retry for a short read/)).toBeInTheDocument();
    await expect(finding.getByRole("list", { name: "Read" })).toHaveTextContent("src/reader/mod.rs");
    await expect(finding.getByRole("list", { name: "Searched" })).toHaveTextContent("retry_short_read in src/");
    await expect(finding.getByRole("list", { name: "Commits" })).toHaveTextContent("9d41e07 Retry a short read once");
    await expect(finding.getByLabelText("Commit it read: 9d41e07")).toBeInTheDocument();
    await expect(finding.getByRole("img", { name: "Uncommitted changes on top" })).toBeInTheDocument();
    await expect(finding.getByLabelText(/cut before it read them: 1200/)).toHaveTextContent("1200 cut");
    await expect(finding.queryByRole("img", { name: "Reading" })).toBeNull();
    await expect(bay(canvas, 4).queryByRole("button", { name: /Rescue|Stop/ })).toBeNull();

    await userEvent.click(bay(canvas, 4).getByRole("button", { name: "Stash" }));
    await expect(args.onRescue).toHaveBeenCalledTimes(1);
    await expect(args.onRescue).toHaveBeenCalledWith("stash", 4);
  },
};

/**
 * A Scout that ended without an answer. A failed one shows why it failed, a
 * stopped one is marked stopped, both keep what they read, and **both still
 * offer Scrap and Stash**. Nothing is cut and nothing was left uncommitted, so
 * neither mark is drawn.
 */
export const FindingFailed: Story = {
  name: "Finding failed",
  args: {
    rows: [
      {
        slot: slot(4, {
          ...STRANDED,
          rescue: { ...FOUND, uncommitted: false, state: "failed", why: "The Scout ended before it answered" },
        }),
      },
      { slot: slot(5, { ...STRANDED, rescue: { ...FOUND, uncommitted: false, state: "stopped" } }) },
    ],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ canvas }) => {
    await expect(bay(canvas, 4).getByText("The Scout ended before it answered")).toBeInTheDocument();
    await expect(bay(canvas, 4).queryByRole("img", { name: "Stopped" })).toBeNull();
    await expect(bay(canvas, 5).getByRole("img", { name: "Stopped" })).toBeInTheDocument();
    for (const n of [4, 5]) {
      await expect(bay(canvas, n).getByRole("list", { name: "Read" })).toHaveTextContent("src/reader/retry.rs");
      await expect(bay(canvas, n).getByRole("button", { name: "Scrap" })).toBeInTheDocument();
      await expect(bay(canvas, n).getByRole("button", { name: "Stash" })).toBeInTheDocument();
      await expect(bay(canvas, n).queryByRole("img", { name: "Uncommitted changes on top" })).toBeNull();
      await expect(bay(canvas, n).queryByLabelText(/cut before/)).toBeNull();
    }
  },
};

/**
 * Scrap asks first, and the confirm names what goes: every uncommitted file,
 * and the commits not pushed anywhere. **A pushed commit is not named**, and
 * nothing is sent until the confirm's own Scrap.
 */
export const ScrapConfirm: Story = {
  name: "Scrap confirm",
  args: {
    rows: [{ slot: slot(4, { ...STRANDED, rescue: { ...FOUND, state: "answered", summary: "Half moved." } }) }],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(bay(canvas, 4).getByRole("button", { name: "Scrap" }));
    await expect(args.onRescue).not.toHaveBeenCalled();

    const confirm = within(canvas.getByRole("group", { name: "Scrap slot-4" }));
    await expect(confirm.getByRole("list", { name: "Uncommitted" })).toHaveTextContent("src/lib.rs");
    await expect(confirm.getByRole("list", { name: "Uncommitted" })).toHaveTextContent("src/reader/retry.rs");
    await expect(confirm.getByRole("list", { name: "Unpushed" })).toHaveTextContent("Retry a short read once");
    await expect(confirm.getByRole("list", { name: "Unpushed" })).not.toHaveTextContent("Split the reader");
    // The bay's own Scrap gives way to the confirm's, so there is one to press.
    await expect(canvas.getAllByRole("button", { name: "Scrap" })).toHaveLength(1);

    await userEvent.click(confirm.getByRole("button", { name: "Cancel" }));
    await expect(canvas.queryByRole("group", { name: "Scrap slot-4" })).toBeNull();
    await expect(args.onRescue).not.toHaveBeenCalled();

    await userEvent.click(bay(canvas, 4).getByRole("button", { name: "Scrap" }));
    await userEvent.click(within(canvas.getByRole("group", { name: "Scrap slot-4" })).getByRole("button", { name: "Scrap" }));
    await expect(args.onRescue).toHaveBeenCalledTimes(1);
    await expect(args.onRescue).toHaveBeenCalledWith("scrap", 4);
    await expect(canvas.queryByRole("group", { name: "Scrap slot-4" })).toBeNull();
  },
};

/**
 * A slot with nothing unpushed and nothing uncommitted still confirms, and its
 * confirm names nothing: an empty list stays empty.
 */
export const ScrapConfirmNothingToName: Story = {
  name: "Scrap confirm, nothing to name",
  args: {
    rows: [
      {
        slot: slot(4, {
          ...STRANDED,
          stranded: { uncommitted: [], commits: [], unpushed: 0 },
          rescue: { ...FOUND, state: "answered" },
        }),
      },
    ],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(bay(canvas, 4).getByRole("button", { name: "Scrap" }));
    const confirm = within(canvas.getByRole("group", { name: "Scrap slot-4" }));
    await expect(confirm.queryByRole("list")).toBeNull();
    await expect(confirm.getByRole("button", { name: "Scrap" })).toBeInTheDocument();
  },
};

/**
 * Fleet's refusal of a rescue act, **on the bay it was about**, by the same
 * path as the pool's acts; and what a Scrap or Stash did, said as a bare fact
 * on the slot it freed.
 */
export const RescueRefusedAndReceipt: Story = {
  name: "Rescue refused, and receipt",
  args: {
    rows: [
      {
        slot: slot(4, { ...STRANDED, rescue: { ...FOUND, state: "answered", summary: "Half moved." } }),
        refused: "Not stashed: a Scout is reading it",
      },
      { slot: slot(5, { held: { state: "free" } }), said: "fleet/old-try kept" },
      { slot: slot(6, { held: { state: "free" } }), said: "c0ffee1 on fleet/old-try" },
    ],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ canvas }) => {
    await expect(bay(canvas, 4).getByRole("alert")).toHaveTextContent("Not stashed: a Scout is reading it");
    await expect(bay(canvas, 5).getByRole("status")).toHaveTextContent("fleet/old-try kept");
    await expect(bay(canvas, 6).getByRole("status")).toHaveTextContent("c0ffee1 on fleet/old-try");
    await expect(bay(canvas, 4).queryByRole("status")).toBeNull();
  },
};

/** A pool drawn with no rescue wiring offers none of its acts, even on a stranded bay. */
export const RescueNotWired: Story = {
  name: "Rescue not wired",
  args: { rows: [{ slot: slot(4, STRANDED) }], onAct: fn() },
  play: async ({ canvas }) => {
    await expect(bay(canvas, 4).queryByRole("button", { name: /Rescue|Stop|Scrap|Stash/ })).toBeNull();
  },
};

const KEPT: Pick<WorktreeSlot, "held" | "branch" | "behind" | "stranded"> = {
  held: {
    state: "job",
    job_id: "01KEPT",
    job_title: "Retry the manifest read",
    job_status: "killed",
    kept: "5 uncommitted, first crates/api/src/routes.rs",
  },
  branch: "armada/14-retry-the-manifest-read",
  behind: 3,
  stranded: {
    uncommitted: ["crates/api/src/routes.rs", "docs/notes/retry.md"],
    commits: [
      { sha: "b61d3a0e94", subject: "Retry the manifest read on a short answer" },
      { sha: "28c7f5d1a3", subject: "Name the manifest in the read error" },
    ],
    unpushed: 1,
  },
};

/**
 * A Job that ended and kept its slot is its own bay, apart from a live hold and
 * from a stranded slot: it says Kept and why, as a bare fact, and keeps its
 * Job's title as a link. A Job still holding with nothing kept stays Held.
 */
export const KeptJob: Story = {
  name: "Kept Job",
  args: {
    rows: [
      { slot: slot(1, { held: { state: "job", job_id: "01JOB", job_title: "Fix the reader" } }) },
      { slot: slot(8, KEPT), heldFor: "26 hours" },
    ],
    onAct: fn(),
    onRescue: fn(),
    onOpenJob: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(bay(canvas, 1).getByRole("img", { name: "Held" })).toBeInTheDocument();
    const kept = bay(canvas, 8);
    await expect(kept.getByRole("img", { name: "Kept: 5 uncommitted, first crates/api/src/routes.rs" })).toBeInTheDocument();
    await expect(kept.getByText("5 uncommitted, first crates/api/src/routes.rs")).toBeInTheDocument();
    await expect(kept.queryByRole("img", { name: "Held" })).toBeNull();
    await userEvent.click(kept.getByRole("button", { name: "Retry the manifest read" }));
    await expect(args.onOpenJob).toHaveBeenCalledWith("01KEPT");
    // Rescue is offered on the kept bay alone: the live hold has no work to rescue.
    await expect(bay(canvas, 1).queryByRole("button", { name: "Rescue" })).toBeNull();
    await userEvent.click(kept.getByRole("button", { name: "Rescue" }));
    await expect(args.onRescue).toHaveBeenCalledWith("start", 8);
  },
};

/**
 * The kept bay's rescue is a stranded bay's: its Finding opens on it, Stash
 * acts at once, and Scrap confirms by naming the files and the unpushed commit.
 */
export const KeptJobFinding: Story = {
  name: "Kept Job finding",
  args: {
    rows: [{ slot: slot(8, { ...KEPT, rescue: { ...FOUND, state: "answered", summary: "Route edited, test not." } }) }],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(bay(canvas, 8).getByRole("region", { name: "Finding" })).toHaveTextContent("Route edited, test not.");
    await userEvent.click(bay(canvas, 8).getByRole("button", { name: "Stash" }));
    await expect(args.onRescue).toHaveBeenLastCalledWith("stash", 8);
    await userEvent.click(bay(canvas, 8).getByRole("button", { name: "Scrap" }));
    const confirm = within(canvas.getByRole("group", { name: "Scrap slot-8" }));
    await expect(confirm.getByRole("list", { name: "Uncommitted" })).toHaveTextContent("docs/notes/retry.md");
    await expect(confirm.getByRole("list", { name: "Unpushed" })).toHaveTextContent("Retry the manifest read on a short answer");
    await expect(confirm.getByRole("list", { name: "Unpushed" })).not.toHaveTextContent("Name the manifest");
    await userEvent.click(confirm.getByRole("button", { name: "Scrap" }));
    await expect(args.onRescue).toHaveBeenLastCalledWith("scrap", 8);
  },
};
