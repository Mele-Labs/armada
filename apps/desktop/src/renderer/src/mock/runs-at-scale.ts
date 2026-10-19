// A repository with workspaces, whose runs Fleet saw pass across them, for the walk `runs-at-scale`: the repository's row in the picker opens them as a layer
// of workspaces. The onboarding slice answers them as `get_run_memory` does. Mock only.

import type { RunFact, RunFactKind } from "@armada/protocol";

export const MY_APP = "/Users/user/code/my-app";

const WEB = "apps/web";
const API = "packages/api";
const UI = "packages/ui";

const fact = (workspace: string, kind: RunFactKind, command: string, port?: number): RunFact => ({
  workspace,
  kind,
  command,
  ...(port === undefined ? {} : { port }),
  succeeded: true,
  last_ran: "2026-10-18T16:20:00Z",
  source: "jobs",
});

export const RUNS: readonly RunFact[] = [
  fact(".", "tests", "pnpm test"),
  fact(".", "lint", "pnpm lint"),
  fact(WEB, "dev_server", "pnpm dev", 5173),
  fact(WEB, "tests", "pnpm test"),
  fact(WEB, "build", "pnpm build"),
  fact(WEB, "lint", "pnpm lint"),
  fact(API, "dev_server", "pnpm dev", 3000),
  fact(API, "tests", "pnpm test"),
  fact(API, "build", "pnpm build"),
  fact(UI, "tests", "pnpm test"),
  fact(UI, "dev_server", "pnpm storybook", 6006),
  fact(UI, "build", "pnpm build"),
];
