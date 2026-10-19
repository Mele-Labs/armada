// A repository with workspaces, whose runs Fleet saw pass across them, all kept on this Mac, for the walk `runs-at-scale`: the repository's row in the
// picker opens them as a layer of workspaces. Mock only.

import { keptRuns, type RunsRead } from "./runs-kept";
import { sessionsStore } from "./sessions/script";
import type { SessionsStore } from "./sessions/script";

export const MY_APP = "/Users/user/code/my-app";

const WEB = "apps/web";
const API = "packages/api";
const UI = "packages/ui";

const JOBS: RunsRead["jobs"] = [
  { id: "root-tests", job: "Tests", command: "pnpm test" },
  { id: "root-lint", job: "Lint", command: "pnpm lint" },
  { id: "web-dev", job: "Dev server", workspace: WEB, command: "pnpm dev", port: 5173 },
  { id: "web-tests", job: "Tests", workspace: WEB, command: "pnpm test" },
  { id: "web-build", job: "Build", workspace: WEB, command: "pnpm build" },
  { id: "web-lint", job: "Lint", workspace: WEB, command: "pnpm lint" },
  { id: "api-dev", job: "Dev server", workspace: API, command: "pnpm dev", port: 3000 },
  { id: "api-tests", job: "Tests", workspace: API, command: "pnpm test" },
  { id: "api-build", job: "Build", workspace: API, command: "pnpm build" },
  { id: "ui-tests", job: "Tests", workspace: UI, command: "pnpm test" },
  { id: "ui-storybook", job: "Storybook", workspace: UI, command: "pnpm storybook", port: 6006 },
  { id: "ui-build", job: "Build", workspace: UI, command: "pnpm build" },
];

export function runsAtScaleStore(): SessionsStore {
  const inner = sessionsStore([], [], new Map(), []);
  const bump = () => inner.edit("none", (one) => one);
  const read = (root: string): RunsRead | undefined => (root === MY_APP ? { repository: "my-app", source: "From the last Job", jobs: JOBS } : undefined);
  return {
    ...inner,
    runs: keptRuns(read, bump, { root: MY_APP, place: { id: "mac" } }),
  };
}
