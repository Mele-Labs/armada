// The Cockpit's merge line with two repositories served, each with open pull requests. On All it is dimmed until a Job, a
// Session or a pull request is open, and then draws that one's repository; one repository picked draws its own. The walk
// `merge-line-by-repository` plays it, over the `cockpit-list-by-repository` Jobs and Sessions.

import type { MergeLine } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s245CockpitListByRepository } from "./cockpit-list-by-repository";

const open = (number: number, branch: string, ci: "passed" | "running" | "failed") => ({ number, title: branch, branch, url: `https://git.example/pull/${number}`, ci });

const line = (root: string, pulls: ReturnType<typeof open>[]): MergeLine => ({ root, line: [], off: [], landed: [], sent_back: [], hub: { pull_requests: pulls } });

const [armada, storefront] = s245CockpitListByRepository.state.holds.repositories ?? [];

const lines: MergeLine[] = [
  line(armada!.root, [open(2101, "fix/trim-feed", "running"), open(2102, "docs/readme-links", "passed"), open(2103, "chore/bump-the-lockfile", "passed")]),
  line(storefront!.root, [open(3011, "fix/image-sizes", "passed"), open(3012, "cart/rounding", "failed")]),
];

export const s246MergeLineByRepository: Scenario = {
  ...s245CockpitListByRepository,
  name: "merge-line-by-repository",
  says: "The Cockpit's merge line over two served repositories: dimmed on All with nothing open, then the open Job's or Session's own, then one picked",
  state: { ...s245CockpitListByRepository.state, mergeLines: { lines } },
};
