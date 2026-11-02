// Worktree slots, Studios and Doctor before Fleet has answered: the reads are held at `reading`, so each page draws its skeleton and
// nothing lands. The same Fleet as `pages-look`, which is the one with data to land on.

import type { Scenario } from "../moment";
import { pagesLook } from "./pages-look";

export const loadingPages: Scenario = {
  ...pagesLook,
  name: "loading/pages",
  says: "Worktree slots and Studios while Fleet's reads are still out: skeletons where the bays and rows will be",
  state: { ...pagesLook.state, health: { state: "reading" } },
  behaves: (fleet) => ({
    ...pagesLook.behaves?.(fleet),
    readHeld: async (want) => fleet.publish({ held: want ? { state: "reading" } : { state: "none" } }),
    watchStudios: async (manifestId) =>
      fleet.publish({
        studios:
          manifestId === null
            ? { state: "none" }
            : typeof manifestId === "string"
              ? { state: "reading", manifestId }
              : { state: "many", reads: manifestId.map((one) => ({ state: "reading" as const, manifestId: one })) },
      }),
  }),
};
