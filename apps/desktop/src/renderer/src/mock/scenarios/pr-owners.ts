// Pull requests with the owner registered to each, over two machines under one GitHub login. The
// repository that registers (armada) has five open pull requests: two Sessions' on this
// machine, two on the MacBook Air, and one with no row. The repository that does
// not (pocket) has two, and no chip anywhere. Each serves its Manifest, which carries the switch.
// The walks `pr-owners` and `pr-owners-opt-in` play it.

import type { MergeLine, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";
import { s236SessionPrPanelWork } from "./session-pr-panel-work";

const POCKET: RepositorySummary = {
  root: "/Users/user/code/pocket",
  records_root: "/records/pocket",
  manifest: { ...repository().manifest!, id: "pocket", repository: "pocket", path: "pocket/armada.yml" },
};

const ARMADA = "https://github.com/mele-labs/armada/pull/";
const PHONE = "https://github.com/mele-labs/pocket/pull/";

const pull = (url: string, number: number, title: string, branch: string, rest: Record<string, unknown> = {}) => ({ number, title, branch, url: `${url}${number}`, ci: "passed", ...rest });

function lines(): MergeLine[] {
  const empty = { line: [], off: [], landed: [], sent_back: [] };
  return [
    {
      root: repository().root,
      ...empty,
      hub: {
        pull_requests: [
          pull(ARMADA, 2209, "Open pull requests in Armada", "claude/pr-panel-redesign"),
          pull(ARMADA, 2212, "Tidy the verdict sheet", "chore/verdict-sheet"),
          pull(ARMADA, 2215, "Wire Cockpit holds", "feat/cockpit-holds", { ci: "running" }),
          pull(ARMADA, 2216, "Spike 031 notes", "docs/spike-031"),
          pull(ARMADA, 2218, "Keep the dispatch bar under Helm", "fix/dispatch-bar-under-helm"),
        ],
      },
    },
    {
      root: POCKET.root,
      ...empty,
      hub: {
        pull_requests: [
          pull(PHONE, 418, "Pair with a phone over Tailscale", "pocket/pair-tailscale"),
          pull(PHONE, 421, "Refresh the push token on launch", "fix/push-token", { ci: "failed" }),
        ],
      },
    },
  ];
}

const AIR = { name: "MacBook Air", here: false };
const HERE = { name: "MacBook Pro", here: true };

function build(): Scenario {
  const base = s236SessionPrPanelWork;
  const manifest = manifesting();
  const served = [repository(), POCKET];
  return {
    ...base,
    name: "pr-owners",
    behaves: (handle) => ({ ...base.behaves?.(handle), ...manifest.behaves?.(handle) }),
    says: "Pull requests with the owner registered to each: one repository registers, one does not",
    state: {
      ...base.state,
      repository: null,
      holds: { ...base.state.holds, repositories: served, manifests: served.flatMap((one) => (one.manifest === undefined ? [] : [one.manifest])) },
      mergeLines: { lines: lines() },
    },
    draft: {
      ...base.draft,
      prOwners: {
        repositories: [
          { root: repository().root, name: "mele-labs/armada", registering: true },
          { root: POCKET.root, name: "mele-labs/pocket", registering: false },
        ],
        rows: [
          { repository: "mele-labs/armada", number: 2209, owner: { kind: "session", id: "a3f9c21e", machine: HERE } },
          { repository: "mele-labs/armada", number: 2212, owner: { kind: "session", id: "52c0b7e4", machine: HERE } },
          { repository: "mele-labs/armada", number: 2215, owner: { kind: "session", id: "9c74d82e", machine: AIR } },
          { repository: "mele-labs/armada", number: 2216, owner: { kind: "session", id: "7be40d15", machine: AIR } },
        ],
      },
    },
  };
}

export const prOwners: Scenario = build();
