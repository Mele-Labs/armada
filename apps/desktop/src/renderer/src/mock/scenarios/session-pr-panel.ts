// A Session holding three pull requests, one in each state its panel draws: third of seven in the
// merge queue, every check passed, and checks failed. The walks `pullRequestPanel` and
// `pullRequestPanelNarrow` play it; the page each serves for the tabs is `../session-pr-pages.ts`.

import type { MergeLine } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { SessionAttachment } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { others } from "../sessions/script";
import { s200Sessions } from "./sessions";
import { pullRequestPages } from "../session-pr-pages";

const PULL = "https://github.com/mele-labs/armada/pull/";

function line(): MergeLine {
  const pull = (number: number, title: string, branch: string, position: number, state: string) => ({
    number,
    title,
    branch,
    url: `${PULL}${number}`,
    ci: "passed",
    queue: { state, position },
  });
  return {
    root: repository().root,
    line: [],
    off: [],
    landed: [],
    sent_back: [],
    hub: {
      pull_requests: [
        pull(2201, "Repair the failing check", "fix/repair-the-check", 1, "mergeable"),
        pull(2204, "Keep the dispatch bar under Helm", "fix/dispatch-bar-under-helm", 2, "awaiting_checks"),
        pull(2209, "Open pull requests in Armada", "claude/pr-panel-redesign", 3, "queued"),
        pull(2211, "Settings: per-repo merge method", "feat/merge-method", 4, "queued"),
        pull(2212, "Tidy the verdict sheet", "chore/verdict-sheet", 5, "queued"),
        pull(2215, "Wire Cockpit holds", "feat/cockpit-holds", 6, "queued"),
        pull(2216, "Spike 031 notes", "docs/spike-031", 7, "queued"),
        { number: 2210, title: "Show the merge line in a Session", branch: "feat/session-merge-line", url: `${PULL}2210` },
        { number: 2213, title: "Read write clashes from Fleet", branch: "feat/write-clash-read", url: `${PULL}2213` },
      ],
    },
  };
}

const attached = (number: number, title: string, branch: string, rest: Partial<Extract<SessionAttachment, { kind: "pull_request" }>>): SessionAttachment => ({
  kind: "pull_request",
  number,
  title,
  branch,
  address: `${PULL}${number}`,
  checks: { state: "passed" },
  state: "open",
  auto: false,
  ...rest,
});

const holder = () => {
  const one = others().find((it) => it.id === "s3")!;
  return {
    ...one,
    attachments: [
      ...one.attachments.filter((it) => it.kind !== "pull_request"),
      attached(2209, "Open pull requests in Armada", "claude/pr-panel-redesign", {
        queued: true,
        body: "## What changes\n\nPull request links open the same way everywhere, behind a setting: in **Armada** or in the **browser**.\n\n- `links.openPullRequestsIn` picks which\n- the Session reading sheet draws a full panel, not a card\n\n### Not in this change\n\n1. Comments and reviews\n2. The Files and Commits tabs\n\nSee [the design note](https://example.com/pr-panel) for the plates.",
      }),
      attached(2210, "Show the merge line in a Session", "feat/session-merge-line", {
        body: "Shows where a Session's pull request stands in the merge line.\n\n- reads `placeOf` from the merge lines Fleet serves\n- draws nothing when the pull request is not in one",
      }),
      attached(2213, "Read write clashes from Fleet", "feat/write-clash-read", {
        body: "Two Sessions writing the same file now say so before either merges.\n\nThe clash is read from `git merge-tree`, not guessed from the paths.",
        checks: { state: "failed", failing: "test · bridge, verify-foundations" },
      }),
    ],
  };
};

export const s236SessionPrPanel: Scenario = {
  ...s200Sessions,
  name: "session-pr-panel",
  says: "A Session's pull requests: in the merge queue, ready to merge, checks failed",
  state: { ...s200Sessions.state, mergeLines: { lines: [line()] } },
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [holder()]), pullRequestPages },
};
