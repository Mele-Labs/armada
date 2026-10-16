// A repository whose served checkout main has moved past, and what stops Fleet catching it up: four holds, one moment each.
// The hub's `checkout` is what Fleet serves (`HeldCheckout`); the light it raises on the merge queue names the one command that
// is safe to press, or none. `checkout-held-back.ts` plays it over the flight deck's ship.
//
//   0  On main, one file uncommitted and main never touched it: the autostash pull is safe. Run, and the checkout is level.
//      Or send the file as a pull request: Fleet opens #2402 with auto-merge on and the hold carries it (`pull_request`).
//   1  Two files are uncommitted and main changed one of them: an autostash could stop on a conflict.
//   2  On main, with two commits main lacks, so the history diverged.
//   3  A loose agent left 100 files uncommitted, three of them changed on main too: the list is long, the clashing ones scattered through it.
//
// The first `later` moment is the safe hold read again with no pull request, so the walk can play Run after the press of
// Pull request; the second is the checkout level, so a light a Run has not yet taken away is gone before the next hold; the card of
// the holds after it stays open and changes in place.

import type { HeldCheckout, MergeLines } from "@armada/protocol";

import type { BridgeState } from "../../../shared/bridge";

/** A generated file in the long list, whose change Fleet answers cut at its line bound. */
export const GENERATED = "crates/fleet/src/generated/schema.rs";
const GENERATED_AT = 4;
/** The most lines Fleet sends of one file's change (`CHECKOUT_DIFF_LINES` in `crates/fleet/src/catching_up.rs`). */
const FLEET_LINES = 2000;

/** 100 uncommitted files under crates/ and apps/, in the order git would list them, with three that main also changed placed through them. */
const LOOSE_CLASHING: string[] = ["crates/fleet/src/noticing.rs", "apps/desktop/src/renderer/src/Dashboard.tsx", "crates/fleet/src/slots/lease.rs"];
const LOOSE: string[] = (() => {
  const others = Array.from({ length: 97 }, (_, i) => {
    const n = String(i + 1).padStart(2, "0");
    if (i === GENERATED_AT) return GENERATED;
    return i % 2 === 0 ? `crates/fleet/src/loose/part${n}.rs` : `apps/desktop/src/renderer/src/loose/part${n}.tsx`;
  });
  const [a, b, c] = LOOSE_CLASHING as [string, string, string];
  return [...others.slice(0, 17), a, ...others.slice(17, 48), b, ...others.slice(48, 83), c, ...others.slice(83)];
})();

/** The unified diff against HEAD of a file the walk's holds list as uncommitted, as `git diff HEAD -- <path>` prints it. */
const DIFFS: Record<string, string> = {
  "armada.yml": [
    "diff --git a/armada.yml b/armada.yml",
    "index 3f2a9c1..b84e0d7 100644",
    "--- a/armada.yml",
    "+++ b/armada.yml",
    "@@ -4,6 +4,7 @@ checks:",
    "   - name: test",
    "     run: cargo xtask test",
    " merge:",
    "+  auto_merge: checks-pass",
    "   method: squash",
    "   delete_branch: true",
    "",
  ].join("\n"),
  "crates/fleet/src/noticing.rs": [
    "diff --git a/crates/fleet/src/noticing.rs b/crates/fleet/src/noticing.rs",
    "index 91c4be2..0a7d6f3 100644",
    "--- a/crates/fleet/src/noticing.rs",
    "+++ b/crates/fleet/src/noticing.rs",
    "@@ -118,7 +118,9 @@ impl Noticing {",
    "     fn note(&mut self, seen: Seen) {",
    "-        self.seen.push(seen);",
    "+        if self.seen.last() != Some(&seen) {",
    "+            self.seen.push(seen);",
    "+        }",
    "         self.changed = true;",
    "     }",
    "",
  ].join("\n"),
};

/** A file the walk lists with no diff of its own: one changed line, so every path in a long list opens on something. */
const plainDiff = (path: string): string =>
  [`diff --git a/${path} b/${path}`, "index 5d1e8a4..c27b0f9 100644", `--- a/${path}`, `+++ b/${path}`, "@@ -1,3 +1,3 @@", " // loose", "-// before", "+// after", " // end", ""].join("\n");

/** The generated file's change as Fleet sends it: a header, a hunk and added lines up to Fleet's bound. */
const generatedDiff = (): string =>
  [
    `diff --git a/${GENERATED} b/${GENERATED}`,
    "index 8e41c07..2b9f6d3 100644",
    `--- a/${GENERATED}`,
    `+++ b/${GENERATED}`,
    "@@ -1,3 +1,4200 @@",
    ...Array.from({ length: FLEET_LINES - 5 }, (_, i) => `+pub const FIELD_${String(i + 1).padStart(4, "0")}: u16 = ${i + 1};`),
    "",
  ].join("\n");

/** What the walk answers for one uncommitted file: its fixture diff, or the plain one, and whether Fleet cut it. */
export const checkoutDiffOfWalk = (path: string): { diff: string; cut?: true } => (path === GENERATED ? { diff: generatedDiff(), cut: true } : { diff: DIFFS[path] ?? plainDiff(path) });

const HELD: readonly HeldCheckout[] = [
  { base: "main", uncommitted: ["armada.yml"], behind: 880, ahead: 0 },
  { base: "main", uncommitted: ["armada.yml", "crates/fleet/src/noticing.rs"], clashing: ["crates/fleet/src/noticing.rs"], behind: 12, ahead: 0 },
  { base: "main", behind: 5, ahead: 2 },
  { base: "main", uncommitted: LOOSE, clashing: LOOSE_CLASHING, behind: 7, ahead: 0 },
];

/** The lines with one repository's hub carrying the checkout, or none. */
function withCheckout(lines: MergeLines, root: string, checkout?: HeldCheckout): MergeLines {
  return {
    lines: lines.lines.map((line) => {
      if (line.root !== root) return line;
      const { checkout: _held, ...hub } = line.hub ?? {};
      return { ...line, hub: { ...hub, ...(checkout === undefined ? {} : { checkout }) } };
    }),
  };
}

/** The pull request Fleet opens from the files of the safe hold. */
export const SENT_PULL_REQUEST = 2402;

/** The state at the first hold, then the safe hold again, level and each later hold as the changes a walk publishes, the checkout level as the change a Run leaves, and the safe hold carrying a pull request as the change a press of Pull request leaves. */
export function checkoutHeld(
  lines: MergeLines,
  root: string,
): { first: Partial<BridgeState>; later: Partial<BridgeState>[]; level: Partial<BridgeState>; sent: Partial<BridgeState>; safe: HeldCheckout } {
  const at = (checkout?: HeldCheckout): Partial<BridgeState> => ({ mergeLines: withCheckout(lines, root, checkout) });
  const safe = HELD[0]!;
  return {
    first: at(safe),
    later: [at(safe), at(), ...HELD.slice(1).map(at)],
    level: at(),
    sent: at({ ...safe, pull_request: { number: SENT_PULL_REQUEST, auto_merge: true } }),
    safe,
  };
}
