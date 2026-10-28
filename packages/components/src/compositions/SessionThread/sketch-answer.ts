import type { SketchHow } from "@armada/protocol";

// Beside `SketchRow.tsx` and not in it: the node build reads these types through the Sessions draft and sets no `--jsx`.

/** How a sketch put to the person ended, as `detail.settled`, `detail.settled_at` and `detail.words` hold it. */
export type SketchSettled = { how: SketchHow; words?: string; at: string };

/** His answer to a sketch put to him, in `answer_waiting`'s words: approved, or sent back with a note. */
export type SketchAnswer = { choice: 0 } | { text: string };
