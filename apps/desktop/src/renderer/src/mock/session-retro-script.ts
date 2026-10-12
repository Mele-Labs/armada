// A Session's retro as Fleet writes it, frame by frame (`docs/concepts/retro.md`, *A Session's
// retro*): asking the agent, reading the record, then the model's items settling a field at a time
// in the order it writes them, and the end. The frames are `retro` rows replaced by one id, and the
// last one carries no items, as Fleet's does: the kept retro is read for them.
//
// A frame marked `pause` is where a walk stops until time passes; the rest follow at once.

import type { PartialRetroItem, RetroRow } from "@armada/protocol";

/** One frame, and whether a walk holds on it. `at` and `stage_since` are the player's to stamp. */
export type RetroFrame = { pause: boolean; row: Pick<RetroRow, "stage" | "characters" | "items" | "retro" | "why"> };

/** The order the model writes an item's fields in, and the sentence each is. */
const ORDER = ["who", "lands_in", "title", "what", "fix"] as const;

/**
 * The frames of a retro of `items`. The two stages before the writing each hold; the writing holds
 * once the first item is whole, once the second has only its headline (so the card shows the last
 * item as still being written, its other fields awaited), and once the last is whole.
 */
export function retroFrames(items: readonly PartialRetroItem[], retro: number): RetroFrame[] {
  const frames: RetroFrame[] = [
    { pause: true, row: { stage: "asking_agent", characters: 0, items: [] } },
    { pause: true, row: { stage: "reading_record", characters: 0, items: [] } },
  ];
  let characters = 0;
  items.forEach((item, at) => {
    const done = items.slice(0, at);
    const fields: PartialRetroItem = {};
    for (const field of ORDER) {
      const value = item[field];
      if (value === undefined) continue;
      Object.assign(fields, { [field]: value });
      characters += value.length + 18;
      const whole = field === ORDER[ORDER.length - 1];
      const headline = field === "title";
      frames.push({
        pause: (at === 0 && whole) || (at === 1 && headline) || (at === items.length - 1 && whole),
        row: { stage: "writing", characters, items: [...done, { ...fields }] },
      });
    }
  });
  frames.push({ pause: true, row: { stage: "written", characters, items: [], retro } });
  return frames;
}
