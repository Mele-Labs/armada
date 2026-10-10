// Cleanup's grid from the keyboard alone: the arrows move across it by where tiles are drawn, `j`
// and `k` through it in drawn order, `o` and Enter open a tile's panel, an act in that panel is
// sent from the keyboard, and closing the panel puts the cursor back on the tile it came from.
// **The cursor is DOM focus**, so every assertion here is about what holds focus, by role.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { ChangeSlotPool, WorktreeSlot } from "@armada/protocol";

import { mount, unmount } from "@armada/screens/src/mounted";
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

/** A pool of eight bays, enough for two rows at any width the tests run at; returns what the slot acts sent. */
function pool(): ChangeSlotPool[] {
  const sent: ChangeSlotPool[] = [];
  mount(
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
      now={NOW}
      onClose={() => {}}
      onCopied={() => {}}
      onOpenJob={() => {}}
    />,
  );
  return sent;
}

const name = (n: number) => page.getByRole("button", { name: `slot-${n}`, exact: true });

/** How many tiles a row of the grid holds, as drawn. */
function columns(): number {
  const list = page.getByRole("list", { name: "Worktree slots" }).element();
  return getComputedStyle(list).gridTemplateColumns.split(" ").length;
}

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
