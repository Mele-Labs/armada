// Cleanup's grid from the keyboard alone: it is one Tab stop, the arrows move across it by where
// tiles are drawn, `j` and `k` through it in drawn order, `o` and Enter open a tile's panel, an act
// in that panel is sent from the keyboard, and closing the panel puts the cursor back on the tile.
// **The cursor is DOM focus**, so every assertion here is about what holds focus, by role.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { ChangeSlotPool, WorktreeSlot } from "@armada/protocol";

import { mount, rerender, unmount } from "@armada/screens/src/mounted";
import { Worktrees } from "./Worktrees";

afterEach(unmount);

const WANT = (): void => {};
const NOW = Date.parse("2026-09-03T12:00:00Z");

const slot = (n: number, over: Partial<WorktreeSlot> = {}): WorktreeSlot => ({
  manifest_id: "armada",
  slot: n,
  path: `/r/.armada/slots/slot-${n}`,
  base: "main",
  warm: false,
  held: { state: "free" },
  ...over,
});

/** A pool of eight bays, enough for two rows at any width the tests run at, recording what the slot acts sent. */
function screen(sent: ChangeSlotPool[], now = NOW) {
  return (
    <Worktrees
      onWant={WANT}
      held={{ state: "read", held: { worktrees: [], slots: [1, 2, 3, 4, 5, 6, 7, 8].map((n) => slot(n)) } }}
      onReclaim={() => Promise.resolve({ ok: true })}
      onDeleteBranch={() => Promise.resolve({ ok: true })}
      onForget={() => Promise.resolve({ ok: true })}
      onChangeSlotPool={(_, change) => {
        sent.push(change);
        return Promise.resolve({ ok: true });
      }}
      now={now}
      onClose={() => {}}
      onCopied={() => {}}
      onOpenJob={() => {}}
    />
  );
}

/** Mount the pool; returns what the slot acts sent. */
function pool(): ChangeSlotPool[] {
  const sent: ChangeSlotPool[] = [];
  mount(screen(sent));
  return sent;
}

const name = (n: number) => page.getByRole("button", { name: `slot-${n}`, exact: true });

/** How many tiles a row of the grid holds, as drawn. */
function columns(): number {
  const list = page.getByRole("list", { name: "Worktree slots" }).element();
  return getComputedStyle(list).gridTemplateColumns.split(" ").length;
}

test("the grid is one Tab stop: Tab enters one tile and leaves without visiting another", async () => {
  const sent = pool();
  await expect.element(name(1)).toBeInTheDocument();
  // The clock moves and every tile draws again: a tooltip's trigger re-takes its Tab stop then, and must not keep it.
  rerender(screen(sent, NOW + 60_000));
  const list = page.getByRole("list", { name: "Worktree slots" }).element();
  const visited = new Set<string | null>();
  let entered = false;
  for (let tab = 0; tab < 30; tab++) {
    await userEvent.keyboard("{Tab}");
    const tile = document.activeElement?.closest(".armada-bay") ?? null;
    if (list.contains(document.activeElement)) {
      entered = true;
      visited.add(tile?.getAttribute("aria-label") ?? "add");
    } else if (entered) break;
  }
  expect(entered).toBe(true);
  expect([...visited]).toEqual(["slot-1"]);

  // Shift+Tab comes back into the same tile, at its last control, and Esc goes to the tile's name.
  await userEvent.keyboard("{Shift>}{Tab}{/Shift}");
  expect(page.getByRole("listitem", { name: "slot-1" }).element().contains(document.activeElement)).toBe(true);
  await userEvent.keyboard("{Escape}");
  await expect.element(name(1)).toHaveFocus();
  // The tile Tab enters at is the one last chosen, not always the first.
  await userEvent.keyboard("{ArrowRight}{ArrowRight}");
  await expect.element(name(3)).toHaveFocus();
  await userEvent.keyboard("{Shift>}{Tab}{/Shift}{Tab}");
  await expect.element(name(3)).toHaveFocus();
});

test("the arrows move across the grid as it is drawn, and j and k through it in order", async () => {
  pool();
  await expect.element(name(1)).toBeInTheDocument();

  // Nothing holds focus yet: j enters the grid at its first tile.
  await userEvent.keyboard("j");
  await expect.element(name(1)).toHaveFocus();
  await userEvent.keyboard("j");
  await expect.element(name(2)).toHaveFocus();
  await userEvent.keyboard("k");
  await expect.element(name(1)).toHaveFocus();

  await userEvent.keyboard("{ArrowRight}");
  await expect.element(name(2)).toHaveFocus();
  // Down is the tile under this one, a row's width on.
  const cols = columns();
  expect(cols).toBeGreaterThan(1);
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(name(2 + cols)).toHaveFocus();
  await userEvent.keyboard("{ArrowUp}");
  await expect.element(name(2)).toHaveFocus();
  await userEvent.keyboard("{ArrowLeft}");
  await expect.element(name(1)).toHaveFocus();
  // Clamped at the edge, never wrapped.
  await userEvent.keyboard("{ArrowLeft}");
  await expect.element(name(1)).toHaveFocus();
});

test("o and Enter open the focused tile's panel, its acts are sent from the keyboard, and Esc comes back to the tile", async () => {
  const sent = pool();
  await expect.element(name(1)).toBeInTheDocument();

  await userEvent.keyboard("jj");
  await expect.element(name(2)).toHaveFocus();
  await userEvent.keyboard("o");
  const panel = page.getByRole("dialog", { name: "slot-2" });
  await expect.element(panel).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(panel).not.toBeInTheDocument();
  await expect.element(name(2)).toHaveFocus();

  await userEvent.keyboard("{ArrowRight}{Enter}");
  const third = page.getByRole("dialog", { name: "slot-3" });
  await expect.element(third).toBeVisible();
  // Tab walks the panel's acts; Enter on one sends it.
  const close = third.getByRole("button", { name: "Close slot" });
  for (let tab = 0; tab < 6 && document.activeElement !== close.element(); tab++) await userEvent.keyboard("{Tab}");
  await expect.element(close).toHaveFocus();
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => sent).toEqual([{ act: "close", slot: 3 }]);
});

test("kept tiles, a bay and a worktree outside the pool, are in Kept and not the pool, and the keys reach them", async () => {
  mount(
    <Worktrees
      onWant={WANT}
      held={{
        state: "read",
        held: {
          slots: [
            slot(1),
            slot(2, { held: { state: "job", job_id: "01KEPT", job_title: "Retry the read", kept: "2 uncommitted, first a.rs" } }),
            slot(3),
          ],
          worktrees: [
            {
              job_id: "outside",
              job_title: "Send the digest",
              status: "rejected",
              last_moved_at: "2026-08-30T09:14:00Z",
              path: "/r/.armada/worktrees/outside",
              branch: "armada/outside",
              held: [{ why: "uncommitted", files: ["src/a.rs"] }],
              on_disk: true,
            },
          ],
        },
      }}
      onReclaim={() => Promise.resolve({ ok: true })}
      onDeleteBranch={() => Promise.resolve({ ok: true })}
      onForget={() => Promise.resolve({ ok: true })}
      now={NOW}
      onClose={() => {}}
      onCopied={() => {}}
      onOpenJob={() => {}}
    />,
  );
  const pooled = page.getByRole("list", { name: "Worktree slots" });
  const kept = page.getByRole("list", { name: "Kept" });
  await expect.element(kept).toBeInTheDocument();
  expect(kept.getByRole("listitem").elements().map((one) => one.getAttribute("aria-label"))).toEqual(["slot-2", "outside"]);
  expect(pooled.getByRole("listitem").elements().map((one) => one.getAttribute("aria-label"))).toEqual(["slot-1", "slot-3"]);

  // j crosses from the pool's last tile into Kept, and Up crosses back.
  await userEvent.keyboard("jj");
  await expect.element(name(3)).toHaveFocus();
  await userEvent.keyboard("j");
  await expect.element(name(2)).toHaveFocus();
  await userEvent.keyboard("{ArrowUp}");
  expect(pooled.element().contains(document.activeElement)).toBe(true);
  await userEvent.keyboard("{ArrowDown}");
  expect(kept.element().contains(document.activeElement)).toBe(true);

  // A panel opened in Kept gives focus back to its tile there.
  await userEvent.keyboard("{ArrowRight}o");
  await expect.element(page.getByRole("dialog", { name: "outside" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(kept.getByRole("button", { name: "outside", exact: true })).toHaveFocus();
});

test("nothing kept draws no Kept section", async () => {
  pool();
  await expect.element(name(1)).toBeInTheDocument();
  expect(page.getByRole("list", { name: "Kept" }).elements()).toHaveLength(0);
});
