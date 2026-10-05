import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import type { WorktreeSlot } from "@armada/protocol";

import { PoolSlots } from "./PoolSlots";
import { FOUND, STRANDED, openTile, panel, slot, tile } from "./PoolSlots.fixtures";

/** A stranded bay's rescue, read and acted on in its tile's panel. */
const meta: Meta<typeof PoolSlots> = {
  title: "Compositions/Pool slots",
  component: PoolSlots,
  args: { onOpenJob: fn() },
};
export default meta;

type Story = StoryObj<typeof PoolSlots>;

const bay = (canvas: Parameters<typeof tile>[0], n: number) => tile(canvas, `slot-${n}`);

/** Open the panel of bay `n`, where its Finding is. */
const opened = (
  canvas: Parameters<typeof tile>[0],
  userEvent: Parameters<typeof openTile>[1],
  n: number,
) => openTile(canvas, userEvent, `slot-${n}`);

const sheet = (canvas: Parameters<typeof tile>[0], n = 4) => panel(canvas, `slot-${n}`);

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
      { sha: "b61d3a0e94", subject: "Retry the manifest read on a short answer", home: "only_here" },
      { sha: "28c7f5d1a3", subject: "Name the manifest in the read error", home: "on_remote" },
    ],
    unpushed: 1,
  },
};

/**
 * A stranded bay with no rescue offers Rescue in its panel, and only a stranded
 * one does: a held, a free and a ghost bay have nothing to rescue. The act sends
 * the slot.
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
    // The tile itself carries no act.
    await expect(canvas.queryByRole("button", { name: "Rescue" })).toBeNull();
    for (const n of [1, 6]) {
      const open = await opened(canvas, userEvent, n);
      await expect(open.queryByRole("button", { name: "Rescue" })).toBeNull();
      await userEvent.keyboard("{Escape}");
    }
    const open = await opened(canvas, userEvent, 4);
    await expect(open.queryByRole("heading", { name: "Unfinished" })).toBeNull();
    await userEvent.click(open.getByRole("button", { name: "Rescue" }));
    await expect(args.onRescue).toHaveBeenCalledWith("start", 4);
    await expect(getComputedStyle(open.getByRole("button", { name: "Rescue" })).borderTopStyle).toBe("solid");
    // The pool's own acts stay beside it.
    await expect(open.getByRole("button", { name: "Close" })).toBeInTheDocument();
  },
};

/**
 * A Scout reading: the tile shows the live mark, the panel Stop where Rescue
 * was, and the files so far are read there. **Scrap and Stash wait for the
 * Finding**, so neither is offered while it reads.
 */
export const Reading: Story = {
  name: "Reading",
  args: {
    rows: [{ slot: slot(4, { ...STRANDED, rescue: { ...FOUND, state: "reading", searched: [] } }) }],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(bay(canvas, 4).getByRole("img", { name: "Reading" })).toBeInTheDocument();
    await expect(bay(canvas, 4).queryByRole("list")).toBeNull();
    const finding = await opened(canvas, userEvent, 4);
    await expect(finding.queryByRole("button", { name: /Rescue|Scrap|Stash/ })).toBeNull();
    await expect(finding.getByRole("img", { name: "Reading" })).toBeInTheDocument();
    await expect(finding.getByRole("list", { name: "Read" })).toHaveTextContent("src/reader/retry.rs");
    await expect(finding.queryByRole("list", { name: "Searched" })).toBeNull();
    await expect(finding.queryByRole("button", { name: /Scrap|Stash|Pick up/ })).toBeNull();
    // Stop is offered where Rescue was, and again in the footer.
    const stops = finding.getAllByRole("button", { name: "Stop" });
    await expect(stops).toHaveLength(2);
    for (const stop of stops) {
      await userEvent.click(stop);
      await expect(args.onRescue).toHaveBeenLastCalledWith("stop", 4);
    }
    await expect(args.onRescue).toHaveBeenCalledTimes(2);
  },
};

/**
 * The Finding, in the sheet: the verdict and what is left, what it read and searched, the slot's
 * commits, the commit it read and that uncommitted changes sat on it, and what
 * was cut. **The tile holds none of it.** Stash acts at once and the panel stays,
 * so what Fleet answers is read there; Scrap waits on its confirm.
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
            verdict: "unfinished",
            items: ["src/lib.rs still calls the old read loop", "src/reader/retry.rs has no test"],
          },
        }),
      },
    ],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.queryByRole("dialog")).toBeNull();
    await expect(bay(canvas, 4).queryByText(/A retry for a short read/)).toBeNull();
    await expect(bay(canvas, 4).queryByRole("button", { name: /Rescue|Stop|Scrap|Stash/ })).toBeNull();
    // An answered Finding has nothing to rescue again.

    const finding = await opened(canvas, userEvent, 4);
    await expect(finding.getByRole("heading", { name: "Unfinished" })).toBeInTheDocument();
    const left = finding.getByRole("list", { name: "Left to do" });
    await expect(within(left).getAllByRole("listitem")).toHaveLength(2);
    await expect(left).toHaveTextContent("src/lib.rs still calls the old read loop");
    await expect(finding.getByRole("list", { name: "Read" })).toHaveTextContent("src/reader/mod.rs");
    await expect(finding.getByRole("list", { name: "Searched" })).toHaveTextContent("retry_short_read in src/");
    await expect(finding.getByRole("list", { name: "Commits" })).toHaveTextContent("9d41e07 Retry a short read once");
    await expect(finding.getByLabelText("Commit it read: 9d41e07")).toBeInTheDocument();
    await expect(finding.getByRole("img", { name: "Uncommitted changes on top" })).toBeInTheDocument();
    await expect(finding.getByLabelText(/cut before it read them: 1200/)).toHaveTextContent("1200 cut");
    await expect(finding.queryByRole("img", { name: "Reading" })).toBeNull();
    await expect(finding.queryByRole("button", { name: /Rescue|Stop/ })).toBeNull();

    await userEvent.click(finding.getByRole("button", { name: "Stash" }));
    await expect(args.onRescue).toHaveBeenCalledTimes(1);
    await expect(args.onRescue).toHaveBeenCalledWith("stash", 4);
    await expect(canvas.getByRole("dialog", { name: "slot-4" })).toBeInTheDocument();
  },
};

/**
 * **Pick up acts at once**, like Stash: the proposal it makes is Fleet's, and
 * shows wherever a new proposal does. A refusal is said in the panel.
 */
export const FindingPickedUp: Story = {
  name: "Finding picked up",
  args: {
    rows: [
      {
        slot: slot(4, {
          ...STRANDED,
          rescue: { ...FOUND, state: "answered", verdict: "unfinished", items: ["src/reader/retry.rs has no test"] },
        }),
        refused: "Not picked up: no origin to push to",
      },
    ],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const finding = await opened(canvas, userEvent, 4);
    await expect(finding.getByRole("alert")).toHaveTextContent("Not picked up: no origin to push to");
    await userEvent.click(finding.getByRole("button", { name: "Pick up" }));
    await expect(args.onRescue).toHaveBeenCalledTimes(1);
    await expect(args.onRescue).toHaveBeenCalledWith("pick_up", 4);
  },
};

/**
 * **Scraps is one line**, under its one word: no list, and nothing under it to
 * do. **A Scout that did not answer in the shape** leaves its own words as they
 * are, with no verdict drawn.
 */
export const FindingScraps: Story = {
  name: "Finding scraps",
  args: {
    rows: [
      {
        slot: slot(4, {
          ...STRANDED,
          rescue: { ...FOUND, state: "answered", verdict: "scraps", items: ["A draft note in notes/lease.md"] },
        }),
      },
      {
        slot: slot(5, {
          ...STRANDED,
          rescue: { ...FOUND, state: "answered", summary: "The parser is half written; the lexer is done." },
        }),
      },
    ],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    const scraps = await opened(canvas, userEvent, 4);
    await expect(scraps.getByRole("heading", { name: "Scraps" })).toBeInTheDocument();
    await expect(scraps.getByText("A draft note in notes/lease.md")).toBeInTheDocument();
    await expect(scraps.queryByRole("list", { name: "Left to do" })).toBeNull();
    await expect(scraps.queryByRole("heading", { name: "Unfinished" })).toBeNull();
    await expect(scraps.queryByRole("button", { name: "Pick up" })).toBeNull();
    await expect(scraps.getByRole("button", { name: "Stash" })).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");

    const prose = await opened(canvas, userEvent, 5);
    await expect(prose.getByText("The parser is half written; the lexer is done.")).toBeInTheDocument();
    await expect(prose.queryByRole("heading", { name: /Unfinished|Scraps/ })).toBeNull();
    await expect(prose.getByRole("button", { name: "Pick up" })).toBeInTheDocument();
  },
};

/**
 * Each commit is tagged with where else it exists, and **the ones that exist
 * only here come first**, the rest in the order they came. The Scrap's confirm
 * names only those.
 */
export const CommitHomes: Story = {
  name: "Commit homes",
  args: {
    rows: [
      {
        slot: slot(4, {
          ...STRANDED,
          base: "develop",
          stranded: {
            uncommitted: [],
            commits: [
              { sha: "1111111aaa", subject: "Merge develop", home: "on_main" },
              { sha: "2222222bbb", subject: "Pushed elsewhere", home: "on_remote" },
              { sha: "3333333ccc", subject: "Only in the slot", home: "only_here" },
            ],
            unpushed: 1,
          },
          rescue: { ...FOUND, state: "answered" },
        }),
      },
    ],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    const finding = await opened(canvas, userEvent, 4);
    const rows = within(finding.getByRole("list", { name: "Commits" })).getAllByRole("listitem");
    await expect(rows).toHaveLength(3);
    await expect(rows[0]).toHaveTextContent("3333333 Only in the slot");
    await expect(within(rows[0]!).getByText("Only here")).toBeInTheDocument();
    await expect(rows[1]).toHaveTextContent("1111111 Merge develop");
    await expect(within(rows[1]!).getByText("On develop")).toBeInTheDocument();
    await expect(rows[2]).toHaveTextContent("2222222 Pushed elsewhere");
    await expect(within(rows[2]!).getByText("On the remote")).toBeInTheDocument();
    await userEvent.click(finding.getByRole("button", { name: "Scrap" }));
    const unpushed = within(canvas.getByRole("group", { name: "Scrap slot-4" })).getByRole("list", { name: "Unpushed" });
    await expect(unpushed).toHaveTextContent("Only in the slot");
    await expect(unpushed).not.toHaveTextContent("Pushed elsewhere");
  },
};

/**
 * **Escape closes the panel and nothing is sent**, and the tile is as it was.
 * The labelled Close is the other way out, and a press on the ground behind is
 * neither.
 */
export const FindingClosed: Story = {
  name: "Finding closed",
  args: {
    rows: [{ slot: slot(4, { ...STRANDED, rescue: { ...FOUND, state: "answered", verdict: "unfinished", items: ["src/lib.rs still calls the old loop"] } }) }],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await opened(canvas, userEvent, 4);
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("dialog")).toBeNull();
    await expect(bay(canvas, 4).getByRole("button", { name: "slot-4" })).toHaveAttribute("aria-expanded", "false");

    const again = await opened(canvas, userEvent, 4);
    await userEvent.click(again.getByRole("button", { name: "Close Esc" }));
    await expect(canvas.queryByRole("dialog")).toBeNull();
    await expect(args.onRescue).not.toHaveBeenCalled();
  },
};

/**
 * A Scout that ended without an answer. A failed one shows why it failed, a
 * stopped one is marked stopped, both keep what they read, and **both still
 * offer Scrap and Stash** in the sheet. Nothing is cut and nothing was left uncommitted, so
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
  play: async ({ canvas, userEvent }) => {
    const reading = async (n: number) => {
      const finding = await opened(canvas, userEvent, n);
      await expect(finding.getByRole("list", { name: "Read" })).toHaveTextContent("src/reader/retry.rs");
      await expect(finding.getByRole("button", { name: "Scrap" })).toBeInTheDocument();
      await expect(finding.getByRole("button", { name: "Stash" })).toBeInTheDocument();
      await expect(finding.queryByRole("button", { name: "Pick up" })).toBeNull();
      await expect(finding.queryByRole("img", { name: "Uncommitted changes on top" })).toBeNull();
      await expect(finding.queryByLabelText(/cut before/)).toBeNull();
      return finding;
    };
    const failed = await reading(4);
    await expect(failed.getByText("The Scout ended before it answered")).toBeInTheDocument();
    await expect(failed.queryByRole("img", { name: "Stopped" })).toBeNull();
    await userEvent.keyboard("{Escape}");
    const stopped = await reading(5);
    await expect(stopped.getByRole("img", { name: "Stopped" })).toBeInTheDocument();
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
    rows: [{ slot: slot(4, { ...STRANDED, rescue: { ...FOUND, state: "answered", verdict: "unfinished", items: ["src/lib.rs still calls the old loop"] } }) }],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const finding = await opened(canvas, userEvent, 4);
    await userEvent.click(finding.getByRole("button", { name: "Scrap" }));
    await expect(args.onRescue).not.toHaveBeenCalled();

    const confirm = within(canvas.getByRole("group", { name: "Scrap slot-4" }));
    await expect(confirm.getByRole("list", { name: "Uncommitted" })).toHaveTextContent("src/lib.rs");
    await expect(confirm.getByRole("list", { name: "Uncommitted" })).toHaveTextContent("src/reader/retry.rs");
    await expect(confirm.getByRole("list", { name: "Unpushed" })).toHaveTextContent("Retry a short read once");
    await expect(confirm.getByRole("list", { name: "Unpushed" })).not.toHaveTextContent("Split the reader");
    // The sheet's own Scrap gives way to the confirm's, so there is one to press.
    await expect(canvas.getAllByRole("button", { name: "Scrap" })).toHaveLength(1);

    await userEvent.click(confirm.getByRole("button", { name: "Cancel" }));
    await expect(canvas.queryByRole("group", { name: "Scrap slot-4" })).toBeNull();
    await expect(args.onRescue).not.toHaveBeenCalled();

    await userEvent.click(sheet(canvas).getByRole("button", { name: "Scrap" }));
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
    const finding = await opened(canvas, userEvent, 4);
    await userEvent.click(finding.getByRole("button", { name: "Scrap" }));
    const confirm = within(canvas.getByRole("group", { name: "Scrap slot-4" }));
    await expect(confirm.queryByRole("list")).toBeNull();
    await expect(confirm.getByRole("button", { name: "Scrap" })).toBeInTheDocument();
  },
};

/**
 * Fleet's refusal of a rescue act, **in the panel of the tile it was about**, by
 * the same path as the pool's acts; and what a Scrap or Stash did, said as a
 * bare fact on the slot it freed.
 */
export const RescueRefusedAndReceipt: Story = {
  name: "Rescue refused, and receipt",
  args: {
    rows: [
      {
        slot: slot(4, { ...STRANDED, rescue: { ...FOUND, state: "answered", verdict: "unfinished", items: ["src/lib.rs still calls the old loop"] } }),
        refused: "Not stashed: a Scout is reading it",
      },
      { slot: slot(5, { held: { state: "free" } }), said: "fleet/old-try kept" },
      { slot: slot(6, { held: { state: "free" } }), said: "c0ffee1 on fleet/old-try" },
    ],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    const refused = await opened(canvas, userEvent, 4);
    await expect(refused.getByRole("alert")).toHaveTextContent("Not stashed: a Scout is reading it");
    await expect(refused.queryByRole("status")).toBeNull();
    await userEvent.keyboard("{Escape}");
    const scrapped = await opened(canvas, userEvent, 5);
    await expect(scrapped.getByRole("status")).toHaveTextContent("fleet/old-try kept");
    await userEvent.keyboard("{Escape}");
    const stashed = await opened(canvas, userEvent, 6);
    await expect(stashed.getByRole("status")).toHaveTextContent("c0ffee1 on fleet/old-try");
    // A tile says none of it.
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("alert")).toBeNull();
    await expect(canvas.queryByRole("status")).toBeNull();
  },
};

/** A pool drawn with no rescue wiring offers none of its acts, even on a stranded bay. */
export const RescueNotWired: Story = {
  name: "Rescue not wired",
  args: { rows: [{ slot: slot(4, STRANDED) }], onAct: fn() },
  play: async ({ canvas, userEvent }) => {
    const open = await opened(canvas, userEvent, 4);
    await expect(open.queryByRole("button", { name: /Rescue|Stop|Scrap|Stash/ })).toBeNull();
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
    const live = await opened(canvas, userEvent, 1);
    await expect(live.queryByRole("button", { name: "Rescue" })).toBeNull();
    await userEvent.keyboard("{Escape}");
    const open = await opened(canvas, userEvent, 8);
    await userEvent.click(open.getByRole("button", { name: "Rescue" }));
    await expect(args.onRescue).toHaveBeenCalledWith("start", 8);
  },
};

/**
 * The kept bay's rescue is a stranded bay's: its Finding is in its panel,
 * Stash acts at once, and Scrap confirms by naming the files and the unpushed
 * commit.
 */
export const KeptJobFinding: Story = {
  name: "Kept Job finding",
  args: {
    rows: [{ slot: slot(8, { ...KEPT, rescue: { ...FOUND, state: "answered", verdict: "unfinished", items: ["The route test asserts a retry count the read does not return"] } }) }],
    onAct: fn(),
    onRescue: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const finding = await opened(canvas, userEvent, 8);
    await expect(finding.getByText("The route test asserts a retry count the read does not return")).toBeInTheDocument();
    await userEvent.click(finding.getByRole("button", { name: "Stash" }));
    await expect(args.onRescue).toHaveBeenLastCalledWith("stash", 8);
    await userEvent.click(sheet(canvas, 8).getByRole("button", { name: "Scrap" }));
    const confirm = within(canvas.getByRole("group", { name: "Scrap slot-8" }));
    await expect(confirm.getByRole("list", { name: "Uncommitted" })).toHaveTextContent("docs/notes/retry.md");
    await expect(confirm.getByRole("list", { name: "Unpushed" })).toHaveTextContent("Retry the manifest read on a short answer");
    await expect(confirm.getByRole("list", { name: "Unpushed" })).not.toHaveTextContent("Name the manifest");
    await userEvent.click(confirm.getByRole("button", { name: "Scrap" }));
    await expect(args.onRescue).toHaveBeenLastCalledWith("scrap", 8);
  },
};
