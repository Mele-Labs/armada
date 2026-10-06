// Cleanup as one grid of tiles, and what a press on a tile does.
//
// # Why these are browser tests and not stories
//
// `packages/components` sits below this package, so no story there can mount
// this surface. A story proves what one tile and its panel draw; this proves
// what the screen does with a read: that each worktree is a tile, that a bay
// and its Job's worktree share one panel, that every destructive act confirms
// before it is sent, that a Job still running is offered nothing, and that a
// refusal or a receipt is said in the panel it was about.
//
// What a tile offers and what its receipts say is arithmetic, next door in
// `held.test.ts`, where a hundred cases cost what one costs.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type {
  Outcome,
  WorktreeHeld,
  WorktreeReclaimed,
  WorktreeSlot,
} from "@armada/protocol";

import { mount, unmount } from "./mounted";
import { Worktrees } from "./Worktrees";
import type { WorktreesProps } from "./Worktrees";

afterEach(unmount);

/** Stable, because the surface depends on it in an effect. */
const WANT = (): void => {};

/** A fixed instant, so an age on screen is arithmetic and not a race. */
const NOW = Date.parse("2026-09-03T12:00:00Z");

function held(over: Partial<WorktreeHeld> = {}): WorktreeHeld {
  return {
    job_id: "job-a",
    job_title: "Port the settings selectors",
    status: "completed_success",
    last_moved_at: "2026-08-30T09:14:00Z",
    path: "/Users/user/armada/.armada/worktrees/job-a",
    branch: "armada/job-a",
    held: [],
    on_disk: true,
    ...over,
  };
}

const UNMERGED = { why: "unmerged", base: "main", commits: 3, tip: "9f1c2ab84d5e" } as const;

type Answers = {
  reclaim?: (jobId: string) => Outcome;
  deleteBranch?: (jobId: string, tip: string) => Outcome;
  forget?: (jobId: string) => Outcome;
};

/** What each act was sent for. */
type Sent = { reclaimed: string[]; branchesDeleted: [string, string][]; forgotten: string[] };

/** Mount the surface over one read, recording every id each of the three acts was sent for. */
function opened(worktrees: WorktreeHeld[], answers: Answers = {}, slots: WorktreeSlot[] = [], rest: Partial<WorktreesProps> = {}): Sent {
  const sent: Sent = { reclaimed: [], branchesDeleted: [], forgotten: [] };
  mount(
    <Worktrees
      onWant={WANT}
      held={{ state: "read", held: { worktrees, slots } }}
      onReclaim={(jobId) => {
        sent.reclaimed.push(jobId);
        return Promise.resolve(answers.reclaim?.(jobId) ?? { ok: true });
      }}
      onDeleteBranch={(jobId, tip) => {
        sent.branchesDeleted.push([jobId, tip]);
        return Promise.resolve(answers.deleteBranch?.(jobId, tip) ?? { ok: true });
      }}
      onForget={(jobId) => {
        sent.forgotten.push(jobId);
        return Promise.resolve(answers.forget?.(jobId) ?? { ok: true });
      }}
      now={NOW}
      onClose={() => {}}
      onCopied={() => {}}
      onOpenJob={() => {}}
      {...rest}
    />,
  );
  return sent;
}

/** Press a tile by its name and answer its panel. */
async function open(name: string) {
  await userEvent.click(page.getByRole("button", { name, exact: true }));
  return page.getByRole("dialog", { name });
}

/** The confirm up in a panel, named for its act and its tile. */
const confirmOf = (act: string, name: string) => page.getByRole("group", { name: `${act} ${name}` });

/** A reclaim's own receipt, as Fleet answers. */
const RECLAIMED: WorktreeReclaimed = {
  job_id: "job-a",
  worktree: { path: "/p", removed: true },
  branch: { branch: "armada/job-a", deleted: false, unmerged_commits: 3 },
};

/** A tile is the only way in: the old list's cards, boxes and bulk press are gone. */
test("the page is one grid, with no list below it and no bulk press", async () => {
  opened([
    held({ job_id: "job-a", held: [UNMERGED] }),
    held({ job_id: "job-b", held: [{ why: "not_terminal", status: "running" }] }),
    held({ job_id: "job-c", held: [] }),
  ]);

  await expect.element(page.getByRole("button", { name: "job-a", exact: true })).toBeInTheDocument();
  await expect.element(page.getByRole("button", { name: "job-b", exact: true })).toBeInTheDocument();
  await expect.element(page.getByRole("button", { name: "job-c", exact: true })).toBeInTheDocument();
  expect(page.getByRole("checkbox").elements()).toHaveLength(0);
  expect(page.getByRole("button", { name: /^Clean up/ }).elements()).toHaveLength(0);
  expect(page.getByText(/Waiting on you|Still running|on its own/).elements()).toHaveLength(0);
});

/** **A Job that has not ended is drawn and offered nothing**: Fleet would answer every act with a 409. */
test("a job still running is a tile whose panel offers no act", async () => {
  opened([held({ job_id: "job-b", status: "running", held: [{ why: "not_terminal", status: "running" }, UNMERGED] })]);

  const panel = await open("job-b");
  expect(panel.getByRole("button", { name: /^(Clear|Delete branch|Forget Job)$/ }).elements()).toHaveLength(0);
  await expect.element(panel.getByRole("region", { name: "What it holds" })).toBeInTheDocument();
});

/**
 * **The confirm says what will happen, and nothing is lost.** The uncommitted
 * files are committed to the branch as a WIP commit before the worktree goes,
 * and the branch is kept. No figure is drawn beside the list.
 */
test("Clear confirms first, listing the files it commits to the branch, the worktree it removes and the branch it keeps", async () => {
  const sent = opened([held({ held: [{ why: "uncommitted", files: ["src/log.rs", "notes.md"] }, UNMERGED] })]);

  const panel = await open("job-a");
  await userEvent.click(panel.getByRole("button", { name: "Clear" }));
  expect(sent.reclaimed, "nothing is sent before the confirm").toEqual([]);

  const confirm = confirmOf("Clear", "job-a");
  await expect.element(confirm.getByText("Removes the worktree at /Users/user/armada/.armada/worktrees/job-a")).toBeInTheDocument();
  await expect.element(confirm.getByText("Commits the uncommitted files to branch armada/job-a as a WIP commit")).toBeInTheDocument();
  await expect.element(confirm.getByRole("list", { name: "Uncommitted files" })).toHaveTextContent("src/log.rs");
  await expect.element(confirm.getByRole("list", { name: "Uncommitted files" })).toHaveTextContent("notes.md");
  await expect.element(confirm.getByText("Keeps branch armada/job-a")).toBeInTheDocument();
  expect(confirm.getByText("Deletes uncommitted files").elements()).toHaveLength(0);
  expect(confirm.getByText(/commits not on/).elements()).toHaveLength(0);

  await userEvent.click(confirm.getByRole("button", { name: "Cancel" }));
  expect(sent.reclaimed).toEqual([]);

  await userEvent.click(panel.getByRole("button", { name: "Clear" }));
  await userEvent.click(confirmOf("Clear", "job-a").getByRole("button", { name: "Clear" }));
  expect(sent.reclaimed).toEqual(["job-a"]);
});

/** A checkout holding nothing uncommitted still confirms, and names nothing destroyed. */
test("a Clear that destroys nothing names no file", async () => {
  opened([held({ held: [UNMERGED] })]);

  const panel = await open("job-a");
  await userEvent.click(panel.getByRole("button", { name: "Clear" }));

  const confirm = confirmOf("Clear", "job-a");
  expect(confirm.getByRole("list", { name: "Uncommitted files" }).elements()).toHaveLength(0);
  await expect.element(confirm.getByText("Keeps branch armada/job-a: 3 commits not on main")).toBeInTheDocument();
});

/** **The receipt is said in the panel**, half by half, and an unmerged branch stays. */
test("Clear says what it did in the panel: the checkout gone and the unmerged branch kept", async () => {
  opened([held({ held: [UNMERGED] })], { reclaim: () => ({ ok: true, reclaimed: RECLAIMED }) });

  const panel = await open("job-a");
  await userEvent.click(panel.getByRole("button", { name: "Clear" }));
  await userEvent.click(confirmOf("Clear", "job-a").getByRole("button", { name: "Clear" }));

  await expect.element(panel.getByRole("status")).toHaveTextContent("Worktree removed");
  await expect.element(panel.getByRole("status")).toHaveTextContent("Branch kept: 3 commits not on main");
});

/** A refusal is the panel's, led by what failed to happen, and the tile says none of it. */
test("a refused Clear is said in the panel", async () => {
  opened([held({ held: [UNMERGED] })], { reclaim: () => ({ ok: false, why: "not_connected" }) });

  const panel = await open("job-a");
  await userEvent.click(panel.getByRole("button", { name: "Clear" }));
  await userEvent.click(confirmOf("Clear", "job-a").getByRole("button", { name: "Clear" }));

  await expect.element(panel.getByRole("alert")).toHaveTextContent("Worktree not removed");
  await userEvent.keyboard("{Escape}");
  expect(page.getByRole("alert").elements()).toHaveLength(0);
});

/** **Deleting a branch is its own act**, naming its commit count and its tip, and it sends that tip. */
test("Delete branch confirms with the tip, and sends that tip", async () => {
  const sent = opened([held({ job_title: "Rework the retry ceiling", on_disk: false, held: [UNMERGED] })]);

  const panel = await open("job-a");
  expect(panel.getByRole("button", { name: "Clear" }).elements(), "the checkout is already gone").toHaveLength(0);
  await userEvent.click(panel.getByRole("button", { name: "Delete branch" }));
  expect(sent.branchesDeleted).toEqual([]);

  const confirm = confirmOf("Delete branch", "job-a");
  await expect.element(confirm.getByText("Deletes branch armada/job-a at 9f1c2ab84d5e")).toBeInTheDocument();
  await expect.element(confirm.getByText("3 commits not on main stay reachable only from 9f1c2ab84d5e")).toBeInTheDocument();
  await userEvent.click(confirm.getByRole("button", { name: "Delete branch" }));
  expect(sent.branchesDeleted).toEqual([["job-a", "9f1c2ab84d5e"]]);
});

test("a branch Fleet will not delete says why in the panel", async () => {
  opened([held({ on_disk: false, held: [UNMERGED] })], {
    deleteBranch: () => ({
      ok: false,
      why: "refused",
      error: { code: "fleet.tip_moved", message: "the tip has moved", run_id: "r", fields: {}, chain: [] },
    }),
  });

  const panel = await open("job-a");
  await userEvent.click(panel.getByRole("button", { name: "Delete branch" }));
  await userEvent.click(confirmOf("Delete branch", "job-a").getByRole("button", { name: "Delete branch" }));

  await expect.element(panel.getByRole("alert")).toHaveTextContent("Branch not deleted: the tip has moved");
});

/**
 * **Forget waits on the checkout and the branch.** A record forgotten while
 * either stands orphans that disk from this page, so it is offered only once
 * both are gone, and it confirms because there is no undo.
 */
test("Forget Job is withheld while a checkout stands, and confirms once it is offered", async () => {
  const sent = opened([
    held({ job_id: "job-a", on_disk: true, held: [] }),
    held({ job_id: "job-b", job_title: "Old", on_disk: false, held: [] }),
  ]);

  const standing = await open("job-a");
  expect(standing.getByRole("button", { name: "Forget Job" }).elements()).toHaveLength(0);
  await userEvent.keyboard("{Escape}");

  const gone = await open("job-b");
  await userEvent.click(gone.getByRole("button", { name: "Forget Job" }));
  expect(sent.forgotten).toEqual([]);
  await expect.element(confirmOf("Forget Job", "job-b").getByText("Deletes the record of Job Old")).toBeInTheDocument();
  await userEvent.click(confirmOf("Forget Job", "job-b").getByRole("button", { name: "Forget Job" }));
  expect(sent.forgotten).toEqual(["job-b"]);
});

const slot = (n: number, over: Partial<WorktreeSlot> & Pick<WorktreeSlot, "held">): WorktreeSlot => ({
  manifest_id: "armada",
  slot: n,
  path: `/r/.armada/slots/slot-${n}`,
  base: "main",
  warm: false,
  ...over,
});

/**
 * A held Job's bay and its worktree are one tile: the panel carries the slot's
 * acts and the reclaim's, and the worktree is not drawn a second time after the bays.
 */
test("a bay joins its Job's worktree in one panel, and the worktree is not drawn twice", async () => {
  const sent = opened(
    [held({ job_id: "01JOB", held: [{ why: "uncommitted", files: ["src/log.rs"] }] }), held({ job_id: "outside", held: [UNMERGED] })],
    {},
    [slot(1, { held: { state: "job", job_id: "01JOB", job_title: "Fix the reader" }, branch: "armada/1-fix-the-reader" })],
    { onChangeSlotPool: () => Promise.resolve({ ok: true }) },
  );

  await expect.element(page.getByRole("button", { name: "outside", exact: true })).toBeInTheDocument();
  // The bay and the outside worktree: Job 01JOB is the bay's, so it has no second tile.
  expect(page.getByRole("listitem").elements().filter((one) => one.getAttribute("aria-label") !== null)).toHaveLength(2);
  const panel = await open("slot-1");
  await expect.element(panel.getByRole("button", { name: "Clear" })).toBeInTheDocument();
  await expect.element(panel.getByRole("button", { name: "Close slot" })).toBeInTheDocument();
  await expect.element(panel.getByRole("region", { name: "What it holds" })).toHaveTextContent("src/log.rs");

  await userEvent.click(panel.getByRole("button", { name: "Clear" }));
  await userEvent.click(confirmOf("Clear", "slot-1").getByRole("button", { name: "Clear" }));
  expect(sent.reclaimed).toEqual(["01JOB"]);
});

/**
 * **A bay is released to the pool, and its worktree stays.** The confirm and
 * the receipt say that, in git's words, and never that a worktree was removed.
 */
test("Clear on a bay releases the slot and says the worktree stays", async () => {
  const sent = opened(
    [held({ job_id: "01JOB", branch: "armada/1-fix", held: [UNMERGED] })],
    { reclaim: () => ({ ok: true, reclaimed: RECLAIMED }) },
    [slot(1, { held: { state: "job", job_id: "01JOB", job_title: "Fix the reader" }, branch: "armada/1-fix" })],
  );

  const panel = await open("slot-1");
  await userEvent.click(panel.getByRole("button", { name: "Clear" }));
  const confirm = confirmOf("Clear", "slot-1");
  await expect.element(confirm.getByText("Releases slot-1 to the pool")).toBeInTheDocument();
  await expect.element(confirm.getByText("Keeps the worktree at .armada/slots/slot-1, detached from armada/1-fix")).toBeInTheDocument();
  expect(confirm.getByText(/^Removes the worktree/).elements()).toHaveLength(0);

  await userEvent.click(confirm.getByRole("button", { name: "Clear" }));
  expect(sent.reclaimed).toEqual(["01JOB"]);
  await expect.element(panel.getByRole("status")).toHaveTextContent("Slot released, worktree kept");
  await expect.element(panel.getByRole("status")).toHaveTextContent("Branch kept: 3 commits not on main");
});

/** The confirm names the commit, the release and the kept branch, and the receipt says where the files went. */
test("Clear on a bay holding uncommitted files commits them, releases the slot and says so", async () => {
  const saved = { commit: "d41f8a6c20be", files: ["src/log.rs"] };
  const sent = opened(
    [held({ job_id: "01JOB", branch: "armada/1-fix", held: [{ why: "uncommitted", files: ["src/log.rs"] }] })],
    { reclaim: () => ({ ok: true, reclaimed: { ...RECLAIMED, saved } }) },
    [slot(1, { held: { state: "job", job_id: "01JOB", job_title: "Fix the reader" }, branch: "armada/1-fix" })],
  );

  const panel = await open("slot-1");
  await userEvent.click(panel.getByRole("button", { name: "Clear" }));
  const confirm = confirmOf("Clear", "slot-1");
  await expect.element(confirm.getByText("Commits the uncommitted files to branch armada/1-fix as a WIP commit")).toBeInTheDocument();
  await expect.element(confirm.getByText("Releases slot-1")).toBeInTheDocument();
  await expect.element(confirm.getByText("Keeps branch armada/1-fix")).toBeInTheDocument();
  expect(confirm.getByText(/Refused/).elements()).toHaveLength(0);

  await userEvent.click(confirm.getByRole("button", { name: "Clear" }));
  expect(sent.reclaimed).toEqual(["01JOB"]);
  await expect.element(panel.getByRole("status")).toHaveTextContent("Committed to armada/job-a, slot released");
});

test("a refused release is said as a slot not released", async () => {
  opened(
    [held({ job_id: "01JOB", held: [{ why: "uncommitted", files: ["src/log.rs"] }] })],
    { reclaim: () => ({ ok: false, why: "not_connected" }) },
    [slot(1, { held: { state: "job", job_id: "01JOB", job_title: "Fix the reader" } })],
  );

  const panel = await open("slot-1");
  await userEvent.click(panel.getByRole("button", { name: "Clear" }));
  await userEvent.click(confirmOf("Clear", "slot-1").getByRole("button", { name: "Clear" }));
  await expect.element(panel.getByRole("alert")).toHaveTextContent("Slot not released");
});

/** Fleet answers a branch delete with a 409 while the worktree is on disk, so the act is not offered then. */
test("Delete branch is not offered while the worktree is on disk", async () => {
  opened([held({ on_disk: true, held: [UNMERGED] })]);

  const panel = await open("job-a");
  await expect.element(panel.getByRole("button", { name: "Clear" })).toBeInTheDocument();
  expect(panel.getByRole("button", { name: "Delete branch" }).elements()).toHaveLength(0);
});

/** Each act's tooltip says what it acts on, in git's words, with the screen's own paths and branches. */
test("Clear's tooltip names the worktree it removes and what becomes of the branch", async () => {
  opened([held({ held: [UNMERGED] })]);

  const panel = await open("job-a");
  await expect
    .element(panel.getByRole("button", { name: "Clear" }))
    .toHaveAccessibleDescription(
      "Removes the worktree at /Users/user/armada/.armada/worktrees/job-a. Deletes branch armada/job-a only if main has all its commits, otherwise keeps it.",
    );
});

/** The Job's status row is the Job's: its handle, and the Board's badge. */
test("a running Job's row says whose status it is", async () => {
  opened([held({ job_id: "01JOB", status: "running", held: [{ why: "not_terminal", status: "running" }] })], {}, [], {
    jobs: [{ id: "01JOB", handle: "4-fix-the-reader" } as never],
  });

  const panel = await open("4-fix-the-reader");
  await expect.element(panel.getByText("Job 4-fix-the-reader")).toBeInTheDocument();
  await expect.element(panel.getByLabelText("Job status: running")).toHaveTextContent("running");
});

/** A worktree outside the pool is named by its Job's handle where the board has it. */
test("a worktree outside the pool is named by its Job's handle", async () => {
  opened([held({ job_id: "01JOB", held: [UNMERGED] })], {}, [], {
    jobs: [{ id: "01JOB", handle: "9-fix-the-retry-ceiling" } as never],
  });

  await expect.element(page.getByRole("button", { name: "9-fix-the-retry-ceiling", exact: true })).toBeInTheDocument();
});

test("Escape closes the panel and nothing is sent", async () => {
  const sent = opened([held({ held: [UNMERGED] })]);

  await open("job-a");
  await userEvent.keyboard("{Escape}");

  expect(page.getByRole("dialog").elements()).toHaveLength(0);
  expect(sent).toEqual({ reclaimed: [], branchesDeleted: [], forgotten: [] });
});

/** A read that failed says so rather than drawing an empty page. */
test("a read that failed says so rather than drawing an empty page", async () => {
  mount(
    <Worktrees
      onWant={WANT}
      held={{ state: "failed", outcome: { ok: false, why: "not_connected" } }}
      onReclaim={() => Promise.resolve({ ok: true })}
      onDeleteBranch={() => Promise.resolve({ ok: true })}
      onForget={() => Promise.resolve({ ok: true })}
      now={NOW}
      onClose={() => {}}
      onCopied={() => {}}
      onOpenJob={() => {}}
    />,
  );

  await expect.element(page.getByText("What fleet is holding could not be read")).toBeInTheDocument();
  expect(page.getByRole("listitem").elements()).toHaveLength(0);
});

/** An empty read draws no tile and no sentence in their place. */
test("nothing held draws nothing", async () => {
  opened([]);

  expect(page.getByRole("listitem").elements()).toHaveLength(0);
  expect(page.getByText(/nothing|empty/i).elements()).toHaveLength(0);
});

/**
 * The worktree pool, a bay per slot, each figure named by its label. **A Job
 * holding one opens that Job**, and the tile's other presses open its panel.
 */
test("the pool draws a bay per slot, and a slot's Job opens from it", async () => {
  const opens: string[] = [];
  opened(
    [],
    {},
    [
      slot(1, {
        held: { state: "job", job_id: "01JOB", job_title: "Fix the reader" },
        branch: "armada/1-fix-the-reader",
        since: "2026-09-03T10:00:00.000Z",
        warm: true,
        behind: 4,
      }),
      slot(2, { held: { state: "unmade" } }),
    ],
    { onOpenJob: (jobId) => opens.push(jobId) },
  );

  await expect.element(page.getByRole("list", { name: "Worktree slots" })).toBeInTheDocument();
  await expect.element(page.getByRole("listitem", { name: "slot-1" })).toBeInTheDocument();
  await expect.element(page.getByRole("img", { name: "Held" })).toBeInTheDocument();
  await expect.element(page.getByRole("img", { name: "Warm" })).toBeInTheDocument();
  await expect.element(page.getByRole("img", { name: "Not made yet" })).toBeInTheDocument();
  await expect.element(page.getByLabelText("4 commits behind main")).toBeInTheDocument();
  await expect.element(page.getByLabelText("Held for 2 hours")).toBeInTheDocument();
  expect(page.getByText(/slots? free/).elements()).toHaveLength(0);

  await userEvent.click(page.getByRole("button", { name: "Fix the reader" }));
  expect(opens).toEqual(["01JOB"]);
  expect(page.getByRole("dialog").elements()).toHaveLength(0);
});

const stranded = (rescue?: WorktreeSlot["rescue"]): WorktreeSlot => ({
  manifest_id: "armada",
  slot: 4,
  path: "/r/.armada/slots/slot-4",
  base: "main",
  warm: false,
  held: { state: "stranded", why: "2 uncommitted, first src/lib.rs" },
  branch: "fleet/an-old-try",
  stranded: {
    uncommitted: ["src/lib.rs", "src/reader/retry.rs"],
    commits: [{ sha: "9d41e07b2c", subject: "Retry a short read once", home: "only_here" }],
    unpushed: 1,
  },
  ...(rescue === undefined ? {} : { rescue }),
});

const FINDING = { commit: "9d41e07b2c", uncommitted: true, read: ["src/lib.rs"], searched: [] };

/** Mount the surface over one slot, with the rescue's answer and a count of every ask to read. */
function rescuing(slot: WorktreeSlot, onRescueSlot: NonNullable<WorktreesProps["onRescueSlot"]>) {
  const wants: boolean[] = [];
  opened([], {}, [slot], { onWant: (want) => wants.push(want), onRescueSlot });
  return wants;
}

const RECEIPT = { manifest_id: "armada", slot: 4 };

/**
 * Fleet sends nothing when a Scout reads one more file, so the surface asks for
 * the pool again while one reads. **Only then**: a pool nobody is reading into
 * is not polled.
 */
test("the pool is read again while a Scout reads, and not after it answers", async () => {
  const reading = rescuing(stranded({ ...FINDING, state: "reading" }), () => Promise.resolve({ ok: true, rescued: RECEIPT }));
  await new Promise((done) => setTimeout(done, 2_300));
  expect(reading.filter(Boolean).length).toBeGreaterThanOrEqual(3);
  unmount();

  const answered = rescuing(stranded({ ...FINDING, state: "answered", verdict: "unfinished", items: ["src/lib.rs still calls the old loop"] }), () =>
    Promise.resolve({ ok: true, rescued: RECEIPT }),
  );
  await new Promise((done) => setTimeout(done, 1_500));
  expect(answered).toEqual([true]);
});

/** A rescue act Fleet refuses is said in the panel of the tile it was about, as the pool's acts are. */
test("a refused rescue is said in its panel", async () => {
  rescuing(stranded(), () =>
    Promise.resolve({
      ok: false,
      why: "refused",
      error: { code: "fleet.slot_busy", message: "a lease is under way", run_id: "r", fields: {}, chain: [] },
    }),
  );
  const panel = await open("slot-4");
  await userEvent.click(panel.getByRole("button", { name: "Rescue" }));
  await expect.element(panel.getByRole("alert")).toHaveTextContent("Scout not started: a lease is under way");
});

/**
 * **The Finding is in the tile's panel, not a second sheet**, and Escape closes
 * it without sending anything.
 */
test("a Finding is in the panel and Escape closes it", async () => {
  const sent: string[] = [];
  rescuing(stranded({ ...FINDING, state: "answered", verdict: "unfinished", items: ["src/lib.rs still calls the old loop"] }), (_manifest, rescue) => {
    sent.push(rescue.act);
    return Promise.resolve({ ok: true, rescued: RECEIPT });
  });
  expect(page.getByText("src/lib.rs still calls the old loop").elements()).toHaveLength(0);
  const panel = await open("slot-4");
  await expect.element(panel.getByText("src/lib.rs still calls the old loop")).toBeInTheDocument();
  expect(page.getByRole("dialog").elements()).toHaveLength(1);
  await userEvent.keyboard("{Escape}");
  expect(page.getByRole("dialog").elements()).toHaveLength(0);
  expect(sent).toEqual([]);
});

/** A Scrap that kept the branch says so in the panel, once its confirm is answered. */
test("a scrap that kept its branch says so, after its confirm", async () => {
  const sent: string[] = [];
  rescuing(stranded({ ...FINDING, state: "answered", verdict: "unfinished", items: ["src/lib.rs still calls the old loop"] }), (_manifest, rescue) => {
    sent.push(rescue.act);
    return Promise.resolve({ ok: true, rescued: { ...RECEIPT, branch: "fleet/an-old-try", branch_kept: true } });
  });
  const panel = await open("slot-4");
  await userEvent.click(panel.getByRole("button", { name: "Scrap" }));
  expect(sent).toEqual([]);
  await userEvent.click(page.getByRole("group", { name: "Scrap slot-4" }).getByRole("button", { name: "Scrap" }));
  expect(sent).toEqual(["scrap"]);
  await expect.element(panel.getByRole("status")).toHaveTextContent("Branch fleet/an-old-try kept");
});

/** Pick up is sent from the Finding and says its commit, as a Stash does; Scraps does not offer it. */
test("a pick up is sent from the Finding, and Scraps offers none", async () => {
  const sent: string[] = [];
  rescuing(stranded({ ...FINDING, state: "answered", verdict: "unfinished", items: ["src/lib.rs still calls the old loop"] }), (_manifest, rescue) => {
    sent.push(rescue.act);
    return Promise.resolve({ ok: true, rescued: { ...RECEIPT, branch: "fleet/an-old-try", committed: "c0ffee1a4d9" } });
  });
  const panel = await open("slot-4");
  await userEvent.click(panel.getByRole("button", { name: "Pick up" }));
  expect(sent).toEqual(["pick_up"]);
  await expect.element(panel.getByRole("status")).toHaveTextContent("Committed c0ffee1 on fleet/an-old-try");
  unmount();

  rescuing(stranded({ ...FINDING, state: "answered", verdict: "scraps", items: ["A draft note"] }), () => Promise.resolve({ ok: true, rescued: RECEIPT }));
  const scraps = await open("slot-4");
  await expect.element(scraps.getByRole("button", { name: "Stash" })).toBeInTheDocument();
  expect(page.getByRole("button", { name: "Pick up" }).elements()).toHaveLength(0);
});

/** A Pick up Fleet refuses is said in the panel, led by what it failed to do. */
test("a refused pick up is said in the panel", async () => {
  rescuing(stranded({ ...FINDING, state: "answered", verdict: "unfinished", items: ["src/lib.rs still calls the old loop"] }), () =>
    Promise.resolve({
      ok: false,
      why: "refused",
      error: { code: "fleet.rescue_no_remote", message: "no origin to push to", run_id: "r", fields: {}, chain: [] },
    }),
  );
  const panel = await open("slot-4");
  await userEvent.click(panel.getByRole("button", { name: "Pick up" }));
  await expect.element(panel.getByRole("alert")).toHaveTextContent("Work not picked up: no origin to push to");
});

/** A Job's kept slot is rescued from the screen as a stranded one is: the start reaches Fleet with its number. */
test("a kept Job's slot offers Rescue, and the press is sent", async () => {
  const sent: string[] = [];
  rescuing(
    {
      ...stranded(),
      slot: 8,
      held: { state: "job", job_id: "01KEPT", job_title: "Retry the read", kept: "2 uncommitted, first a.rs" },
    },
    (_manifest, rescue) => {
      sent.push(`${rescue.act} ${rescue.slot}`);
      return Promise.resolve({ ok: true, rescued: { manifest_id: "armada", slot: 8 } });
    },
  );
  await expect.element(page.getByRole("img", { name: "Kept: 2 uncommitted, first a.rs" })).toBeInTheDocument();
  const panel = await open("slot-8");
  await userEvent.click(panel.getByRole("button", { name: "Rescue" }));
  expect(sent).toEqual(["start 8"]);
});

/**
 * A slot an agent session holds is released from its own confirm, which names
 * the holder and branch and lists the files it commits. The act is sent for the
 * holder shown, and the receipt says where the files went.
 */
test("Release on a session's slot commits its files, sends the holder shown and says so", async () => {
  const sentChanges: unknown[] = [];
  opened(
    [],
    {},
    [
      slot(2, {
        held: { state: "session", holder: "claude (pid 44698)" },
        branch: "fleet/by-hand",
        stranded: { uncommitted: ["src/half.rs"], commits: [], unpushed: 0 },
      }),
    ],
    {
      onChangeSlotPool: (_manifest, change) => {
        sentChanges.push(change);
        return Promise.resolve({
          ok: true,
          slotChanged: {
            manifest_id: "m",
            slot: 2,
            released: { branch: "fleet/by-hand", saved: { commit: "d41f8a6c20be", files: ["src/half.rs"] } },
          },
        });
      },
    },
  );

  const panel = await open("slot-2");
  await userEvent.click(panel.getByRole("button", { name: "Release" }));
  const confirm = confirmOf("Release", "slot-2");
  await expect.element(confirm.getByText("Held by claude, on branch fleet/by-hand")).toBeInTheDocument();
  await expect.element(confirm.getByText("pid 44698")).toBeInTheDocument();
  await expect.element(confirm.getByText("Commits the uncommitted files to branch fleet/by-hand as a WIP commit")).toBeInTheDocument();
  await expect.element(confirm.getByRole("list", { name: "Uncommitted files" })).toHaveTextContent("src/half.rs");
  await expect.element(confirm.getByText("Releases slot-2")).toBeInTheDocument();
  expect(sentChanges, "nothing is sent before the confirm").toEqual([]);

  await userEvent.click(confirm.getByRole("button", { name: "Release" }));
  expect(sentChanges).toEqual([{ act: "release", slot: 2, holder: "claude (pid 44698)" }]);
  await expect.element(panel.getByRole("status")).toHaveTextContent("Committed to fleet/by-hand, slot released");
});

test("Release on a session's slot with nothing uncommitted commits nothing", async () => {
  opened([], {}, [slot(2, { held: { state: "session", holder: "zsh (pid 4120)" }, branch: "fleet/by-hand" })], {
    onChangeSlotPool: () => Promise.resolve({ ok: true, slotChanged: { manifest_id: "m", slot: 2, released: { branch: "fleet/by-hand" } } }),
  });

  const panel = await open("slot-2");
  await userEvent.click(panel.getByRole("button", { name: "Release" }));
  const confirm = confirmOf("Release", "slot-2");
  expect(confirm.getByText(/^Commits the uncommitted files/).elements()).toHaveLength(0);
  await expect.element(confirm.getByText("Keeps branch fleet/by-hand")).toBeInTheDocument();
  await userEvent.click(confirm.getByRole("button", { name: "Release" }));
  await expect.element(panel.getByRole("status")).toHaveTextContent("Slot released");
});

test("a refused Release is said as a slot not released", async () => {
  opened([], {}, [slot(2, { held: { state: "session", holder: "claude (pid 1)" }, branch: "b" })], {
    onChangeSlotPool: () => Promise.resolve({ ok: false, why: "not_connected" }),
  });
  const panel = await open("slot-2");
  await userEvent.click(panel.getByRole("button", { name: "Release" }));
  await userEvent.click(confirmOf("Release", "slot-2").getByRole("button", { name: "Release" }));
  await expect.element(panel.getByRole("alert")).toHaveTextContent("Slot not released");
});
