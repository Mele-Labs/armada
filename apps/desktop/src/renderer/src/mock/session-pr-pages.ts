// What a mock Fleet serves of three pull requests for the panel's tabs, and what a write does to
// them: it is appended to the conversation, and the next read shows it. Kept apart from the
// scenario, which is about the three states the panel draws. Read through `readReviewPage` and
// `writeReview` by the side panel (`slices/reviews.ts`).

import type { PullRequestCheckRow, PullRequestCommit, PullRequestEntry, PullRequestFile, PullRequestPage } from "@armada/protocol";

const PULL = "https://github.com/mele-labs/armada/pull/";
const minutesAgo = (minutes: number): string => new Date(Date.now() - minutes * 60_000).toISOString();

const checks = (number: number, rows: readonly (readonly [string, PullRequestCheckRow["state"], number?])[]): PullRequestCheckRow[] =>
  rows.map(([name, state, seconds], at) => ({
    name,
    state,
    ...(seconds === undefined ? {} : { seconds }),
    ...(state === "skipped" ? {} : { address: `${PULL}${number}/checks?check_run_id=${at + 1}` }),
  }));

const files = (rows: readonly (readonly [string, PullRequestFile["status"], number, number])[]): PullRequestFile[] =>
  rows.map(([path, status, additions, deletions]) => ({ path, status, additions, deletions }));

const commits = (rows: readonly (readonly [string, string, string, number])[]): PullRequestCommit[] =>
  rows.map(([sha, headline, author, ago]) => ({ sha, headline, author, at: minutesAgo(ago) }));

function initial(): Record<number, PullRequestPage> {
  const conversation2209: PullRequestEntry[] = [
    {
      kind: "comment",
      author: "claude",
      at: minutesAgo(95),
      body: "Every link goes through `openPullRequest` now.\n\n- the setting is read once, in main\n- **Open in browser** stays on the panel\n\n```ts\nconst where = settings.links.openPullRequestsIn;\n```",
    },
    { kind: "review", author: "marta", at: minutesAgo(70), verdict: "changes_requested", body: "The setting has no default in `settings.toml`, so a fresh machine opens nothing." },
    { kind: "comment", author: "nick", at: minutesAgo(52), body: "Added the default. `armada` is the value." },
    { kind: "review", author: "marta", at: minutesAgo(40), verdict: "approved", body: "" },
    { kind: "review", author: "nick", at: minutesAgo(12), verdict: "commented", body: "The queue estimate waits on the forge." },
  ];
  return {
    2209: {
      manifest_id: "armada",
      number: 2209,
      base: "main",
      author: "nick",
      created_at: minutesAgo(125),
      additions: 412,
      deletions: 88,
      mergeable: "mergeable",
      review: "approved",
      approvals: 1,
      conversation: conversation2209,
      checks: checks(2209, [
        ["build", "passed", 131],
        ["test · rust", "passed", 242],
        ["test · bridge", "passed", 220],
        ["test · protocol", "passed", 74],
        ["verify-foundations", "passed", 58],
        ["lint", "passed", 31],
        ["typecheck", "passed", 96],
        ["storybook", "passed", 188],
        ["Claude Approvals", "skipped"],
      ]),
      files: files([
        ["apps/desktop/src/renderer/src/session-pull-request.tsx", "modified", 64, 12],
        ["apps/desktop/src/main/forge.ts", "modified", 21, 9],
        ["packages/components/src/compositions/PullRequestPanel/PullRequestPanel.tsx", "added", 188, 0],
        ["packages/components/src/compositions/PullRequestPanel/PullRequestTabs.tsx", "added", 143, 0],
        ["packages/components/src/compositions/PullRequestCard/PullRequestCard.tsx", "removed", 0, 74],
        ["crates/config/settings.toml", "modified", 6, 0],
        ["docs/practices/bridge.md", "renamed", 3, 3],
        ["apps/desktop/resources/icon.png", "modified", 0, 0],
      ]),
      commits: commits([
        ["3fc86fa", "Open pull requests in the panel", "nick", 130],
        ["ed96c0f", "Read the setting once, in main", "nick", 110],
        ["3d35bd9", "Add a default for links.openPullRequestsIn", "nick", 55],
        ["9a01c2e", "Keep Open in browser on the panel", "claude", 30],
      ]),
    },
    2210: {
      manifest_id: "armada",
      number: 2210,
      base: "main",
      author: "marta",
      created_at: minutesAgo(310),
      additions: 96,
      deletions: 14,
      mergeable: "mergeable",
      review: "required",
      approvals: 0,
      conversation: [
        { kind: "comment", author: "marta", at: minutesAgo(280), body: "Reads `placeOf` from the merge lines Fleet serves.\n\nDraws nothing when the pull request is in none." },
      ],
      checks: checks(2210, [
        ["build", "passed", 118],
        ["test · rust", "passed", 251],
        ["test · bridge", "passed", 207],
        ["verify-foundations", "passed", 61],
        ["lint", "passed", 28],
        ["typecheck", "passed", 92],
      ]),
      files: files([
        ["apps/desktop/src/renderer/src/session-pull-request.tsx", "modified", 31, 4],
        ["packages/screens/src/merge-line-place.ts", "modified", 22, 6],
        ["packages/components/src/compositions/MergeLine/MergeLine.tsx", "modified", 29, 4],
        ["packages/components/src/compositions/MergeLine/MergeLine.stories.tsx", "added", 14, 0],
        ["packages/screens/src/merge-line-place.test.ts", "added", 38, 0],
      ]),
      commits: commits([
        ["b71d9f0", "Place a Session's pull request in its merge line", "marta", 300],
        ["c24e8a3", "Draw nothing when there is no line", "marta", 285],
        ["5e60b17", "Cover placeOf for a pull request in none", "marta", 281],
      ]),
    },
    2213: {
      manifest_id: "armada",
      number: 2213,
      base: "main",
      author: "claude",
      created_at: minutesAgo(48),
      additions: 233,
      deletions: 41,
      mergeable: "conflicting",
      review: "changes_requested",
      approvals: 0,
      conversation: [
        { kind: "comment", author: "claude", at: minutesAgo(40), body: "The clash is read from `git merge-tree`, not guessed from the paths." },
        { kind: "review", author: "nick", at: minutesAgo(18), verdict: "changes_requested", body: "Two Sessions can write the same file and each reads the other's change as a clash.\n\n1. Ignore a Session's own writes\n2. Say which Session wrote the other side" },
      ],
      checks: checks(2213, [
        ["build", "passed", 126],
        ["test · rust", "passed", 249],
        ["test · bridge", "failed", 192],
        ["test · protocol", "passed", 70],
        ["verify-foundations", "failed", 58],
        ["lint", "passed", 33],
        ["typecheck", "passed", 99],
        ["storybook", "running"],
      ]),
      files: files([
        ["crates/fleet/src/clashes.rs", "added", 121, 0],
        ["crates/fleet/src/lib.rs", "modified", 2, 0],
        ["packages/protocol/src/clashes.ts", "modified", 18, 3],
        ["apps/desktop/src/main/clashes.ts", "modified", 44, 21],
        ["docs/practices/protocol.md", "modified", 12, 8],
        ["docs/INDEX.md", "modified", 1, 0],
      ]),
      commits: commits([
        ["e08a4d1", "Read write clashes from git merge-tree", "claude", 46],
        ["41bb37c", "Serve the clashes on get_clashes", "claude", 44],
        ["7d92f6a", "Draw the clash on the Session's row", "claude", 41],
      ]),
    },
  };
}

/** What a write does to a pull request's page: it joins the conversation and moves the review, and the next read shows it, as the forge would. */
export type PullRequestPages = {
  read: (number: number) => PullRequestPage | undefined;
  write: (number: number, write: { kind: "comment" | "approve" | "request_changes"; body?: string }) => PullRequestPage | { said: string };
};

export function pullRequestPages(): PullRequestPages {
  const pages = initial();
  const read = (number: number): PullRequestPage | undefined => {
    const page = pages[number];
    return page === undefined ? undefined : { ...page, conversation: [...page.conversation] };
  };
  return {
    read,
    write: (number, write) => {
      const page = pages[number];
      if (page === undefined) return { said: "The forge has no such pull request." };
      const body = write.body?.trim() ?? "";
      if (write.kind !== "approve" && body === "") return { said: "A comment needs words." };
      const at = new Date().toISOString();
      const entry: PullRequestEntry =
        write.kind === "comment"
          ? { kind: "comment", author: "nick", at, body }
          : { kind: "review", author: "nick", at, verdict: write.kind === "approve" ? "approved" : "changes_requested", body };
      pages[number] = {
        ...page,
        conversation: [...page.conversation, entry],
        ...(write.kind === "approve" ? { review: "approved" as const, approvals: page.approvals + 1 } : {}),
        ...(write.kind === "request_changes" ? { review: "changes_requested" as const } : {}),
      };
      return read(number)!;
    },
  };
}
