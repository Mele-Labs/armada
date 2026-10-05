// Choosing which of a row's three acts to send, and what the screen says
// before it does.
//
// # Why these are browser tests and not stories
//
// `packages/components` sits below this package, so no story there can mount
// this surface — a story proves what one held row draws, and this proves what
// the surface does with a set of them. What is asserted here is behaviour a
// rendering cannot show: that each act is independent, that a job still
// running is drawn and never offered, that the confirmation reads out what it
// is about to end, and that a refusal on one row's checkout cancels that
// row's forget rather than swallowing the rest of the batch.
//
// The arithmetic is next door in `held.test.ts`, where a hundred cases cost
// what one costs.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type {
  BranchDeleted,
  HeldWorktrees,
  Outcome,
  WorktreeHeld,
  WorktreeReclaimed,
  WorktreeSlot,
} from "@armada/protocol";

import { mount, unmount } from "./mounted";
import { Worktrees } from "./Worktrees";

afterEach(unmount);

/** Stable, because the surface depends on it in an effect. */
const WANT = (): void => {};

/** A fixed instant, so an age on screen is arithmetic and not a race. */
const NOW = Date.parse("2026-09-03T12:00:00Z");

function held(over: Partial<WorktreeHeld> = {}): WorktreeHeld {
  return {
    job_id: "01JOB0001",
    job_title: "Port the settings selectors",
    status: "completed_success",
    last_moved_at: "2026-08-30T09:14:00Z",
    path: "/Users/user/armada/.armada/worktrees/01JOB0001",
    branch: "armada/01JOB0001",
    held: [],
    on_disk: true,
    ...over,
  };
}

const UNMERGED = { why: "unmerged", base: "main", commits: 3, tip: "9f1c2ab84d5e" } as const;

/** Mount the surface over one answer, recording every id each of the three acts was sent for. */
function opened(
  worktrees: WorktreeHeld[],
  answers: {
    reclaim?: (jobId: string) => Outcome;
    deleteBranch?: (jobId: string, tip: string) => Outcome;
    forget?: (jobId: string) => Outcome;
  } = {},
): { reclaimed: string[]; branchesDeleted: string[]; forgotten: string[] } {
  const reclaimed: string[] = [];
  const branchesDeleted: string[] = [];
  const forgotten: string[] = [];
  const read: HeldWorktrees = { state: "read", held: { worktrees } };
  mount(
    <Worktrees
      onWant={WANT}
      held={read}
      onReclaim={(jobId) => {
        reclaimed.push(jobId);
        return Promise.resolve(answers.reclaim?.(jobId) ?? { ok: true });
      }}
      onDeleteBranch={(jobId, tip) => {
        branchesDeleted.push(jobId);
        return Promise.resolve(answers.deleteBranch?.(jobId, tip) ?? { ok: true });
      }}
      onForget={(jobId) => {
        forgotten.push(jobId);
        return Promise.resolve(answers.forget?.(jobId) ?? { ok: true });
      }}
      now={NOW}
      onClose={() => {}}
      onCopied={() => {}}
      onOpenJob={() => {}}
    />,
  );
  return { reclaimed, branchesDeleted, forgotten };
}

function removeCheckoutBox(title: string) {
  return page.getByRole("checkbox", { name: `Remove the checkout — ${title}` });
}

function deleteBranchBox(title: string) {
  return page.getByRole("checkbox", { name: `Delete the branch — ${title}` });
}

function forgetBox(title: string) {
  return page.getByRole("checkbox", { name: `Forget the job — ${title}` });
}

function cleanUp() {
  return page.getByRole("button", { name: /^Clean up/ });
}

async function confirm() {
  await userEvent.click(page.getByRole("dialog").getByRole("button", { name: "Clean up" }));
}

/**
 * **The act is per row and per choice, and there is no select-all.** `armada
 * clean --everything` is the one bulk act in armada and it is the one nobody
 * should reach for from a screen; a control here that took the whole list
 * would be that act with a friendlier name.
 */
test("nothing is chosen until somebody chooses it, one row at a time", async () => {
  const { reclaimed } = opened([
    held({ job_id: "a", job_title: "First", held: [UNMERGED] }),
    held({ job_id: "b", job_title: "Second", held: [UNMERGED] }),
  ]);

  await expect.element(cleanUp()).toBeDisabled();
  // Two rows, each offering a checkout and a branch — four checkboxes, and no
  // fifth control that takes them all at once.
  expect(page.getByRole("checkbox").elements()).toHaveLength(4);

  await userEvent.click(removeCheckoutBox("First"));
  await expect.element(cleanUp()).toHaveTextContent("Clean up 1 row");

  await userEvent.click(cleanUp());
  await confirm();
  expect(reclaimed, "only the row that was chosen").toEqual(["a"]);
});

/**
 * **A job that has not ended is drawn and never offered.** Fleet refuses every
 * act on a status that is not terminal, so a checkbox would be a control whose
 * only outcome is a refusal — and the row still has to be on the page, or a
 * worktree missing from the list above reads as disk already returned.
 */
test("a job still running is on the page with no control on it", async () => {
  opened([
    held({ job_id: "a", job_title: "Finished", held: [UNMERGED] }),
    held({
      job_id: "b",
      job_title: "Still going",
      status: "running",
      held: [{ why: "not_terminal", status: "running" }],
    }),
  ]);

  await expect.element(page.getByText("Still going", { exact: true })).toBeInTheDocument();
  // The finished row offers a checkout and a branch; the running one offers
  // nothing at all.
  expect(page.getByRole("checkbox").elements(), "two checkboxes, none on the running row").toHaveLength(2);
  await expect.element(removeCheckoutBox("Finished")).toBeInTheDocument();
  await expect.element(deleteBranchBox("Finished")).toBeInTheDocument();
});

/**
 * **The confirmation says what is lost, and never how much disk comes back.**
 * Uncommitted files are destroyed only where the checkout is chosen, and the
 * branch survives whole because deleting it is its own, unchosen checkbox.
 */
test("the confirmation names the files it destroys and the branch it keeps", async () => {
  opened([
    held({
      job_id: "a",
      job_title: "Trial the judge prompt",
      held: [UNMERGED, { why: "uncommitted", files: ["crates/config/src/judge.rs"] }],
    }),
  ]);

  await userEvent.click(removeCheckoutBox("Trial the judge prompt"));
  await userEvent.click(cleanUp());

  const dialog = page.getByRole("dialog");
  await expect.element(dialog).toHaveTextContent("One file is destroyed");
  await expect.element(dialog).toHaveTextContent("crates/config/src/judge.rs");
  await expect.element(dialog).toHaveTextContent("These branches are kept");
  await expect.element(dialog).toHaveTextContent("armada/01JOB0001");
});

/**
 * The ordinary case, and it is said rather than left as an absence. Choosing
 * only the checkout, with no force on that seam, ends nothing at all — a
 * confirmation that listed nothing would read as one that failed to say what
 * it costs.
 */
test("removing a checkout that ends nothing says so", async () => {
  opened([held({ job_id: "a", job_title: "Finished", held: [UNMERGED] })]);

  await userEvent.click(removeCheckoutBox("Finished"));
  await userEvent.click(cleanUp());

  await expect.element(page.getByRole("dialog")).toHaveTextContent("Nothing is lost.");
});

/**
 * **Deleting the branch is a separate, explicit force**, and the confirmation
 * names its commit count and its tip — rule 4's own words — rather than
 * folding it into the checkout's own line.
 */
test("deleting a branch names its commit count and its tip, and sends that tip", async () => {
  const { branchesDeleted } = opened([
    held({ job_id: "a", job_title: "Rework the retry ceiling", on_disk: false, held: [UNMERGED] }),
  ]);

  await userEvent.click(deleteBranchBox("Rework the retry ceiling"));
  await userEvent.click(cleanUp());

  const dialog = page.getByRole("dialog");
  await expect.element(dialog).toHaveTextContent("One branch is deleted");
  await expect.element(dialog).toHaveTextContent("3 commits");
  await expect.element(dialog).toHaveTextContent("9f1c2ab84d5e");

  await confirm();
  expect(branchesDeleted).toEqual(["a"]);
});

/**
 * **One refusal does not swallow the rest.** There is no bulk route on the
 * wire, so this is one call per id and some can refuse while others land — and
 * the one that refused has to be named, or a person is left comparing the list
 * against what they remember choosing.
 */
test("a refusal on one job is named and the others still go", async () => {
  const { reclaimed } = opened(
    [
      held({ job_id: "a", job_title: "First", held: [UNMERGED] }),
      held({ job_id: "b", job_title: "Second", held: [UNMERGED] }),
    ],
    { reclaim: (jobId) => (jobId === "a" ? { ok: false, why: "not_connected" } : { ok: true }) },
  );

  await userEvent.click(removeCheckoutBox("First"));
  await userEvent.click(removeCheckoutBox("Second"));
  await userEvent.click(cleanUp());
  await confirm();

  expect(reclaimed, "the second was sent after the first refused").toEqual(["a", "b"]);
  await expect.element(page.getByText("First: the checkout was not removed")).toBeInTheDocument();
});

/**
 * **The rule this whole change exists for.** Forgetting a record whose
 * checkout would not go orphans that disk from this page — `worktrees_held`
 * walks Job records, not directories — so a refused checkout cancels that
 * row's forget even though it was chosen in the same act.
 */
test("a refused checkout cancels that row's forget, in the same act", async () => {
  // `locked` and not `held: []`: an empty reason list is provably safe, which
  // fleet reclaims on its own and this row would never be choosable at all.
  const row = held({
    job_id: "a",
    job_title: "Rework the retry ceiling",
    on_disk: true,
    held: [{ why: "locked", reason: "still checked out elsewhere" }],
  });
  const { reclaimed, forgotten } = opened([row], {
    reclaim: () => ({ ok: false, why: "not_connected" }),
  });

  await userEvent.click(removeCheckoutBox("Rework the retry ceiling"));
  await userEvent.click(forgetBox("Rework the retry ceiling"));
  await userEvent.click(cleanUp());
  await confirm();

  expect(reclaimed).toEqual(["a"]);
  expect(forgotten, "forget was never sent for a row whose checkout would not go").toEqual([]);
  await expect.element(page.getByText("Rework the retry ceiling: the checkout was not removed")).toBeInTheDocument();
});

/**
 * **Fleet's own half is on this page too.** A worktree the sweep is about to
 * take is drawn rather than filtered out: a person who came looking for one and
 * does not find it cannot tell "already given back" from "held and not said".
 */
test("what fleet takes on its own is drawn, and offers nothing", async () => {
  opened([held({ job_id: "a", job_title: "Already safe" })]);

  await expect.element(page.getByText("Already safe")).toBeInTheDocument();
  // Nothing waiting on a person draws nothing: no card and no sentence.
  expect(page.getByText("Waiting on you").query()).toBeNull();
  expect(page.getByText("Nothing is waiting on you").query()).toBeNull();
  expect(page.getByRole("checkbox").elements()).toHaveLength(0);
});

/**
 * A failed read is a failure of the read and never an empty list. An empty list
 * here would claim fleet is holding nothing, which is the one answer on this
 * page nobody should be given by accident.
 */
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

  await expect
    .element(page.getByText("What fleet is holding could not be read"))
    .toBeInTheDocument();
  expect(page.getByRole("checkbox").elements()).toHaveLength(0);
});

/**
 * **How long it has sat is on the row that can lose something, and on nothing
 * else.** An unmerged branch survives a checkout removal, so an age beside it
 * is a number with no decision attached; uncommitted files do not survive, and
 * the age is half of what makes that answerable.
 */
test("only the row where something is destroyed says how long it has sat", async () => {
  opened([
    held({
      job_id: "a",
      job_title: "Forgotten",
      held: [{ why: "uncommitted", files: ["src/log.rs"] }],
    }),
    held({ job_id: "b", job_title: "Merged away", held: [UNMERGED] }),
  ]);

  await expect
    .element(page.getByText(/Armada last moved this job 4 days ago/))
    .toBeInTheDocument();
  expect(
    page.getByText(/Armada last moved this job/).elements(),
    "the unmerged row carries no age",
  ).toHaveLength(1);
});

/**
 * And it survives onto the confirmation, which is the last screen before the
 * files are gone — the place a person recognises work they had forgotten.
 */
test("the confirmation says how long the work it is about to destroy has sat", async () => {
  opened([
    held({
      job_id: "a",
      job_title: "Forgotten",
      held: [{ why: "uncommitted", files: ["src/log.rs"] }],
    }),
  ]);

  await userEvent.click(removeCheckoutBox("Forgotten"));
  await userEvent.click(cleanUp());

  await expect
    .element(page.getByRole("dialog"))
    .toHaveTextContent("Forgotten — last moved 4 days ago");
});

/**
 * The worktree pool, a bay per slot, each figure named by its label. **A Job
 * holding one opens that Job.**
 */
test("the pool draws a bay per slot, and a slot's Job opens from it", async () => {
  const opens: string[] = [];
  const slot = (n: number, over: Partial<WorktreeSlot> & Pick<WorktreeSlot, "held">): WorktreeSlot => ({
    manifest_id: "armada",
    slot: n,
    path: `/r/.armada/slots/slot-${n}`,
    base: "main",
    warm: false,
    ...over,
  });
  mount(
    <Worktrees
      onWant={WANT}
      held={{
        state: "read",
        held: {
          worktrees: [],
          slots: [
            slot(1, {
              held: { state: "job", job_id: "01JOB", job_title: "Fix the reader" },
              branch: "armada/1-fix-the-reader",
              since: "2026-09-03T10:00:00.000Z",
              warm: true,
              behind: 4,
            }),
            slot(2, { held: { state: "unmade" } }),
          ],
        },
      }}
      onReclaim={() => Promise.resolve({ ok: true })}
      onDeleteBranch={() => Promise.resolve({ ok: true })}
      onForget={() => Promise.resolve({ ok: true })}
      now={NOW}
      onClose={() => {}}
      onCopied={() => {}}
      onOpenJob={(jobId) => opens.push(jobId)}
    />,
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
function rescuing(slot: WorktreeSlot, onRescueSlot: NonNullable<Parameters<typeof Worktrees>[0]["onRescueSlot"]>) {
  const wants: boolean[] = [];
  mount(
    <Worktrees
      onWant={(want) => wants.push(want)}
      held={{ state: "read", held: { worktrees: [], slots: [slot] } }}
      onReclaim={() => Promise.resolve({ ok: true })}
      onDeleteBranch={() => Promise.resolve({ ok: true })}
      onForget={() => Promise.resolve({ ok: true })}
      now={NOW}
      onClose={() => {}}
      onCopied={() => {}}
      onOpenJob={() => {}}
      onRescueSlot={onRescueSlot}
    />,
  );
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

  const answered = rescuing(stranded({ ...FINDING, state: "answered", summary: "Half moved." }), () =>
    Promise.resolve({ ok: true, rescued: RECEIPT }),
  );
  await new Promise((done) => setTimeout(done, 1_500));
  expect(answered).toEqual([true]);
});

/** A rescue act Fleet refuses is said on the bay it was about, as the pool's acts are. */
test("a refused rescue is said on its bay", async () => {
  rescuing(stranded(), () =>
    Promise.resolve({
      ok: false,
      why: "refused",
      error: { code: "fleet.slot_busy", message: "a lease is under way", run_id: "r", fields: {}, chain: [] },
    }),
  );
  await userEvent.click(page.getByRole("button", { name: "Rescue" }));
  await expect.element(page.getByRole("alert")).toHaveTextContent("Not started: a lease is under way");
});

/**
 * The Finding opens in the app's trailing sheet and not on the bay, and Escape
 * closes it without sending anything.
 */
test("a Finding opens in the sheet and Escape closes it", async () => {
  const sent: string[] = [];
  rescuing(stranded({ ...FINDING, state: "answered", summary: "Half moved." }), (_manifest, rescue) => {
    sent.push(rescue.act);
    return Promise.resolve({ ok: true, rescued: RECEIPT });
  });
  expect(page.getByText("Half moved.").elements()).toHaveLength(0);
  await userEvent.click(page.getByRole("button", { name: "Finding" }));
  await expect.element(page.getByRole("dialog", { name: "Finding" }).getByText("Half moved.")).toBeInTheDocument();
  await userEvent.keyboard("{Escape}");
  expect(page.getByRole("dialog").elements()).toHaveLength(0);
  expect(sent).toEqual([]);
});

/** A Scrap that kept the branch says so on the slot it freed, once its confirm is answered. */
test("a scrap that kept its branch says so, after its confirm", async () => {
  const sent: string[] = [];
  rescuing(stranded({ ...FINDING, state: "answered", summary: "Half moved." }), (_manifest, rescue) => {
    sent.push(rescue.act);
    return Promise.resolve({ ok: true, rescued: { ...RECEIPT, branch: "fleet/an-old-try", branch_kept: true } });
  });
  await userEvent.click(page.getByRole("button", { name: "Finding" }));
  await userEvent.click(page.getByRole("dialog", { name: "Finding" }).getByRole("button", { name: "Scrap" }));
  expect(sent).toEqual([]);
  await userEvent.click(page.getByRole("group", { name: "Scrap slot-4" }).getByRole("button", { name: "Scrap" }));
  expect(sent).toEqual(["scrap"]);
  await expect.element(page.getByRole("status")).toHaveTextContent("fleet/an-old-try kept");
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
  await userEvent.click(page.getByRole("button", { name: "Rescue" }));
  expect(sent).toEqual(["start 8"]);
});

// Only referenced for their types, so the answer functions above stay honest
// about what they hand back.
void (null as unknown as BranchDeleted);
void (null as unknown as WorktreeReclaimed);
