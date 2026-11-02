// The context window as a ring in the composer beside Send, at three fills. Over the `compact-session-view` scenario.

import { button, deckRow, walk } from "../walk";

const ring = (percent: number) => button(`Context ${percent}%`, { exact: true });

const contextRing = walk("compact-session-view", [
  { press: deckRow("Retry the slot lease"), say: "The Job's context is a small ring in the composer, just left of Send, with its percent beside it" },
  { hover: ring(32), say: "Hovering it opens the breakdown, and a press keeps it open" },
  { press: deckRow("Rotate the audit log"), say: "From 80% the ring and its percent turn amber" },
  { look: ring(83), say: "Amber at 83%" },
  { press: deckRow("Name the store migrations"), say: "From 95% they turn red" },
  { look: ring(98), say: "Red at 98%" },
]);

export { contextRing };
