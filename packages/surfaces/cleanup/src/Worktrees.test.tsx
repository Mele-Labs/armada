// Cleanup as one grid of tiles, and what a press on a tile does.
//
// # Why these are browser tests and not stories
//
// `packages/components` sits below this package, so no story there can mount
// this surface. A story proves what one tile and its panel draw; this proves
// what the screen does with a read: that each slot is a tile, that every
// destructive act confirms before it is sent, and that a refusal or a receipt
// is said in the panel it was about.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Outcome, WorktreeSlot } from "@armada/protocol";

import { ChipOwnership } from "@armada/components";
import { mount, unmount } from "@armada/screens/src/mounted";
import { Worktrees } from "./Worktrees";
import type { WorktreesProps } from "./Worktrees";

afterEach(unmount);

/** Stable, because the surface depends on it in an effect. */
const WANT = (): void => {};

/** A fixed instant, so an age on screen is arithmetic and not a race. */
const NOW = Date.parse("2026-09-03T12:00:00Z");

const slot = (n: number, over: Partial<WorktreeSlot> & Pick<WorktreeSlot, "held">): WorktreeSlot => ({
  manifest_id: "armada",
  slot: n,
  path: `/r/.armada/slots/slot-${n}`,
  base: "main",
  warm: false,
  ...over,
});

/** Mount the surface over one pool. */
function opened(slots: WorktreeSlot[], rest: Partial<WorktreesProps> = {}): void {
  mount(<Worktrees onWant={WANT} held={{ state: "read", held: { slots } }} now={NOW} onClose={() => {}} onCopied={() => {}} {...rest} />);
}

/** Press a tile by its name and answer its panel. */
async function open(name: string) {
  await userEvent.click(page.getByRole("button", { name, exact: true }));
  return page.getByRole("dialog", { name });
}

/** The confirm up in a panel, named for its act and its tile. */
const confirmOf = (act: string, name: string) => page.getByRole("group", { name: `${act} ${name}` });

test("Escape closes the panel and nothing is sent", async () => {
  const sent: unknown[] = [];
  opened([slot(1, { held: { state: "free" } })], { onChangeSlotPool: (_manifest, change) => (sent.push(change), Promise.resolve({ ok: true })) });

  await open("slot-1");
  await userEvent.keyboard("{Escape}");

  expect(page.getByRole("dialog").elements()).toHaveLength(0);
  expect(sent).toEqual([]);
});

/** A read that failed says so rather than drawing an empty page. */
test("a read that failed says so rather than drawing an empty page", async () => {
  mount(
    <Worktrees
      onWant={WANT}
      held={{ state: "failed", outcome: { ok: false, why: "not_connected" } }}
      now={NOW}
      onClose={() => {}}
      onCopied={() => {}}
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

/** The worktree pool, a bay per slot, each figure named by its label. */
test("the pool draws a bay per slot", async () => {
  opened([
    slot(1, {
      held: { state: "hosted_session", session_id: "s-reader", session_title: "Fix the reader" },
      branch: "fleet/fix-the-reader",
      since: "2026-09-03T10:00:00.000Z",
      warm: true,
      behind: 4,
    }),
    slot(2, { held: { state: "unmade" } }),
  ]);

  await expect.element(page.getByRole("list", { name: "Worktree slots" })).toBeInTheDocument();
  await expect.element(page.getByRole("listitem", { name: "slot-1" })).toBeInTheDocument();
  await expect.element(page.getByRole("img", { name: "Held" })).toBeInTheDocument();
  await expect.element(page.getByRole("img", { name: "Warm" })).toBeInTheDocument();
  await expect.element(page.getByRole("img", { name: "Not made yet" })).toBeInTheDocument();
  await expect.element(page.getByLabelText("4 commits behind main")).toBeInTheDocument();
  await expect.element(page.getByLabelText("Held for 2 hours")).toBeInTheDocument();
  expect(page.getByText(/slots? free/).elements()).toHaveLength(0);
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
  opened([slot], { onWant: (want) => wants.push(want), onRescueSlot });
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

/**
 * A slot an agent session holds is released from its own confirm, which names
 * the holder and branch and lists the files it commits. The act is sent for the
 * holder shown, and the receipt says where the files went.
 */
test("Release on a session's slot commits its files, sends the holder shown and says so", async () => {
  const sentChanges: unknown[] = [];
  opened(
    [
      slot(2, {
        held: { state: "session", holder: "nvim (pid 44698)" },
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
  await expect.element(confirm.getByText("Held by nvim, on branch fleet/by-hand")).toBeInTheDocument();
  await expect.element(confirm.getByText("pid 44698")).toBeInTheDocument();
  await expect.element(confirm.getByText("Commits the uncommitted files to branch fleet/by-hand as a WIP commit")).toBeInTheDocument();
  await expect.element(confirm.getByRole("list", { name: "Uncommitted files" })).toHaveTextContent("src/half.rs");
  await expect.element(confirm.getByText("Releases slot-2")).toBeInTheDocument();
  expect(sentChanges, "nothing is sent before the confirm").toEqual([]);

  await userEvent.click(confirm.getByRole("button", { name: "Release" }));
  expect(sentChanges).toEqual([{ act: "release", slot: 2, holder: "nvim (pid 44698)" }]);
  await expect.element(panel.getByRole("status")).toHaveTextContent("Committed to fleet/by-hand, slot released");
});

test("Release on a session's slot with nothing uncommitted commits nothing", async () => {
  opened([slot(2, { held: { state: "session", holder: "zsh (pid 4120)" }, branch: "fleet/by-hand" })], {
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
  opened([slot(2, { held: { state: "session", holder: "nvim (pid 1)" }, branch: "b" })], {
    onChangeSlotPool: () => Promise.resolve({ ok: false, why: "not_connected" }),
  });
  const panel = await open("slot-2");
  await userEvent.click(panel.getByRole("button", { name: "Release" }));
  await userEvent.click(confirmOf("Release", "slot-2").getByRole("button", { name: "Release" }));
  await expect.element(panel.getByRole("alert")).toHaveTextContent("Slot not released");
});

/** A hosted Session holds its slot under its own id: the id is the holder sent, and the confirm names the Session. */
test("Release on a hosted Session's slot sends its id as the holder", async () => {
  const sentChanges: unknown[] = [];
  opened([slot(3, { held: { state: "hosted_session", session_id: "s-audit", session_title: "Theme token audit" }, branch: "fleet/audit" })], {
    onChangeSlotPool: (_manifest, change) => {
      sentChanges.push(change);
      return Promise.resolve({ ok: true, slotChanged: { manifest_id: "m", slot: 3, released: { branch: "fleet/audit" } } });
    },
  });

  const panel = await open("slot-3");
  await userEvent.click(panel.getByRole("button", { name: "Release" }));
  const confirm = confirmOf("Release", "slot-3");
  await expect.element(confirm.getByText("Held by Theme token audit, on branch fleet/audit")).toBeInTheDocument();
  await userEvent.click(confirm.getByRole("button", { name: "Release" }));
  expect(sentChanges).toEqual([{ act: "release", slot: 3, holder: "s-audit" }]);
});

/** A Session mid-turn is not released: the refusal is Fleet's bare fact, and the Session opens from beside it. */
test("a Release refused because the Session is working offers to open it", async () => {
  const opens: string[] = [];
  const working: Outcome = {
    ok: false,
    why: "refused",
    error: { code: "fleet.slot_session_working", message: "Pull Request Panel Design is working", run_id: "r", fields: {}, chain: [] },
  };
  const slots = [slot(3, { held: { state: "hosted_session", session_id: "s-panel", session_title: "Pull Request Panel Design" }, branch: "fleet/panel" })];
  mount(
    <ChipOwnership.Provider value={{ ownerOf: () => undefined, open: (id) => opens.push(id) }}>
      <Worktrees
        onWant={WANT}
        held={{ state: "read", held: { slots } }}
        onChangeSlotPool={() => Promise.resolve(working)}
        now={NOW}
        onClose={() => {}}
        onCopied={() => {}}
        />
    </ChipOwnership.Provider>,
  );

  const panel = await open("slot-3");
  await userEvent.click(panel.getByRole("button", { name: "Release" }));
  await userEvent.click(confirmOf("Release", "slot-3").getByRole("button", { name: "Release" }));
  await expect.element(panel.getByRole("alert")).toHaveTextContent("Slot not released: Pull Request Panel Design is working");
  await userEvent.click(panel.getByRole("button", { name: "Open Job" }));
  expect(opens).toEqual(["s-panel"]);
});
