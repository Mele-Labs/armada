// The Cockpit's merge line with two repositories served, each with open pull requests, storefront's main red. On All with
// nothing open it stacks them a row each, led by each one's main; a Session or a pull request open draws that one's
// repository; one repository picked draws its own. The walk
// `merge-line-by-repository` plays it, over the `cockpit-list-by-repository` Sessions.

import type { MainStanding, MergeLine } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s245CockpitListByRepository } from "./cockpit-list-by-repository";

const open = (number: number, branch: string, ci: "passed" | "running" | "failed", position?: number) => ({
  number,
  title: branch,
  branch,
  url: `https://git.example/pull/${number}`,
  ci,
  ...(position === undefined ? {} : { queue: { state: "queued" as const, position } }),
});

const line = (root: string, pulls: ReturnType<typeof open>[], main?: MainStanding): MergeLine => ({ root, line: [], off: [], landed: [], sent_back: [], hub: { pull_requests: pulls, ...(main === undefined ? {} : { main }) } });

// storefront's main is red, so on All its row comes into view first.
const RED: MainStanding = { state: "red", commit: "4be2c91", read_at: "2026-10-17T18:40:00Z", red_since: "2026-10-17T18:31:00Z", failed: [{ name: "test", check: "test", tests: ["cart.test.ts > rounds the total"] }] };

const BUSY = [
  "fix/session-scroll", "feat/plan-export", "chore/rename-bays", "fix/toast-stack", "docs/merge-queue", "feat/dock-width",
  "fix/clock-drift", "chore/vitest-bump", "fix/ledger-gap", "feat/walk-keys", "fix/tray-icon", "docs/pocket-setup",
  "fix/retro-copy", "feat/slot-close", "fix/search-case", "chore/lint-rules", "fix/review-cache",
];

const [armada, storefront] = s245CockpitListByRepository.state.holds.repositories ?? [];

const lines: MergeLine[] = [
  // Twenty open in armada, to show what a busy repository does to the line; two in its forge's queue, whose track is the row's tallest item.
  line(armada!.root, [
    open(2101, "fix/trim-feed", "running", 1),
    open(2102, "docs/readme-links", "passed", 2),
    open(2103, "chore/bump-the-lockfile", "passed"),
    ...BUSY.map((branch, at) => open(2104 + at, branch, at % 5 === 0 ? "running" : "passed")),
  ]),
  line(storefront!.root, [open(3011, "fix/image-sizes", "passed"), open(3012, "cart/rounding", "failed")], RED),
];

export const s246MergeLineByRepository: Scenario = {
  ...s245CockpitListByRepository,
  name: "merge-line-by-repository",
  says: "The Cockpit's merge line over two served repositories: stacked a row each on All with nothing open, then the open Session's own, then one picked",
  state: { ...s245CockpitListByRepository.state, mergeLines: { lines } },
};
