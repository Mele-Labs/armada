// A held checkout as a call: what the card says and the one command that is safe to press. Pure, so
// the rule is read without drawing a card. Fleet fast-forwards a clean checkout itself and never
// forces or stashes the served one, so the call is how the owner does it where one press cannot hurt
// (`HeldCheckout`). `on` reaches Bridge only when Fleet could not move the base ref, and has no command.

import type { HeldCheckout } from "@armada/protocol";

/** One row of the card's facts: a label and its lines. A line may carry a note, which draws as a warning with that tooltip. */
export type Fact = { label: string; lines: readonly { text: string; note?: string }[] };

export type CheckoutCall = {
  /** `880 commits behind main`. */
  title: string;
  /** The one reason it is held, in the order the card ranks them. Empty where none applies. */
  fact: string;
  facts: readonly Fact[];
  /** Absent where no single press is safe, and once the files went up as a pull request. */
  command?: string;
  /** Set where the uncommitted files can go up as a pull request: on the base branch, none clashing, no pull request yet. */
  sendable?: true;
  /** The pull request opened from the uncommitted files. */
  pullRequest?: { number: number; autoMerge: boolean };
};

const commits = (count: number): string => `${count} ${count === 1 ? "commit" : "commits"}`;

/** A path or branch as a shell word: bare where it is plain, else in single quotes. */
const word = (text: string): string => (/^[\w./@:+=,-]+$/.test(text) ? text : `'${text.replace(/'/g, `'\\''`)}'`);

/** Uncommitted files on the base branch that main did not also change: the pull and the pull request are both safe. */
const sound = (held: HeldCheckout): boolean => held.on === undefined && (held.uncommitted ?? []).length > 0 && (held.clashing ?? []).length === 0;

/** The one command that brings the checkout back to `base` where nothing can be lost or stopped on a conflict. */
function commandOf(root: string, held: HeldCheckout): string | undefined {
  return sound(held) && held.ahead === 0 && held.pull_request === undefined ? `git -C ${word(root)} pull --ff-only --autostash` : undefined;
}

/** The uncommitted paths with the clashing ones first, each group in path order, so a long list opens on the files that matter. */
function inOrder(paths: readonly string[], clashing: ReadonlySet<string>): string[] {
  const byPath = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);
  return [...paths.filter((path) => clashing.has(path)).sort(byPath), ...paths.filter((path) => !clashing.has(path)).sort(byPath)];
}

export function checkoutCall(root: string, held: HeldCheckout): CheckoutCall {
  const clashing = new Set(held.clashing ?? []);
  const uncommitted = inOrder(held.uncommitted ?? [], clashing);
  const fact =
    clashing.size > 0
      ? "Uncommitted changes main also changed"
      : uncommitted.length > 0
        ? "Uncommitted changes"
        : held.ahead > 0
          ? `${commits(held.ahead)} not on ${held.base}`
          : held.on !== undefined
            ? `On ${held.on}`
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
    ...(sound(held) && held.pull_request === undefined ? { sendable: true as const } : {}),
    ...(held.pull_request === undefined ? {} : { pullRequest: { number: held.pull_request.number, autoMerge: held.pull_request.auto_merge } }),
  };
}
