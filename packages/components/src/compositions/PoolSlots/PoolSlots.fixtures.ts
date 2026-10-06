// What the PoolSlots stories draw, and how they reach a tile's panel. A sibling
// of the stories, not a story: it is data and two helpers, shared by the files
// that split the stories by what they prove.

import { within } from "storybook/test";
import type { WorktreeHeld, WorktreeSlot } from "@armada/protocol";

const AT = "/Users/user/armada/.armada/slots";

export function slot(n: number, rest: Partial<WorktreeSlot> & Pick<WorktreeSlot, "held">): WorktreeSlot {
  return { manifest_id: "armada", slot: n, path: `${AT}/slot-${n}`, base: "main", warm: false, ...rest };
}

export function held(over: Partial<WorktreeHeld> = {}): WorktreeHeld {
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

export const STRANDED: Pick<WorktreeSlot, "held" | "branch" | "behind" | "stranded"> = {
  held: { state: "stranded", why: "2 uncommitted, first src/lib.rs" },
  branch: "fleet/old-try",
  behind: 12,
  stranded: {
    uncommitted: ["src/lib.rs", "src/reader/retry.rs"],
    commits: [
      { sha: "9d41e07b2c", subject: "Retry a short read once", home: "only_here" },
      { sha: "3b7a1c9e55", subject: "Split the reader from the parser", home: "on_remote" },
    ],
    unpushed: 1,
  },
};

export const FOUND = {
  commit: "9d41e07b2c",
  uncommitted: true,
  read: ["src/reader/mod.rs", "src/reader/retry.rs"],
  searched: ["retry_short_read in src/"],
};

type Canvas = { getByRole: (role: "listitem" | "dialog", options: { name: string }) => HTMLElement };

/** A tile, by its name. */
export const tile = (canvas: Canvas, name: string) => within(canvas.getByRole("listitem", { name }));

/** The panel open on the tile `name`. */
export const panel = (canvas: Canvas, name: string) => within(canvas.getByRole("dialog", { name }));

/** Press the tile `name` and answer its panel. */
export async function openTile(canvas: Canvas, userEvent: { click: (element: Element) => Promise<void> }, name: string) {
  await userEvent.click(tile(canvas, name).getByRole("button", { name }));
  return panel(canvas, name);
}
