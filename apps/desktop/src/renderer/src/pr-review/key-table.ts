// The review surface's keys, as a table of its own so the parts that draw a keycap and the part that listens read one spelling.
// What they were checked against, and why they are not registry rows yet, is in `keys.tsx`.

/** The review surface's own keys. `⇧` letters are capitals here, as the keymap's grammar has them. */
export const REVIEW_KEYS = {
  list: "l",
  reader: "r",
  tools: "f",
  lists: "L",
  search: "/",
  quick: "Q",
  ask: "A",
  description: "D",
} as const;

/** A key as it is drawn on a cap: a capital is Shift and the letter. */
export const capOf = (key: string): string => (/^[A-Z]$/.test(key) ? `⇧${key}` : key);
