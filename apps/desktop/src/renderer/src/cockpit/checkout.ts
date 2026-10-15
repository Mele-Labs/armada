// A held checkout as a call: what the card says and the one command that is safe to press. Pure, so
// the rule is read without drawing a card. Fleet never forces, stashes or switches the served
// checkout, so the call is how the owner does it where one press cannot hurt (`HeldCheckout`).

import type { HeldCheckout } from "@armada/protocol";

/** One row of the card's facts: a label and its lines. A line may carry a note, which draws as a warning with that tooltip. */
export type Fact = { label: string; lines: readonly { text: string; note?: string }[] };

export type CheckoutCall = {
  /** `880 commits behind main`. */
  title: string;
  /** The one reason it is held, in the order the card ranks them. Empty where none applies. */
  fact: string;
  facts: readonly Fact[];
  /** Absent where no single press is safe. */
  command?: string;
};

const commits = (count: number): string => `${count} ${count === 1 ? "commit" : "commits"}`;

/** A path or branch as a shell word: bare where it is plain, else in single quotes. */
const word = (text: string): string => (/^[\w./@:+=,-]+$/.test(text) ? text : `'${text.replace(/'/g, `'\\''`)}'`);

/** The one command that brings the checkout back to `base` where nothing can be lost or stopped on a conflict. */
function commandOf(root: string, held: HeldCheckout): string | undefined {
  const uncommitted = held.uncommitted ?? [];
  const clashing = held.clashing ?? [];
  if (held.on === undefined) {
    return uncommitted.length > 0 && clashing.length === 0 && held.ahead === 0 ? `git -C ${word(root)} pull --ff-only --autostash` : undefined;
  }
  return uncommitted.length === 0 ? `git -C ${word(root)} switch ${word(held.base)}` : undefined;
}

export function checkoutCall(root: string, held: HeldCheckout): CheckoutCall {
  const uncommitted = held.uncommitted ?? [];
  const clashing = new Set(held.clashing ?? []);
  const fact =
    held.on !== undefined
      ? `On ${held.on}`
      : clashing.size > 0
        ? "Uncommitted changes main also changed"
        : uncommitted.length > 0
          ? "Uncommitted changes"
          : held.ahead > 0
            ? `${commits(held.ahead)} not on ${held.base}`
            : "";
  const command = commandOf(root, held);
  return {
    title: `${commits(held.behind)} behind ${held.base}`,
    fact,
    facts: [
      ...(held.on === undefined ? [] : [{ label: "Branch", lines: [{ text: held.on }] }]),
      ...(uncommitted.length === 0
        ? []
        : [{ label: "Uncommitted", lines: uncommitted.map((path) => ({ text: path, ...(clashing.has(path) ? { note: "Also changed on main" } : {}) })) }]),
      ...(held.ahead > 0 ? [{ label: `Not on ${held.base}`, lines: [{ text: commits(held.ahead) }] }] : []),
    ],
    ...(command === undefined ? {} : { command }),
  };
}
