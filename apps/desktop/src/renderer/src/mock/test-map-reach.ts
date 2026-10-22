// What the test map says of the files of pull request 2209 that the `test-map-reach` walk looks at, in the shape
// `GET /test_map/reaching` answers (`crates/ipc/src/test_map.rs`), and the reading of it the Files list draws. Mock only:
// no TypeScript mirror of the wire exists yet, so this file holds its own, and `reachOf` is the mapping the wired read will need.

import type { FilesReach, ReachTest } from "@armada/components";

type Answer = {
  checks: readonly {
    check: string;
    member: string;
    tests: readonly { test: string; file: string; source: "measured" | "static" | "reading" | "declared"; grain: "test" | "file"; commit: string; stale: boolean }[];
  }[];
  unreached: readonly string[];
  holds_edges: boolean;
};

const SESSION = "apps/desktop/src/renderer/src/session-pull-request.tsx";
const PANEL = "packages/components/src/compositions/PullRequestPanel/PullRequestPanel.tsx";
const TABS = "packages/components/src/compositions/PullRequestPanel/PullRequestTabs.tsx";
const SETTINGS = "crates/config/settings.toml";
const FORGE = "apps/desktop/src/main/forge.ts";

const ANSWER: Answer = {
  holds_edges: true,
  // Five paths were asked. Nothing was asked of the removed file, the doc, the picture or the rename.
  unreached: [FORGE],
  checks: [
    {
      check: "desktop:test",
      member: "",
      tests: [
        { test: "opens the held pull request from the ledger", file: SESSION, source: "measured", grain: "test", commit: "3fc86fa", stale: false },
        { test: "apps/desktop/src/renderer/src/session-pull-request.test.tsx", file: SESSION, source: "measured", grain: "file", commit: "3fc86fa", stale: false },
      ],
    },
    {
      check: "components:test",
      member: "",
      tests: [
        { test: "PullRequestPanel floats the file list beside the panel", file: PANEL, source: "measured", grain: "test", commit: "ed96c0f", stale: true },
        { test: "PullRequestPanel puts Open in browser on the panel", file: PANEL, source: "measured", grain: "test", commit: "3fc86fa", stale: false },
        { test: "PullRequestTabs draws the commits", file: TABS, source: "static", grain: "test", commit: "3fc86fa", stale: false },
        { test: "packages/components/src/compositions/PullRequestPanel/PullRequestPanel.stories.tsx", file: TABS, source: "static", grain: "file", commit: "3fc86fa", stale: false },
      ],
    },
    {
      check: "rust:test",
      member: "config",
      tests: [
        { test: "settings_default_opens_in_armada", file: SETTINGS, source: "reading", grain: "test", commit: "3fc86fa", stale: false },
        { test: "crates/config/tests/settings.rs", file: SETTINGS, source: "reading", grain: "file", commit: "3fc86fa", stale: false },
      ],
    },
  ],
};

/**
 * The Files list's reading of an answer: the tests under each path, and none under a path the map holds edges for and none
 * reaches. A map that holds no edge says nothing of any path, so it draws no marks rather than the mark for none.
 */
export function reachOf(answer: Answer): FilesReach | undefined {
  if (!answer.holds_edges) return undefined;
  const out: Record<string, { tests: ReachTest[] }> = {};
  for (const path of answer.unreached) out[path] = { tests: [] };
  for (const one of answer.checks)
    for (const edge of one.tests) (out[edge.file] ??= { tests: [] }).tests.push({ check: one.check, test: edge.test, source: edge.source, grain: edge.grain, stale: edge.stale });
  return out;
}

export const testsReaching = async (): Promise<FilesReach | undefined> => reachOf(ANSWER);
